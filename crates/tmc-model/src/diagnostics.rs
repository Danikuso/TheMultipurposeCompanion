//! Debug-only structured diagnostics. Never pass raw errors or user-indexed object keys.
use serde::Serialize;
use serde_json::Value;
pub use serde_json::json;

/// Types are safe for positional database values, whose field names are unavailable.
pub fn value_types(values: &[Value]) -> Vec<&'static str> {
    values
        .iter()
        .map(|v| match v {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        })
        .collect()
}

/// Correlate generic SQL calls without exposing interpolated literals or identifiers.
pub fn sql_fingerprint(sql: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    sql.hash(&mut hash);
    hash.finish()
}

const SQL_TEMPLATE_LIMIT: usize = 512;

fn sql_template_is_constant(sql: &str) -> bool {
    if sql.contains('\'')
        || sql.contains('"')
        || sql.contains('`')
        || sql.contains("--")
        || sql.contains("/*")
        || sql.contains('[')
    {
        return false;
    }
    // A standalone digit run is a possible interpolated literal; identifiers like t1 are fine.
    let bytes = sql.as_bytes();
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    for (index, byte) in bytes.iter().enumerate() {
        if byte.is_ascii_digit() && (index == 0 || !word(bytes[index - 1])) {
            // `?1` positional markers are placeholders, not literals.
            if index > 0 && bytes[index - 1] == b'?' {
                continue;
            }
            return false;
        }
    }
    true
}

/// SQL text as a template (<= 512 bytes) only when it has no quoted or numeric literals
/// that could have been interpolated; otherwise callers log just the fingerprint.
pub fn sql_template(sql: &str) -> Option<&str> {
    if !sql_template_is_constant(sql) {
        return None;
    }
    let mut end = sql.len().min(SQL_TEMPLATE_LIMIT);
    while !sql.is_char_boundary(end) {
        end -= 1;
    }
    Some(&sql[..end])
}

/// Logical payload size without serializing a private SQL value into a log.
pub fn value_bytes(value: &Value) -> usize {
    match value {
        Value::Null | Value::Bool(_) => 1,
        Value::Number(_) => 8,
        Value::String(text) => text.len(),
        Value::Array(values) => values.iter().map(value_bytes).sum(),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| key.len() + value_bytes(value))
            .sum(),
    }
}

/// Decode embedded JSON before context redaction, never treat it as opaque public text.
pub fn decoded_json(text: &str) -> Value {
    serde_json::from_str(text)
        .unwrap_or_else(|_| json!({"invalid_json": true, "bytes": text.len()}))
}

/// Only game-defined basenames can be used for source correlation.
pub fn source_file(path: &std::path::Path) -> &str {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if safe_source_name(name) {
        name
    } else {
        "local_source"
    }
}

fn safe_source_name(name: &str) -> bool {
    if matches!(
        name,
        "local_source"
            | "Status.json"
            | "Cargo.json"
            | "NavRoute.json"
            | "Market.json"
            | "Outfitting.json"
            | "Shipyard.json"
            | "ModulesInfo.json"
            | "Backpack.json"
            | "ShipLocker.json"
            | "FCMaterials.json"
    ) {
        return true;
    }
    let Some(body) = name
        .strip_prefix("Journal.")
        .and_then(|s| s.strip_suffix(".log"))
    else {
        return false;
    };
    let Some((date, part)) = body.rsplit_once('.') else {
        return false;
    };
    part.len() == 2
        && part.bytes().all(|b| b.is_ascii_digit())
        && match date.len() {
            12 => date.bytes().all(|b| b.is_ascii_digit()),
            17 => date.bytes().enumerate().all(|(i, b)| match i {
                4 | 7 => b == b'-',
                10 => b == b'T',
                _ => b.is_ascii_digit(),
            }),
            _ => false,
        }
}

/// ErrorKind is stable metadata; the Display text may contain player paths.
pub fn io_class(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        std::io::ErrorKind::InvalidData => "invalid_data",
        std::io::ErrorKind::UnexpectedEof => "unexpected_eof",
        std::io::ErrorKind::BrokenPipe => "broken_pipe",
        _ => "other_io",
    }
}

/// The schema that gives otherwise ambiguous field names their meaning.
/// Context names describe schemas, not arbitrary log messages.
#[derive(Clone, Copy, Debug)]
pub enum DiagnosticContext<'a> {
    Journal { event: &'a str },
    State,
    Topic { name: &'a str },
    NetQuery,
    NetOutcome,
    Action { name: &'a str },
    Http,
    Auth,
    Ui,
    Sql,
}

impl DiagnosticContext<'_> {
    fn schema(self) -> &'static str {
        match self {
            Self::Journal { event } => registered(event, EVENTS),
            Self::Topic { name } => registered(name, TOPICS),
            Self::Action { name } => registered(name, ACTIONS),
            _ => self.label(),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Journal { .. } => "journal",
            Self::State => "state",
            Self::Topic { .. } => "topic",
            Self::NetQuery => "net_query",
            Self::NetOutcome => "net_outcome",
            Self::Action { .. } => "action",
            Self::Http => "http",
            Self::Auth => "auth",
            Self::Ui => "ui",
            Self::Sql => "sql",
        }
    }
}

const REDACTED: &str = "[redacted]";
const TOPICS: &[&str] = &[
    "core.cmdr",
    "core.ship",
    "core.location",
    "core.status",
    "core.route",
    "core.session",
    "core.game",
    "core.clock",
    "core.modules",
    "core.theme",
    "dashboard.alerts",
    "navigation.route",
    "navigation.jump",
    "navigation.plan",
    "exploration.system",
    "exploration.unsold",
    "exobiology.body",
    "exobiology.sampling",
    "exobiology.unsold",
    "combat.target",
    "combat.ship",
    "combat.session",
    "combat.conflict_zone",
    "combat.ax",
    "mining.prospector",
    "mining.session",
    "mining.sell",
    "trade.market",
    "trade.cargo",
    "trade.route",
    "trade.session",
    "trade.search",
    "trade.commodities",
    "missions.active",
    "missions.massacre",
    "missions.alerts",
    "missions.rewards",
    "missions.passengers",
    "missions.tourism",
    "missions.community_goals",
    "engineering.catalogue",
    "engineering.inventory",
    "engineering.locker",
    "engineering.session",
    "engineering.engineers",
    "engineering.ship",
    "engineering.flow",
    "engineering.goals",
    "engineering.sources",
    "engineering.alerts",
    "carrier.overview",
    "carrier.jump",
    "carrier.finances",
    "colonisation.depot",
    "powerplay.status",
    "powerplay.session",
    "bgs.system",
    "bgs.tick",
    "accounts.status",
    "crime.status",
    "ship.outfitting_catalog",
    "ship.outfitting_search",
];
const MODULES: &[&str] = &[
    "dashboard",
    "ship",
    "navigation",
    "exploration",
    "exobiology",
    "combat",
    "mining",
    "trade",
    "missions",
    "engineering",
    "carrier",
    "colonisation",
    "powerplay",
    "bgs",
    "accounts",
    "crime",
];
const ACTIONS: &[&str] = &[
    "clipboard.copy-next-system",
    "crime.copy_system",
    "ship.outfitting_search",
    "ship.outfitting_clear",
    "navigation.plan",
    "navigation.recalculate",
    "navigation.accept-plan",
    "navigation.clear-plan",
    "combat.reset_session",
    "mining.reset_session",
    "trade.reset_session",
    "trade.route_request",
    "trade.route_clear",
    "trade.search",
    "missions.to_route",
    "missions.dismiss_alert",
    "missions.reset_session",
    "engineering.select_module",
    "engineering.preview",
    "engineering.close_flow",
    "engineering.pin_goal",
    "engineering.unpin_goal",
    "engineering.move_goal",
    "engineering.select_engineer",
    "engineering.dismiss_alert",
    "engineering.reset_session",
    "accounts.sync_now",
    "accounts.set_settings",
    "frontier.login",
    "frontier.logout",
    "frontier.fleet",
    "voice.intent",
    "voice.voiceattack",
];
const EVENTS: &[&str] = &[
    "Fileheader",
    "Commander",
    "LoadGame",
    "Rank",
    "Progress",
    "Location",
    "FSDJump",
    "CarrierJump",
    "Docked",
    "Undocked",
    "SupercruiseEntry",
    "SupercruiseExit",
    "ApproachBody",
    "LeaveBody",
    "Touchdown",
    "Liftoff",
    "Embark",
    "Disembark",
    "StartJump",
    "FSDTarget",
    "NavRoute",
    "NavRouteClear",
    "Loadout",
    "Cargo",
    "HullDamage",
    "UnderAttack",
    "Interdicted",
    "FuelScoop",
    "RefuelAll",
    "RefuelPartial",
    "ShipyardSwap",
    "Died",
    "Resurrect",
    "Shutdown",
    "Market",
    "Outfitting",
    "Shipyard",
    "ModuleInfo",
    "Backpack",
    "ShipLocker",
    "FCMaterials",
    "Scan",
    "FSSDiscoveryScan",
    "FSSAllBodiesFound",
    "FSSBodySignals",
    "SAAScanComplete",
    "SAASignalsFound",
    "ScanOrganic",
    "SellOrganicData",
    "SellExplorationData",
    "MultiSellExplorationData",
    "CodexEntry",
    "JetConeBoost",
    "ShipTargeted",
    "ShipTargetted",
    "Bounty",
    "FactionKillBond",
    "CapShipBond",
    "RedeemVoucher",
    "ShieldState",
    "HeatWarning",
    "HeatDamage",
    "EscapeInterdiction",
    "PVPKill",
    "FighterDestroyed",
    "Promotion",
    "ProspectedAsteroid",
    "AsteroidCracked",
    "MiningRefined",
    "LaunchDrone",
    "EjectCargo",
    "MarketSell",
    "RepairAll",
    "Repair",
    "ScanBody",
    "Materials",
    "MaterialCollected",
    "MaterialDiscarded",
    "MaterialTrade",
    "Synthesis",
    "EngineerProgress",
    "EngineerCraft",
    "EngineerContribution",
    "TechnologyBroker",
    "BackpackChange",
    "CommitCrime",
    "CrimeVictim",
    "PayFines",
    "PayBounties",
];
/// Indica si un tema está en el registro de diagnóstico (si no, su nombre se redacta).
pub fn is_registered_topic(name: &str) -> bool {
    registered(name, TOPICS) != REDACTED
}

/// Indica si una acción está en el registro de diagnóstico (si no, su nombre se redacta).
pub fn is_registered_action(name: &str) -> bool {
    registered(name, ACTIONS) != REDACTED
}

fn registered(value: &str, registry: &[&'static str]) -> &'static str {
    registry
        .iter()
        .copied()
        .find(|known| *known == value)
        .unwrap_or(REDACTED)
}

/// Redact scalar data while retaining object/array shape and useful public game facts.
/// Object keys must be schema names, never names supplied by the player.
pub fn redact_diagnostic(value: &Value, context: DiagnosticContext<'_>) -> Value {
    redact(value, context, "", "", "", false)
}

fn normalized(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn secret_field(key: &str) -> bool {
    [
        "token",
        "password",
        "passwd",
        "secret",
        "apikey",
        "authorization",
        "cookie",
        "credential",
        "clientid",
        "pairing",
        "qrsvg",
        "chat",
        "message",
        "content",
        "error",
        "exception",
        "backtrace",
        "filepath",
        "directory",
        "journalpath",
        "dbpath",
        "url",
        "uri",
    ]
    .iter()
    .any(|part| key.contains(part))
        || matches!(
            key,
            "fid"
                | "commanderid"
                | "commandername"
                | "cmdrname"
                | "shipname"
                | "shipident"
                | "shipidentifier"
                | "path"
                | "key"
                | "keys"
                | "headers"
                | "text"
                | "clipboard"
                | "auth"
                | "uri"
                | "requestbody"
                | "responsebody"
        )
}

fn public_text(key: &str, parent: &str, path: &str, context: DiagnosticContext<'_>) -> bool {
    if matches!(context, DiagnosticContext::Auth) {
        return false;
    }
    if key == "body" {
        return matches!(context, DiagnosticContext::State) && parent == "location"
            || matches!(
                context,
                DiagnosticContext::Topic {
                    name: "core.location"
                }
            );
    }
    if key == "name" {
        return match context {
            DiagnosticContext::Journal {
                event: "Scan" | "ScanBody",
            } => parent.is_empty(),
            DiagnosticContext::Topic {
                name: "navigation.route",
            } => matches!(
                path,
                "current.name"
                    | "next.name"
                    | "following.name"
                    | "remote.data.name"
                    | "current.remote.data.name"
                    | "next.remote.data.name"
                    | "following.remote.data.name"
            ),
            DiagnosticContext::NetQuery => parent == "system",
            DiagnosticContext::NetOutcome => matches!(parent, "system" | "autocomplete" | "hops"),
            _ => false,
        };
    }
    if matches!(context, DiagnosticContext::NetQuery) {
        if key == "prefix" && parent == "autocomplete" {
            return true;
        }
        if matches!(key, "from" | "to") && matches!(parent, "neutronroute" | "galaxyroute") {
            return true;
        }
    }
    matches!(
        key,
        "system"
            | "starsystem"
            | "systemname"
            | "nextsystem"
            | "bodyname"
            | "stationname"
            | "station"
            | "faction"
            | "factionname"
            | "controllingfaction"
            | "allegiance"
            | "government"
            | "economy"
            | "secondeconomy"
            | "security"
            | "starclass"
            | "starclasses"
            | "startype"
            | "planetclass"
            | "atmosphere"
            | "atmospheretype"
            | "terraformstate"
            | "volcanism"
            | "luminosity"
            | "scantype"
            | "commodity"
            | "commodityname"
            | "material"
            | "materialname"
            | "genus"
            | "species"
            | "variant"
            | "shiptype"
            | "shiptypelocalised"
            | "shipmodel"
            | "bodytype"
            | "stationtype"
            | "primary_star"
            | "primarystar"
            | "powers"
    )
}

fn route_template(text: &str) -> bool {
    matches!(
        text,
        "/ws"
            | "/api/health"
            | "/api/config"
            | "/api/history/events"
            | "/api/actions/{name}"
            | "/api/navigation/autocomplete"
            | "/api/engineering/goals"
            | "/api/engineering/locker"
            | "/api/engineering/catalogue"
            | "/api/engineering/engineers"
            | "/api/engineering/flow"
            | "/api/pairing"
            | "<asset>"
            | "<unmatched>"
    )
}

fn schema_label(key: &str, path: &str, text: &str, context: DiagnosticContext<'_>) -> bool {
    if matches!(context, DiagnosticContext::Auth) {
        return false;
    }
    let engineering_label = match context {
        DiagnosticContext::Topic {
            name: "engineering.flow",
        } => match path {
            "moduletype" | "request.moduletype" => matches!(text, "engine" | "fsd"),
            "request.blueprint" => matches!(text, "engine_dirty" | "fsd_longrange"),
            "request.experimental" => text == "special_engine_overloaded",
            "status" => matches!(text, "idle" | "ok" | "no_data"),
            "lines.state" => matches!(text, "ok" | "trade" | "missing"),
            "engineer.nearestunlocked.status" | "engineer.nearestany.status" => matches!(
                text,
                "Unknown" | "Known" | "Invited" | "Acquainted" | "Unlocked" | "Barred"
            ),
            _ => false,
        },
        DiagnosticContext::Topic {
            name: "engineering.goals",
        } => path == "status" && matches!(text, "ok" | "too_many"),
        DiagnosticContext::Topic {
            name: "engineering.engineers",
        } => {
            path == "engineers.status"
                && matches!(
                    text,
                    "Unknown" | "Known" | "Invited" | "Acquainted" | "Unlocked" | "Barred"
                )
        }
        DiagnosticContext::Topic {
            name: "engineering.session",
        } => match path {
            "recent.origin" => matches!(
                text,
                "Collected"
                    | "Scan"
                    | "Mining"
                    | "Trade"
                    | "Synthesis"
                    | "Engineering"
                    | "Mission"
                    | "Discarded"
            ),
            "recent.category" => matches!(text, "Raw" | "Manufactured" | "Encoded"),
            _ => false,
        },
        DiagnosticContext::Topic {
            name: "engineering.sources",
        } => {
            matches!(
                path,
                "traders.raw"
                    | "traders.manufactured"
                    | "traders.encoded"
                    | "brokers.human"
                    | "brokers.guardian"
            ) && matches!(text, "Loading" | "NoData" | "Disabled")
        }
        _ => false,
    };
    if engineering_label {
        return true;
    }
    if matches!(context, DiagnosticContext::NetOutcome)
        && ((matches!(path, "" | "outcome") && matches!(text, "NoData" | "Disabled"))
            || (matches!(path, "failed" | "outcome.failed")
                && matches!(
                    text,
                    "Timeout"
                        | "Malformed"
                        | "Down"
                        | "RateLimited"
                        | "UnknownOrigin"
                        | "UnknownDestination"
                )))
    {
        return true;
    }
    // Tipos cerrados de los servicios cercanos de F4 (comerciante / bróker).
    if matches!(context, DiagnosticContext::NetQuery)
        && match key {
            "materialtrader" => matches!(text, "Raw" | "Manufactured" | "Encoded"),
            "technologybroker" => matches!(text, "Human" | "Guardian"),
            _ => false,
        }
    {
        return true;
    }
    // Etiquetas de `NetOutcome::diag_summary` (resumen de `net.query.finished`).
    if matches!(context, DiagnosticContext::NetOutcome)
        && path == "outcome"
        && matches!(
            text,
            "system"
                | "route"
                | "autocomplete"
                | "body_bio"
                | "markets"
                | "stations"
                | "trade_route"
                | "exports"
                | "coords"
                | "bgs_tick"
                | "outfitting_stations"
                | "no_data"
                | "disabled"
                | "failed"
        )
    {
        return true;
    }
    if matches!(
        context,
        DiagnosticContext::Topic {
            name: "navigation.route"
        }
    ) && matches!(
        path,
        "remote.status"
            | "current.remote.status"
            | "next.remote.status"
            | "following.remote.status"
    ) && matches!(text, "ok" | "pending" | "no-data" | "disabled" | "failed")
    {
        return true;
    }
    match key {
        "file" => safe_source_name(text),
        "sqltemplate" => {
            matches!(context, DiagnosticContext::Sql)
                && text.len() <= SQL_TEMPLATE_LIMIT
                && sql_template_is_constant(text)
        }
        "failureclass" => matches!(
            text,
            "not_found"
                | "permission_denied"
                | "invalid_data"
                | "unexpected_eof"
                | "broken_pipe"
                | "other_io"
                | "sqlite"
                | "io"
                | "poisoned"
                | "integer_range"
                | "unsupported"
                | "json"
                | "missing_field"
                | "empty"
                | "typed_json"
        ),
        "parametertypes" | "celltypes" => matches!(
            text,
            "null" | "boolean" | "number" | "string" | "array" | "object"
        ),
        "tab" | "slot" | "panel" | "view" | "relevance" | "state" | "kind" | "layout" | "level"
        | "context" | "manualcontext" => {
            matches!(context, DiagnosticContext::Ui) && ui_value(key, text)
        }
        "reason" => matches!(
            text,
            "enabled"
                | "stale"
                | "game_not_running"
                | "replay"
                | "no_center"
                | "legacy"
                | "offline"
                | "inactive"
        ),
        "language" => matches!(text, "es" | "en" | "fr" | "de" | "pt"),
        "galaxy" => matches!(text, "Live" | "Legacy" | "Unknown"),
        "mode" => matches!(
            text,
            "Unknown"
                | "Docked"
                | "Landed"
                | "NormalSpace"
                | "Supercruise"
                | "Hyperspace"
                | "OnFoot"
                | "InSrv"
                | "InFighter"
                | "InTaxi"
                | "Multicrew"
        ),
        "method" => {
            matches!(context, DiagnosticContext::Http)
                && matches!(
                    text,
                    "GET" | "PUT" | "POST" | "DELETE" | "OPTIONS" | "HEAD" | "PATCH"
                )
        }
        "route" => matches!(context, DiagnosticContext::Http) && route_template(text),
        "topic" => registered(text, TOPICS) != REDACTED,
        "module" => registered(text, MODULES) != REDACTED,
        "action" => registered(text, ACTIONS) != REDACTED,
        "event" => registered(text, EVENTS) != REDACTED,
        "source" | "provider" => {
            matches!(
                text,
                "Spansh" | "Edsm" | "Canonn" | "Ardent" | "Cache" | "Edcd"
            )
        }
        "endpoint" => {
            matches!(
                context,
                DiagnosticContext::NetQuery | DiagnosticContext::NetOutcome
            ) && matches!(
                text,
                "spansh.system"
                    | "spansh.route.job"
                    | "spansh.route.results"
                    | "spansh.autocomplete"
                    | "edcd.ticks"
                    | "edsm.system"
                    | "canonn.codex_ref"
                    | "canonn.system_poi"
                    | "ardent.nearby.imports"
                    | "ardent.nearest.service"
                    | "ardent.nearest.material_trader"
                    | "ardent.nearest.technology_broker"
                    | "spansh.stations.material_trader"
                    | "spansh.stations.technology_broker"
                    | "spansh.stations.outfitting"
                    | "spansh.trade.job"
                    | "ardent.nearby.exports"
                    | "ardent.system.coords"
                    | "edsm.coords"
                    | "unknown"
            )
        }
        _ => false,
    }
}
/// Disposiciones de la UI (`ui/src/shell/layouts.ts`); `none` cuando no hay ninguna activa.
const UI_LAYOUTS: &[&str] = &[
    "carrier",
    "colonisation",
    "combat",
    "engineering",
    "exploration",
    "mining",
    "missions",
    "none",
    "powerplay",
    "ship",
    "trade",
];
/// Ranuras de las disposiciones (`ui/src/shell/layouts.ts`).
const UI_SLOTS: &[&str] = &[
    "bgs-system",
    "carrier-finances",
    "carrier-jump",
    "carrier-overview",
    "colonisation-progress",
    "colonisation-supply",
    "powerplay-status",
    "materials",
    "engineers",
    "eng-alerts",
    "eng-ship",
    "eng-flow",
    "shopping",
    "trades",
    "sources",
    "route",
    "context-bodies",
    "context-exobiology",
    "planner",
    "jump",
    "ship-jump",
    "ship-cargo",
    "ship-detail",
    "target",
    "hunting",
    "ship",
    "ax",
    "cz",
    "prospector",
    "mining-session",
    "sell",
    "market",
    "cargo",
    "trade-session",
    "trade-route",
    "search",
    "missions",
    "mission-alerts",
    "massacre",
    "passengers",
    "tourism",
    "rewards",
    "community-goals",
];
/// Claves de título de los paneles (`titleKey` en `ui/src`); un test de la UI las mantiene sincronizadas.
const UI_PANELS: &[&str] = &[
    "panel-accounts",
    "panel-bgs-system",
    "panel-carrier-finances",
    "panel-carrier-jump",
    "panel-carrier-overview",
    "panel-colonisation-depot",
    "panel-colonisation-supply",
    "panel-engineering-materials",
    "panel-engineering-engineers",
    "panel-engineering-ship",
    "panel-engineering-flow",
    "panel-engineering-shopping",
    "panel-engineering-trades",
    "panel-engineering-sources",
    "panel-engineering-alerts",
    "combat-target",
    "crime-title",
    "nav-settings",
    "panel-alerts",
    "panel-cmdr",
    "panel-combat-ax",
    "panel-combat-hunting",
    "panel-combat-ship",
    "panel-combat-zone",
    "panel-exobiology",
    "panel-jump",
    "panel-location",
    "panel-mining-prospector",
    "panel-mining-sell",
    "panel-mining-session",
    "panel-missions-active",
    "panel-missions-alerts",
    "panel-missions-community-goals",
    "panel-missions-massacre",
    "panel-missions-passengers",
    "panel-missions-rewards",
    "panel-missions-tourism",
    "panel-planner",
    "panel-powerplay-status",
    "panel-route",
    "panel-route-strip",
    "panel-session",
    "panel-ship",
    "panel-status",
    "panel-system-bodies",
    "panel-trade-cargo",
    "panel-trade-market",
    "panel-trade-route",
    "panel-trade-search",
    "panel-trade-session",
];
const UI_RELEVANCE: &[&str] = &["hidden", "normal", "active", "urgent"];
const UI_STATES: &[&str] = &[
    "pending",
    "ok",
    "no-data",
    "disabled",
    "failed",
    "reconnect",
    "open",
    "closed",
];
const UI_KINDS: &[&str] = &[
    "error",
    "rejection",
    "socket",
    "parse",
    "handler",
    "pairing-get",
    "config-get",
    "config-put",
    "plan",
    "nav-route",
    "current",
    "neutron",
    "white_dwarf",
    "none",
];
const UI_LEVELS: &[&str] = &["info", "warning", "critical"];
const UI_CONTEXTS: &[&str] = &["exploration", "exobiology", "none"];

/// Valores de interfaz admitidos: conjuntos cerrados por clave, nunca texto libre.
fn ui_value(key: &str, text: &str) -> bool {
    let set = |list: &[&str]| list.contains(&text);
    match key {
        "layout" => set(UI_LAYOUTS),
        "tab" => {
            set(MODULES)
                || text == "settings"
                || text
                    .strip_prefix("layout:")
                    .is_some_and(|id| id != "none" && UI_LAYOUTS.contains(&id))
        }
        "view" => set(MODULES) || text == "settings",
        "slot" => set(UI_SLOTS),
        "panel" => set(UI_PANELS),
        "relevance" => set(UI_RELEVANCE),
        "state" => set(UI_STATES),
        "kind" => set(UI_KINDS),
        "level" => set(UI_LEVELS),
        "context" | "manualcontext" => set(UI_CONTEXTS),
        _ => false,
    }
}

fn unsafe_text(value: &str) -> bool {
    // Public text is still not an escape hatch for URLs, query strings or local paths.
    value.contains("://")
        || value.contains('\\')
        || value.contains('/')
        || value.contains('?')
        || value.contains('@')
        || value.contains(":\\")
}

fn redact(
    value: &Value,
    context: DiagnosticContext<'_>,
    key: &str,
    parent: &str,
    path: &str,
    private: bool,
) -> Value {
    let private = private
        || secret_field(key)
        || matches!(
            key,
            "shipid" | "otp" | "pin" | "pincode" | "authcode" | "passcode"
        )
        || (matches!(context, DiagnosticContext::Auth) && key == "code")
        || (parent == "ship" && matches!(key, "ident" | "identifier"))
        || (matches!(parent, "commander" | "commanders" | "cmdr" | "pilot")
            && matches!(key, "id" | "name"))
        || (matches!(key, "commander" | "commanders" | "cmdr")
            && !value.is_object()
            && !value.is_array())
        || (key == "body" && !public_text(key, parent, path, context));
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(child_key, child)| {
                    let normalized_key = normalized(child_key);
                    (
                        child_key.clone(),
                        redact(
                            child,
                            context,
                            &normalized_key,
                            key,
                            &if path.is_empty() {
                                normalized_key.clone()
                            } else {
                                format!("{path}.{normalized_key}")
                            },
                            private,
                        ),
                    )
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact(item, context, key, parent, path, private))
                .collect(),
        ),
        Value::String(text) => {
            if !private
                && (schema_label(key, path, text, context)
                    || (public_text(key, parent, path, context) && !unsafe_text(text)))
            {
                Value::String(text.clone())
            } else {
                Value::String(REDACTED.into())
            }
        }
        Value::Number(number) if private => {
            if number.is_f64() {
                serde_json::json!(0.0)
            } else {
                serde_json::json!(0)
            }
        }
        Value::Bool(_) if private => Value::Bool(false),
        other => other.clone(),
    }
}

fn serialize_diagnostic<T: Serialize + ?Sized>(
    payload: &T,
    context: DiagnosticContext<'_>,
) -> Value {
    match serde_json::to_value(payload) {
        Ok(value) => redact_diagnostic(&value, context),
        Err(_) => serde_json::json!({"serialization_failed": true}),
    }
}

/// Emit sanitized TRACE JSON. Sequence/session/time belong exclusively to the writer.
pub fn emit<T: Serialize + ?Sized>(
    context: DiagnosticContext<'_>,
    code: &'static str,
    payload: &T,
) {
    let payload = serialize_diagnostic(payload, context);
    tracing::trace!(target: "tmc::diagnostics", context = context.label(), schema = context.schema(), code, payload = %payload);
}
/// Captura local de registros de diagnóstico para pruebas (sin E/S ni dependencias).
#[cfg(debug_assertions)]
#[doc(hidden)]
pub mod capture {
    use std::fmt::Debug;
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record as SpanRecord};
    use tracing::{Event, Metadata, Subscriber};

    #[derive(Clone, Debug, Default)]
    pub struct Record {
        pub context: String,
        pub schema: String,
        pub code: String,
        pub payload: String,
    }

    impl Visit for Record {
        fn record_str(&mut self, field: &Field, value: &str) {
            match field.name() {
                "context" => self.context = value.to_owned(),
                "schema" => self.schema = value.to_owned(),
                "code" => self.code = value.to_owned(),
                _ => {}
            }
        }
        fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
            if field.name() == "payload" {
                self.payload = format!("{value:?}");
            }
        }
    }

    thread_local! {
        /// Búfer de la captura activa en este hilo; fuera de una captura los eventos se ignoran.
        static ACTIVE: std::cell::RefCell<Option<Vec<Record>>> = const { std::cell::RefCell::new(None) };
    }

    /// Recolector global instalado una sola vez: su interés es estático, así que el caché de
    /// callsites no depende de qué pruebas corran en paralelo.
    struct Collector;

    impl Subscriber for Collector {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.target() == "tmc::diagnostics"
        }
        fn register_callsite(
            &self,
            metadata: &'static Metadata<'static>,
        ) -> tracing::subscriber::Interest {
            if metadata.target() == "tmc::diagnostics" {
                tracing::subscriber::Interest::always()
            } else {
                tracing::subscriber::Interest::never()
            }
        }
        fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
            Some(tracing::metadata::LevelFilter::TRACE)
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &SpanRecord<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            let mut record = Record::default();
            event.record(&mut record);
            ACTIVE.with(|active| {
                if let Some(records) = active.borrow_mut().as_mut() {
                    records.push(record);
                }
            });
        }
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }

    /// Ejecuta `f` y devuelve los registros de diagnóstico emitidos en este hilo.
    pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<Record>) {
        static GLOBAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let global =
            *GLOBAL.get_or_init(|| tracing::subscriber::set_global_default(Collector).is_ok());
        let previous = ACTIVE.with(|a| a.replace(Some(Vec::new())));
        let result = if global {
            f()
        } else {
            // Otro suscriptor global ocupa el sitio: recolector local como último recurso.
            tracing::subscriber::with_default(Collector, || {
                tracing::callsite::rebuild_interest_cache();
                f()
            })
        };
        let records = ACTIVE.with(|a| a.replace(previous)).unwrap_or_default();
        (result, records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sql_template_rejects_comments_and_bracket_identifiers() {
        assert!(sql_template("SELECT a FROM t WHERE b = ?1").is_some());
        assert!(sql_template("SELECT a /* x */ FROM t").is_none());
        assert!(sql_template("SELECT [secret col] FROM t").is_none());
        assert!(sql_template("SELECT a FROM t -- x").is_none());
    }

    #[test]
    fn nested_private_containers_override_public_fields() {
        let raw = json!({"StarSystem":"Sol", "credits":42, "ok":true,
            "Commander":{"Name":"CANARY-Ω", "credits":900,"id":42},
            "nested":[{"FID":"CANARY-FID","ship_name":"CANARY-SHIP",
                "ship_ident":"CANARY-ID","chat":{"BodyName":"CANARY-CHAT"}}]});
        let safe = redact_diagnostic(&raw, DiagnosticContext::State);
        assert_eq!(safe["StarSystem"], "Sol");
        assert_eq!(safe["credits"], 42);
        assert_eq!(safe["ok"], true);
        assert_eq!(safe["Commander"]["id"], 0);
        assert_eq!(safe["Commander"]["credits"], 900);
        assert_eq!(safe["nested"].as_array().unwrap().len(), 1);
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn scan_body_name_is_public_but_commander_name_is_not() {
        for event in ["Scan", "ScanBody"] {
            let safe = redact_diagnostic(
                &json!({"Name":"Sol A 1","BodyName":"Sol A 1",
                "StarSystem":"Sol", "StationName":"Galileo", "MassEM":1.3}),
                DiagnosticContext::Journal { event },
            );
            assert_eq!(safe["Name"], "Sol A 1");
            assert_eq!(safe["BodyName"], "Sol A 1");
            assert_eq!(safe["StationName"], "Galileo");
        }
        let safe = redact_diagnostic(
            &json!({"Name":"CANARY", "BodyName":"Sol A 1"}),
            DiagnosticContext::Journal { event: "Commander" },
        );
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn unknown_context_and_free_text_are_safe_without_losing_shape() {
        let raw = json!({"Name":"CANARY", "error":"CANARY", "message":"CANARY",
            "content":"CANARY", "value":"CANARY", "path":"C:/Users/CANARY",
            "url":"https://host/CANARY?token=CANARY",
            "headers":{"Authorization":"CANARY","cookie":"CANARY"},
            "nested":[null, false, 7, {"unknown":"CANARY", "system":"Sol"}]});
        for context in [
            DiagnosticContext::Ui,
            DiagnosticContext::Auth,
            DiagnosticContext::Journal { event: "Unknown" },
        ] {
            let safe = redact_diagnostic(&raw, context);
            assert!(!safe.to_string().contains("CANARY"));
            assert_eq!(safe["nested"][0], Value::Null);
            assert_eq!(safe["nested"][1], false);
            assert_eq!(safe["nested"][2], 7);
            assert_eq!(
                safe["nested"][3]["system"],
                if matches!(context, DiagnosticContext::Auth) {
                    "[redacted]"
                } else {
                    "Sol"
                }
            );
        }
    }

    #[test]
    fn secret_aliases_and_private_numeric_ids_are_redacted() {
        let safe = redact_diagnostic(
            &json!({"access_token":"CANARY",
            "apiKey":"CANARY", "password":"CANARY", "lan_token":"CANARY",
            "client_secret":"CANARY", "commander_id":123, "ShipIdent":"CANARY",
            "ShipName":"CANARY", "api_keys":["CANARY"], "system_address":123}),
            DiagnosticContext::Topic { name: "core.state" },
        );
        assert!(!safe.to_string().contains("CANARY"));
        assert_eq!(safe["commander_id"], 0);
        assert_eq!(safe["system_address"], 123);
    }

    #[test]
    fn actual_state_retains_counters_and_public_location() {
        let mut state = crate::state::GameState::default();
        state.cmdr = Some(crate::state::CommanderState {
            name: "CANARY-CMDR".into(),
            fid: "CANARY-FID".into(),
            credits: Some(900),
            ..Default::default()
        });
        state.cmdr.as_mut().unwrap().ranks.combat = Some(7);
        state.location.system = "Sol".into();
        state.location.body = Some("Earth".into());
        let safe = redact_diagnostic(
            &serde_json::to_value(state).unwrap(),
            DiagnosticContext::State,
        );
        assert_eq!(safe["cmdr"]["credits"], 900);
        assert_eq!(safe["cmdr"]["ranks"]["combat"], 7);
        assert_eq!(safe["location"]["body"], "Earth");
        assert!(!safe.to_string().contains("CANARY"));
        let topic = redact_diagnostic(
            &json!({"body":"Earth"}),
            DiagnosticContext::Topic {
                name: "core.location",
            },
        );
        assert_eq!(topic["body"], "Earth");
    }

    #[test]
    fn actual_network_types_retain_public_system_names() {
        let query = crate::net::NetQuery::System {
            id64: 42,
            name: "Sol".into(),
        };
        let safe = redact_diagnostic(
            &serde_json::to_value(query).unwrap(),
            DiagnosticContext::NetQuery,
        );
        assert_eq!(safe["System"]["name"], "Sol");
        let system: crate::net::RemoteSystem = serde_json::from_value(json!({
            "id64":42,"name":"Sol","powers":[]
        }))
        .unwrap();
        let outcome = crate::net::NetOutcome::System(system);
        let safe = redact_diagnostic(
            &serde_json::to_value(outcome).unwrap(),
            DiagnosticContext::NetOutcome,
        );
        assert_eq!(safe["System"]["name"], "Sol");
        let query = crate::net::NetQuery::Autocomplete {
            prefix: "So".into(),
        };
        let safe = redact_diagnostic(
            &serde_json::to_value(query).unwrap(),
            DiagnosticContext::NetQuery,
        );
        assert_eq!(safe["Autocomplete"]["prefix"], "So");
        let outcome = crate::net::NetOutcome::Autocomplete(vec![crate::net::SystemSuggestion {
            name: "Sol".into(),
            id64: Some(42),
        }]);
        let safe = redact_diagnostic(
            &serde_json::to_value(outcome).unwrap(),
            DiagnosticContext::NetOutcome,
        );
        assert_eq!(safe["Autocomplete"][0]["name"], "Sol");
        let route = redact_diagnostic(
            &json!({"Route":{"hops":[{"name":"Sol","distance_ly":3}]}}),
            DiagnosticContext::NetOutcome,
        );
        assert_eq!(route["Route"]["hops"][0]["name"], "Sol");
    }

    #[test]
    fn credential_containers_and_unknown_journal_free_text_never_leak() {
        let raw = json!({"auth":{"code":"CANARY","system":"CANARY"},
            "credentials":{"station":"CANARY"}, "reason":"CANARY error privado"});
        for context in [
            DiagnosticContext::Auth,
            DiagnosticContext::Journal { event: "Unknown" },
        ] {
            assert!(
                !redact_diagnostic(&raw, context)
                    .to_string()
                    .contains("CANARY")
            );
        }
        let safe = redact_diagnostic(
            &json!({"route":"/api/config","url":"/api/config?token=CANARY"}),
            DiagnosticContext::Http,
        );
        assert_eq!(safe["route"], "/api/config");
        assert!(!safe.to_string().contains("CANARY"));
    }
    #[test]
    fn auth_numeric_credentials_are_private() {
        let safe = redact_diagnostic(
            &json!({"code":123456,"pin":1234,"otp":98765,
            "pin_code":4444,"auth_code":5555,"attempts":2}),
            DiagnosticContext::Auth,
        );
        for field in ["code", "pin", "otp", "pin_code", "auth_code"] {
            assert_eq!(safe[field], 0);
        }
        assert_eq!(safe["attempts"], 2);
    }

    #[test]
    fn known_navigation_route_paths_retain_only_public_names() {
        let safe = redact_diagnostic(
            &json!({
                "current":{"name":"Sol"},"next":{"name":"Alpha Centauri"},
                "following":[{"name":"Sirius"}],"remote":{"data":{"name":"Sol"}},
                "commander":{"name":"CANARY"},"unknown":{"name":"CANARY"}
            }),
            DiagnosticContext::Topic {
                name: "navigation.route",
            },
        );
        assert_eq!(safe["current"]["name"], "Sol");
        assert_eq!(safe["next"]["name"], "Alpha Centauri");
        assert_eq!(safe["following"][0]["name"], "Sirius");
        assert_eq!(safe["remote"]["data"]["name"], "Sol");
        assert!(!safe.to_string().contains("CANARY"));
    }
    #[test]
    fn trade_topics_and_actions_are_registered_and_unknown_ones_redacted() {
        for topic in [
            "trade.market",
            "trade.cargo",
            "trade.route",
            "trade.session",
            "trade.search",
            "trade.commodities",
        ] {
            assert_eq!(DiagnosticContext::Topic { name: topic }.schema(), topic);
        }
        for action in [
            "trade.reset_session",
            "trade.route_request",
            "trade.route_clear",
            "trade.search",
        ] {
            assert_eq!(DiagnosticContext::Action { name: action }.schema(), action);
        }
        let safe = redact_diagnostic(
            &json!({"topic":"trade.CANARY","action":"trade.CANARY","module":"trade"}),
            DiagnosticContext::State,
        );
        assert_eq!(safe["module"], "trade");
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn missions_topics_and_actions_are_registered_and_unknown_ones_redacted() {
        for topic in [
            "missions.active",
            "missions.massacre",
            "missions.alerts",
            "missions.rewards",
            "missions.passengers",
            "missions.tourism",
            "missions.community_goals",
        ] {
            assert_eq!(DiagnosticContext::Topic { name: topic }.schema(), topic);
        }
        for action in [
            "missions.to_route",
            "missions.dismiss_alert",
            "missions.reset_session",
        ] {
            assert_eq!(DiagnosticContext::Action { name: action }.schema(), action);
        }
        let safe = redact_diagnostic(
            &json!({"topic":"missions.CANARY","action":"missions.CANARY","module":"missions"}),
            DiagnosticContext::State,
        );
        assert_eq!(safe["module"], "missions");
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn engineering_topic_public_fields_and_nested_private_canaries() {
        let context = DiagnosticContext::Topic {
            name: "engineering.flow",
        };
        let raw = json!({"status":"ok","module_type":"engine","request":{"blueprint":"engine_dirty","experimental":"special_engine_overloaded"},
            "lines":[{"state":"trade","category":"Raw"}],"unknown":{"status":"CANARY","blueprint":"CANARY"},
            "cmdr":{"name":"CANARY","credits":42},"ship":{"name":"CANARY","ident":"CANARY"},
            "token":{"blueprint":"CANARY","module_type":"CANARY","status":"ok"}});
        let safe = redact_diagnostic(&raw, context);
        assert_eq!(safe["status"], "ok");
        assert_eq!(safe["request"]["blueprint"], "engine_dirty");
        assert_eq!(safe["request"]["experimental"], "special_engine_overloaded");
        assert_eq!(safe["module_type"], "engine");
        assert_eq!(safe["lines"][0]["state"], "trade");
        assert_eq!(safe["token"]["status"], REDACTED);
        assert!(!safe.to_string().contains("CANARY"));
        assert_eq!(
            redact_diagnostic(&raw, DiagnosticContext::Topic { name: "unknown" })["status"],
            REDACTED
        );
        for (topic, path) in [
            ("engineering.session", "recent"),
            ("engineering.inventory", "by_category"),
        ] {
            let raw = if path == "recent" {
                json!({"recent":[{"key":"iron","origin":"Scan","category":"Raw"}],"token":{"recent":[{"key":"CANARY"}]},"unknown":{"key":"CANARY"}})
            } else {
                json!({"by_category":{"Raw":{"1":[{"key":"iron","name_localised":"Hierro"}]}},"token":{"key":"CANARY"},"unknown":{"key":"CANARY"}})
            };
            let safe = redact_diagnostic(&raw, DiagnosticContext::Topic { name: topic });
            if path == "recent" {
                assert_eq!(safe["recent"][0]["key"], REDACTED);
                assert_eq!(safe["recent"][0]["origin"], "Scan");
            } else {
                assert_eq!(safe["by_category"]["Raw"]["1"][0]["key"], REDACTED);
                assert_eq!(
                    safe["by_category"]["Raw"]["1"][0]["name_localised"],
                    REDACTED
                );
            }
            assert!(!safe.to_string().contains("CANARY"));
        }
    }

    #[test]
    fn engineering_topics_actions_and_module_registered_and_canary_redacted() {
        for topic in [
            "engineering.catalogue",
            "engineering.inventory",
            "engineering.locker",
            "engineering.session",
            "engineering.engineers",
            "engineering.ship",
            "engineering.flow",
            "engineering.goals",
            "engineering.sources",
            "engineering.alerts",
        ] {
            assert_eq!(DiagnosticContext::Topic { name: topic }.schema(), topic);
        }
        for action in [
            "engineering.select_module",
            "engineering.preview",
            "engineering.close_flow",
            "engineering.pin_goal",
            "engineering.unpin_goal",
            "engineering.move_goal",
            "engineering.select_engineer",
            "engineering.dismiss_alert",
            "engineering.reset_session",
        ] {
            assert_eq!(DiagnosticContext::Action { name: action }.schema(), action);
        }
        let safe = redact_diagnostic(
            &json!({"topic":"engineering.CANARY","action":"engineering.CANARY","module":"engineering","cmdr":"CMDR TESTER"}),
            DiagnosticContext::State,
        );
        assert_eq!(safe["module"], "engineering");
        assert_eq!(safe["topic"], REDACTED);
        assert_eq!(safe["action"], REDACTED);
        assert!(!safe.to_string().contains("CANARY"));
        assert!(!safe.to_string().contains("CMDR TESTER"));
    }

    #[test]
    fn material_events_are_allowlisted_and_canary_is_redacted() {
        let safe = redact_diagnostic(
            &json!({"event":"MaterialTrade","other":{"event":"MaterialTrade.CANARY"}}),
            DiagnosticContext::State,
        );
        assert_eq!(safe["event"], "MaterialTrade");
        assert_eq!(safe["other"]["event"], REDACTED);
        for e in [
            "Materials",
            "MaterialCollected",
            "MaterialDiscarded",
            "Synthesis",
            "EngineerProgress",
            "EngineerCraft",
            "EngineerContribution",
            "TechnologyBroker",
            "BackpackChange",
        ] {
            assert_eq!(DiagnosticContext::Journal { event: e }.schema(), e);
        }
    }

    #[test]
    fn missions_module_is_allowlisted_and_canary_is_redacted() {
        let safe = redact_diagnostic(
            &json!({"module":"missions","view":"missions","other":{"module":"missions.CANARY"}}),
            DiagnosticContext::State,
        );
        assert_eq!(safe["module"], "missions");
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn ship_identifiers_and_known_schema_names_have_explicit_policy() {
        let safe = redact_diagnostic(
            &json!({"ship_id":9,"ship":{"ident":123,"name":"CANARY"},
            "topic":"navigation.route","module":"navigation","action":"navigation.plan","event":"FSDJump",
            "unknown":{"topic":"CANARY","module":"CANARY","action":"CANARY","event":"CANARY"}}),
            DiagnosticContext::State,
        );
        assert_eq!(safe["ship_id"], 0);
        assert_eq!(safe["ship"]["ident"], 0);
        assert_eq!(safe["topic"], "navigation.route");
        assert_eq!(safe["module"], "navigation");
        assert_eq!(safe["action"], "navigation.plan");
        assert_eq!(safe["event"], "FSDJump");
        assert!(!safe.to_string().contains("CANARY"));
        assert_eq!(
            DiagnosticContext::Topic {
                name: "navigation.route"
            }
            .schema(),
            "navigation.route"
        );
        assert_eq!(
            DiagnosticContext::Action { name: "CANARY" }.schema(),
            "[redacted]"
        );
        assert_eq!(
            DiagnosticContext::Journal { event: "Scan" }.schema(),
            "Scan"
        );
    }
    #[test]
    fn f3_labels_are_closed_sets() {
        let ctx = DiagnosticContext::NetOutcome;
        let safe = redact_diagnostic(
            &json!({"outcome":"trade_route","endpoint":"ardent.nearby.exports"}),
            ctx,
        );
        assert_eq!(safe["outcome"], "trade_route");
        assert_eq!(safe["endpoint"], "ardent.nearby.exports");
        let safe = redact_diagnostic(&json!({"outcome":"CANARY","endpoint":"CANARY"}), ctx);
        assert_eq!(safe["outcome"], REDACTED);
        assert_eq!(safe["endpoint"], REDACTED);
        let state = DiagnosticContext::State;
        assert_eq!(
            redact_diagnostic(&json!({"reason":"legacy"}), state)["reason"],
            "legacy"
        );
        assert_eq!(
            redact_diagnostic(&json!({"reason":"CANARY"}), state)["reason"],
            REDACTED
        );
        let query = redact_diagnostic(
            &json!({"SystemCoordsByName":{"name":"CANARY"}}),
            DiagnosticContext::NetQuery,
        );
        assert_eq!(query["SystemCoordsByName"]["name"], REDACTED);
    }
    #[test]
    fn nearby_service_variants_have_closed_diag_labels() {
        let ctx = DiagnosticContext::NetQuery;
        let safe = redact_diagnostic(
            &json!({"NearestService":{"service":{"MaterialTrader":"Raw"},"system_name":"Sol",
                "service2":{"TechnologyBroker":"Guardian"}}}),
            ctx,
        );
        assert_eq!(safe["NearestService"]["service"]["MaterialTrader"], "Raw");
        // `system_name` es público (nombre de sistema del juego): se conserva.
        assert_eq!(safe["NearestService"]["system_name"], "Sol");
        assert_eq!(
            safe["NearestService"]["service2"]["TechnologyBroker"],
            "Guardian"
        );
        let safe = redact_diagnostic(
            &json!({"NearestService":{"service":{"MaterialTrader":"CANARY"},
                "service2":{"TechnologyBroker":"CANARY"}}}),
            ctx,
        );
        assert_eq!(
            safe["NearestService"]["service"]["MaterialTrader"],
            REDACTED
        );
        assert_eq!(
            safe["NearestService"]["service2"]["TechnologyBroker"],
            REDACTED
        );
        // Un tipo de bróker no vale como tipo de comerciante.
        let safe = redact_diagnostic(&json!({"MaterialTrader":"Guardian"}), ctx);
        assert_eq!(safe["MaterialTrader"], REDACTED);
    }

    #[test]
    fn new_endpoint_labels_are_closed_set() {
        let ctx = DiagnosticContext::NetOutcome;
        for label in [
            "ardent.nearest.material_trader",
            "ardent.nearest.technology_broker",
            "spansh.stations.material_trader",
            "spansh.stations.technology_broker",
        ] {
            let safe = redact_diagnostic(&json!({ "endpoint": label }), ctx);
            assert_eq!(safe["endpoint"], label);
        }
        for canary in ["spansh.stations.CANARY", "ardent.nearest.CANARY"] {
            let safe = redact_diagnostic(&json!({ "endpoint": canary }), ctx);
            assert_eq!(safe["endpoint"], REDACTED);
        }
    }

    #[test]
    fn network_outcomes_keep_finite_statuses_and_cache_source() {
        use crate::net::{NetFailure, NetOutcome, NetSource};
        for failure in [
            NetFailure::Timeout,
            NetFailure::Malformed,
            NetFailure::Down,
            NetFailure::RateLimited,
            NetFailure::UnknownOrigin,
            NetFailure::UnknownDestination,
        ] {
            let raw = serde_json::to_value(NetOutcome::Failed(failure)).unwrap();
            assert_eq!(redact_diagnostic(&raw, DiagnosticContext::NetOutcome), raw);
        }
        for outcome in [NetOutcome::NoData, NetOutcome::Disabled] {
            let raw = serde_json::to_value(outcome).unwrap();
            assert_eq!(redact_diagnostic(&raw, DiagnosticContext::NetOutcome), raw);
            let wrapped = json!({"outcome":raw});
            assert_eq!(
                redact_diagnostic(&wrapped, DiagnosticContext::NetOutcome),
                wrapped
            );
        }
        let raw = json!({"source":NetSource::Cache});
        assert_eq!(redact_diagnostic(&raw, DiagnosticContext::NetOutcome), raw);
        let arbitrary = json!({"unknown":"Timeout", "Failed":"CANARY"});
        let safe = redact_diagnostic(&arbitrary, DiagnosticContext::NetOutcome);
        assert_eq!(safe["unknown"], "[redacted]");
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn navigation_remote_status_is_public_only_at_known_schema_paths() {
        for status in ["ok", "pending", "no-data", "disabled", "failed"] {
            let raw = json!({"current":{"remote":{"status":status}},
                "next":{"remote":{"status":status}}, "following":[{"remote":{"status":status}}],
                "remote":{"status":status}});
            assert_eq!(
                redact_diagnostic(
                    &raw,
                    DiagnosticContext::Topic {
                        name: "navigation.route"
                    }
                ),
                raw
            );
        }
        let raw = json!({"current":{"remote":{"status":"CANARY"}},
            "unrelated":{"status":"ok"}});
        let safe = redact_diagnostic(
            &raw,
            DiagnosticContext::Topic {
                name: "navigation.route",
            },
        );
        assert!(!safe.to_string().contains("CANARY"));
        assert_eq!(safe["unrelated"]["status"], "[redacted]");
        let wrong_schema = redact_diagnostic(
            &json!({"remote":{"status":"ok"}}),
            DiagnosticContext::Topic { name: "unknown" },
        );
        assert_eq!(wrong_schema["remote"]["status"], "[redacted]");
    }
    struct Fails;
    impl Serialize for Fails {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("CANARY-serialization-error"))
        }
    }

    #[test]
    fn serialization_failure_has_only_a_fixed_marker() {
        let safe = serialize_diagnostic(&Fails, DiagnosticContext::Sql);
        assert_eq!(safe, json!({"serialization_failed":true}));
        assert!(!safe.to_string().contains("CANARY"));
    }

    #[test]
    fn ui_values_are_closed_sets_per_key() {
        let keys = [
            "tab",
            "slot",
            "panel",
            "view",
            "relevance",
            "state",
            "kind",
            "layout",
            "level",
            "context",
            "manual_context",
        ];
        for key in keys {
            for canary in ["danikuso", "colonia", "layout:danikuso", "ok-id", "x"] {
                let out = redact_diagnostic(
                    &json!({ key: canary, "n": [canary] }),
                    DiagnosticContext::Ui,
                );
                assert_eq!(out[key], REDACTED, "{key}={canary}");
                assert_eq!(out["n"][0], REDACTED);
            }
        }
        let known = json!({
            "tab": "layout:combat", "slot": ["route", "cz"], "panel": "panel-route-strip",
            "view": "navigation", "relevance": ["active", "urgent"], "state": "no-data",
            "kind": "white_dwarf", "layout": "none", "level": "warning",
            "context": "exobiology", "manual_context": "none"
        });
        assert_eq!(redact_diagnostic(&known, DiagnosticContext::Ui), known);
        // Fuera del contexto UI el mismo valor no se relaja.
        let other = redact_diagnostic(&json!({"tab": "layout:combat"}), DiagnosticContext::Http);
        assert_eq!(other["tab"], REDACTED);
    }
    #[test]
    fn engineering_page_routes_are_finite_and_never_expose_queries() {
        for route in [
            "/api/engineering/goals",
            "/api/engineering/locker",
            "/api/engineering/catalogue",
            "/api/engineering/engineers",
            "/api/engineering/flow",
        ] {
            let safe = redact_diagnostic(
                &json!({"route":route,"url":format!("{route}?revision=1&section=CANARY"),"section":"CANARY","key":"CANARY","name":"CANARY"}),
                DiagnosticContext::Http,
            );
            assert_eq!(safe["route"], route);
            assert!(!safe.to_string().contains("CANARY"));
        }
        for route in [
            "/api/engineering/CANARY",
            "/api/engineering/goals?revision=1",
        ] {
            let safe = redact_diagnostic(&json!({"route":route}), DiagnosticContext::Http);
            assert_eq!(safe["route"], REDACTED);
        }
    }

    #[test]
    fn ship_outfitting_topics_actions_and_endpoints_are_registered() {
        assert!(is_registered_topic("ship.outfitting_catalog"));
        assert!(is_registered_topic("ship.outfitting_search"));
        assert!(is_registered_action("ship.outfitting_search"));
        assert!(is_registered_action("ship.outfitting_clear"));
        let safe = redact_diagnostic(
            &json!({
                "module": "ship",
                "endpoint": "spansh.stations.outfitting",
                "outcome": "outfitting_stations",
                "pledged_power": "CANARY_SECRET"
            }),
            DiagnosticContext::NetOutcome,
        );
        assert_eq!(safe["module"], "ship");
        assert_eq!(safe["endpoint"], "spansh.stations.outfitting");
        assert_eq!(safe["outcome"], "outfitting_stations");
        assert_eq!(safe["pledged_power"], REDACTED);
    }
}

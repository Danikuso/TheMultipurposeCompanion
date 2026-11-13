//! Esquemas EDDN de eventos con archivo compañero (`commodity/3`, `shipyard/2`, `outfitting/2`, `navroute/1`, `fcmaterials_journal/1`).

use super::OmitReason;
use super::sanitize::{
    augment_horizons_odyssey, crosscheck_and_augment_location, strip_localised, validate_local,
};
use crate::tracker::TransientSnapshot;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use tmc_model::companion::{CompanionFile, CompanionKind, NavRouteFile};
use tmc_model::events::JournalEvent;

/// Devuelve `true` si el evento del diario requiere emparejarse con su archivo compañero.
pub fn wants_companion(ev: &JournalEvent) -> bool {
    matches!(
        ev.name.as_str(),
        "Market" | "Shipyard" | "Outfitting" | "NavRoute" | "FCMaterials"
    )
}

/// Comprueba si el archivo compañero recibido corresponde al tipo esperado por el evento del diario.
pub fn companion_matches_event(ev_name: &str, companion: &CompanionFile) -> bool {
    matches!(
        (ev_name, companion),
        ("Market", CompanionFile::Raw(CompanionKind::Market, _))
            | ("Shipyard", CompanionFile::Raw(CompanionKind::Shipyard, _))
            | (
                "Outfitting",
                CompanionFile::Raw(CompanionKind::Outfitting, _)
            )
            | ("NavRoute", CompanionFile::NavRoute(_))
            | (
                "FCMaterials",
                CompanionFile::Raw(CompanionKind::FcMaterials, _)
            )
            | ("Cargo", CompanionFile::Cargo(_))
            | (
                "ShipLocker",
                CompanionFile::Raw(CompanionKind::ShipLocker, _)
            )
    )
}

/// Mapea el evento anunciador junto con su archivo compañero verificado (`None` → `Omitted`).
pub fn map_companion(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    companion: Option<&CompanionFile>,
) -> Option<(&'static str, Result<Value, OmitReason>)> {
    let schema = match ev.name.as_str() {
        "Market" => "commodity/3",
        "Shipyard" => "shipyard/2",
        "Outfitting" => "outfitting/2",
        "NavRoute" => "navroute/1",
        "FCMaterials" => "fcmaterials_journal/1",
        _ => return None,
    };

    let Some(comp) = companion else {
        return Some((schema, Err(OmitReason::Inconsistent)));
    };
    if !companion_matches_event(ev.name.as_str(), comp) {
        return Some((schema, Err(OmitReason::Inconsistent)));
    }

    match (ev.name.as_str(), comp) {
        ("Market", CompanionFile::Raw(CompanionKind::Market, raw)) => {
            map_market(ev, snap, raw).map(|r| (schema, r))
        }
        ("Shipyard", CompanionFile::Raw(CompanionKind::Shipyard, raw)) => {
            map_shipyard(ev, snap, raw).map(|r| (schema, r))
        }
        ("Outfitting", CompanionFile::Raw(CompanionKind::Outfitting, raw)) => {
            map_outfitting(ev, snap, raw).map(|r| (schema, r))
        }
        ("NavRoute", CompanionFile::NavRoute(route)) => {
            map_navroute(ev, snap, route).map(|r| (schema, r))
        }
        ("FCMaterials", CompanionFile::Raw(CompanionKind::FcMaterials, raw)) => {
            map_fcmaterials(ev, snap, raw).map(|r| (schema, r))
        }
        _ => Some((schema, Err(OmitReason::Inconsistent))),
    }
}

fn timestamps_match(ev: &JournalEvent, raw_ts: Option<&str>) -> bool {
    let Some(ts) = raw_ts else {
        return false;
    };
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc) == ev.timestamp)
        .unwrap_or(false)
}

fn timestamp_string(ev: &JournalEvent, raw_ts: Option<&str>) -> String {
    raw_ts
        .map(str::to_string)
        .unwrap_or_else(|| ev.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn market_ids_consistent(ev: &JournalEvent, raw: &Value) -> Option<i64> {
    let comp_id = raw.get("MarketID").and_then(Value::as_i64)?;
    if let Ok(ev_val) = serde_json::from_str::<Value>(ev.raw.get())
        && let Some(ev_id) = ev_val.get("MarketID").and_then(Value::as_i64)
        && ev_id != comp_id
    {
        return None;
    }
    Some(comp_id)
}

fn check_station_header(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    raw: &Value,
) -> Result<(String, String, i64, String), OmitReason> {
    let raw_ts = raw.get("timestamp").and_then(Value::as_str);
    if !timestamps_match(ev, raw_ts) {
        return Err(OmitReason::Inconsistent);
    }
    let Some(market_id) = market_ids_consistent(ev, raw) else {
        return Err(OmitReason::Inconsistent);
    };
    let Some(station_name) = raw
        .get("StationName")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return Err(OmitReason::Inconsistent);
    };

    let mut loc = Map::new();
    if let Some(sys) = raw.get("StarSystem") {
        loc.insert("StarSystem".to_string(), sys.clone());
    }
    if !crosscheck_and_augment_location(&mut loc, snap, Some("StarSystem"), false, false) {
        return Err(OmitReason::Inconsistent);
    }
    let system_name = loc
        .get("StarSystem")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Ok((
        system_name,
        station_name.to_string(),
        market_id,
        timestamp_string(ev, raw_ts),
    ))
}

fn clean_commodity_name(raw: &str) -> String {
    let trimmed = raw.strip_prefix('$').unwrap_or(raw);
    trimmed
        .strip_suffix("_name;")
        .or_else(|| trimmed.strip_suffix("_NAME;"))
        .unwrap_or(trimmed)
        .to_string()
}

fn map_market(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    raw: &Value,
) -> Option<Result<Value, OmitReason>> {
    let (system_name, station_name, market_id, timestamp) =
        match check_station_header(ev, snap, raw) {
            Ok(parts) => parts,
            Err(reason) => return Some(Err(reason)),
        };

    let Some(items) = raw.get("Items").and_then(Value::as_array) else {
        return Some(Err(OmitReason::Inconsistent));
    };

    let mut commodities = Vec::with_capacity(items.len());
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        if obj
            .get("Category")
            .and_then(Value::as_str)
            .is_some_and(|c| c == "NonMarketable")
        {
            continue;
        }
        if obj
            .get("Legality")
            .and_then(Value::as_str)
            .is_some_and(|l| !l.is_empty())
        {
            continue;
        }
        let Some(raw_name) = obj.get("Name").and_then(Value::as_str) else {
            continue;
        };
        let name = clean_commodity_name(raw_name);
        if name.is_empty() {
            continue;
        }
        let (
            Some(mean_price),
            Some(buy_price),
            Some(stock),
            Some(stock_bracket),
            Some(sell_price),
            Some(demand),
            Some(demand_bracket),
        ) = (
            obj.get("MeanPrice").and_then(Value::as_i64),
            obj.get("BuyPrice").and_then(Value::as_i64),
            obj.get("Stock").and_then(Value::as_i64),
            obj.get("StockBracket").cloned(),
            obj.get("SellPrice").and_then(Value::as_i64),
            obj.get("Demand").and_then(Value::as_i64),
            obj.get("DemandBracket").cloned(),
        )
        else {
            return Some(Err(OmitReason::Inconsistent));
        };

        let mut c = Map::new();
        c.insert("name".to_string(), Value::String(name));
        c.insert("meanPrice".to_string(), json!(mean_price));
        c.insert("buyPrice".to_string(), json!(buy_price));
        c.insert("stock".to_string(), json!(stock));
        c.insert("stockBracket".to_string(), stock_bracket);
        c.insert("sellPrice".to_string(), json!(sell_price));
        c.insert("demand".to_string(), json!(demand));
        c.insert("demandBracket".to_string(), demand_bracket);
        if let Some(flags) = obj.get("statusFlags").and_then(Value::as_array)
            && !flags.is_empty()
        {
            c.insert("statusFlags".to_string(), Value::Array(flags.clone()));
        }
        commodities.push(Value::Object(c));
    }

    let mut msg = Map::new();
    msg.insert("systemName".to_string(), Value::String(system_name));
    msg.insert("stationName".to_string(), Value::String(station_name));
    if let Some(access) = raw
        .get("CarrierDockingAccess")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        msg.insert(
            "carrierDockingAccess".to_string(),
            Value::String(access.to_string()),
        );
    }
    msg.insert("marketId".to_string(), json!(market_id));
    augment_horizons_odyssey(&mut msg, snap);
    msg.insert("timestamp".to_string(), Value::String(timestamp));
    msg.insert("commodities".to_string(), Value::Array(commodities));

    let val = Value::Object(msg);
    if let Err(field) = validate_local(&val) {
        return Some(Err(OmitReason::LocalValidation(field)));
    }
    Some(Ok(val))
}

fn map_shipyard(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    raw: &Value,
) -> Option<Result<Value, OmitReason>> {
    let (system_name, station_name, market_id, timestamp) =
        match check_station_header(ev, snap, raw) {
            Ok(parts) => parts,
            Err(reason) => return Some(Err(reason)),
        };

    let Some(list) = raw.get("PriceList").and_then(Value::as_array) else {
        return Some(Err(OmitReason::Inconsistent));
    };
    let mut seen = HashSet::new();
    let mut ships = Vec::with_capacity(list.len());
    for entry in list {
        let Some(ship_type) = entry
            .get("ShipType")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        if seen.insert(ship_type.to_string()) {
            ships.push(Value::String(ship_type.to_string()));
        }
    }
    if ships.is_empty() {
        return None;
    }

    let mut msg = Map::new();
    msg.insert("systemName".to_string(), Value::String(system_name));
    msg.insert("stationName".to_string(), Value::String(station_name));
    msg.insert("marketId".to_string(), json!(market_id));
    augment_horizons_odyssey(&mut msg, snap);
    if let Some(allow) = raw.get("AllowCobraMkIV").and_then(Value::as_bool) {
        msg.insert("allowCobraMkIV".to_string(), Value::Bool(allow));
    }
    msg.insert("timestamp".to_string(), Value::String(timestamp));
    msg.insert("ships".to_string(), Value::Array(ships));

    let val = Value::Object(msg);
    if let Err(field) = validate_local(&val) {
        return Some(Err(OmitReason::LocalValidation(field)));
    }
    Some(Ok(val))
}

fn is_valid_outfitting_module(name: &str, sku: Option<&Value>) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower == "int_planetapproachsuite" {
        return false;
    }
    if !(lower.starts_with("hpt_") || lower.starts_with("int_") || lower.contains("_armour_")) {
        return false;
    }
    match sku {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s == "ELITE_HORIZONS_V_PLANETARY_LANDINGS",
        Some(_) => false,
    }
}

fn map_outfitting(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    raw: &Value,
) -> Option<Result<Value, OmitReason>> {
    let (system_name, station_name, market_id, timestamp) =
        match check_station_header(ev, snap, raw) {
            Ok(parts) => parts,
            Err(reason) => return Some(Err(reason)),
        };

    let Some(items) = raw.get("Items").and_then(Value::as_array) else {
        return Some(Err(OmitReason::Inconsistent));
    };
    let mut seen = HashSet::new();
    let mut modules = Vec::with_capacity(items.len());
    for item in items {
        let Some(name) = item
            .get("Name")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        if !is_valid_outfitting_module(name, item.get("sku")) {
            continue;
        }
        if seen.insert(name.to_string()) {
            modules.push(Value::String(name.to_string()));
        }
    }
    if modules.is_empty() {
        return None;
    }

    let mut msg = Map::new();
    msg.insert("systemName".to_string(), Value::String(system_name));
    msg.insert("stationName".to_string(), Value::String(station_name));
    msg.insert("marketId".to_string(), json!(market_id));
    augment_horizons_odyssey(&mut msg, snap);
    msg.insert("timestamp".to_string(), Value::String(timestamp));
    msg.insert("modules".to_string(), Value::Array(modules));

    let val = Value::Object(msg);
    if let Err(field) = validate_local(&val) {
        return Some(Err(OmitReason::LocalValidation(field)));
    }
    Some(Ok(val))
}

fn map_navroute(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    route: &NavRouteFile,
) -> Option<Result<Value, OmitReason>> {
    if !timestamps_match(ev, route.timestamp.as_deref()) {
        return Some(Err(OmitReason::Inconsistent));
    }
    if route.route.is_empty() {
        return None;
    }

    let mut hops = Vec::with_capacity(route.route.len());
    for hop in &route.route {
        let Some(star_class) = hop.star_class.as_deref().filter(|s| !s.is_empty()) else {
            return Some(Err(OmitReason::Inconsistent));
        };
        if hop.star_system.is_empty() || !hop.star_pos.iter().all(|v| v.is_finite()) {
            return Some(Err(OmitReason::Inconsistent));
        }
        hops.push(json!({
            "StarSystem": hop.star_system,
            "SystemAddress": hop.system_address,
            "StarPos": hop.star_pos,
            "StarClass": star_class,
        }));
    }

    let mut msg = Map::new();
    msg.insert(
        "timestamp".to_string(),
        Value::String(timestamp_string(ev, route.timestamp.as_deref())),
    );
    msg.insert("event".to_string(), Value::String("NavRoute".to_string()));
    augment_horizons_odyssey(&mut msg, snap);
    msg.insert("Route".to_string(), Value::Array(hops));

    let val = Value::Object(msg);
    if let Err(field) = validate_local(&val) {
        return Some(Err(OmitReason::LocalValidation(field)));
    }
    Some(Ok(val))
}

fn map_fcmaterials(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
    raw: &Value,
) -> Option<Result<Value, OmitReason>> {
    let raw_ts = raw.get("timestamp").and_then(Value::as_str);
    if !timestamps_match(ev, raw_ts) {
        return Some(Err(OmitReason::Inconsistent));
    }
    let Some(market_id) = market_ids_consistent(ev, raw) else {
        return Some(Err(OmitReason::Inconsistent));
    };
    let Some(carrier_name) = raw
        .get("CarrierName")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return Some(Err(OmitReason::Inconsistent));
    };
    let Some(carrier_id) = raw
        .get("CarrierID")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return Some(Err(OmitReason::Inconsistent));
    };
    let Some(raw_items) = raw.get("Items").and_then(Value::as_array) else {
        return Some(Err(OmitReason::Inconsistent));
    };

    let mut items = Vec::with_capacity(raw_items.len());
    for item in raw_items {
        let Some(obj) = item.as_object() else {
            return Some(Err(OmitReason::Inconsistent));
        };
        let (Some(id), Some(name), Some(price), Some(stock), Some(demand)) = (
            obj.get("id").and_then(Value::as_i64),
            obj.get("Name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty()),
            obj.get("Price").and_then(Value::as_i64),
            obj.get("Stock").and_then(Value::as_i64),
            obj.get("Demand").and_then(Value::as_i64),
        ) else {
            return Some(Err(OmitReason::Inconsistent));
        };
        items.push(json!({
            "id": id,
            "Name": name,
            "Price": price,
            "Stock": stock,
            "Demand": demand,
        }));
    }

    let mut msg = Map::new();
    msg.insert(
        "timestamp".to_string(),
        Value::String(timestamp_string(ev, raw_ts)),
    );
    msg.insert(
        "event".to_string(),
        Value::String("FCMaterials".to_string()),
    );
    augment_horizons_odyssey(&mut msg, snap);
    msg.insert("MarketID".to_string(), json!(market_id));
    msg.insert(
        "CarrierName".to_string(),
        Value::String(carrier_name.to_string()),
    );
    msg.insert(
        "CarrierID".to_string(),
        Value::String(carrier_id.to_string()),
    );
    msg.insert("Items".to_string(), Value::Array(items));

    let mut val = Value::Object(msg);
    strip_localised(&mut val);
    if let Err(field) = validate_local(&val) {
        return Some(Err(OmitReason::LocalValidation(field)));
    }
    Some(Ok(val))
}

//! Esquemas EDDN de eventos sueltos (`E1-4`) y agrupación de `FSSSignalDiscovered`.

use super::OmitReason;
use super::sanitize::{
    augment_horizons_odyssey, crosscheck_and_augment_location, strip_localised, validate_local,
};
use crate::tracker::TransientSnapshot;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use tmc_model::events::JournalEvent;

const DISCOVERY_SCAN_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "SystemName",
    "StarPos",
    "SystemAddress",
    "BodyCount",
    "NonBodyCount",
];

const ALL_BODIES_FOUND_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "SystemName",
    "StarPos",
    "SystemAddress",
    "Count",
];

const NAV_BEACON_SCAN_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "StarSystem",
    "StarPos",
    "SystemAddress",
    "NumBodies",
];

const CODEX_ENTRY_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "System",
    "StarPos",
    "SystemAddress",
    "Name",
    "Region",
    "EntryID",
    "Category",
    "Latitude",
    "Longitude",
    "SubCategory",
    "NearestDestination",
    "VoucherAmount",
    "Traits",
    "BodyID",
    "BodyName",
];

const SCAN_BARYCENTRE_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "StarSystem",
    "StarPos",
    "SystemAddress",
    "BodyID",
    "SemiMajorAxis",
    "Eccentricity",
    "OrbitalInclination",
    "Periapsis",
    "OrbitalPeriod",
    "AscendingNode",
    "MeanAnomaly",
];

const FSS_BODY_SIGNALS_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "StarSystem",
    "StarPos",
    "SystemAddress",
    "BodyID",
    "BodyName",
    "Signals",
];

const APPROACH_SETTLEMENT_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "StarSystem",
    "StarPos",
    "SystemAddress",
    "Name",
    "MarketID",
    "BodyID",
    "BodyName",
    "Latitude",
    "Longitude",
    "StationGovernment",
    "StationAllegiance",
    "StationEconomy",
    "StationEconomies",
    "StationFaction",
    "StationServices",
];

const DOCKING_GRANTED_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "MarketID",
    "StationName",
    "StationType",
    "LandingPad",
];

const DOCKING_DENIED_KEYS: &[&str] = &[
    "timestamp",
    "event",
    "MarketID",
    "StationName",
    "StationType",
    "Reason",
];

const SIGNAL_ITEM_KEYS: &[&str] = &[
    "timestamp",
    "SignalName",
    "SignalType",
    "IsStation",
    "USSType",
    "SpawningState",
    "SpawningFaction",
    "SpawningPower",
    "OpposingPower",
    "ThreatLevel",
];

struct EventSpec {
    schema: &'static str,
    allowed_keys: &'static [&'static str],
    system_key: Option<&'static str>,
    need_coords: bool,
}

fn spec_for_event(name: &str) -> Option<EventSpec> {
    Some(match name {
        "FSSDiscoveryScan" => EventSpec {
            schema: "fssdiscoveryscan/1",
            allowed_keys: DISCOVERY_SCAN_KEYS,
            system_key: Some("SystemName"),
            need_coords: true,
        },
        "FSSAllBodiesFound" => EventSpec {
            schema: "fssallbodiesfound/1",
            allowed_keys: ALL_BODIES_FOUND_KEYS,
            system_key: Some("SystemName"),
            need_coords: true,
        },
        "NavBeaconScan" => EventSpec {
            schema: "navbeaconscan/1",
            allowed_keys: NAV_BEACON_SCAN_KEYS,
            system_key: Some("StarSystem"),
            need_coords: true,
        },
        "CodexEntry" => EventSpec {
            schema: "codexentry/1",
            allowed_keys: CODEX_ENTRY_KEYS,
            system_key: Some("System"),
            need_coords: true,
        },
        "ScanBaryCentre" => EventSpec {
            schema: "scanbarycentre/1",
            allowed_keys: SCAN_BARYCENTRE_KEYS,
            system_key: Some("StarSystem"),
            need_coords: true,
        },
        "FSSBodySignals" => EventSpec {
            schema: "fssbodysignals/1",
            allowed_keys: FSS_BODY_SIGNALS_KEYS,
            system_key: Some("StarSystem"),
            need_coords: true,
        },
        "ApproachSettlement" => EventSpec {
            schema: "approachsettlement/1",
            allowed_keys: APPROACH_SETTLEMENT_KEYS,
            system_key: Some("StarSystem"),
            need_coords: true,
        },
        "DockingGranted" => EventSpec {
            schema: "dockinggranted/1",
            allowed_keys: DOCKING_GRANTED_KEYS,
            system_key: None,
            need_coords: false,
        },
        "DockingDenied" => EventSpec {
            schema: "dockingdenied/1",
            allowed_keys: DOCKING_DENIED_KEYS,
            system_key: None,
            need_coords: false,
        },
        _ => return None,
    })
}

fn retain_keys(obj: &mut Map<String, Value>, allowed: &[&str]) {
    obj.retain(|k, _| allowed.contains(&k.as_str()));
}

fn clean_nested_object_array(obj: &mut Map<String, Value>, field: &str, allowed: &[&str]) {
    if let Some(items) = obj.get_mut(field).and_then(Value::as_array_mut) {
        for item in items {
            if let Some(m) = item.as_object_mut() {
                retain_keys(m, allowed);
            }
        }
    }
}

/// Traduce los 9 esquemas EDDN de evento único (`E1-4`).
pub fn map_event(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
) -> Option<(&'static str, Result<Value, OmitReason>)> {
    let spec = spec_for_event(ev.name.as_str())?;
    let Ok(mut val) = serde_json::from_str::<Value>(ev.raw.get()) else {
        return Some((spec.schema, Err(OmitReason::Inconsistent)));
    };
    strip_localised(&mut val);

    let Some(obj) = val.as_object_mut() else {
        return Some((spec.schema, Err(OmitReason::Inconsistent)));
    };
    retain_keys(obj, spec.allowed_keys);

    match ev.name.as_str() {
        "FSSBodySignals" => clean_nested_object_array(obj, "Signals", &["Type", "Count"]),
        "ApproachSettlement" => {
            clean_nested_object_array(obj, "StationEconomies", &["Name", "Proportion"]);
            if let Some(faction) = obj.get_mut("StationFaction").and_then(Value::as_object_mut) {
                retain_keys(faction, &["Name", "FactionState"]);
            }
        }
        _ => {}
    }

    if (spec.system_key.is_some() || spec.need_coords)
        && !crosscheck_and_augment_location(
            obj,
            snap,
            spec.system_key,
            spec.need_coords,
            spec.need_coords,
        )
    {
        return Some((spec.schema, Err(OmitReason::Inconsistent)));
    }

    augment_horizons_odyssey(obj, snap);

    if let Err(path) = validate_local(&val) {
        return Some((spec.schema, Err(OmitReason::LocalValidation(path))));
    }
    Some((spec.schema, Ok(val)))
}

/// Extrae el `SystemAddress` común de una racha de `FSSSignalDiscovered` para comparar con el cierre.
pub fn fss_group_system_address(evs: &[JournalEvent]) -> Option<u64> {
    for ev in evs {
        if let Ok(val) = serde_json::from_str::<Value>(ev.raw.get())
            && let Some(addr) = val.get("SystemAddress").and_then(Value::as_u64)
        {
            return Some(addr);
        }
    }
    None
}

/// Construye el mensaje agrupado `fsssignaldiscovered/1` sobre una racha contigua.
/// Devuelve `(timestamp_primera_senal_valida, resultado)`. Si todas se filtran, devuelve `None`.
pub fn map_fss_signal_group(
    evs: &[JournalEvent],
    snap: &TransientSnapshot,
) -> Option<(DateTime<Utc>, Result<Value, OmitReason>)> {
    if evs.is_empty() {
        return None;
    }

    let mut signals = Vec::with_capacity(evs.len());
    let mut first_ts: Option<(DateTime<Utc>, String)> = None;

    for ev in evs {
        let Ok(mut val) = serde_json::from_str::<Value>(ev.raw.get()) else {
            continue;
        };
        strip_localised(&mut val);
        let Some(mut obj) = val.as_object().cloned() else {
            continue;
        };

        if obj.get("USSType").and_then(Value::as_str) == Some("$USS_Type_MissionTarget;") {
            continue;
        }
        if let Some(sig_addr) = obj.get("SystemAddress").and_then(Value::as_u64)
            && let Some(tracked_addr) = snap.system_address
            && sig_addr != tracked_addr
        {
            continue;
        }

        let ts_str = obj
            .get("timestamp")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| ev.timestamp.to_rfc3339());
        retain_keys(&mut obj, SIGNAL_ITEM_KEYS);
        let Some(name) = obj.get("SignalName").and_then(Value::as_str) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        if first_ts.is_none() {
            first_ts = Some((ev.timestamp, ts_str));
        }
        signals.push(Value::Object(obj));
    }

    let (ev_ts, ts_str) = first_ts?;
    if signals.is_empty() {
        return None;
    }

    let mut msg = Map::new();
    msg.insert(
        "event".to_string(),
        Value::String("FSSSignalDiscovered".to_string()),
    );
    augment_horizons_odyssey(&mut msg, snap);
    msg.insert("timestamp".to_string(), Value::String(ts_str));
    if !crosscheck_and_augment_location(&mut msg, snap, Some("StarSystem"), true, true) {
        return Some((ev_ts, Err(OmitReason::Inconsistent)));
    }
    msg.insert("signals".to_string(), Value::Array(signals));

    let val = Value::Object(msg);
    if let Err(path) = validate_local(&val) {
        return Some((ev_ts, Err(OmitReason::LocalValidation(path))));
    }
    Some((ev_ts, Ok(val)))
}

//! Saneado y validación local previa al envío a EDDN (§7.3, C6).

use crate::tracker::TransientSnapshot;
use serde_json::{Map, Value, json};

pub const PERSONAL_FIELDS: &[&str] = &[
    "FID",
    "Commander",
    "Credits",
    "Loan",
    "Wanted",
    "ActiveFine",
    "CockpitBreach",
    "MyReputation",
    "BoostUsed",
    "FuelLevel",
    "FuelUsed",
    "JumpDist",
    "HappiestSystem",
    "HomeSystem",
    "SquadronFaction",
    "IsNewEntry",
    "NewTraitsDiscovered",
    "Message",
    "From",
    "To",
    "Channel",
];

/// Elimina recursivamente toda clave que termine en `_Localised`.
pub fn strip_localised(v: &mut Value) {
    match v {
        Value::Object(map) => {
            map.retain(|k, _| !k.ends_with("_Localised"));
            for val in map.values_mut() {
                strip_localised(val);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                strip_localised(item);
            }
        }
        _ => {}
    }
}

/// Comprueba recursivamente que no quede ninguna clave `_Localised` ni personal.
pub fn validate_local(msg: &Value) -> Result<(), String> {
    validate_at(msg, "")
}

fn validate_at(v: &Value, prefix: &str) -> Result<(), String> {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                if k.ends_with("_Localised") || PERSONAL_FIELDS.contains(&k.as_str()) {
                    return Err(path);
                }
                validate_at(val, &path)?;
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let path = format!("{prefix}[{i}]");
                validate_at(item, &path)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Añade `horizons` y `odyssey` exclusivamente si provienen de `LoadGame` (nunca `null`).
pub(crate) fn augment_horizons_odyssey(obj: &mut Map<String, Value>, snap: &TransientSnapshot) {
    match snap.horizons {
        Some(v) => {
            obj.insert("horizons".to_string(), Value::Bool(v));
        }
        None => {
            obj.remove("horizons");
        }
    }
    match snap.odyssey {
        Some(v) => {
            obj.insert("odyssey".to_string(), Value::Bool(v));
        }
        None => {
            obj.remove("odyssey");
        }
    }
}

fn extract_pos(v: &Value) -> Option<[f64; 3]> {
    let arr = v.as_array()?;
    if arr.len() != 3 {
        return None;
    }
    let x = arr[0].as_f64()?;
    let y = arr[1].as_f64()?;
    let z = arr[2].as_f64()?;
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some([x, y, z])
    } else {
        None
    }
}

fn pos_matches(a: [f64; 3], b: [f64; 3]) -> bool {
    (a[0] - b[0]).abs() <= 1e-4 && (a[1] - b[1]).abs() <= 1e-4 && (a[2] - b[2]).abs() <= 1e-4
}

/// Comprueba coherencia de ubicación respecto al último `Location`/`FSDJump`/`CarrierJump` y rellena faltantes.
pub(crate) fn crosscheck_and_augment_location(
    obj: &mut Map<String, Value>,
    snap: &TransientSnapshot,
    system_key: Option<&str>,
    need_starpos: bool,
    need_address: bool,
) -> bool {
    if let Some(addr_val) = obj.get("SystemAddress") {
        let Some(addr) = addr_val.as_u64() else {
            return false;
        };
        if let Some(tracked) = snap.system_address
            && addr != tracked
        {
            return false;
        }
    } else if need_address {
        let Some(tracked) = snap.system_address else {
            return false;
        };
        obj.insert("SystemAddress".to_string(), json!(tracked));
    }

    if let Some(sys_key) = system_key {
        if let Some(sys_val) = obj.get(sys_key) {
            let Some(sys) = sys_val.as_str() else {
                return false;
            };
            if sys.is_empty() {
                return false;
            }
            if let Some(tracked) = snap.system_name.as_deref()
                && sys != tracked
            {
                return false;
            }
        } else {
            let Some(tracked) = snap.system_name.as_deref() else {
                return false;
            };
            if tracked.is_empty() {
                return false;
            }
            obj.insert(sys_key.to_string(), Value::String(tracked.to_string()));
        }
    }

    if let Some(pos_val) = obj.get("StarPos") {
        let Some(pos) = extract_pos(pos_val) else {
            return false;
        };
        if let Some(tracked) = snap.star_pos
            && !pos_matches(pos, tracked)
        {
            return false;
        }
    } else if need_starpos {
        let Some(tracked) = snap.star_pos else {
            return false;
        };
        obj.insert("StarPos".to_string(), json!(tracked));
    }

    true
}

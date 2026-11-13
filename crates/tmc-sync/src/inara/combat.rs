//! Mapeo de combate e interdicciones hacia Inara (`addCommanderCombat*`).

use serde_json::{Map, Value, json};
use tmc_model::events::EventKind;

use super::{inara_row, raw_value, ts_iso};
use crate::router::{MappedInput, MappedRow};

pub fn map_combat(fid: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) -> bool {
    let ts = ts_iso(input);
    match &input.ev.kind {
        EventKind::Died => {
            let Some(sys) = input
                .snap
                .system_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return true;
            };
            let raw = raw_value(input);
            let mut data = Map::new();
            data.insert("starsystemName".into(), json!(sys));
            if let Some(killers) = raw
                .as_ref()
                .and_then(|r| r.get("Killers"))
                .and_then(Value::as_array)
            {
                let names: Vec<Value> = killers
                    .iter()
                    .filter_map(|k| {
                        k.get("Name")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(|s| json!(s))
                    })
                    .collect();
                if !names.is_empty() {
                    data.insert("wingOpponentNames".into(), Value::Array(names));
                }
            } else if let Some(killer) = raw
                .as_ref()
                .and_then(|r| r.get("KillerName"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("opponentName".into(), json!(killer));
            }
            out.push(inara_row(
                fid,
                "addCommanderCombatDeath",
                0,
                &ts,
                Value::Object(data),
                None,
            ));
            true
        }
        EventKind::Interdicted(i) => {
            let Some(sys) = input
                .snap
                .system_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return true;
            };
            let raw = raw_value(input);
            let mut data = Map::new();
            data.insert("starsystemName".into(), json!(sys));
            if let Some(opp) = first_non_empty(&[
                i.interdictor.as_deref(),
                raw.as_ref()
                    .and_then(|r| r.get("Faction"))
                    .and_then(Value::as_str),
                raw.as_ref()
                    .and_then(|r| r.get("Power"))
                    .and_then(Value::as_str),
            ]) {
                data.insert("opponentName".into(), json!(opp));
            }
            if let Some(is_p) = raw
                .as_ref()
                .and_then(|r| r.get("IsPlayer"))
                .and_then(Value::as_bool)
            {
                data.insert("isPlayer".into(), json!(is_p));
            }
            if let Some(subm) = i.submitted {
                data.insert("isSubmit".into(), json!(subm));
            }
            out.push(inara_row(
                fid,
                "addCommanderCombatInterdicted",
                0,
                &ts,
                Value::Object(data),
                None,
            ));
            true
        }
        EventKind::Interdiction(i) => {
            let Some(sys) = input
                .snap
                .system_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return true;
            };
            let raw = raw_value(input);
            let mut data = Map::new();
            data.insert("starsystemName".into(), json!(sys));
            if let Some(opp) = first_non_empty(&[
                i.interdicted.as_deref(),
                raw.as_ref()
                    .and_then(|r| r.get("Faction"))
                    .and_then(Value::as_str),
                raw.as_ref()
                    .and_then(|r| r.get("Power"))
                    .and_then(Value::as_str),
            ]) {
                data.insert("opponentName".into(), json!(opp));
            }
            if let Some(is_p) = i.is_player {
                data.insert("isPlayer".into(), json!(is_p));
            }
            if let Some(succ) = i.success {
                data.insert("isSuccess".into(), json!(succ));
            }
            out.push(inara_row(
                fid,
                "addCommanderCombatInterdiction",
                0,
                &ts,
                Value::Object(data),
                None,
            ));
            true
        }
        EventKind::EscapeInterdiction(ei) => {
            let Some(sys) = input
                .snap
                .system_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return true;
            };
            let raw = raw_value(input);
            let mut data = Map::new();
            data.insert("starsystemName".into(), json!(sys));
            if let Some(opp) = first_non_empty(&[
                ei.interdictor.as_deref(),
                raw.as_ref()
                    .and_then(|r| r.get("Faction"))
                    .and_then(Value::as_str),
                raw.as_ref()
                    .and_then(|r| r.get("Power"))
                    .and_then(Value::as_str),
            ]) {
                data.insert("opponentName".into(), json!(opp));
            }
            if let Some(is_p) = ei.is_player {
                data.insert("isPlayer".into(), json!(is_p));
            }
            out.push(inara_row(
                fid,
                "addCommanderCombatInterdictionEscape",
                0,
                &ts,
                Value::Object(data),
                None,
            ));
            true
        }
        _ => false,
    }
}

fn first_non_empty<'a>(candidates: &[Option<&'a str>]) -> Option<&'a str> {
    candidates
        .iter()
        .flatten()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
}

//! Mapeo de misiones hacia Inara (`addCommanderMission`, `setCommanderMissionCompleted`,
//! `setCommanderMissionFailed`, `setCommanderMissionAbandoned`).

use chrono::SecondsFormat;
use serde_json::{Map, Value, json};
use tmc_model::events::EventKind;

use super::inventory::emit_materials;
use super::{inara_row, raw_value, ts_iso};
use crate::router::{MappedInput, MappedRow};

pub fn map_missions(fid: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) -> bool {
    let ts = ts_iso(input);
    match &input.ev.kind {
        EventKind::MissionAccepted(m) => {
            let name = m.name.trim();
            if name.is_empty() {
                return true;
            }
            let Some(sys_origin) = input
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
            data.insert("missionName".into(), json!(name));
            data.insert("missionGameID".into(), json!(m.mission_id));
            if let Some(exp) = m.expiry {
                data.insert(
                    "missionExpiry".into(),
                    json!(exp.to_rfc3339_opts(SecondsFormat::Secs, true)),
                );
            }
            if let Some(inf) = m
                .influence
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("influenceGain".into(), json!(inf));
            }
            if let Some(rep) = m
                .reputation
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("reputationGain".into(), json!(rep));
            }
            data.insert("starsystemNameOrigin".into(), json!(sys_origin));
            if let Some(st) = input
                .snap
                .station_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("stationNameOrigin".into(), json!(st));
            }
            if let Some(fac) = m
                .faction
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("minorfactionNameOrigin".into(), json!(fac));
            }
            if let Some(sys) = m
                .destination_system
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("starsystemNameTarget".into(), json!(sys));
            }
            if let Some(st) = m
                .destination_station
                .as_deref()
                .or(m.destination_settlement.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("stationNameTarget".into(), json!(st));
            }
            if let Some(tfac) = m
                .target_faction
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("minorfactionNameTarget".into(), json!(tfac));
            }
            if let Some(comm) = m
                .commodity
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("commodityName".into(), json!(comm));
            }
            if let Some(cnt) = m.count {
                data.insert("commodityCount".into(), json!(cnt));
            }
            if let Some(tname) = raw
                .as_ref()
                .and_then(|r| r.get("Target"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("targetName".into(), json!(tname));
            }
            if let Some(ttype) = m
                .target_type
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("targetType".into(), json!(ttype));
            }
            if let Some(kc) = m.kill_count {
                data.insert("killCount".into(), json!(kc));
            }
            if let Some(pt) = m
                .passenger_type
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("passengerType".into(), json!(pt));
            }
            if let Some(pc) = m.passenger_count {
                data.insert("passengerCount".into(), json!(pc));
            }
            if let Some(vip) = m.passenger_vips {
                data.insert("passengerIsVIP".into(), json!(vip));
            }
            if let Some(wanted) = m.passenger_wanted {
                data.insert("passengerIsWanted".into(), json!(wanted));
            }

            out.push(inara_row(
                fid,
                "addCommanderMission",
                0,
                &ts,
                Value::Object(data),
                None,
            ));
            true
        }
        EventKind::MissionCompleted(m) => {
            let raw = raw_value(input);
            let mut data = Map::new();
            data.insert("missionGameID".into(), json!(m.mission_id));
            data.insert("donationCredits".into(), json!(m.donation.unwrap_or(0)));
            data.insert("rewardCredits".into(), json!(m.reward.unwrap_or(0)));

            if let Some(permits) = raw
                .as_ref()
                .and_then(|r| r.get("PermitsAwarded"))
                .and_then(Value::as_array)
            {
                let list: Vec<Value> = permits
                    .iter()
                    .filter_map(|p| {
                        let s = p.as_str().map(str::trim).filter(|s| !s.is_empty())?;
                        Some(json!({ "starsystemName": s }))
                    })
                    .collect();
                if !list.is_empty() {
                    data.insert("rewardPermits".into(), Value::Array(list));
                }
            }

            if let Some(comms) = raw
                .as_ref()
                .and_then(|r| r.get("CommodityReward"))
                .and_then(Value::as_array)
            {
                let list: Vec<Value> = comms
                    .iter()
                    .filter_map(|c| {
                        let n = c
                            .get("Name")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty())?;
                        let cnt = c.get("Count").and_then(Value::as_u64)?;
                        Some(json!({ "itemName": n, "itemCount": cnt }))
                    })
                    .collect();
                if !list.is_empty() {
                    data.insert("rewardCommodities".into(), Value::Array(list));
                }
            }

            if !m.materials_reward.is_empty() {
                let mats: Vec<Value> = m
                    .materials_reward
                    .iter()
                    .filter(|r| !r.name.trim().is_empty() && r.count > 0)
                    .map(|r| {
                        json!({
                            "itemName": r.name.trim(),
                            "itemCount": r.count,
                        })
                    })
                    .collect();
                if !mats.is_empty() {
                    data.insert("rewardMaterials".into(), Value::Array(mats));
                }
            }

            if let Some(effects) = raw
                .as_ref()
                .and_then(|r| r.get("FactionEffects"))
                .and_then(Value::as_array)
            {
                let list: Vec<Value> = effects
                    .iter()
                    .filter_map(|fx| {
                        let fname = fx
                            .get("Faction")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty())?;
                        let mut f_obj = Map::new();
                        f_obj.insert("minorfactionName".into(), json!(fname));
                        if let Some(inf_entry) = fx
                            .get("Influence")
                            .and_then(Value::as_array)
                            .and_then(|a| a.first())
                            && let Some(inf) = inf_entry
                                .get("Influence")
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                        {
                            let trend_down = inf_entry
                                .get("Trend")
                                .and_then(Value::as_str)
                                .is_some_and(|t| t.starts_with("Down"));
                            let gain = if trend_down {
                                inf.replace('+', "-")
                            } else {
                                inf.to_string()
                            };
                            f_obj.insert("influenceGain".into(), json!(gain));
                        }
                        if let Some(rep) = fx
                            .get("Reputation")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                        {
                            let trend_down = fx
                                .get("ReputationTrend")
                                .and_then(Value::as_str)
                                .is_some_and(|t| t.starts_with("Down"));
                            let gain = if trend_down {
                                rep.replace('+', "-")
                            } else {
                                rep.to_string()
                            };
                            f_obj.insert("reputationGain".into(), json!(gain));
                        }
                        Some(Value::Object(f_obj))
                    })
                    .collect();
                if !list.is_empty() {
                    data.insert("minorfactionEffects".into(), Value::Array(list));
                }
            }

            out.push(inara_row(
                fid,
                "setCommanderMissionCompleted",
                0,
                &ts,
                Value::Object(data),
                None,
            ));

            if !m.materials_reward.is_empty() {
                emit_materials(fid, input, 1, out);
            }
            true
        }
        EventKind::MissionFailed(m) => {
            out.push(inara_row(
                fid,
                "setCommanderMissionFailed",
                0,
                &ts,
                json!({ "missionGameID": m.mission_id }),
                None,
            ));
            true
        }
        EventKind::MissionAbandoned(m) => {
            out.push(inara_row(
                fid,
                "setCommanderMissionAbandoned",
                0,
                &ts,
                json!({ "missionGameID": m.mission_id }),
                None,
            ));
            true
        }
        _ => false,
    }
}

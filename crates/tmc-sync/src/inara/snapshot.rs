//! Instantánea de estado actual hacia Inara al arrancar o activar el servicio.

use super::commander::push_pilot_ranks;
use super::inara_row;
use crate::router::MappedRow;
use crate::tracker::TransientTracker;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value, json};

/// Construye una instantánea de eventos `set*` a partir del estado transitorio actual.
pub fn build_snapshot(
    tracker: &TransientTracker,
    fid: &str,
    activated_at: DateTime<Utc>,
) -> Vec<MappedRow> {
    let fid = fid.trim();
    if fid.is_empty() {
        return Vec::new();
    }
    let snap = tracker.snapshot();
    if snap.cmdr_fid.as_deref().map(str::trim) != Some(fid) {
        return Vec::new();
    }

    let ts = activated_at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut out = Vec::new();

    if snap.ranks.is_some() || snap.rank_progress.is_some() {
        push_pilot_ranks(
            fid,
            &ts,
            snap.ranks.as_ref(),
            snap.rank_progress.as_ref(),
            &mut out,
        );
    }

    if let Some(engineers) = &snap.engineers
        && !engineers.is_empty()
    {
        let entries: Vec<Value> = engineers
            .iter()
            .map(|(name, stage, rank)| {
                let mut m = Map::new();
                m.insert("engineerName".into(), json!(name));
                m.insert("rankStage".into(), json!(stage));
                if let Some(r) = rank {
                    m.insert("rankValue".into(), json!(r));
                }
                Value::Object(m)
            })
            .collect();
        out.push(inara_row(
            fid,
            "setCommanderRankEngineer",
            0,
            &ts,
            Value::Array(entries),
            Some("engineers"),
        ));
    }

    if let Some((power, rank, merits)) = &snap.powerplay
        && !power.trim().is_empty()
    {
        out.push(inara_row(
            fid,
            "setCommanderRankPower",
            0,
            &ts,
            json!({
                "powerName": power.trim(),
                "rankValue": rank,
                "meritsValue": merits,
            }),
            Some(power.trim()),
        ));
    }

    if let Some(major) = &snap.major_reputation
        && !major.is_empty()
    {
        let entries: Vec<Value> = major
            .iter()
            .map(|(name, rep)| {
                json!({
                    "majorfactionName": name,
                    "majorfactionReputation": rep,
                })
            })
            .collect();
        out.push(inara_row(
            fid,
            "setCommanderReputationMajorFaction",
            0,
            &ts,
            Value::Array(entries),
            Some("major"),
        ));
    }

    if let Some((credits, loan)) = snap.credits {
        let mut m = Map::new();
        m.insert("commanderCredits".into(), json!(credits));
        if let Some(l) = loan {
            m.insert("commanderLoan".into(), json!(l));
        }
        out.push(inara_row(
            fid,
            "setCommanderCredits",
            0,
            &ts,
            Value::Object(m),
            Some("credits"),
        ));
    }

    if let (Some(ship_type), Some(ship_id)) = (
        snap.ship_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        snap.ship_id,
    ) {
        let mut m = Map::new();
        m.insert("shipType".into(), json!(ship_type));
        m.insert("shipGameID".into(), json!(ship_id));
        if let Some(name) = snap
            .ship_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            m.insert("shipName".into(), json!(name));
        }
        if let Some(ident) = snap
            .ship_ident
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            m.insert("shipIdent".into(), json!(ident));
        }
        m.insert("isCurrentShip".into(), json!(true));
        let key = ship_id.to_string();
        out.push(inara_row(
            fid,
            "setCommanderShip",
            0,
            &ts,
            Value::Object(m),
            Some(&key),
        ));

        if let Some(loadout) = &snap.ship_loadout
            && loadout.get("shipGameID").and_then(Value::as_u64) == Some(ship_id)
        {
            out.push(inara_row(
                fid,
                "setCommanderShipLoadout",
                0,
                &ts,
                loadout.clone(),
                Some(&key),
            ));
        }
    }

    if let Some(mats) = &snap.materials {
        let entries: Vec<Value> = mats
            .iter()
            .map(|(name, &count)| {
                json!({
                    "itemName": name,
                    "itemCount": count,
                })
            })
            .collect();
        out.push(inara_row(
            fid,
            "setCommanderInventoryMaterials",
            0,
            &ts,
            Value::Array(entries),
            Some("materials"),
        ));
    }

    if let Some(cargo) = &snap.cargo {
        let entries: Vec<Value> = cargo
            .iter()
            .filter(|i| i.count > 0 && !i.name.trim().is_empty())
            .map(|i| {
                let mut m = Map::new();
                m.insert("itemName".into(), json!(i.name.trim()));
                m.insert("itemCount".into(), json!(i.count));
                m.insert("isStolen".into(), json!(i.is_stolen));
                if let Some(mid) = i.mission_id {
                    m.insert("missionGameID".into(), json!(mid));
                }
                Value::Object(m)
            })
            .collect();
        out.push(inara_row(
            fid,
            "setCommanderInventoryCargo",
            0,
            &ts,
            Value::Array(entries),
            Some("cargo"),
        ));
    }

    if let Some(locker) = &snap.ship_locker
        && !locker.is_empty()
    {
        let entries: Vec<Value> = locker
            .iter()
            .map(|i| {
                json!({
                    "itemName": i.name,
                    "itemCount": i.count,
                    "itemType": i.item_type,
                    "itemLocation": "ShipLocker",
                })
            })
            .collect();
        out.push(inara_row(
            fid,
            "setCommanderInventory",
            0,
            &ts,
            Value::Array(entries),
            Some("ShipLocker"),
        ));
    }

    if let Some(sys) = snap
        .system_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let mut m = Map::new();
        m.insert("starsystemName".into(), json!(sys));
        if let Some([x, y, z]) = snap.star_pos {
            m.insert("starsystemCoords".into(), json!([x, y, z]));
        }
        if let Some(st) = snap
            .station_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            m.insert("stationName".into(), json!(st));
        }
        if let Some(mk) = snap.market_id {
            m.insert("marketID".into(), json!(mk));
        }
        if let Some(ship_type) = snap
            .ship_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            m.insert("shipType".into(), json!(ship_type));
        }
        if let Some(ship_id) = snap.ship_id {
            m.insert("shipGameID".into(), json!(ship_id));
        }
        out.push(inara_row(
            fid,
            "setCommanderTravelLocation",
            0,
            &ts,
            Value::Object(m),
            Some("location"),
        ));
    }

    for (idx, row) in out.iter_mut().enumerate() {
        row.sub = idx as u32;
    }
    out
}

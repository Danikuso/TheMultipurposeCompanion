//! Mapeo de nave y equipamiento hacia Inara (`*CommanderShip*`).

use serde_json::{Map, Value, json};
use tmc_model::events::{EventKind, Loadout};

use super::{inara_row, raw_value, ts_iso};
use crate::router::{MappedInput, MappedRow};

pub fn map_ship(fid: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) -> bool {
    let ts = ts_iso(input);
    match &input.ev.kind {
        EventKind::LoadGame(l) => {
            let Some(ship_type) = l.ship.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                return false;
            };
            let Some(ship_id) = l.ship_id else {
                return false;
            };
            let mut data = Map::new();
            data.insert("shipType".into(), json!(ship_type));
            data.insert("shipGameID".into(), json!(ship_id));
            if let Some(name) = l
                .ship_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("shipName".into(), json!(name));
            }
            if let Some(ident) = l
                .ship_ident
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("shipIdent".into(), json!(ident));
            }
            data.insert("isCurrentShip".into(), json!(true));
            let key = ship_id.to_string();
            // sub = 1 porque commander::map_commander emite setCommanderCredits con sub = 0.
            out.push(inara_row(
                fid,
                "setCommanderShip",
                1,
                &ts,
                Value::Object(data),
                Some(&key),
            ));
            true
        }
        EventKind::Loadout(l) => {
            let Some(ship_type) = l.ship.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                return true;
            };
            let Some(ship_id) = l.ship_id else {
                return true;
            };
            let raw = raw_value(input);
            let key = ship_id.to_string();

            let mut ship_data = Map::new();
            ship_data.insert("shipType".into(), json!(ship_type));
            ship_data.insert("shipGameID".into(), json!(ship_id));
            if let Some(name) = l
                .ship_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                ship_data.insert("shipName".into(), json!(name));
            }
            if let Some(ident) = l
                .ship_ident
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                ship_data.insert("shipIdent".into(), json!(ident));
            }
            ship_data.insert("isCurrentShip".into(), json!(true));
            if let Some(v) = raw
                .as_ref()
                .and_then(|r| r.get("HullValue"))
                .and_then(Value::as_u64)
            {
                ship_data.insert("shipHullValue".into(), json!(v));
            }
            if let Some(v) = raw
                .as_ref()
                .and_then(|r| r.get("ModulesValue"))
                .and_then(Value::as_u64)
            {
                ship_data.insert("shipModulesValue".into(), json!(v));
            }
            if let Some(v) = raw
                .as_ref()
                .and_then(|r| r.get("Hot"))
                .and_then(Value::as_bool)
            {
                ship_data.insert("isHot".into(), json!(v));
            }
            if let Some(v) = l.max_jump_range {
                ship_data.insert("shipMaxJumpRange".into(), json!(v));
            }
            if let Some(v) = l.cargo_capacity {
                ship_data.insert("shipCargoCapacity".into(), json!(v));
            }
            if let Some(v) = l.rebuy {
                ship_data.insert("shipRebuyValue".into(), json!(v));
            }
            out.push(inara_row(
                fid,
                "setCommanderShip",
                0,
                &ts,
                Value::Object(ship_data),
                Some(&key),
            ));

            let loadout_data = build_loadout_data(ship_type, ship_id, l, raw.as_ref());
            out.push(inara_row(
                fid,
                "setCommanderShipLoadout",
                1,
                &ts,
                loadout_data,
                Some(&key),
            ));
            true
        }
        EventKind::ShipyardSwap(sw) => {
            let raw = raw_value(input);
            let mut sub = 0u32;
            if let (Some(sell_type), Some(sell_id)) = (
                raw.as_ref()
                    .and_then(|r| r.get("SellOldShip"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                raw.as_ref()
                    .and_then(|r| r.get("SellShipID"))
                    .and_then(Value::as_u64),
            ) {
                out.push(inara_row(
                    fid,
                    "delCommanderShip",
                    sub,
                    &ts,
                    json!({
                        "shipType": sell_type,
                        "shipGameID": sell_id,
                    }),
                    None,
                ));
                sub += 1;
            }
            if let (Some(ship_type), Some(ship_id)) = (
                sw.ship_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                sw.ship_id,
            ) {
                let key = ship_id.to_string();
                out.push(inara_row(
                    fid,
                    "setCommanderShip",
                    sub,
                    &ts,
                    json!({
                        "shipType": ship_type,
                        "shipGameID": ship_id,
                        "isCurrentShip": true,
                    }),
                    Some(&key),
                ));
            }
            true
        }
        EventKind::Other => match input.ev.name.as_str() {
            "StoredShips" => {
                map_stored_ships(fid, &ts, input, out);
                true
            }
            "ShipyardNew" => {
                if let Some(raw) = raw_value(input)
                    && let (Some(ship_type), Some(ship_id)) = (
                        raw.get("ShipType")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty()),
                        raw.get("NewShipID").and_then(Value::as_u64),
                    )
                {
                    out.push(inara_row(
                        fid,
                        "addCommanderShip",
                        0,
                        &ts,
                        json!({
                            "shipType": ship_type,
                            "shipGameID": ship_id,
                        }),
                        None,
                    ));
                }
                true
            }
            "ShipyardSell" | "SellShipOnRebuy" => {
                if let Some(raw) = raw_value(input)
                    && let (Some(ship_type), Some(ship_id)) = (
                        raw.get("ShipType")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty()),
                        raw.get("SellShipID").and_then(Value::as_u64),
                    )
                {
                    out.push(inara_row(
                        fid,
                        "delCommanderShip",
                        0,
                        &ts,
                        json!({
                            "shipType": ship_type,
                            "shipGameID": ship_id,
                        }),
                        None,
                    ));
                }
                true
            }
            "ShipyardBuy" => {
                if let Some(raw) = raw_value(input)
                    && let (Some(ship_type), Some(ship_id)) = (
                        raw.get("SellOldShip")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|s| !s.is_empty()),
                        raw.get("SellShipID").and_then(Value::as_u64),
                    )
                {
                    out.push(inara_row(
                        fid,
                        "delCommanderShip",
                        0,
                        &ts,
                        json!({
                            "shipType": ship_type,
                            "shipGameID": ship_id,
                        }),
                        None,
                    ));
                }
                true
            }
            "ShipyardTransfer" => {
                map_shipyard_transfer(fid, &ts, input, out);
                true
            }
            "SetUserShipName" => {
                map_set_ship_name(fid, &ts, input, out);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn build_loadout_data(
    ship_type: &str,
    ship_id: u64,
    loadout: &Loadout,
    raw: Option<&Value>,
) -> Value {
    let raw_modules = raw
        .and_then(|r| r.get("Modules"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut modules_out = Vec::with_capacity(loadout.modules.len());
    for (idx, m) in loadout.modules.iter().enumerate() {
        let Some(slot) = m.slot.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let Some(item) = m.item.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let raw_mod = raw_modules.get(idx);

        let mut mod_obj = Map::new();
        mod_obj.insert("slotName".into(), json!(slot));
        mod_obj.insert("itemName".into(), json!(item));
        if let Some(val) = raw_mod.and_then(|r| r.get("Value")).and_then(Value::as_u64) {
            mod_obj.insert("itemValue".into(), json!(val));
        }
        if let Some(health) = m.health {
            mod_obj.insert("itemHealth".into(), json!(health));
        }
        if let Some(on) = m.on {
            mod_obj.insert("isOn".into(), json!(on));
        }
        if let Some(hot) = raw_mod.and_then(|r| r.get("Hot")).and_then(Value::as_bool) {
            mod_obj.insert("isHot".into(), json!(hot));
        }
        if let Some(prio) = m.priority {
            mod_obj.insert("itemPriority".into(), json!(prio));
        }
        if let Some(clip) = m.ammo_in_clip {
            mod_obj.insert("itemAmmoClip".into(), json!(clip));
        }
        if let Some(hopper) = m.ammo_in_hopper {
            mod_obj.insert("itemAmmoHopper".into(), json!(hopper));
        }
        if let Some(eng) = &m.engineering
            && let Some(bp) = eng
                .blueprint_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        {
            let mut eng_obj = Map::new();
            eng_obj.insert("blueprintName".into(), json!(bp));
            if let Some(lvl) = eng.level {
                eng_obj.insert("blueprintLevel".into(), json!(lvl));
            }
            if let Some(q) = eng.quality {
                eng_obj.insert("blueprintQuality".into(), json!(q));
            }
            if let Some(exp) = eng
                .experimental_effect
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                eng_obj.insert("experimentalEffect".into(), json!(exp));
            }
            if !eng.modifiers.is_empty() {
                let mods: Vec<Value> = eng
                    .modifiers
                    .iter()
                    .filter_map(|mf| {
                        let label = mf.label.trim();
                        let val = mf.value?;
                        if label.is_empty() {
                            return None;
                        }
                        let mut m_obj = Map::new();
                        m_obj.insert("name".into(), json!(label));
                        m_obj.insert("value".into(), json!(val));
                        if let Some(orig) = mf.original_value {
                            m_obj.insert("originalValue".into(), json!(orig));
                        }
                        if let Some(lig) = mf.less_is_good {
                            m_obj.insert("lessIsGood".into(), json!(lig != 0));
                        }
                        Some(Value::Object(m_obj))
                    })
                    .collect();
                if !mods.is_empty() {
                    eng_obj.insert("modifiers".into(), Value::Array(mods));
                }
            }
            mod_obj.insert("engineering".into(), Value::Object(eng_obj));
        }
        modules_out.push(Value::Object(mod_obj));
    }

    json!({
        "shipType": ship_type,
        "shipGameID": ship_id,
        "shipLoadout": modules_out,
    })
}

fn map_stored_ships(fid: &str, ts: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
    let Some(raw) = raw_value(input) else {
        return;
    };
    let here_sys = raw
        .get("StarSystem")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let here_station = raw
        .get("StationName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let here_market = raw.get("MarketID").and_then(Value::as_u64);

    let mut sub = 0u32;
    if let Some(here) = raw.get("ShipsHere").and_then(Value::as_array) {
        for entry in here {
            let Some(ship_type) = entry
                .get("ShipType")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let Some(ship_id) = entry.get("ShipID").and_then(Value::as_u64) else {
                continue;
            };
            let mut data = Map::new();
            data.insert("shipType".into(), json!(ship_type));
            data.insert("shipGameID".into(), json!(ship_id));
            if let Some(name) = entry
                .get("Name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("shipName".into(), json!(name));
            }
            if let Some(hot) = entry.get("Hot").and_then(Value::as_bool) {
                data.insert("isHot".into(), json!(hot));
            }
            if let Some(sys) = here_sys {
                data.insert("starsystemName".into(), json!(sys));
            }
            if let Some(st) = here_station {
                data.insert("stationName".into(), json!(st));
            }
            if let Some(mk) = here_market {
                data.insert("marketID".into(), json!(mk));
            }
            let key = ship_id.to_string();
            out.push(inara_row(
                fid,
                "setCommanderShip",
                sub,
                ts,
                Value::Object(data),
                Some(&key),
            ));
            sub += 1;
        }
    }

    if let Some(remote) = raw.get("ShipsRemote").and_then(Value::as_array) {
        for entry in remote {
            let Some(ship_type) = entry
                .get("ShipType")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let Some(ship_id) = entry.get("ShipID").and_then(Value::as_u64) else {
                continue;
            };
            let in_transit = entry
                .get("InTransit")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut data = Map::new();
            data.insert("shipType".into(), json!(ship_type));
            data.insert("shipGameID".into(), json!(ship_id));
            if let Some(name) = entry
                .get("Name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                data.insert("shipName".into(), json!(name));
            }
            if let Some(hot) = entry.get("Hot").and_then(Value::as_bool) {
                data.insert("isHot".into(), json!(hot));
            }
            if !in_transit {
                if let Some(sys) = entry
                    .get("StarSystem")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    data.insert("starsystemName".into(), json!(sys));
                }
                if let Some(st) = entry
                    .get("StationName")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    data.insert("stationName".into(), json!(st));
                }
                if let Some(mk) = entry
                    .get("ShipMarketID")
                    .or_else(|| entry.get("MarketID"))
                    .and_then(Value::as_u64)
                {
                    data.insert("marketID".into(), json!(mk));
                }
            }
            let key = ship_id.to_string();
            out.push(inara_row(
                fid,
                "setCommanderShip",
                sub,
                ts,
                Value::Object(data),
                Some(&key),
            ));
            sub += 1;
        }
    }
}

fn map_shipyard_transfer(fid: &str, ts: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
    let Some(raw) = raw_value(input) else {
        return;
    };
    let Some(ship_type) = raw
        .get("ShipType")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };
    let Some(ship_id) = raw.get("ShipID").and_then(Value::as_u64) else {
        return;
    };
    let Some(sys) = input
        .snap
        .system_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };

    let mut data = Map::new();
    data.insert("shipType".into(), json!(ship_type));
    data.insert("shipGameID".into(), json!(ship_id));
    data.insert("starsystemName".into(), json!(sys));
    if let Some(st) = input
        .snap
        .station_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        data.insert("stationName".into(), json!(st));
    }
    if let Some(mk) = input.snap.market_id {
        data.insert("marketID".into(), json!(mk));
    }
    if let Some(secs) = raw.get("TransferTime").and_then(Value::as_u64) {
        data.insert("transferTime".into(), json!(secs));
    }
    out.push(inara_row(
        fid,
        "setCommanderShipTransfer",
        0,
        ts,
        Value::Object(data),
        None,
    ));
}

fn map_set_ship_name(fid: &str, ts: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
    let raw = raw_value(input);
    let ship_type = raw
        .as_ref()
        .and_then(|r| r.get("Ship"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(input.snap.ship_type.as_deref());
    let ship_id = raw
        .as_ref()
        .and_then(|r| r.get("ShipID"))
        .and_then(Value::as_u64)
        .or(input.snap.ship_id);
    let (Some(ship_type), Some(ship_id)) = (ship_type, ship_id) else {
        return;
    };
    let mut data = Map::new();
    data.insert("shipType".into(), json!(ship_type));
    data.insert("shipGameID".into(), json!(ship_id));
    if let Some(name) = raw
        .as_ref()
        .and_then(|r| r.get("UserShipName"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        data.insert("shipName".into(), json!(name));
    }
    if let Some(ident) = raw
        .as_ref()
        .and_then(|r| r.get("UserShipId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        data.insert("shipIdent".into(), json!(ident));
    }
    data.insert("isCurrentShip".into(), json!(true));
    let key = ship_id.to_string();
    out.push(inara_row(
        fid,
        "setCommanderShip",
        0,
        ts,
        Value::Object(data),
        Some(&key),
    ));
}

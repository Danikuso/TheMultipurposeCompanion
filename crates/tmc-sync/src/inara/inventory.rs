//! Mapeo de inventarios hacia Inara (`setCommanderInventoryMaterials`,
//! `setCommanderInventoryCargo`, `setCommanderInventory` / `resetCommanderInventory`).

use serde_json::{Map, Value, json};
use tmc_model::companion::{CompanionFile, CompanionKind};
use tmc_model::events::{EventKind, ShipLockerContents};

use super::{inara_row, ts_iso};
use crate::router::{MappedInput, MappedRow};
use crate::tracker::locker_items_from_contents;

pub fn map_inventory(fid: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) -> bool {
    match &input.ev.kind {
        EventKind::Materials(_)
        | EventKind::MaterialCollected(_)
        | EventKind::MaterialDiscarded(_)
        | EventKind::MaterialTrade(_)
        | EventKind::Synthesis(_)
        | EventKind::EngineerCraft(_)
        | EventKind::TechnologyBroker(_) => {
            emit_materials(fid, input, 0, out);
            true
        }
        EventKind::EngineerContribution(ec) => {
            if ec.material.is_some() {
                emit_materials(fid, input, 0, out);
                return true;
            }
            false
        }
        EventKind::Cargo(c) => {
            map_cargo(fid, input, c, out);
            true
        }
        EventKind::ShipLocker(sl) => {
            map_ship_locker(fid, input, sl, out);
            true
        }
        _ => false,
    }
}

pub(crate) fn emit_materials(
    fid: &str,
    input: &MappedInput<'_>,
    sub: u32,
    out: &mut Vec<MappedRow>,
) {
    let Some(mats) = &input.snap.materials else {
        // Sin evento `Materials` previo en el tracker, nunca enviamos lista parcial.
        return;
    };
    let items: Vec<Value> = mats
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
        sub,
        &ts_iso(input),
        Value::Array(items),
        Some("materials"),
    ));
}

fn map_cargo(
    fid: &str,
    input: &MappedInput<'_>,
    cargo: &tmc_model::events::Cargo,
    out: &mut Vec<MappedRow>,
) {
    if cargo
        .vessel
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("SRV"))
    {
        return;
    }

    let items: Vec<Value> = if let Some(inv) = &cargo.inventory {
        inv.iter()
            .filter(|i| i.count > 0 && !i.name.trim().is_empty())
            .map(|i| cargo_item_json(&i.name, i.count, i.stolen_count() > 0, i.mission_id))
            .collect()
    } else if let Some(CompanionFile::Cargo(cf)) = input.companion {
        if cf
            .vessel
            .as_deref()
            .is_some_and(|v| v.eq_ignore_ascii_case("SRV"))
        {
            return;
        }
        cf.inventory
            .iter()
            .filter(|i| i.count > 0 && !i.name.trim().is_empty())
            .map(|i| cargo_item_json(&i.name, i.count, i.stolen > 0, i.mission_id))
            .collect()
    } else {
        return;
    };

    out.push(inara_row(
        fid,
        "setCommanderInventoryCargo",
        0,
        &ts_iso(input),
        Value::Array(items),
        Some("cargo"),
    ));
}

fn cargo_item_json(name: &str, count: u32, is_stolen: bool, mission_id: Option<u64>) -> Value {
    let mut obj = Map::new();
    obj.insert("itemName".into(), json!(name.trim()));
    obj.insert("itemCount".into(), json!(count));
    obj.insert("isStolen".into(), json!(is_stolen));
    if let Some(mid) = mission_id {
        obj.insert("missionGameID".into(), json!(mid));
    }
    Value::Object(obj)
}

fn map_ship_locker(
    fid: &str,
    input: &MappedInput<'_>,
    sl: &ShipLockerContents,
    out: &mut Vec<MappedRow>,
) {
    let owned;
    let contents: &ShipLockerContents = if !sl.is_stub() {
        sl
    } else if let Some(CompanionFile::Raw(CompanionKind::ShipLocker, val)) = input.companion {
        let Some(parsed) = ShipLockerContents::from_value(val).filter(|c| !c.is_stub()) else {
            return;
        };
        owned = parsed;
        &owned
    } else {
        return;
    };

    let items = locker_items_from_contents(contents);
    let ts = ts_iso(input);
    if items.is_empty() {
        out.push(inara_row(
            fid,
            "resetCommanderInventory",
            0,
            &ts,
            json!({ "itemLocation": "ShipLocker" }),
            Some("ShipLocker"),
        ));
        return;
    }

    let payload: Vec<Value> = items
        .into_iter()
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
        Value::Array(payload),
        Some("ShipLocker"),
    ));
}

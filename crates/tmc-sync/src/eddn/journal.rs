//! Esquema `https://eddn.edcd.io/schemas/journal/1`.

use super::OmitReason;
use super::sanitize::{
    augment_horizons_odyssey, crosscheck_and_augment_location, strip_localised, validate_local,
};
use crate::tracker::TransientSnapshot;
use serde_json::Value;
use tmc_model::events::JournalEvent;

const DISALLOWED_TOP_LEVEL: &[&str] = &[
    "ActiveFine",
    "CockpitBreach",
    "BoostUsed",
    "FuelLevel",
    "FuelUsed",
    "JumpDist",
    "Latitude",
    "Longitude",
    "Wanted",
    "IsNewEntry",
    "NewTraitsDiscovered",
    "Traits",
    "VoucherAmount",
];

const DISALLOWED_FACTION_FIELDS: &[&str] = &[
    "HappiestSystem",
    "HomeSystem",
    "MyReputation",
    "SquadronFaction",
];

/// Sanea eventos del esquema `journal/1` (`Docked`, `FSDJump`, `Scan`, `Location`, `SAASignalsFound`, `CarrierJump`).
pub fn map_journal(
    ev: &JournalEvent,
    snap: &TransientSnapshot,
) -> Option<Result<Value, OmitReason>> {
    match ev.name.as_str() {
        "Docked" | "FSDJump" | "Scan" | "Location" | "SAASignalsFound" | "CarrierJump" => {}
        _ => return None,
    }

    let Ok(mut val) = serde_json::from_str::<Value>(ev.raw.get()) else {
        return Some(Err(OmitReason::Inconsistent));
    };
    strip_localised(&mut val);

    let Some(obj) = val.as_object_mut() else {
        return Some(Err(OmitReason::Inconsistent));
    };

    for key in DISALLOWED_TOP_LEVEL {
        obj.remove(*key);
    }
    if let Some(factions) = obj.get_mut("Factions").and_then(Value::as_array_mut) {
        for faction in factions {
            if let Some(fobj) = faction.as_object_mut() {
                for key in DISALLOWED_FACTION_FIELDS {
                    fobj.remove(*key);
                }
            }
        }
    }

    if !crosscheck_and_augment_location(obj, snap, Some("StarSystem"), true, true) {
        return Some(Err(OmitReason::Inconsistent));
    }

    augment_horizons_odyssey(obj, snap);

    if let Err(path) = validate_local(&val) {
        return Some(Err(OmitReason::LocalValidation(path)));
    }
    Some(Ok(val))
}

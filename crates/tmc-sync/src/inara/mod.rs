//! Mapeo, coalescencia, instantánea y trabajador de Inara (InAPI v1).

pub mod combat;
pub mod commander;
pub mod inventory;
pub mod missions;
pub mod ship;
pub mod travel;

use crate::router::{MappedInput, MappedRow, ServiceMapper};
use serde_json::{Value, json};
use tmc_model::events::{EventKind, JournalEvent};
use tmc_model::sync::{OutboxState, SyncService};

#[derive(Debug, Default)]
pub struct InaraMapper;

impl InaraMapper {
    pub fn new() -> Self {
        Self
    }
}

pub(crate) fn raw_value(input: &MappedInput<'_>) -> Option<Value> {
    serde_json::from_str(input.ev.raw.get()).ok()
}

pub(crate) fn ts_iso(input: &MappedInput<'_>) -> String {
    input
        .ev
        .timestamp
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub(crate) fn inara_row(
    fid: &str,
    event_name: &str,
    sub: u32,
    timestamp: &str,
    event_data: Value,
    coalesce_segment: Option<&str>,
) -> MappedRow {
    let coalesce_key = coalesce_segment.map(|k| format!("{fid}|{event_name}|{k}"));
    MappedRow {
        kind: event_name.to_string(),
        sub,
        payload: json!({
            "eventName": event_name,
            "eventTimestamp": timestamp,
            "eventData": event_data,
        }),
        coalesce_key,
        state: OutboxState::Pending,
        class: None,
    }
}

impl ServiceMapper for InaraMapper {
    fn service(&self) -> SyncService {
        SyncService::Inara
    }

    fn wants_companion(&self, ev: &JournalEvent) -> bool {
        match &ev.kind {
            EventKind::Cargo(c) => {
                !c.vessel
                    .as_deref()
                    .is_some_and(|v| v.eq_ignore_ascii_case("SRV"))
                    && c.inventory.is_none()
            }
            EventKind::ShipLocker(sl) => sl.is_stub(),
            _ => false,
        }
    }

    fn map(&mut self, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
        let Some(fid) = input
            .snap
            .cmdr_fid
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return;
        };

        if travel::map_travel(fid, input, out) {
            return;
        }
        if matches!(input.ev.kind, EventKind::LoadGame(_)) {
            commander::map_commander(fid, input, out);
            ship::map_ship(fid, input, out);
            return;
        }
        commander::map_commander(fid, input, out);
        if ship::map_ship(fid, input, out) {
            return;
        }
        if inventory::map_inventory(fid, input, out) {
            return;
        }
        if missions::map_missions(fid, input, out) {
            return;
        }
        combat::map_combat(fid, input, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::support::ev;
    use crate::tracker::TransientTracker;
    use serde_json::Value;

    fn load_event_fixture(name: &str) -> Value {
        let path = format!(
            "{}/../../tests/fixtures/sync/inara/events/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn apply_and_map(
        tracker: &mut TransientTracker,
        mapper: &mut InaraMapper,
        raw_ev: &Value,
    ) -> Vec<MappedRow> {
        let line = serde_json::to_string(raw_ev).unwrap();
        let jev = ev(&line);
        tracker.apply(&jev);
        let snap = tracker.snapshot();
        let input = MappedInput {
            ev: &jev,
            file: "Journal.2026-11-10.01.log",
            line: 1,
            snap: &snap,
            companion: None,
            tracker,
        };
        let mut out = Vec::new();
        mapper.map(&input, &mut out);
        out
    }

    fn seed_context(tracker: &mut TransientTracker, ctx: Option<&Value>) {
        let cmdr = json!({
            "timestamp": "2026-11-10T17:55:00Z",
            "event": "LoadGame",
            "Commander": "ExampleCmdr",
            "FID": "F000001",
            "Ship": ctx.and_then(|c| c.get("shipType")).and_then(Value::as_str).unwrap_or("sidewinder"),
            "ShipID": ctx.and_then(|c| c.get("shipGameID")).and_then(Value::as_u64).unwrap_or(1),
            "Credits": 1000
        });
        tracker.apply(&ev(&cmdr.to_string()));

        if let Some(coords) = ctx.and_then(|c| c.get("starsystemCoords")) {
            let loc = json!({
                "timestamp": "2026-11-10T17:56:00Z",
                "event": "Location",
                "StarSystem": ctx.and_then(|c| c.get("starsystemName")).and_then(Value::as_str).unwrap_or("Sol"),
                "SystemAddress": 10477373803u64,
                "StarPos": coords,
                "Docked": false
            });
            tracker.apply(&ev(&loc.to_string()));
        }
    }

    #[test]
    fn travel_and_commander_fixtures_match() {
        let names = [
            "addCommanderTravelFSDJump",
            "addCommanderTravelCarrierJump",
            "addCommanderTravelDock",
            "addCommanderTravelLand",
            "setCommanderTravelLocation",
            "setCommanderRankPilot",
            "setCommanderRankEngineer",
            "setCommanderRankPower",
            "setCommanderReputationMajorFaction",
            "setCommanderReputationMinorFaction",
            "setCommanderCredits",
        ];

        for name in names {
            let fix = load_event_fixture(name);
            let mut tracker = TransientTracker::new();
            let mut mapper = InaraMapper::new();
            seed_context(&mut tracker, fix.get("context"));

            let journal_entries = match &fix["journal"] {
                Value::Array(arr) => arr.clone(),
                obj => vec![obj.clone()],
            };

            let mut all_rows = Vec::new();
            for entry in &journal_entries {
                all_rows.extend(apply_and_map(&mut tracker, &mut mapper, entry));
            }

            let expected = &fix["inara"];
            let expected_name = expected["eventName"].as_str().unwrap();
            let matching: Vec<&MappedRow> = all_rows
                .iter()
                .filter(|r| r.kind == expected_name)
                .collect();
            let actual = matching
                .last()
                .unwrap_or_else(|| panic!("falta fila para {name}: {all_rows:?}"));
            assert_eq!(actual.payload, *expected, "mismatch en {name}");
            if expected_name.starts_with("set") {
                assert!(
                    actual
                        .coalesce_key
                        .as_deref()
                        .is_some_and(|k| k.starts_with("F000001|")),
                    "set* debe tener coalesce_key con fid en {name}"
                );
            } else {
                assert!(
                    actual.coalesce_key.is_none(),
                    "add* no debe coalescer en {name}"
                );
            }
        }
    }

    #[test]
    fn location_never_emits_dock() {
        let mut tracker = TransientTracker::new();
        let mut mapper = InaraMapper::new();
        seed_context(&mut tracker, None);
        let rows = apply_and_map(
            &mut tracker,
            &mut mapper,
            &json!({
                "timestamp": "2026-11-10T18:00:00Z",
                "event": "Location",
                "Docked": true,
                "StarSystem": "Sol",
                "SystemAddress": 10477373803u64,
                "StarPos": [0.0, 0.0, 0.0],
                "StationName": "Galileo",
                "MarketID": 128004520
            }),
        );
        assert!(rows.iter().any(|r| r.kind == "setCommanderTravelLocation"));
        assert!(!rows.iter().any(|r| r.kind == "addCommanderTravelDock"));
    }

    #[test]
    fn credits_only_from_loadgame() {
        let mut tracker = TransientTracker::new();
        let mut mapper = InaraMapper::new();
        seed_context(&mut tracker, None);
        for ev in [
            json!({"timestamp": "2026-11-10T18:01:00Z", "event": "MarketSell", "Type": "gold", "Count": 1, "SellPrice": 50000, "TotalSale": 50000}),
            json!({"timestamp": "2026-11-10T18:02:00Z", "event": "RedeemVoucher", "Type": "bounty", "Amount": 100000}),
        ] {
            let rows = apply_and_map(&mut tracker, &mut mapper, &ev);
            assert!(
                !rows.iter().any(|r| r.kind == "setCommanderCredits"),
                "no debe emitir setCommanderCredits fuera de LoadGame"
            );
        }

        let lg_rows = apply_and_map(
            &mut tracker,
            &mut mapper,
            &json!({"timestamp": "2026-11-10T18:03:00Z", "event": "LoadGame", "Commander": "ExampleCmdr", "FID": "F000001", "Credits": 500000, "Loan": 0}),
        );
        assert!(lg_rows.iter().any(|r| r.kind == "setCommanderCredits"));
    }

    #[test]
    fn fsdjump_emits_travel_and_minor_reputation_with_distinct_subs() {
        let mut tracker = TransientTracker::new();
        let mut mapper = InaraMapper::new();
        seed_context(&mut tracker, None);
        let rows = apply_and_map(
            &mut tracker,
            &mut mapper,
            &json!({
                "timestamp": "2026-11-10T18:04:00Z",
                "event": "FSDJump",
                "StarSystem": "Sol",
                "SystemAddress": 10477373803u64,
                "StarPos": [0.0, 0.0, 0.0],
                "JumpDist": 8.5,
                "Factions": [
                    {"Name": "Faction A", "MyReputation": 50.0},
                    {"Name": "Faction B", "MyReputation": -25.0}
                ]
            }),
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].kind, "addCommanderTravelFSDJump");
        assert_eq!(rows[0].sub, 0);
        assert_eq!(rows[1].kind, "setCommanderReputationMinorFaction");
        assert_eq!(rows[1].sub, 1);
        assert_eq!(rows[2].kind, "setCommanderReputationMinorFaction");
        assert_eq!(rows[2].sub, 2);
    }

    #[test]
    fn ship_inventory_mission_combat_fixtures_match() {
        let names = [
            "setCommanderShip",
            "storedShips",
            "setCommanderShipLoadout",
            "addCommanderShip",
            "delCommanderShip",
            "setCommanderShipTransfer",
            "setCommanderInventoryMaterials",
            "setCommanderInventoryCargo",
            "setCommanderInventory",
            "addCommanderMission",
            "setCommanderMissionCompleted",
            "setCommanderMissionFailed",
            "setCommanderMissionAbandoned",
            "addCommanderCombatDeath",
            "addCommanderCombatInterdicted",
            "addCommanderCombatInterdiction",
            "addCommanderCombatInterdictionEscape",
        ];

        for name in names {
            let fix = load_event_fixture(name);
            let mut tracker = TransientTracker::new();
            let mut mapper = InaraMapper::new();
            seed_context(&mut tracker, fix.get("context"));

            if let Some(ctx) = fix.get("context")
                && let (Some(sys), Some(st)) = (
                    ctx.get("starsystemNameOrigin").and_then(Value::as_str),
                    ctx.get("stationNameOrigin").and_then(Value::as_str),
                )
            {
                tracker.apply(&ev(&json!({
                    "timestamp": "2026-11-10T17:57:00Z",
                    "event": "Location",
                    "StarSystem": sys,
                    "SystemAddress": 10477373803u64,
                    "StarPos": [0.0, 0.0, 0.0],
                    "Docked": true,
                    "StationName": st,
                    "MarketID": ctx.get("marketID").and_then(Value::as_u64).unwrap_or(128004512u64)
                })
                .to_string()));
            }
            if let Some(ctx) = fix.get("context")
                && let (Some(sys), Some(st)) = (
                    ctx.get("starsystemName").and_then(Value::as_str),
                    ctx.get("stationName").and_then(Value::as_str),
                )
            {
                tracker.apply(&ev(&json!({
                    "timestamp": "2026-11-10T17:57:00Z",
                    "event": "Location",
                    "StarSystem": sys,
                    "SystemAddress": 10477373803u64,
                    "StarPos": [0.0, 0.0, 0.0],
                    "Docked": true,
                    "StationName": st,
                    "MarketID": ctx.get("marketID").and_then(Value::as_u64).unwrap_or(128004512u64)
                })
                .to_string()));
            } else if let Some(ctx) = fix.get("context")
                && let Some(sys) = ctx.get("starsystemName").and_then(Value::as_str)
            {
                tracker.apply(&ev(&json!({
                    "timestamp": "2026-11-10T17:57:00Z",
                    "event": "Location",
                    "StarSystem": sys,
                    "SystemAddress": 10477373803u64,
                    "StarPos": [0.0, 0.0, 0.0],
                    "Docked": false
                })
                .to_string()));
            }

            let journal_entries = match &fix["journal"] {
                Value::Array(arr) => arr.clone(),
                obj => vec![obj.clone()],
            };

            let mut all_rows = Vec::new();
            for entry in &journal_entries {
                all_rows.extend(apply_and_map(&mut tracker, &mut mapper, entry));
            }

            let expected_list: Vec<Value> = match &fix["inara"] {
                Value::Array(arr) => arr.clone(),
                obj => vec![obj.clone()],
            };

            for expected in &expected_list {
                let expected_name = expected["eventName"].as_str().unwrap();
                let found = all_rows
                    .iter()
                    .rev()
                    .find(|r| r.kind == expected_name && r.payload == *expected);
                assert!(
                    found.is_some(),
                    "mismatch o ausencia para {name} esperando {expected}; generado: {all_rows:?}"
                );
            }
        }
    }

    #[test]
    fn materials_sent_as_full_set_after_delta_events() {
        let mut tracker = TransientTracker::new();
        let mut mapper = InaraMapper::new();
        seed_context(&mut tracker, None);

        // Delta sin Materials previo no debe mandar lista parcial a Inara
        let early = apply_and_map(
            &mut tracker,
            &mut mapper,
            &json!({"timestamp": "2026-11-10T18:00:00Z", "event": "MaterialCollected", "Category": "Raw", "Name": "iron", "Count": 3}),
        );
        assert!(early.is_empty());

        let base = apply_and_map(
            &mut tracker,
            &mut mapper,
            &json!({
                "timestamp": "2026-11-10T18:01:00Z",
                "event": "Materials",
                "Raw": [{"Name": "iron", "Count": 120}, {"Name": "nickel", "Count": 40}],
                "Manufactured": [],
                "Encoded": []
            }),
        );
        assert_eq!(base.len(), 1);
        assert_eq!(base[0].kind, "setCommanderInventoryMaterials");

        let delta = apply_and_map(
            &mut tracker,
            &mut mapper,
            &json!({"timestamp": "2026-11-10T18:02:00Z", "event": "MaterialCollected", "Category": "Raw", "Name": "iron", "Count": 3}),
        );
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0].kind, "setCommanderInventoryMaterials");
        let items = delta[0].payload["eventData"].as_array().unwrap();
        assert!(items.contains(&json!({"itemName": "iron", "itemCount": 123})));
        assert!(items.contains(&json!({"itemName": "nickel", "itemCount": 40})));
    }

    #[test]
    fn cargo_from_companion_cargo_json() {
        use tmc_model::companion::{CargoFile, CargoItem, CompanionFile};

        let mut tracker = TransientTracker::new();
        let mut mapper = InaraMapper::new();
        seed_context(&mut tracker, None);

        let stub =
            ev(r#"{"timestamp":"2026-11-10T18:00:00Z","event":"Cargo","Vessel":"Ship","Count":5}"#);
        assert!(mapper.wants_companion(&stub));

        let cargo_comp = CompanionFile::Cargo(CargoFile {
            vessel: Some("Ship".into()),
            count: 5,
            inventory: vec![CargoItem {
                name: "gold".into(),
                name_localised: None,
                count: 5,
                stolen: 1,
                mission_id: Some(99),
            }],
        });
        tracker.apply(&stub);
        let snap = tracker.snapshot();
        let input = MappedInput {
            ev: &stub,
            file: "Journal.2026-11-10.01.log",
            line: 1,
            snap: &snap,
            companion: Some(&cargo_comp),
            tracker: &tracker,
        };
        let mut out = Vec::new();
        mapper.map(&input, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "setCommanderInventoryCargo");
        assert_eq!(
            out[0].payload["eventData"],
            json!([{"itemName": "gold", "itemCount": 5, "isStolen": true, "missionGameID": 99}])
        );
    }

    #[test]
    fn out_of_scope_events_never_mapped() {
        let mut tracker = TransientTracker::new();
        let mut mapper = InaraMapper::new();
        seed_context(&mut tracker, None);

        for raw in [
            json!({"timestamp": "2026-11-10T18:00:00Z", "event": "Statistics", "Bank_Account": {"Current_Wealth": 1000}}),
            json!({"timestamp": "2026-11-10T18:01:00Z", "event": "StoredModules", "Items": []}),
            json!({"timestamp": "2026-11-10T18:02:00Z", "event": "CommunityGoal", "CurrentGoals": []}),
            json!({"timestamp": "2026-11-10T18:03:00Z", "event": "PVPKill", "Victim": "Cmdr Foo", "CombatRank": 5}),
        ] {
            let rows = apply_and_map(&mut tracker, &mut mapper, &raw);
            assert!(
                rows.is_empty(),
                "evento fuera de alcance DR-29 generó filas: {rows:?}"
            );
        }
    }
}

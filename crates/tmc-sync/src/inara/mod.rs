//! Mapeo, coalescencia, instantánea y trabajador de Inara (InAPI v1).

pub mod coalesce;
pub mod combat;
pub mod commander;
pub mod inventory;
pub mod missions;
pub mod ship;
pub mod snapshot;
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

    fn snapshot(
        &mut self,
        tracker: &crate::tracker::TransientTracker,
        fid: &str,
        ts: chrono::DateTime<chrono::Utc>,
    ) -> Vec<MappedRow> {
        snapshot::build_snapshot(tracker, fid, ts)
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

    #[test]
    fn coalesce_keeps_latest_set_and_never_merges_add() {
        use chrono::{TimeZone, Utc};
        use tmc_model::sync::RowOrigin;
        use tmc_store::OutboxRow;

        let t0 = Utc.with_ymd_and_hms(2026, 11, 10, 18, 0, 0).unwrap();
        let t1 = Utc.with_ymd_and_hms(2026, 11, 10, 18, 1, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 11, 10, 18, 2, 0).unwrap();
        let row = |id: i64, kind: &str, ck: Option<&str>, ts| OutboxRow {
            id,
            service: SyncService::Inara,
            cmdr_fid: "F000001".into(),
            kind: kind.into(),
            coalesce_key: ck.map(str::to_string),
            event_ts: ts,
            payload: format!(r#"{{"eventName":"{kind}","id":{id}}}"#),
            origin: RowOrigin::Live,
            attempts: 0,
        };

        let input = vec![
            row(
                1,
                "setCommanderCredits",
                Some("F000001|setCommanderCredits|credits"),
                t0,
            ),
            row(2, "addCommanderTravelFSDJump", Some("F000001|add|x"), t0),
            row(3, "addCommanderTravelFSDJump", Some("F000001|add|x"), t1),
            row(
                4,
                "setCommanderCredits",
                Some("F000001|setCommanderCredits|credits"),
                t2,
            ),
            row(
                5,
                "setCommanderShip",
                Some("F000001|setCommanderShip|4"),
                t1,
            ),
            row(6, "addCommanderTravelDock", None, t1),
        ];

        let (kept, superseded) = coalesce::coalesce(input);
        let kept_ids: Vec<i64> = kept.iter().map(|r| r.id).collect();
        assert_eq!(kept_ids, vec![2, 3, 4, 5, 6]);
        assert_eq!(superseded, vec![1]);
    }

    #[test]
    fn snapshot_contains_only_set_events() {
        use chrono::{TimeZone, Utc};

        let mut tracker = TransientTracker::new();
        for raw in [
            json!({"timestamp": "2026-11-10T18:00:00Z", "event": "LoadGame", "Commander": "ExampleCmdr", "FID": "F000001", "Ship": "krait_mkii", "ShipID": 4, "ShipName": "Nostromo", "Credits": 500000, "Loan": 0}),
            json!({"timestamp": "2026-11-10T18:00:10Z", "event": "Location", "StarSystem": "Sol", "SystemAddress": 10477373803u64, "StarPos": [0.0, 0.0, 0.0], "Docked": true, "StationName": "Galileo", "MarketID": 128004520}),
            json!({"timestamp": "2026-11-10T18:00:20Z", "event": "Rank", "Combat": 3, "Trade": 5, "Explore": 4}),
            json!({"timestamp": "2026-11-10T18:00:30Z", "event": "Progress", "Combat": 25, "Trade": 50, "Explore": 75}),
            json!({"timestamp": "2026-11-10T18:00:40Z", "event": "Reputation", "Federation": 75.0}),
            json!({"timestamp": "2026-11-10T18:00:50Z", "event": "Materials", "Raw": [{"Name": "iron", "Count": 10}], "Manufactured": [], "Encoded": []}),
            json!({"timestamp": "2026-11-10T18:01:00Z", "event": "Cargo", "Vessel": "Ship", "Count": 2, "Inventory": [{"Name": "gold", "Count": 2, "Stolen": 0}]}),
            json!({"timestamp": "2026-11-10T18:01:10Z", "event": "Loadout", "Ship": "krait_mkii", "ShipID": 4, "Modules": []}),
        ] {
            tracker.apply(&ev(&raw.to_string()));
        }

        let at = Utc.with_ymd_and_hms(2026, 11, 10, 18, 5, 0).unwrap();
        let rows = snapshot::build_snapshot(&tracker, "F000001", at);
        assert!(
            rows.len() >= 7,
            "instantánea debe cubrir rangos, reputación, créditos, nave, loadout, materiales, carga y ubicación: {rows:?}"
        );
        assert!(
            rows.iter().all(|r| r.kind.starts_with("set")),
            "solo eventos set* permitidos en la instantánea: {rows:?}"
        );
        assert!(rows.iter().all(|r| r.coalesce_key.is_some()));
        let kinds: Vec<&str> = rows.iter().map(|r| r.kind.as_str()).collect();
        for expected in [
            "setCommanderRankPilot",
            "setCommanderReputationMajorFaction",
            "setCommanderCredits",
            "setCommanderShip",
            "setCommanderShipLoadout",
            "setCommanderInventoryMaterials",
            "setCommanderInventoryCargo",
            "setCommanderTravelLocation",
        ] {
            assert!(kinds.contains(&expected), "falta {expected} en {kinds:?}");
        }
    }

    #[test]
    fn snapshot_once_per_start() {
        use crate::clock::TestClock;
        use crate::router::{EnvGates, LiveSignalPolicy, Router, RouterConfig};
        use chrono::{TimeZone, Utc};
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;
        use tmc_model::state::Galaxy;
        use tmc_model::sync::{FeedOrigin, KeyAssignment, SyncFeed, SyncFeedEvent};
        use tmc_store::{Store, SyncCredential};

        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in_memory().unwrap();
        let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 11, 10, 18, 0, 0).unwrap());
        let activated = Utc.with_ymd_and_hms(2026, 11, 10, 17, 0, 0).unwrap();
        store
            .upsert_known_cmdr("F000001", "CMDR", activated)
            .unwrap();
        store
            .put_sync_credential(&SyncCredential {
                service: SyncService::Inara,
                cmdr_fid: "F000001".into(),
                cmdr_name: "CMDR".into(),
                api_key: "k1".into(),
                created_at: "2026-11-10T17:00:00Z".into(),
                verified_at: Some("2026-11-10T17:00:00Z".into()),
                assignment: KeyAssignment::Confirmed,
            })
            .unwrap();
        store
            .set_activation(SyncService::Inara, "F000001", activated)
            .unwrap();

        let mut router = Router::new(
            store.clone(),
            clock,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(true)),
            RouterConfig {
                journal_dir: dir.path().to_path_buf(),
                env: EnvGates {
                    e2e_simulation: false,
                },
                live_signal: LiveSignalPolicy::ProcessCheck,
                flush_every: std::time::Duration::from_millis(250),
                flush_rows: 200,
                hold_before_process: (200, std::time::Duration::from_secs(10)),
            },
            vec![Box::new(InaraMapper::new())],
        );

        let file = "Journal.2026-11-10T180000.01.log";
        let send = |r: &mut Router, event| {
            r.handle(SyncFeed {
                origin: FeedOrigin::Live,
                galaxy: Galaxy::Live,
                game_running: true,
                event,
            })
            .unwrap();
        };

        send(
            &mut router,
            SyncFeedEvent::Journal {
                ev: ev(
                    r#"{"timestamp":"2026-11-10T16:50:00Z","event":"LoadGame","FID":"F000001","Commander":"CMDR","Ship":"asp","ShipID":1,"Credits":1234,"gameversion":"4.1.0.0"}"#,
                ),
                file: file.into(),
                line: 1,
            },
        );
        send(&mut router, SyncFeedEvent::CatchUpDone);
        send(&mut router, SyncFeedEvent::GameProcess { running: true });
        router.flush().unwrap();

        let count_snap = || {
            store
                .query_json(
                    "SELECT kind FROM sync_outbox WHERE service='inara' AND kind='setCommanderCredits'",
                    &[],
                )
                .unwrap()
                .len()
        };
        assert_eq!(
            count_snap(),
            1,
            "debe emitir una instantánea al cumplirse CatchUpDone+running+activo"
        );

        // Eventos siguientes en la misma sesión NO disparan otra instantánea.
        send(&mut router, SyncFeedEvent::GameProcess { running: true });
        send(
            &mut router,
            SyncFeedEvent::Journal {
                ev: ev(
                    r#"{"timestamp":"2026-11-10T18:05:00Z","event":"Docked","StarSystem":"Sol","StationName":"Galileo","MarketID":1}"#,
                ),
                file: file.into(),
                line: 2,
            },
        );
        router.flush().unwrap();
        assert_eq!(
            count_snap(),
            1,
            "solo una instantánea por arranque y comandante"
        );
    }
}

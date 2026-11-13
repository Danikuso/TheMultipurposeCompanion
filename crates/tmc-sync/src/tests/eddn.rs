use crate::clock::Clock;
use crate::eddn::EddnMapper;
use crate::eddn::sanitize::{strip_localised, validate_local};
use crate::router::{MappedInput, MappedRow, ServiceMapper};
use crate::tracker::TransientTracker;
use serde_json::json;
use std::path::{Path, PathBuf};
use tmc_model::events::parse_line;
use tmc_model::sync::OutboxState;
use tmc_net::eddn_upload::SchemaMode;

fn eddn_fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests/fixtures/sync/eddn")
}

fn run_mapper_fixture(dir: &Path) -> Vec<MappedRow> {
    let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
    let mut tracker = TransientTracker::new();
    let mut mapper = EddnMapper::new(SchemaMode::Live, "test-salt");
    let mut rows = Vec::new();
    for (idx, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let ev = parse_line(trimmed).unwrap();
        tracker.apply(&ev);
        let snap = tracker.snapshot();
        mapper.map(
            &MappedInput {
                ev: &ev,
                file: "Journal.2026-01-01T120000.01.log",
                line: (idx + 1) as u64,
                snap: &snap,
                companion: None,
                tracker: &tracker,
            },
            &mut rows,
        );
    }
    rows
}

#[test]
fn journal_fixtures_match_golden() {
    let jroot = eddn_fixtures_root().join("journal_1");
    let mut count = 0usize;
    for entry in std::fs::read_dir(&jroot).unwrap() {
        let dir = entry.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();
        let rows = run_mapper_fixture(&dir);
        let last = rows
            .last()
            .unwrap_or_else(|| panic!("sin salida en {}", dir.display()));
        assert_eq!(last.state, OutboxState::Pending, "dir {}", dir.display());
        assert_eq!(last.payload, expected, "mismatch en {}", dir.display());
        count += 1;
    }
    assert_eq!(count, 6);
}

#[test]
fn strip_localised_recursive() {
    let mut val = json!({
        "System": "Sol",
        "System_Localised": "Sol",
        "nested": [{"Name": "x", "Name_Localised": "X"}]
    });
    strip_localised(&mut val);
    assert_eq!(
        val,
        json!({
            "System": "Sol",
            "nested": [{"Name": "x"}]
        })
    );
}

#[test]
fn validate_local_rejects_personal_fields_anywhere() {
    let err = validate_local(&json!({ "a": [{ "Wanted": true }] })).unwrap_err();
    assert_eq!(err, "a[0].Wanted");
    assert!(validate_local(&json!({ "StarSystem": "Sol" })).is_ok());
}

#[test]
fn starpos_added_when_missing_and_mismatch_is_omitted() {
    let mut tracker = TransientTracker::new();
    let mut mapper = EddnMapper::new(SchemaMode::Live, "test-salt");
    for line in [
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"C","Horizons":true,"Odyssey":true}"#,
        r#"{"timestamp":"2026-01-01T12:00:02Z","event":"FSDJump","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0]}"#,
    ] {
        let ev = parse_line(line).unwrap();
        tracker.apply(&ev);
    }

    let ok_scan = parse_line(
        r#"{"timestamp":"2026-01-01T12:00:03Z","event":"Scan","ScanType":"Detailed","BodyName":"Sol A 1","BodyID":1,"StarSystem":"Sol","SystemAddress":10477373803}"#,
    )
    .unwrap();
    let mut out = Vec::new();
    mapper.map(
        &MappedInput {
            ev: &ok_scan,
            file: "J.01.log",
            line: 4,
            snap: &tracker.snapshot(),
            companion: None,
            tracker: &tracker,
        },
        &mut out,
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].state, OutboxState::Pending);
    assert_eq!(out[0].payload["message"]["StarPos"], json!([0.0, 0.0, 0.0]));

    let bad_scan = parse_line(
        r#"{"timestamp":"2026-01-01T12:00:04Z","event":"Scan","ScanType":"Detailed","BodyName":"Sol A 2","BodyID":2,"StarSystem":"Sol","SystemAddress":999999}"#,
    )
    .unwrap();
    let mut out_bad = Vec::new();
    mapper.map(
        &MappedInput {
            ev: &bad_scan,
            file: "J.01.log",
            line: 5,
            snap: &tracker.snapshot(),
            companion: None,
            tracker: &tracker,
        },
        &mut out_bad,
    );
    assert_eq!(out_bad.len(), 1);
    assert_eq!(out_bad[0].state, OutboxState::Omitted);
}

fn companion_kind_for_schema_dir(name: &str) -> tmc_model::companion::CompanionKind {
    use tmc_model::companion::CompanionKind;
    match name {
        "commodity_3" => CompanionKind::Market,
        "shipyard_2" => CompanionKind::Shipyard,
        "outfitting_2" => CompanionKind::Outfitting,
        "navroute_1" => CompanionKind::NavRoute,
        "fcmaterials_journal_1" => CompanionKind::FcMaterials,
        other => panic!("esquema sin tipo compañero: {other}"),
    }
}

#[test]
fn companion_fixtures_match_golden() {
    for name in [
        "commodity_3",
        "shipyard_2",
        "outfitting_2",
        "navroute_1",
        "fcmaterials_journal_1",
    ] {
        let dir = eddn_fixtures_root().join(name);
        let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
        let comp_text = std::fs::read_to_string(dir.join("companion.json")).unwrap();
        let comp =
            tmc_model::companion::parse(companion_kind_for_schema_dir(name), &comp_text).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();

        let mut tracker = TransientTracker::new();
        let mut mapper = EddnMapper::new(SchemaMode::Live, "test-salt");
        let mut rows = Vec::new();
        let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        for (idx, line) in lines.iter().enumerate() {
            let ev = parse_line(line).unwrap();
            tracker.apply(&ev);
            let snap = tracker.snapshot();
            let attached = (idx + 1 == lines.len()).then_some(&comp);
            mapper.map(
                &MappedInput {
                    ev: &ev,
                    file: "Journal.2026-01-01T120000.01.log",
                    line: (idx + 1) as u64,
                    snap: &snap,
                    companion: attached,
                    tracker: &tracker,
                },
                &mut rows,
            );
        }
        let last = rows
            .last()
            .unwrap_or_else(|| panic!("sin salida en {name}"));
        assert_eq!(last.state, OutboxState::Pending, "{name}");
        assert_eq!(last.payload, expected, "mismatch en {name}");
    }
}

fn feed_ev(
    origin: tmc_model::sync::FeedOrigin,
    event: tmc_model::sync::SyncFeedEvent,
) -> tmc_model::sync::SyncFeed {
    tmc_model::sync::SyncFeed {
        origin,
        galaxy: tmc_model::state::Galaxy::Live,
        game_running: true,
        event,
    }
}

fn setup_eddn_router(
    dir: &Path,
    store: &tmc_store::Store,
    online: bool,
) -> (crate::Router, std::sync::Arc<crate::clock::TestClock>) {
    use crate::clock::TestClock;
    use crate::router::{EnvGates, LiveSignalPolicy, RouterConfig};
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    let clock = TestClock::new(chrono::Utc.with_ymd_and_hms(2026, 1, 1, 12, 10, 0).unwrap());
    store.kv_set(
        crate::router::EDDN_ENABLED_KEY,
        &chrono::Utc
            .with_ymd_and_hms(2025, 12, 31, 0, 0, 0)
            .unwrap()
            .to_rfc3339(),
    );
    let cfg = RouterConfig {
        journal_dir: dir.to_path_buf(),
        env: EnvGates::default(),
        live_signal: LiveSignalPolicy::ProcessCheck,
        flush_every: Duration::from_secs(60),
        flush_rows: 200,
        hold_before_process: (200, Duration::from_secs(10)),
    };
    let mut router = crate::Router::new(
        store.clone(),
        clock.clone(),
        Arc::new(AtomicBool::new(online)),
        Arc::new(AtomicBool::new(true)),
        cfg,
        vec![Box::new(EddnMapper::new(SchemaMode::Live, "test-salt"))],
    );
    router
        .handle(feed_ev(
            tmc_model::sync::FeedOrigin::Live,
            tmc_model::sync::SyncFeedEvent::GameProcess { running: true },
        ))
        .unwrap();
    (router, clock)
}

#[test]
fn mismatched_companion_timestamp_is_omitted_and_counted() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let (mut router, _clock) = setup_eddn_router(tmp.path(), &store, true);
    let file = "Journal.2026-01-01T120000.01.log";
    let dir = eddn_fixtures_root().join("commodity_3");
    let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
    for (i, line) in input.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        router
            .handle(feed_ev(
                tmc_model::sync::FeedOrigin::Live,
                tmc_model::sync::SyncFeedEvent::Journal {
                    ev: parse_line(line).unwrap(),
                    file: file.into(),
                    line: (i + 1) as u64,
                },
            ))
            .unwrap();
    }
    let bad_comp = tmc_model::companion::parse(
        tmc_model::companion::CompanionKind::Market,
        r#"{"timestamp":"2026-01-01T12:05:00Z","event":"Market","MarketID":128000001,"StationName":"Puerto Ficticio","StarSystem":"Sistema Ficticio","Items":[]}"#,
    )
    .unwrap();
    router
        .handle(feed_ev(
            tmc_model::sync::FeedOrigin::Live,
            tmc_model::sync::SyncFeedEvent::Companion(bad_comp),
        ))
        .unwrap();
    router.flush().unwrap();
    let counts = store
        .sync_counts(tmc_model::sync::SyncService::Eddn, "F0000001")
        .unwrap();
    assert_eq!(counts.omitted_total, 1);
    assert_eq!(store.sync_cursor("F0000001", file).unwrap(), Some(4));
}

#[test]
fn companion_line_cursor_waits_for_pairing() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("sync.db");
    let store = tmc_store::Store::open(&db_path).unwrap();
    let file = "Journal.2026-01-01T120000.01.log";
    let dir = eddn_fixtures_root().join("commodity_3");
    let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
    std::fs::write(tmp.path().join(file), &input).unwrap();

    {
        let (mut router, _clock) = setup_eddn_router(tmp.path(), &store, true);
        for (i, line) in input.lines().filter(|l| !l.trim().is_empty()).enumerate() {
            router
                .handle(feed_ev(
                    tmc_model::sync::FeedOrigin::Live,
                    tmc_model::sync::SyncFeedEvent::Journal {
                        ev: parse_line(line).unwrap(),
                        file: file.into(),
                        line: (i + 1) as u64,
                    },
                ))
                .unwrap();
        }
        // Cierre antes de que llegue el compañero: el cursor NO avanzó a la línea 4.
        router.flush().unwrap();
        assert_eq!(store.sync_cursor("F0000001", file).unwrap(), Some(3));
    }

    // Reapertura: Recovery recupera la línea 4 y, con compañero inconsistente, sale Omitted y avanza el cursor a 4.
    let (mut router2, _clock2) = setup_eddn_router(tmp.path(), &store, true);
    router2.recover_previous_session().unwrap();
    for (i, line) in input.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        router2
            .handle(feed_ev(
                tmc_model::sync::FeedOrigin::CatchUp,
                tmc_model::sync::SyncFeedEvent::Journal {
                    ev: parse_line(line).unwrap(),
                    file: file.into(),
                    line: (i + 1) as u64,
                },
            ))
            .unwrap();
    }
    let bad_comp = tmc_model::companion::parse(
        tmc_model::companion::CompanionKind::Market,
        r#"{"timestamp":"2026-01-01T12:59:59Z","event":"Market","MarketID":128000001,"StationName":"Puerto Ficticio","StarSystem":"Sistema Ficticio","Items":[]}"#,
    )
    .unwrap();
    router2
        .handle(feed_ev(
            tmc_model::sync::FeedOrigin::CatchUp,
            tmc_model::sync::SyncFeedEvent::Companion(bad_comp),
        ))
        .unwrap();
    router2
        .handle(feed_ev(
            tmc_model::sync::FeedOrigin::CatchUp,
            tmc_model::sync::SyncFeedEvent::CatchUpDone,
        ))
        .unwrap();
    router2.flush().unwrap();

    let counts = store
        .sync_counts(tmc_model::sync::SyncService::Eddn, "F0000001")
        .unwrap();
    assert_eq!(counts.omitted_total, 1);
    assert_eq!(store.sync_cursor("F0000001", file).unwrap(), Some(4));
}

#[test]
fn empty_navroute_sends_nothing() {
    let mut tracker = TransientTracker::new();
    let mut mapper = EddnMapper::new(SchemaMode::Live, "test-salt");
    for line in [
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"C","Horizons":true,"Odyssey":true}"#,
    ] {
        let ev = parse_line(line).unwrap();
        tracker.apply(&ev);
    }
    let snap = tracker.snapshot();

    let clear_ev =
        parse_line(r#"{"timestamp":"2026-01-01T12:00:02Z","event":"NavRouteClear"}"#).unwrap();
    assert!(!mapper.wants_companion(&clear_ev));
    let mut out = Vec::new();
    mapper.map(
        &MappedInput {
            ev: &clear_ev,
            file: "J.01.log",
            line: 3,
            snap: &snap,
            companion: None,
            tracker: &tracker,
        },
        &mut out,
    );
    assert!(out.is_empty());

    let nav_ev = parse_line(r#"{"timestamp":"2026-01-01T12:00:03Z","event":"NavRoute"}"#).unwrap();
    assert!(mapper.wants_companion(&nav_ev));
    let empty_route = tmc_model::companion::parse(
        tmc_model::companion::CompanionKind::NavRoute,
        r#"{"timestamp":"2026-01-01T12:00:03Z","event":"NavRoute","Route":[]}"#,
    )
    .unwrap();
    mapper.map(
        &MappedInput {
            ev: &nav_ev,
            file: "J.01.log",
            line: 4,
            snap: &snap,
            companion: Some(&empty_route),
            tracker: &tracker,
        },
        &mut out,
    );
    assert!(out.is_empty());
}

#[test]
fn commodity_excludes_non_marketable_and_illegal() {
    let dir = eddn_fixtures_root().join("commodity_3");
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();
    let list = expected["message"]["commodities"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "gold");

    let mut tracker = TransientTracker::new();
    let mut mapper = EddnMapper::new(SchemaMode::Live, "test-salt");
    let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
    let comp_text = std::fs::read_to_string(dir.join("companion.json")).unwrap();
    let comp = tmc_model::companion::parse(tmc_model::companion::CompanionKind::Market, &comp_text)
        .unwrap();
    let mut out = Vec::new();
    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
    for (idx, line) in lines.iter().enumerate() {
        let ev = parse_line(line).unwrap();
        tracker.apply(&ev);
        let snap = tracker.snapshot();
        mapper.map(
            &MappedInput {
                ev: &ev,
                file: "J.01.log",
                line: (idx + 1) as u64,
                snap: &snap,
                companion: (idx + 1 == lines.len()).then_some(&comp),
                tracker: &tracker,
            },
            &mut out,
        );
    }
    let last = out.last().unwrap();
    let items = last.payload["message"]["commodities"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "gold");
    assert!(last.payload["message"].get("stationType").is_none());
}

#[test]
fn event_fixtures_match_golden() {
    for name in [
        "fssdiscoveryscan_1",
        "fssallbodiesfound_1",
        "navbeaconscan_1",
        "codexentry_1",
        "scanbarycentre_1",
        "fssbodysignals_1",
        "approachsettlement_1",
        "dockinggranted_1",
        "dockingdenied_1",
    ] {
        let dir = eddn_fixtures_root().join(name);
        let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap())
                .unwrap();

        let mut tracker = TransientTracker::new();
        let mut mapper = EddnMapper::new(SchemaMode::Live, "test-salt");
        let mut rows = Vec::new();
        for (idx, line) in input.lines().filter(|l| !l.trim().is_empty()).enumerate() {
            let ev = parse_line(line).unwrap();
            tracker.apply(&ev);
            let snap = tracker.snapshot();
            mapper.map(
                &MappedInput {
                    ev: &ev,
                    file: "Journal.2026-01-01T120000.01.log",
                    line: (idx + 1) as u64,
                    snap: &snap,
                    companion: None,
                    tracker: &tracker,
                },
                &mut rows,
            );
        }
        let last = rows
            .last()
            .unwrap_or_else(|| panic!("sin salida en {name}"));
        assert_eq!(last.state, OutboxState::Pending, "{name}");
        assert_eq!(last.payload, expected, "mismatch en {name}");
    }
}

#[test]
fn fss_signals_grouped_until_other_event() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let (mut router, _clock) = setup_eddn_router(tmp.path(), &store, true);
    let file = "Journal.2026-01-01T120000.01.log";

    let lines = [
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"C","Horizons":true,"Odyssey":true}"#,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"Location","StarSystem":"Sistema A","SystemAddress":1001,"StarPos":[1.0,2.0,3.0],"Docked":false}"#,
        r#"{"timestamp":"2026-01-01T12:00:02Z","event":"FSSSignalDiscovered","SystemAddress":1001,"SignalName":"Señal 1","SignalType":"ResourceExtraction"}"#,
        r#"{"timestamp":"2026-01-01T12:00:03Z","event":"FSSSignalDiscovered","SystemAddress":1001,"SignalName":"Señal 2","SignalType":"USS","USSType":"$USS_Type_Salvage;","ThreatLevel":2}"#,
        r#"{"timestamp":"2026-01-01T12:00:04Z","event":"FSSSignalDiscovered","SystemAddress":1001,"SignalName":"Señal 3","SignalType":"Outpost","IsStation":true}"#,
    ];

    for (idx, line) in lines.iter().enumerate() {
        router
            .handle(feed_ev(
                tmc_model::sync::FeedOrigin::Live,
                tmc_model::sync::SyncFeedEvent::Journal {
                    ev: parse_line(line).unwrap(),
                    file: file.into(),
                    line: (idx + 1) as u64,
                },
            ))
            .unwrap();
    }
    router.flush().unwrap();
    assert_eq!(store.sync_cursor("F0000001", file).unwrap(), Some(3));

    let jump = parse_line(
        r#"{"timestamp":"2026-01-01T12:00:05Z","event":"FSDJump","StarSystem":"Sistema B","SystemAddress":1002,"StarPos":[10.0,20.0,30.0]}"#,
    )
    .unwrap();
    router
        .handle(feed_ev(
            tmc_model::sync::FeedOrigin::Live,
            tmc_model::sync::SyncFeedEvent::Journal {
                ev: jump,
                file: file.into(),
                line: 7,
            },
        ))
        .unwrap();
    router.flush().unwrap();

    assert_eq!(store.sync_cursor("F0000001", file).unwrap(), Some(7));
    let counts = store
        .sync_counts(tmc_model::sync::SyncService::Eddn, "F0000001")
        .unwrap();
    assert_eq!(counts.pending, 3);

    let pending = store
        .claim_ready(
            tmc_model::sync::SyncService::Eddn,
            None,
            10,
            _clock.now_utc(),
        )
        .unwrap();
    let fss_rows: Vec<_> = pending
        .iter()
        .filter(|r| r.kind == "https://eddn.edcd.io/schemas/fsssignaldiscovered/1")
        .collect();
    assert_eq!(fss_rows.len(), 1);
    let msg: serde_json::Value = serde_json::from_str(&fss_rows[0].payload).unwrap();
    assert_eq!(msg["message"]["signals"].as_array().unwrap().len(), 3);
    assert_eq!(msg["message"]["StarSystem"], "Sistema A");
}

#[test]
fn fss_signal_filters_per_t0() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let (mut router, clock) = setup_eddn_router(tmp.path(), &store, true);
    let dir = eddn_fixtures_root().join("fsssignaldiscovered_1");
    let input = std::fs::read_to_string(dir.join("input.log")).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();
    let file = "Journal.2026-01-01T120000.01.log";

    for (idx, line) in input.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        router
            .handle(feed_ev(
                tmc_model::sync::FeedOrigin::Live,
                tmc_model::sync::SyncFeedEvent::Journal {
                    ev: parse_line(line).unwrap(),
                    file: file.into(),
                    line: (idx + 1) as u64,
                },
            ))
            .unwrap();
    }
    router.flush().unwrap();

    let pending = store
        .claim_ready(
            tmc_model::sync::SyncService::Eddn,
            None,
            10,
            clock.now_utc(),
        )
        .unwrap();
    let fss_row = pending
        .iter()
        .find(|r| r.kind == "https://eddn.edcd.io/schemas/fsssignaldiscovered/1")
        .expect("debe emitir fsssignaldiscovered/1");
    let actual: serde_json::Value = serde_json::from_str(&fss_row.payload).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn all_16_golden_fixtures_pass_recursive_privacy_sweep() {
    fn schema_file_for_dir(dir_name: &str) -> &'static str {
        match dir_name {
            "journal_1" => "journal-v1.0.json",
            "commodity_3" => "commodity-v3.0.json",
            "shipyard_2" => "shipyard-v2.0.json",
            "outfitting_2" => "outfitting-v2.0.json",
            "navroute_1" => "navroute-v1.0.json",
            "fcmaterials_journal_1" => "fcmaterials_journal-v1.0.json",
            "fssdiscoveryscan_1" => "fssdiscoveryscan-v1.0.json",
            "fssallbodiesfound_1" => "fssallbodiesfound-v1.0.json",
            "navbeaconscan_1" => "navbeaconscan-v1.0.json",
            "codexentry_1" => "codexentry-v1.0.json",
            "scanbarycentre_1" => "scanbarycentre-v1.0.json",
            "fssbodysignals_1" => "fssbodysignals-v1.0.json",
            "approachsettlement_1" => "approachsettlement-v1.0.json",
            "dockinggranted_1" => "dockinggranted-v1.0.json",
            "dockingdenied_1" => "dockingdenied-v1.0.json",
            "fsssignaldiscovered_1" => "fsssignaldiscovered-v1.0.json",
            other => panic!("esquema desconocido: {other}"),
        }
    }

    let schemas_dir = eddn_fixtures_root().join("_schemas");
    let mut expected_paths = Vec::new();
    for j in [
        "docked",
        "fsdjump",
        "scan",
        "location",
        "saasignalsfound",
        "carrierjump",
    ] {
        expected_paths.push((
            "journal_1",
            eddn_fixtures_root()
                .join("journal_1")
                .join(j)
                .join("expected.json"),
        ));
    }
    for s in [
        "commodity_3",
        "shipyard_2",
        "outfitting_2",
        "navroute_1",
        "fcmaterials_journal_1",
        "fssdiscoveryscan_1",
        "fssallbodiesfound_1",
        "navbeaconscan_1",
        "codexentry_1",
        "scanbarycentre_1",
        "fssbodysignals_1",
        "approachsettlement_1",
        "dockinggranted_1",
        "dockingdenied_1",
        "fsssignaldiscovered_1",
    ] {
        expected_paths.push((s, eddn_fixtures_root().join(s).join("expected.json")));
    }

    assert_eq!(expected_paths.len(), 21);
    for (dir_name, path) in expected_paths {
        let env: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        crate::eddn::sanitize::validate_local(&env["message"])
            .unwrap_or_else(|bad| panic!("campo sensible {bad} en {}", path.display()));
        let schema_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(schemas_dir.join(schema_file_for_dir(dir_name))).unwrap(),
        )
        .unwrap();
        let validator = jsonschema::draft4::new(&schema_json).unwrap();
        assert!(
            validator.is_valid(&env),
            "fallo de esquema Draft-04 en {}: {:?}",
            path.display(),
            validator.iter_errors(&env).collect::<Vec<_>>()
        );
    }
}

fn setup_worker_gates() -> std::sync::Arc<crate::router::SendGates> {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let gates = Arc::new(crate::router::SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    gates
        .galaxy_not_legacy
        .store(true, std::sync::atomic::Ordering::SeqCst);
    gates
}

fn reply_ex(
    status: u16,
    body: &str,
) -> Result<tmc_net::transport::HttpReplyEx, tmc_net::transport::TransportError> {
    Ok(tmc_net::transport::HttpReplyEx {
        status,
        body: body.to_string(),
        retry_after_s: None,
        rate_limit_remaining: None,
        rate_limit_reset_s: None,
    })
}

fn push_eddn_row(
    store: &tmc_store::Store,
    fid: &str,
    idem: &str,
    kind: &str,
    ts: chrono::DateTime<chrono::Utc>,
    payload: serde_json::Value,
) {
    store
        .enqueue_batch_with_cursor(
            &[tmc_store::NewOutboxRow {
                service: tmc_model::sync::SyncService::Eddn,
                cmdr_fid: fid.to_string(),
                idem_key: idem.to_string(),
                kind: kind.to_string(),
                coalesce_key: None,
                event_ts: ts,
                payload: payload.to_string(),
                origin: tmc_model::sync::RowOrigin::Live,
                state: OutboxState::Pending,
                last_class: None,
            }],
            &[],
            ts,
        )
        .unwrap();
}

#[tokio::test]
async fn one_message_per_post_and_max_two_per_second() {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::time::Duration;

    tokio::time::pause();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let base = chrono::Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = crate::clock::TestClock::new(base);
    store.upsert_known_cmdr("F0000001", "CMDR A", base).unwrap();

    let env: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(eddn_fixtures_root().join("journal_1/location/expected.json"))
            .unwrap(),
    )
    .unwrap();
    for i in 0..10 {
        push_eddn_row(
            &store,
            "F0000001",
            &format!("eddn:k:{i}"),
            "https://eddn.edcd.io/schemas/journal/1",
            base + chrono::Duration::seconds(i),
            env.clone(),
        );
    }

    let replies = (0..10).map(|_| reply_ex(200, "OK")).collect();
    let transport = tmc_net::testing::ScriptedTransport::new(replies);
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(crate::eddn::EddnClient::new(
        transport.clone(),
        store.clone(),
    ));
    let gates = setup_worker_gates();
    let wake = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

    let worker = crate::worker::Worker::new(client, store, clock, gates, wake, tx);
    let handle = tokio::spawn(worker.run(sd_rx));

    // En 900 ms deben haberse enviado como máximo 2 POST (1 mensaje/POST, máx 2 POST/s).
    tokio::time::advance(Duration::from_millis(900)).await;
    tokio::task::yield_now().await;
    let early_posts = transport.posts().len();
    assert!(
        (1..=2).contains(&early_posts),
        "posts en 900ms: {early_posts}"
    );

    // Avanzando lo suficiente (> 9 s porque las filas llevan +0..9 s) terminan los 10 POST.
    for _ in 0..20 {
        tokio::time::advance(Duration::from_millis(550)).await;
        tokio::task::yield_now().await;
    }
    let _ = sd_tx.send(true);
    let _ = handle.await;
    rx.close();

    let posts = transport.posts();
    assert_eq!(posts.len(), 10);
    for p in &posts {
        assert_eq!(p.url, tmc_net::eddn_upload::EDDN_UPLOAD_URL);
        assert!(p.http1_only);
        let parsed: serde_json::Value = serde_json::from_slice(&p.body).unwrap();
        assert!(parsed.is_object());
    }
}

#[tokio::test]
async fn retry_waits_at_least_60s_then_backs_off_to_30min() {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::time::Duration;

    tokio::time::pause();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let base = chrono::Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = crate::clock::TestClock::new(base);
    store.upsert_known_cmdr("F0000001", "CMDR A", base).unwrap();

    let env: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(eddn_fixtures_root().join("journal_1/location/expected.json"))
            .unwrap(),
    )
    .unwrap();
    push_eddn_row(
        &store,
        "F0000001",
        "eddn:retry:1",
        "https://eddn.edcd.io/schemas/journal/1",
        base,
        env,
    );

    let replies = (0..8).map(|_| reply_ex(503, "<html>503</html>")).collect();
    let transport = tmc_net::testing::ScriptedTransport::new(replies);
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(crate::eddn::EddnClient::new(
        transport.clone(),
        store.clone(),
    ));
    let gates = setup_worker_gates();
    let wake = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

    let worker = crate::worker::Worker::new(client, store.clone(), clock.clone(), gates, wake, tx);
    let handle = tokio::spawn(worker.run(sd_rx));

    // Primer fallo -> next_attempt_at >= +60 s.
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);
    let next1 = store
        .next_attempt_at(tmc_model::sync::SyncService::Eddn)
        .unwrap()
        .unwrap();
    assert!(
        (next1 - (base + chrono::Duration::seconds(60)))
            .num_seconds()
            .abs()
            <= 1
    );

    // Tras repetir fallos, alcanza el techo de 1800 s (30 min): 60, 120, 240, 480, 960, 1800.
    let expected_steps = [120i64, 240, 480, 960, 1800, 1800];
    for (idx, step) in expected_steps.iter().enumerate() {
        let prev = store
            .next_attempt_at(tmc_model::sync::SyncService::Eddn)
            .unwrap()
            .unwrap();
        let jump_ms = (prev - clock.now_utc()).num_milliseconds().max(0) as u64 + 50;
        tokio::time::advance(Duration::from_millis(jump_ms)).await;
        tokio::task::yield_now().await;
        assert_eq!(transport.calls(), idx + 2);
        let next = store
            .next_attempt_at(tmc_model::sync::SyncService::Eddn)
            .unwrap()
            .unwrap();
        assert!(
            (next - (clock.now_utc() + chrono::Duration::seconds(*step)))
                .num_seconds()
                .abs()
                <= 1
        );
    }

    let _ = sd_tx.send(true);
    let _ = handle.await;
    rx.close();
}

#[tokio::test]
async fn status_400_rejects_without_blocking_queue() {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::time::Duration;

    tokio::time::pause();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let base = chrono::Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = crate::clock::TestClock::new(base);
    store.upsert_known_cmdr("F0000001", "CMDR A", base).unwrap();

    let env: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(eddn_fixtures_root().join("journal_1/location/expected.json"))
            .unwrap(),
    )
    .unwrap();
    push_eddn_row(
        &store,
        "F0000001",
        "eddn:400:1",
        "https://eddn.edcd.io/schemas/journal/1",
        base,
        env.clone(),
    );
    push_eddn_row(
        &store,
        "F0000001",
        "eddn:400:2",
        "https://eddn.edcd.io/schemas/journal/1",
        base + chrono::Duration::seconds(1),
        env,
    );

    let transport = tmc_net::testing::ScriptedTransport::new(vec![
        reply_ex(400, "FAIL: schema invalid"),
        reply_ex(200, "OK"),
    ]);
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(crate::eddn::EddnClient::new(
        transport.clone(),
        store.clone(),
    ));
    let gates = setup_worker_gates();
    let wake = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

    let worker = crate::worker::Worker::new(client, store.clone(), clock, gates, wake, tx);
    let handle = tokio::spawn(worker.run(sd_rx));

    for _ in 0..4 {
        tokio::time::advance(Duration::from_millis(600)).await;
        tokio::task::yield_now().await;
    }
    let _ = sd_tx.send(true);
    let _ = handle.await;
    rx.close();

    assert_eq!(transport.calls(), 2);
    let counts = store
        .sync_counts(tmc_model::sync::SyncService::Eddn, "F0000001")
        .unwrap();
    assert_eq!(counts.rejected_total, 1);
    assert_eq!(counts.sent_total, 1);
    assert_eq!(counts.pending, 0);
}

#[tokio::test]
async fn status_426_pauses_schema_and_warns() {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::time::Duration;

    tokio::time::pause();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let base = chrono::Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = crate::clock::TestClock::new(base);
    store.upsert_known_cmdr("F0000001", "CMDR A", base).unwrap();

    let journal_env: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(eddn_fixtures_root().join("journal_1/location/expected.json"))
            .unwrap(),
    )
    .unwrap();
    let nav_env: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(eddn_fixtures_root().join("navroute_1/expected.json")).unwrap(),
    )
    .unwrap();

    push_eddn_row(
        &store,
        "F0000001",
        "eddn:426:j1",
        "https://eddn.edcd.io/schemas/journal/1",
        base,
        journal_env.clone(),
    );
    push_eddn_row(
        &store,
        "F0000001",
        "eddn:426:j2",
        "https://eddn.edcd.io/schemas/journal/1",
        base + chrono::Duration::seconds(1),
        journal_env,
    );
    push_eddn_row(
        &store,
        "F0000001",
        "eddn:426:nav",
        "https://eddn.edcd.io/schemas/navroute/1",
        base + chrono::Duration::seconds(2),
        nav_env,
    );

    let transport = tmc_net::testing::ScriptedTransport::new(vec![
        reply_ex(426, "Upgrade Required"),
        reply_ex(200, "OK"),
    ]);
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(crate::eddn::EddnClient::new(
        transport.clone(),
        store.clone(),
    ));
    let gates = setup_worker_gates();
    let wake = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

    let mut agg = crate::status::StatusAggregator::new(store.clone(), clock.clone(), "salt".into());
    let worker = crate::worker::Worker::new(client, store.clone(), clock.clone(), gates, wake, tx);
    let handle = tokio::spawn(worker.run(sd_rx));

    for _ in 0..5 {
        tokio::time::advance(Duration::from_millis(600)).await;
        tokio::task::yield_now().await;
    }
    let _ = sd_tx.send(true);
    let _ = handle.await;
    rx.close();
    while let Some(ev) = rx.recv().await {
        agg.on_worker_event(&ev);
    }

    assert_eq!(transport.calls(), 2);
    let counts = store
        .sync_counts(tmc_model::sync::SyncService::Eddn, "F0000001")
        .unwrap();
    assert_eq!(counts.sent_total, 1);
    assert_eq!(counts.pending, 2);

    let reports = agg.build(Some("F0000001"), &|_, _| None);
    let eddn_status = reports
        .into_iter()
        .find_map(|r| match r {
            tmc_model::sync::SyncReport::Eddn(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert_eq!(eddn_status.paused_schemas, vec!["journal/1".to_string()]);

    // Al reiniciar con distinta versión de la aplicación (`CARGO_PKG_VERSION`), la pausa se limpia.
    store.kv_set("sync.eddn.paused_version", "0.0.0");
    let reloaded = crate::eddn::EddnClient::new(transport, store.clone());
    assert!(crate::worker::ServiceClient::excluded_kinds(&reloaded).is_empty());
    assert_eq!(
        store.kv_get("sync.eddn.paused_schemas").as_deref(),
        Some("[]")
    );
}

#[tokio::test]
async fn pending_keep_origin_uploader_after_switch() {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::time::Duration;

    tokio::time::pause();
    let tmp = tempfile::tempdir().unwrap();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let (mut router, clock) = setup_eddn_router(tmp.path(), &store, true);

    for (idx, line) in [
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"C1","Horizons":true,"Odyssey":true}"#,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"Location","StarSystem":"S1","SystemAddress":1001,"StarPos":[1.0,2.0,3.0],"Docked":false}"#,
        r#"{"timestamp":"2026-01-01T12:00:02Z","event":"LoadGame","FID":"F0000002","Commander":"C2","Horizons":true,"Odyssey":true}"#,
        r#"{"timestamp":"2026-01-01T12:00:02Z","event":"Location","StarSystem":"S2","SystemAddress":1002,"StarPos":[4.0,5.0,6.0],"Docked":false}"#,
    ]
    .iter()
    .enumerate()
    {
        router
            .handle(feed_ev(
                tmc_model::sync::FeedOrigin::Live,
                tmc_model::sync::SyncFeedEvent::Journal {
                    ev: parse_line(line).unwrap(),
                    file: "Journal.2026-01-01T120000.01.log".into(),
                    line: (idx + 1) as u64,
                },
            ))
            .unwrap();
    }
    router.flush().unwrap();

    let transport =
        tmc_net::testing::ScriptedTransport::new(vec![reply_ex(200, "OK"), reply_ex(200, "OK")]);
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(crate::eddn::EddnClient::new(
        transport.clone(),
        store.clone(),
    ));
    let gates = setup_worker_gates();
    let wake = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

    let worker = crate::worker::Worker::new(client, store, clock, gates, wake, tx);
    let handle = tokio::spawn(worker.run(sd_rx));
    for _ in 0..4 {
        tokio::time::advance(Duration::from_millis(600)).await;
        tokio::task::yield_now().await;
    }
    let _ = sd_tx.send(true);
    let _ = handle.await;
    rx.close();

    let posts = transport.posts();
    assert_eq!(posts.len(), 2);
    let body0: serde_json::Value = serde_json::from_slice(&posts[0].body).unwrap();
    let body1: serde_json::Value = serde_json::from_slice(&posts[1].body).unwrap();
    assert_eq!(
        body0["header"]["uploaderID"],
        tmc_net::eddn_upload::uploader_id("test-salt", "F0000001")
    );
    assert_eq!(
        body1["header"]["uploaderID"],
        tmc_net::eddn_upload::uploader_id("test-salt", "F0000002")
    );
    let _ = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
}

#[tokio::test]
async fn eddn_rows_sent_mark_counters() {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::time::Duration;

    tokio::time::pause();
    let store = tmc_store::Store::open_in_memory().unwrap();
    let base = chrono::Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = crate::clock::TestClock::new(base);
    store.upsert_known_cmdr("F0000001", "CMDR A", base).unwrap();

    let env: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(eddn_fixtures_root().join("journal_1/location/expected.json"))
            .unwrap(),
    )
    .unwrap();
    push_eddn_row(
        &store,
        "F0000001",
        "eddn:cnt:1",
        "https://eddn.edcd.io/schemas/journal/1",
        base,
        env,
    );

    let transport = tmc_net::testing::ScriptedTransport::new(vec![reply_ex(200, "OK")]);
    let client: Arc<dyn crate::worker::ServiceClient> =
        Arc::new(crate::eddn::EddnClient::new(transport, store.clone()));
    let gates = setup_worker_gates();
    let wake = Arc::new(tokio::sync::Notify::new());
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let (sd_tx, sd_rx) = tokio::sync::watch::channel(false);

    let mut agg = crate::status::StatusAggregator::new(store.clone(), clock.clone(), "salt".into());
    let worker = crate::worker::Worker::new(client, store.clone(), clock, gates, wake, tx);
    let handle = tokio::spawn(worker.run(sd_rx));

    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    let _ = sd_tx.send(true);
    let _ = handle.await;
    rx.close();
    while let Some(ev) = rx.recv().await {
        agg.on_worker_event(&ev);
    }

    let counts = store
        .sync_counts(tmc_model::sync::SyncService::Eddn, "F0000001")
        .unwrap();
    assert_eq!(counts.sent_total, 1);

    let reports = agg.build(Some("F0000001"), &|_, _| None);
    let eddn_status = reports
        .into_iter()
        .find_map(|r| match r {
            tmc_model::sync::SyncReport::Eddn(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        eddn_status.status.state,
        tmc_model::sync::SyncState::Connected
    );
}

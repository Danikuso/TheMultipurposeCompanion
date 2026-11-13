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

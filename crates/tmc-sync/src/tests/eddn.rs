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

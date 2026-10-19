use super::*;
use serde_json::{Value, json};
use tracing_subscriber::filter::FilterExt;
use tracing_subscriber::prelude::*;

fn read_records(dir: &Path) -> Vec<Value> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .flat_map(|p| {
            std::fs::read_to_string(p)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str::<Value>(l).unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn exe_directory_has_no_data_directory_fallback() {
    assert_eq!(
        directory_for_executable(Path::new("C:/app/TMC.exe")).unwrap(),
        Path::new("C:/app/debug_logs")
    );
    assert!(directory_for_executable(Path::new("TMC.exe")).is_err());
}

#[test]
fn real_emit_is_captured_at_trace_and_chunks_reconstruct_sanitized_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer.with_filter(
        tracing_subscriber::filter::filter_fn(|m| m.target() == TARGET),
    ));
    let raw = json!({"BodyName":"ÃƒÅ½Ã‚Â©\"\n".repeat(1200),"FID":"CANARY-SECRET","MassEM":42});
    tracing::subscriber::with_default(subscriber, || {
        tmc_model::diagnostics::emit(
            tmc_model::diagnostics::DiagnosticContext::Journal { event: "Scan" },
            "test.scan",
            &raw,
        );
    });
    drop(guard);
    let records = read_records(dir.path());
    let chunks: Vec<_> = records
        .iter()
        .filter(|r| r["code"] == "test.scan")
        .collect();
    assert!(chunks.len() > 1);
    let reconstructed: String = chunks
        .iter()
        .map(|r| r["chunk"].as_str().unwrap())
        .collect();
    let safe: Value = serde_json::from_str(&reconstructed).unwrap();
    assert_eq!(safe["BodyName"], raw["BodyName"]);
    assert_eq!(safe["MassEM"], 42);
    assert!(!reconstructed.contains("CANARY"));
    for entry in std::fs::read_dir(dir.path()).unwrap().flatten() {
        for line in std::fs::read_to_string(entry.path()).unwrap().lines() {
            assert!(line.len() < RECORD_BYTES);
        }
    }
    assert_eq!(records.last().unwrap()["code"], "diagnostics.stop");
}

#[test]
fn ring_rotates_continuously_and_restarts_never_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("foreign.jsonl"), "foreign").unwrap();
    let limits = Limits {
        segment_bytes: 128,
        segments: 3,
        ..Limits::default()
    };
    for _ in 0..2 {
        let mut ring = Ring::new(
            dir.path().to_owned(),
            limits,
            uuid::Uuid::new_v4().to_string(),
        )
        .unwrap();
        for _ in 0..15 {
            ring.write(b"some record data some record data\n", chrono::Utc::now())
                .unwrap();
        }
    }
    assert_eq!(
        std::fs::read_to_string(dir.path().join("foreign.jsonl")).unwrap(),
        "foreign"
    );
    assert!(owned_files(dir.path()).unwrap().len() <= 3);
}

#[test]
fn directory_failure_is_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    std::fs::write(&path, "x").unwrap();
    assert!(start_writer(path, Limits::default()).is_err());
}

#[test]
fn escaped_metadata_and_payload_still_fit_final_record() {
    let base = json!({"context":"\u{0001}".repeat(32),"schema":"\u{0001}".repeat(128),"code":"\u{0001}".repeat(128),"session":uuid::Uuid::new_v4().to_string(),"utc":Utc::now().to_rfc3339(),"monotonic_us":u64::MAX,"event_id":u64::MAX});
    for record in encode_event(
        base,
        &json!({"BodyName":"\u{0001}".repeat(3000)}).to_string(),
    ) {
        assert!(record.len() + ORDINAL_RESERVE <= RECORD_BYTES);
    }
}

#[test]
fn queue_overflow_is_nonblocking_and_loss_survives_guard_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    // Hold the worker's disk path unavailable while a tiny isolated queue deterministically fills.
    let shared = layer.shared.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let subscriber = tracing_subscriber::registry().with(DiagnosticLayer {
        sender,
        shared: shared.clone(),
    });
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..10 {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "test.overflow",
                json!({"credits":42})
            );
        }
    });
    assert!(receiver.try_recv().is_ok());
    assert!(shared.dropped.load(Ordering::Acquire) >= 9);
    drop(guard);
    let records = read_records(dir.path());
    assert!(records.iter().any(|r| r["code"] == "diagnostics.incomplete"
        && r["details"]["dropped_records"].as_u64().unwrap() >= 9));
}

#[test]
fn daily_rotation_and_retention_leave_foreign_and_future_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut ring = Ring::new(
        dir.path().to_owned(),
        Limits::default(),
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    let old = Utc::now() - chrono::Duration::days(8);
    let old_name = format!(
        "tmc-debug-{}-{}-0000000001.jsonl",
        old.format("%Y%m%dT%H%M%S%.9fZ"),
        uuid::Uuid::new_v4()
    );
    fs::write(dir.path().join(&old_name), "old").unwrap();
    fs::write(dir.path().join("tmc-debug-foreign.jsonl"), "foreign").unwrap();
    let today = ring.day.clone();
    ring.write(b"{}\n", Utc::now() + chrono::Duration::days(1))
        .unwrap();
    assert_ne!(today, ring.day);
    assert!(!dir.path().join(old_name).exists());
    assert!(dir.path().join("tmc-debug-foreign.jsonl").exists());
}

#[test]
fn event_over_one_mib_produces_explicit_marker() {
    let records = encode_event(
        json!({"code":"test.large"}),
        &"x".repeat(MAX_EVENT_BYTES + 1),
    );
    assert_eq!(records.len(), 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&records[0]).unwrap()["oversized"],
        true
    );
}

#[test]
fn shutdown_waits_for_inflight_producer_before_draining() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let shared = layer.shared.clone();
    shared.active.fetch_add(1, Ordering::AcqRel);
    let (done_tx, done_rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        drop(guard);
        done_tx.send(()).unwrap();
    });
    while !shared.closing.load(Ordering::Acquire) {
        std::thread::yield_now();
    }
    assert!(done_rx.recv_timeout(Duration::from_millis(30)).is_err());
    layer
        .sender
        .try_send(
            json!({"code":"test.inflight","event_id":99})
                .to_string()
                .into_bytes(),
        )
        .unwrap();
    shared.active.fetch_sub(1, Ordering::AcqRel);
    done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    handle.join().unwrap();
    let records = read_records(dir.path());
    assert!(records.iter().any(|r| r["code"] == "test.inflight"));
    assert_eq!(records.last().unwrap()["code"], "diagnostics.stop");
}

#[test]
fn failed_disk_markers_keep_counters_until_storage_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().join("unused"), Limits::default()).unwrap();
    drop(guard);
    let shared = layer.shared.clone();
    let mut ring = Ring::new(
        dir.path().join("recover"),
        Limits::default(),
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    ring.current.take();
    fs::remove_dir_all(&ring.dir).unwrap();
    fs::write(&ring.dir, "blocked").unwrap();
    let mut output = Output {
        ring,
        shared: shared.clone(),
        ordinal: 0,
    };
    shared.dropped.store(17, Ordering::Release);
    shared.incomplete.store(2, Ordering::Release);
    output.losses();
    assert_eq!(shared.dropped.load(Ordering::Acquire), 17);
    assert!(shared.disk_errors.load(Ordering::Acquire) >= 1);
    fs::remove_file(&output.ring.dir).unwrap();
    fs::create_dir(&output.ring.dir).unwrap();
    output.losses();
    output.flush();
    assert_eq!(shared.dropped.load(Ordering::Acquire), 0);
    assert!(
        read_records(&output.ring.dir)
            .iter()
            .any(|r| r["details"]["dropped_records"] == 17)
    );
}

#[test]
fn restart_uses_unique_names_without_overwriting_previous_session() {
    let dir = tempfile::tempdir().unwrap();
    let mut ring = Ring::new(
        dir.path().to_owned(),
        Limits::default(),
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    ring.write(b"first session\n", Utc::now()).unwrap();
    drop(ring);
    let before = owned_files(dir.path()).unwrap().pop().unwrap().1;
    let mut ring = Ring::new(
        dir.path().to_owned(),
        Limits::default(),
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    ring.write(b"second session\n", Utc::now()).unwrap();
    drop(ring);
    assert_eq!(fs::read_to_string(before).unwrap(), "first session\n");
    assert_eq!(owned_files(dir.path()).unwrap().len(), 2);
}

#[test]
fn general_info_filter_does_not_disable_diagnostic_trace_or_receive_it() {
    use std::sync::Mutex;
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for filter in ["info", "trace"] {
        let dir = tempfile::tempdir().unwrap();
        let sink = Sink(Arc::new(Mutex::new(Vec::new())));
        let buffer = sink.0.clone();
        let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
        let general = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(move || sink.clone())
            .with_filter(tracing_subscriber::EnvFilter::new(filter).and(
                tracing_subscriber::filter::filter_fn(|m| m.target() != TARGET),
            ));
        let subscriber = tracing_subscriber::registry()
            .with(general)
            .with(
                layer.with_filter(tracing_subscriber::filter::filter_fn(|m| {
                    m.target() == TARGET
                })),
            );
        tracing::subscriber::with_default(subscriber, || {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "test.independent",
                json!({"credits":42})
            );
            tracing::info!("general_info_visible");
        });
        drop(guard);
        assert!(
            read_records(dir.path())
                .iter()
                .any(|r| r["code"] == "test.independent"
                    && r["context"] == "state"
                    && r["schema"] == "state"
                    && r["payload"]["credits"] == 42)
        );
        let general = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(general.contains("general_info_visible"));
        assert!(!general.contains("test.independent"));
    }
}

#[cfg(windows)]
#[test]
fn locked_oldest_segment_never_exceeds_budget_and_recovers_with_marker() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().join("unused"), Limits::default()).unwrap();
    drop(guard);
    let destination = dir.path().join("ring");
    let mut ring = Ring::new(
        destination.clone(),
        Limits {
            segment_bytes: 1024,
            segments: 2,
            ..Limits::default()
        },
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    let record = json!({"code":"test.fill", "padding":"x".repeat(850)})
        .to_string()
        .into_bytes();
    ring.write(b"{}\n", Utc::now()).unwrap();
    ring.flush().unwrap();
    let oldest = owned_files(&destination).unwrap().pop().unwrap().1;
    // FILE_SHARE_READ | FILE_SHARE_WRITE: another process can read/write but cannot delete.
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0x00000001 | 0x00000002)
        .open(oldest)
        .unwrap();
    fs::write(destination.join("foreign.txt"), "foreign").unwrap();
    let mut output = Output {
        ring,
        shared: layer.shared.clone(),
        ordinal: 0,
    };
    for _ in 0..12 {
        output.write(record.clone());
        assert!(
            owned_files(&destination).unwrap().len() <= 2,
            "locked oldest file must never grow ring beyond its budget"
        );
    }
    assert!(output.shared.disk_errors.load(Ordering::Acquire) > 0);
    drop(lock);
    output.losses();
    output.write(json!({"code":"test.recovered"}).to_string().into_bytes());
    output.flush();
    assert!(owned_files(&destination).unwrap().len() <= 2);
    let records = read_records(&destination);
    assert!(records.iter().any(|r| r["code"] == "diagnostics.incomplete"
        && r["details"]["disk_errors"].as_u64().is_some_and(|n| n > 0)));
    assert!(records.iter().any(|r| r["code"] == "test.recovered"));
    assert_eq!(
        fs::read_to_string(destination.join("foreign.txt")).unwrap(),
        "foreign"
    );
}

#[test]
fn producer_entry_racing_guard_close_never_loses_an_accepted_event() {
    // Entry and close start together. Neither thread signals completion/entry to the other.
    for _ in 0..256 {
        let dir = tempfile::tempdir().unwrap();
        let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
        let shared = layer.shared.clone();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let producer_barrier = barrier.clone();
        let producer = std::thread::spawn(move || {
            let subscriber = tracing_subscriber::registry().with(layer);
            tracing::subscriber::with_default(subscriber, || {
                producer_barrier.wait();
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "test.entry_close",
                    json!({"credits":42})
                );
            });
        });
        let closer_barrier = barrier.clone();
        let closer = std::thread::spawn(move || {
            closer_barrier.wait();
            drop(guard);
        });
        barrier.wait();
        producer.join().unwrap();
        closer.join().unwrap();
        let records = read_records(dir.path());
        if shared.next_event.load(Ordering::Acquire) > 1 {
            assert!(
                records.iter().any(|r| r["code"] == "test.entry_close"),
                "an admitted producer must be drained before stop"
            );
        }
        assert_eq!(records.last().unwrap()["code"], "diagnostics.stop");
        assert_eq!(shared.dropped.load(Ordering::Acquire), 0);
        assert_eq!(shared.incomplete.load(Ordering::Acquire), 0);
    }
}

fn capture_sources(f: impl FnOnce()) -> Vec<Value> {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer.with_filter(
        tracing_subscriber::filter::filter_fn(|m| m.target() == TARGET),
    ));
    tracing::subscriber::with_default(subscriber, f);
    drop(guard);
    read_records(dir.path())
}

fn payload_for<'a>(records: &'a [Value], code: &str) -> &'a Value {
    &records
        .iter()
        .find(|r| r["code"] == code)
        .unwrap_or_else(|| panic!("missing {code}"))["payload"]
}

#[test]
fn d3_tail_reports_real_byte_offsets_partial_lossy_and_truncation() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Journal.2026-10-20T010203.01.log");
    // NUL prefix, UTF-8 BOM, CRLF and invalid UTF-8 must not be recomputed as String.len().
    fs::write(&path, b"\0\xef\xbb\xbfX\xff\r\npar").unwrap();
    let records = capture_sources(|| {
        let mut tail = tmc_journal::LineTailer::open(&path, 99).unwrap();
        assert_eq!(tail.read_new_lines().unwrap().len(), 1);
        assert_eq!(tail.offset(), 8);
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"tial\n")
            .unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(2, "partial".into())]);
        fs::write(&path, b"n\n").unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "n".into())]);
    });
    assert_eq!(
        payload_for(&records, "journal.tail.opened")["requested_offset"],
        99
    );
    let first = payload_for(&records, "journal.tail.read");
    assert_eq!(first["read_pos"], 11);
    assert_eq!(first["confirmed_offset"], 8);
    assert_eq!(first["pending_bytes"], 3);
    assert_eq!(payload_for(&records, "journal.tail.line")["end_offset"], 8);
    assert_eq!(
        payload_for(&records, "journal.tail.truncated")["read_pos"],
        16
    );
    assert_eq!(
        payload_for(&records, "journal.tail.opened")["file"],
        "Journal.2026-10-20T010203.01.log"
    );
}

#[test]
fn d3_status_companion_paths_and_parse_text_stay_private() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("CANARY-PERSONAL-PATH");
    fs::create_dir(&private).unwrap();
    let path = private.join("Status.json");
    let records = capture_sources(|| {
        let mut poller = tmc_journal::StatusPoller::new(path.clone());
        assert!(poller.poll().is_none());
        fs::write(
            &path,
            r#"{"timestamp":"2026-10-20T01:00:00Z","event":"Status","Flags":1,"BodyName":"Sol A 1","Destination":{"System":1,"Body":2,"Name":"CANARY-DEST"}}"#,
        )
        .unwrap();
        assert!(poller.poll().is_some());
        assert!(poller.poll().is_none());
        fs::write(
            private.join("Cargo.json"),
            r#"{"Count":"CANARY-PARSE-TEXT"}"#,
        )
        .unwrap();
        assert!(
            tmc_journal::read_companion(&private, tmc_model::companion::CompanionKind::Cargo)
                .is_none()
        );
    });
    let accepted = payload_for(&records, "source.status.accepted");
    assert_eq!(accepted["Flags"], 1);
    assert_eq!(accepted["BodyName"], "Sol A 1");
    assert_eq!(accepted["Destination"]["Name"], "[redacted]");
    payload_for(&records, "source.status.metadata_unchanged");
    payload_for(&records, "source.status.metadata_failed");
    assert_eq!(
        payload_for(&records, "source.companion.exhausted")["attempts"],
        10
    );
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_sql_reports_types_and_outcomes_without_positional_values_or_error_text() {
    let records = capture_sources(|| {
        let store = tmc_store::Store::open_in_memory().unwrap();
        let result = store
            .query_json(
                "SELECT ?1 AS cmdr, ?2 AS json",
                &[
                    json!("CANARY-CMDR"),
                    json!(r#"{"SystemName":"Sol","Message":"CANARY-CHAT"}"#),
                ],
            )
            .unwrap();
        assert_eq!(result[0][0], "CANARY-CMDR");
        assert!(
            store
                .exec_json("SELECT ?1", &[json!({"secret":"CANARY-UNSUPPORTED"})])
                .is_err()
        );
        assert!(
            store
                .query_json("SELECT 'CANARY-INTERPOLATED' FROM absent", &[])
                .is_err()
        );
        store.kv_set("CANARY-KEY", "CANARY-VALUE");
        assert_eq!(store.kv_get("CANARY-KEY").as_deref(), Some("CANARY-VALUE"));
        store
            .api_cache_put("CANARY-CACHE", r#"{"Message":"CANARY-BODY"}"#, Utc::now())
            .unwrap();
        assert!(store.api_cache_get("CANARY-CACHE").unwrap().is_some());
    });
    assert_eq!(payload_for(&records, "store.query.completed")["rows"], 1);
    assert_eq!(
        payload_for(&records, "store.exec.failed")["failure_class"],
        "unsupported"
    );
    assert_eq!(
        payload_for(&records, "store.query.failed")["failure_class"],
        "sqlite"
    );
    payload_for(&records, "store.cache.hit");
    payload_for(&records, "store.kv.get");
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_import_checkpoints_deduplicate_and_redact_embedded_events() {
    let dir = tempfile::tempdir().unwrap();
    let name = "Journal.2026-10-20T010203.01.log";
    fs::write(dir.path().join(name),concat!(
        "{\"timestamp\":\"2026-10-20T01:00:00Z\",\"event\":\"Scan\",\"BodyName\":\"Sol A 1\",\"FID\":\"CANARY-FID\",\"MassEM\":42}\r\n",
        "{\"event\":\"CANARY-BAD-EVENT\"}\n"
    )).unwrap();
    let records = capture_sources(|| {
        let store = tmc_store::Store::open_in_memory().unwrap();
        let opts = tmc_store::ImportOptions {
            throttle: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        assert_eq!(
            tmc_store::import_history(&store, dir.path(), &opts, &mut |_, _| {})
                .unwrap()
                .events,
            1
        );
        assert_eq!(
            tmc_store::import_history(&store, dir.path(), &opts, &mut |_, _| {})
                .unwrap()
                .events,
            0
        );
    });
    assert_eq!(
        payload_for(&records, "store.import.batch_committed")["inserted"],
        1
    );
    assert!(records.iter().all(|r| r["code"] != "store.event.payload"));
    assert!(records.iter().all(|r| r["code"] != "store.event.inserted"));
    assert_eq!(
        payload_for(&records, "store.import.checkpoint")["offset"],
        fs::metadata(dir.path().join(name)).unwrap().len()
    );
    payload_for(&records, "store.import.invalid_line");
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_config_bootstrap_is_retained_until_logger_and_consumed_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        "language = [\"CANARY-CONFIG\"]\nserver.lan_token = \"CANARY-TOKEN\"",
    )
    .unwrap();
    let cfg = crate::Config::load_from_or_default(&path);
    assert_eq!(cfg.language, "es");
    let records = capture_sources(|| {
        let guards = crate::logging::LoggingGuards {
            general: None,
            diagnostics: None,
        };
        guards.application_started();
        guards.application_started();
    });
    assert_eq!(
        records
            .iter()
            .filter(|r| r["code"] == "config.bootstrap")
            .count(),
        1
    );
    let report = payload_for(&records, "config.bootstrap");
    assert_eq!(report["fallback"], true);
    assert_eq!(report["parse_failed"], true);
    assert_eq!(report["backup_succeeded"], true);
    assert_eq!(report["language"], "es");
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_migrations_query_filters_and_file_boundaries_are_observable_and_private() {
    let dir = tempfile::tempdir().unwrap();
    let records = capture_sources(|| {
        let store = tmc_store::Store::open_in_memory().unwrap();
        store
            .apply_migrations(&[tmc_store::Migration {
                id: "CANARY-MIGRATION-ID",
                sql: "CREATE TABLE sample (value TEXT)",
            }])
            .unwrap();
        store
            .apply_migrations(&[tmc_store::Migration {
                id: "CANARY-MIGRATION-ID",
                sql: "SELECT 1",
            }])
            .unwrap();
        assert!(
            store
                .apply_migrations(&[tmc_store::Migration {
                    id: "CANARY-BAD-ID",
                    sql: "INVALID CANARY-SQL"
                }])
                .is_err()
        );
        assert!(
            store
                .query_events(&tmc_store::EventQuery {
                    cmdr: Some("CANARY-CMDR".into()),
                    name: Some("Scan".into()),
                    limit: 9999,
                    ..Default::default()
                })
                .unwrap()
                .is_empty()
        );
        assert!(tmc_journal::resolve_journal_dir(Some(&dir.path().join("CANARY-PATH"))).is_err());
        assert!(tmc_journal::read_hud_theme(Some(&dir.path().join("CANARY-PATH"))).is_default);
        fs::write(dir.path().join("Journal.2026-10-20T010203.01.log"), "").unwrap();
        assert_eq!(tmc_journal::list_journals(dir.path()).unwrap().len(), 1);
    });
    payload_for(&records, "store.migration.applied");
    payload_for(&records, "store.migration.skipped");
    assert_eq!(
        payload_for(&records, "store.migration.failed")["failure_class"],
        "sqlite"
    );
    assert_eq!(
        payload_for(&records, "store.events.query_started")["limit"],
        1000
    );
    assert_eq!(
        payload_for(&records, "store.events.query_completed")["rows"],
        0
    );
    payload_for(&records, "source.locate.configured_invalid");
    payload_for(&records, "source.hud.read_failed");
    assert_eq!(payload_for(&records, "source.files.listed")["files"], 1);
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_unknown_filename_and_typed_parse_errors_do_not_leak_canaries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CANARY-FILE-NAME.log");
    fs::write(&path, "x\n").unwrap();
    let records = capture_sources(|| {
        tmc_journal::LineTailer::open(&path, 0)
            .unwrap()
            .read_new_lines()
            .unwrap();
        let parsed = tmc_model::events::parse_line(
            r#"{"timestamp":"2026-10-20T01:00:00Z","event":"Rank","Combat":"CANARY-ERROR-TEXT"}"#,
        )
        .unwrap();
        assert_eq!(parsed.kind, tmc_model::events::EventKind::Other);
        assert!(
            tmc_model::events::parse_line(r#"{"timestamp":"CANARY-PARSE-TEXT","event":"Scan"}"#)
                .is_err()
        );
    });
    assert_eq!(
        payload_for(&records, "journal.tail.opened")["file"],
        "local_source"
    );
    payload_for(&records, "journal.parse.typed_fallback");
    payload_for(&records, "journal.parse.invalid_json");
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_general_logs_also_classify_errors_without_paths_or_parse_values() {
    #[derive(Clone)]
    struct Sink(Arc<std::sync::Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("CANARY-PRIVATE-PATH");
    fs::create_dir(&private).unwrap();
    fs::write(
        private.join("Journal.2026-10-20T010203.01.log"),
        "{\"timestamp\":\"CANARY-INVALID-TIMESTAMP\",\"event\":\"Rank\"}\n",
    )
    .unwrap();
    let output = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Sink(output.clone());
    let (layer, guard) = start_writer(dir.path().join("logs"), Limits::default()).unwrap();
    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(move || sink.clone())
                .with_filter(tracing_subscriber::filter::filter_fn(|m| {
                    m.target() != TARGET
                })),
        )
        .with(
            layer.with_filter(tracing_subscriber::filter::filter_fn(|m| {
                m.target() == TARGET
            })),
        );
    tracing::subscriber::with_default(subscriber, || {
        assert!(tmc_journal::LineTailer::open(&private.join("CANARY-MISSING"), 0).is_err());
        assert!(
            tmc_journal::read_companion(&private, tmc_model::companion::CompanionKind::Cargo)
                .is_none()
        );
        let store = tmc_store::Store::open_in_memory().unwrap();
        tmc_store::import_history(
            &store,
            &private,
            &tmc_store::ImportOptions {
                throttle: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            },
            &mut |_, _| {},
        )
        .unwrap();
        store.exec_json("DROP TABLE kv", &[]).unwrap();
        assert!(store.kv_get("CANARY-KEY").is_none());
        store.kv_set("CANARY-KEY", "CANARY-VALUE");
    });
    drop(guard);
    let text = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    assert!(text.contains("sqlite"));
    assert!(text.contains("invalid history line skipped"));
    assert!(!text.contains("CANARY"));
    let records = read_records(&dir.path().join("logs"));
    assert_eq!(
        payload_for(&records, "journal.tail.open_failed")["failure_class"],
        "not_found"
    );
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

#[test]
fn d3_writer_flush_and_import_rollback_keep_semantics_and_hide_trigger_text() {
    let dir = tempfile::tempdir().unwrap();
    let name = "Journal.2026-10-20T010203.01.log";
    let raw = r#"{"timestamp":"2026-10-20T01:00:00Z","event":"Scan","BodyName":"Sol A 1","FID":"CANARY-FID"}"#;
    fs::write(dir.path().join(name), format!("{raw}\n")).unwrap();
    let records = capture_sources(|| {
        let store = tmc_store::Store::open_in_memory().unwrap();
        let event = tmc_store::StoredEvent {
            file_name: name.into(),
            line: 1,
            ts: Utc::now(),
            name: "Scan".into(),
            cmdr: Some("CANARY-CMDR".into()),
            json: raw.into(),
        };
        let writer = tmc_store::EventWriter::spawn(&store);
        writer.send(event.clone());
        writer.send(event);
        writer.flush_blocking().unwrap();
        assert_eq!(
            store
                .query_events(&tmc_store::EventQuery {
                    limit: 10,
                    ..Default::default()
                })
                .unwrap()
                .len(),
            1
        );
        drop(writer);
        let broken = tmc_store::Store::open_in_memory().unwrap();
        broken.exec_json("CREATE TRIGGER reject_event BEFORE INSERT ON event BEGIN SELECT RAISE(ABORT,'CANARY-TRIGGER-ERROR'); END",&[]).unwrap();
        assert!(
            tmc_store::import_history(
                &broken,
                dir.path(),
                &tmc_store::ImportOptions {
                    throttle: Arc::new(std::sync::atomic::AtomicBool::new(false))
                },
                &mut |_, _| {}
            )
            .is_err()
        );
        assert_eq!(
            broken
                .query_json("SELECT offset FROM journal_file", &[])
                .unwrap(),
            vec![json!([0])]
        );
        assert_eq!(
            broken
                .query_json("SELECT count(*) FROM event", &[])
                .unwrap(),
            vec![json!([0])]
        );
    });
    payload_for(&records, "store.writer.enqueue");
    assert_eq!(
        payload_for(&records, "store.writer.flush_confirmed")["succeeded"],
        true
    );
    assert_eq!(
        payload_for(&records, "store.import.batch_rollback")["failure_class"],
        "sqlite"
    );
    assert_eq!(
        payload_for(&records, "store.import.failed")["failure_class"],
        "sqlite"
    );
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

fn synthetic_journal(lines: usize) -> String {
    let mut text = String::with_capacity(lines * 120);
    for i in 0..lines {
        text.push_str(&format!(
            "{{\"timestamp\":\"2026-10-20T01:00:00Z\",\"event\":\"Music\",\"MusicTrack\":\"Track{i}\"}}
"
        ));
    }
    text
}

#[test]
fn d3_bulk_import_emits_records_proportional_to_batches_not_lines() {
    let dir = tempfile::tempdir().unwrap();
    let name = "Journal.2026-10-20T010203.01.log";
    fs::write(dir.path().join(name), synthetic_journal(20_000)).unwrap();
    let started = std::time::Instant::now();
    let records = capture_sources(|| {
        let store = tmc_store::Store::open_in_memory().unwrap();
        let opts = tmc_store::ImportOptions {
            throttle: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        assert_eq!(
            tmc_store::import_history(&store, dir.path(), &opts, &mut |_, _| {})
                .unwrap()
                .events,
            20_000
        );
    });
    eprintln!("d3 debug import of 20k lines took {:?}", started.elapsed());
    // 32 full lines (tail.line + context) + 4 batches + file/checkpoint/start/complete.
    assert!(
        records.len() < 200,
        "import emitted {} diagnostic records for 20k lines",
        records.len()
    );
    let batches = records
        .iter()
        .filter(|r| r["code"] == "store.import.batch_committed")
        .count();
    assert_eq!(batches, 4);
}

#[test]
fn d3_live_event_has_single_payload_and_query_payloads_are_capped() {
    let records = capture_sources(|| {
        let store = tmc_store::Store::open_in_memory().unwrap();
        let writer = tmc_store::EventWriter::spawn(&store);
        for line in 1..=30u64 {
            writer.send(tmc_store::StoredEvent {
                file_name: "Journal.2026-10-20T010203.01.log".into(),
                line,
                ts: Utc::now(),
                name: "Music".into(),
                cmdr: None,
                json: r#"{"event":"Music","MusicTrack":"Exploration"}"#.into(),
            });
        }
        writer.flush_blocking().unwrap();
        let rows = store
            .query_events(&tmc_store::EventQuery {
                limit: 100,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 30);
        store
            .query_json("SELECT name FROM event WHERE name = ?1", &[json!("x")])
            .unwrap();
        store
            .query_json("SELECT 'CANARY-LITERAL' FROM event WHERE id = 987654", &[])
            .unwrap();
    });
    assert!(records.iter().all(|r| r["code"] != "store.event.payload"));
    assert_eq!(
        records
            .iter()
            .filter(|r| r["code"] == "store.events.row_payload")
            .count(),
        20
    );
    assert_eq!(
        payload_for(&records, "store.events.row_payload_summary")["omitted"],
        10
    );
    let templates: Vec<_> = records
        .iter()
        .filter(|r| r["code"] == "store.query.started")
        .map(|r| r["payload"]["sql_template"].clone())
        .collect();
    assert_eq!(templates[0], "SELECT name FROM event WHERE name = ?1");
    assert_eq!(templates[1], Value::Null);
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
}

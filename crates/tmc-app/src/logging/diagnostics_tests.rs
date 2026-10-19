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

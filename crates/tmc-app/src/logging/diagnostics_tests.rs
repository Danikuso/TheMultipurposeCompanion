use super::*;
use serde_json::{Value, json};
use std::io::Write;
use tracing_subscriber::filter::FilterExt;
use tracing_subscriber::prelude::*;

/// Timeline rows (ordered by session/seq) with payload parsed back into JSON.
fn read_records(dir: &Path) -> Vec<Value> {
    let conn = Connection::open(dir.join(DB_FILE)).unwrap();
    let mut stmt = conn
        .prepare("SELECT tbl, session_id, seq, mono_ns, ts, code, context, schema, payload, flags, module FROM diag_timeline")
        .unwrap();
    stmt.query_map([], |r| {
        let payload: Option<String> = r.get(8)?;
        Ok(json!({
            "tbl": r.get::<_, String>(0)?, "session": r.get::<_, String>(1)?,
            "seq": r.get::<_, i64>(2)?, "mono_ns": r.get::<_, i64>(3)?,
            "ts": r.get::<_, String>(4)?, "code": r.get::<_, String>(5)?,
            "context": r.get::<_, String>(6)?, "schema": r.get::<_, Option<String>>(7)?,
            "payload": payload.and_then(|p| serde_json::from_str::<Value>(&p).ok()).unwrap_or(Value::Null),
            "flags": r.get::<_, i64>(9)?, "module": r.get::<_, Option<String>>(10)?,
        }))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

fn record(code: &str, ts: String, payload: &str) -> Record {
    Record {
        mono_ns: 1,
        ts,
        context: "state".into(),
        schema: "state".into(),
        code: code.into(),
        payload: Some(payload.into()),
        flags: 0,
    }
}

fn open_db(dir: &Path, limits: Limits) -> Db {
    Db::open(dir, limits, uuid::Uuid::new_v4().to_string()).unwrap()
}

fn count(db: &Db, table: &str) -> i64 {
    db.conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
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
fn routing_by_code_prefix_including_unknown() {
    let table = |code: &str| route(code).table;
    for (code, expected) in [
        ("journal.tail.read", "diag_journal"),
        ("source.status.read", "diag_journal"),
        ("store.import.batch_committed", "diag_store"),
        ("config.bootstrap", "diag_config"),
        ("core.location", "diag_actor"),
        ("actor.event.processed", "diag_actor"),
        ("app.start", "diag_actor"),
        ("topic.changed", "diag_topic"),
        ("net.query", "diag_net"),
        ("eddn.send", "diag_net"),
        ("http.request", "diag_server"),
        ("ws.connect", "diag_server"),
        ("auth.pairing", "diag_server"),
        ("action.run", "diag_server"),
        ("clipboard.copy", "diag_server"),
        ("ui.render", "diag_ui"),
        ("diagnostics.stop", "diag_other"),
        ("unknown.code", "diag_other"),
        ("", "diag_other"),
        ("journalx.a", "diag_other"),
    ] {
        assert_eq!(table(code), expected, "{code}");
        assert_eq!(route(code).module, None);
    }
    for module in MODULES {
        let routed = route(&format!("{module}.decision"));
        assert_eq!(routed.table, "diag_module");
        assert_eq!(routed.module, Some(module));
    }
}

#[test]
fn schema_has_tables_indexes_and_timeline_view() {
    let dir = tempfile::tempdir().unwrap();
    let db = open_db(dir.path(), Limits::default());
    let names = |kind: &str| -> Vec<String> {
        let mut stmt = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type=?1")
            .unwrap();
        stmt.query_map([kind], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    let tables = names("table");
    for table in TABLES.iter().chain(["diag_session"].iter()) {
        assert!(tables.contains(&table.to_string()), "{table}");
    }
    let indexes = names("index");
    for table in TABLES {
        assert!(indexes.contains(&format!("{table}_session_seq")));
        assert!(indexes.contains(&format!("{table}_code")));
    }
    assert!(names("view").contains(&"diag_timeline".to_string()));
    let mode: String = db
        .conn
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn real_emit_is_stored_redacted_and_unicode_intact() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer.with_filter(
        tracing_subscriber::filter::filter_fn(|m| m.target() == TARGET),
    ));
    let raw = json!({"BodyName":"ÃƒÅ½Ã‚Â©\"\n".repeat(1200),"FID":"CANARY-SECRET","MassEM":42});
    scoped(subscriber, || {
        tmc_model::diagnostics::emit(
            tmc_model::diagnostics::DiagnosticContext::Journal { event: "Scan" },
            "test.scan",
            &raw,
        );
    });
    drop(guard);
    let records = read_records(dir.path());
    let row = records.iter().find(|r| r["code"] == "test.scan").unwrap();
    assert_eq!(row["tbl"], "diag_other");
    assert_eq!(row["flags"], 0);
    assert_eq!(row["payload"]["BodyName"], raw["BodyName"]);
    assert_eq!(row["payload"]["MassEM"], 42);
    assert!(!serde_json::to_string(&records).unwrap().contains("CANARY"));
    assert_eq!(records.last().unwrap()["code"], "diagnostics.stop");
}

#[test]
fn batches_commit_together_and_keep_seq_order_across_tables() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer);
    let codes = ["journal.a", "store.b", "combat.c", "net.d", "mystery.e"];
    scoped(subscriber, || {
        for i in 0..500 {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "test.placeholder",
                json!({"i":i})
            );
        }
    });
    drop(guard);
    let records = read_records(dir.path());
    assert_eq!(records.len(), 502);
    let seqs: Vec<i64> = records.iter().map(|r| r["seq"].as_i64().unwrap()).collect();
    assert_eq!(seqs, (1..=502).collect::<Vec<_>>());
    assert_eq!(records[0]["code"], "diagnostics.start");
    let i: Vec<i64> = records[1..501]
        .iter()
        .map(|r| r["payload"]["i"].as_i64().unwrap())
        .collect();
    assert_eq!(i, (0..500).collect::<Vec<_>>());
    // Direct multi-table batch: rows land in their own tables, module column populated.
    let mut db = open_db(dir.path(), Limits::default());
    let batch: Vec<_> = codes
        .iter()
        .map(|c| record(c, now_ts(), "{\"v\":1}"))
        .collect();
    db.write_batch(&batch).unwrap();
    assert_eq!(db.seq, 5);
    for (table, expected) in [
        ("diag_journal", 1),
        ("diag_store", 1),
        ("diag_module", 1),
        ("diag_net", 1),
        ("diag_other", 503),
    ] {
        assert_eq!(count(&db, table), expected, "{table}");
    }
    let module: String = db
        .conn
        .query_row("SELECT module FROM diag_module", [], |r| r.get(0))
        .unwrap();
    assert_eq!(module, "combat");
}

#[test]
fn oversized_and_invalid_payloads_are_flagged_not_stored() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let subscriber = tracing_subscriber::registry().with(layer);
    scoped(subscriber, || {
        tracing::trace!(target: "tmc::diagnostics", context = "state", schema = "state", code = "test.large", payload = "x".repeat(PAYLOAD_MAX_BYTES + 1).as_str());
        tracing::trace!(target: "tmc::diagnostics", context = "state", schema = "state", code = "test.bad", payload = "{not json");
    });
    drop(guard);
    let records = read_records(dir.path());
    let large = records.iter().find(|r| r["code"] == "test.large").unwrap();
    assert_eq!(large["flags"], FLAG_TRUNCATED);
    assert_eq!(large["payload"]["truncated"], true);
    let bad = records.iter().find(|r| r["code"] == "test.bad").unwrap();
    assert_eq!(bad["flags"], FLAG_BAD_PAYLOAD);
    assert!(bad["payload"].is_null());
}

#[test]
fn directory_failure_is_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    std::fs::write(&path, "x").unwrap();
    assert!(start_writer(path, Limits::default()).is_err());
}

#[test]
fn queue_overflow_is_nonblocking_and_loss_survives_guard_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let shared = layer.shared.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let subscriber = tracing_subscriber::registry().with(DiagnosticLayer {
        sender,
        shared: shared.clone(),
    });
    scoped(subscriber, || {
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
    let marker = records
        .iter()
        .find(|r| r["code"] == "diagnostics.incomplete")
        .unwrap();
    assert!(marker["payload"]["dropped_records"].as_u64().unwrap() >= 9);
    assert_eq!(marker["flags"], FLAG_DROPPED | FLAG_INCOMPLETE);
}

#[test]
fn retention_by_age_removes_old_rows_and_sessions_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_db(dir.path(), Limits::default());
    let old = ts_of(Utc::now() - chrono::Duration::days(8));
    let batch = [
        record("journal.old", old.clone(), "{}"),
        record("combat.old", old.clone(), "{}"),
        record("journal.new", now_ts(), "{}"),
    ];
    db.write_batch(&batch).unwrap();
    db.conn
        .execute(
            "INSERT INTO diag_session(session_id, started_ts, ended_ts, version, profile) VALUES('old', ?1, ?1, 'x', 'debug')",
            [&old],
        )
        .unwrap();
    db.prune(Utc::now()).unwrap();
    assert_eq!(count(&db, "diag_journal"), 1);
    assert_eq!(count(&db, "diag_module"), 0);
    assert_eq!(count(&db, "diag_session"), 1);
    let code: String = db
        .conn
        .query_row("SELECT code FROM diag_journal", [], |r| r.get(0))
        .unwrap();
    assert_eq!(code, "journal.new");
}

#[test]
fn retention_by_size_prunes_oldest_batches_and_stays_under_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let max = 400 * 1024;
    let mut db = open_db(
        dir.path(),
        Limits {
            max_bytes: max,
            ..Limits::default()
        },
    );
    let padding = "p".repeat(400);
    for chunk in 0..20 {
        let batch: Vec<_> = (0..100)
            .map(|i| {
                record(
                    "journal.fill",
                    now_ts(),
                    &json!({"n": chunk * 100 + i, "pad": padding}).to_string(),
                )
            })
            .collect();
        db.write_batch(&batch).unwrap();
    }
    assert!(db.live_bytes().unwrap() > max);
    db.prune(Utc::now()).unwrap();
    assert!(db.live_bytes().unwrap() <= max);
    let (min, newest): (i64, i64) = db
        .conn
        .query_row(
            "SELECT MIN(json_extract(payload,'$.n')), MAX(json_extract(payload,'$.n')) FROM diag_journal",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(min > 0, "oldest rows must be pruned first");
    assert_eq!(newest, 1999, "newest row must survive");
    let file = std::fs::metadata(dir.path().join(DB_FILE)).unwrap().len();
    assert!(
        file <= max * 2,
        "incremental vacuum must return pages: {file}"
    );
}

#[test]
#[allow(clippy::let_and_return)]
fn age_pruning_probes_by_rowid_and_deletes_only_up_to_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_db(dir.path(), Limits::default());
    // The "nothing expired" probe must be an index search, never a table scan.
    let plan = |sql: &str| -> Vec<String> {
        let mut stmt = db
            .conn
            .prepare(&format!(
                "EXPLAIN QUERY PLAN {}",
                sql.replace("{t}", "diag_journal")
            ))
            .unwrap();
        let rows = if sql.contains("?1") {
            stmt.query_map([1], |r| r.get::<_, String>(3))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        } else {
            stmt.query_map([], |r| r.get::<_, String>(3))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        rows
    };
    let oldest = plan(OLDEST_TS_SQL);
    assert!(oldest.iter().any(|d| d.contains("SEARCH")), "{oldest:?}");
    assert!(oldest.iter().all(|d| !d.starts_with("SCAN")), "{oldest:?}");
    let from = plan(ROW_FROM_SQL);
    assert!(from.iter().any(|d| d.contains("SEARCH")), "{from:?}");
    let old = ts_of(Utc::now() - chrono::Duration::days(9));
    let mut batch: Vec<_> = (0..300)
        .map(|_| record("journal.o", old.clone(), "{}"))
        .collect();
    batch.extend((0..700).map(|_| record("journal.n", now_ts(), "{}")));
    db.write_batch(&batch).unwrap();
    let cutoff = Utc::now() - chrono::Duration::days(7);
    db.prune_by_age(cutoff).unwrap();
    assert_eq!(count(&db, "diag_journal"), 700);
    // Nothing expired any more: nothing deleted.
    db.prune_by_age(cutoff).unwrap();
    assert_eq!(count(&db, "diag_journal"), 700);
}

#[test]
fn transient_busy_is_retried_without_losing_seq_contiguity() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_db(dir.path(), Limits::default());
    db.conn.busy_timeout(Duration::ZERO).unwrap();
    db.write_batch(&[record("journal.a", now_ts(), "{}")])
        .unwrap();
    let holder = Connection::open(dir.path().join(DB_FILE)).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(60));
        holder.execute_batch("COMMIT").unwrap();
    });
    db.write_batch_retrying(&[
        record("journal.b", now_ts(), "{}"),
        record("journal.c", now_ts(), "{}"),
    ])
    .unwrap();
    release.join().unwrap();
    assert_eq!(db.seq, 3);
    let seqs: Vec<i64> = db
        .conn
        .prepare("SELECT seq FROM diag_journal ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(seqs, vec![1, 2, 3]);
    // A persistent lock exhausts the retries, rolls back and keeps seq unchanged.
    let holder = Connection::open(dir.path().join(DB_FILE)).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(
        db.write_batch_retrying(&[record("journal.d", now_ts(), "{}")])
            .is_err()
    );
    assert_eq!(db.seq, 3);
}

#[test]
fn open_with_database_locked_by_a_viewer_returns_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    drop(open_db(dir.path(), Limits::default()));
    let viewer = Connection::open(dir.path().join(DB_FILE)).unwrap();
    viewer.execute_batch("BEGIN EXCLUSIVE").unwrap();
    // Degrades or reports an error (logging.rs prints one warning); it must not panic.
    let _ = Db::open(
        dir.path(),
        Limits::default(),
        uuid::Uuid::new_v4().to_string(),
    );
}

#[test]
fn orphan_sessions_are_removed_with_per_table_exists() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_db(
        dir.path(),
        Limits {
            max_bytes: 1,
            ..Limits::default()
        },
    );
    for id in ["orphan", "kept"] {
        db.conn
            .execute(
                "INSERT INTO diag_session(session_id, started_ts, version, profile) VALUES(?1, ?2, 'x', 'debug')",
                params![id, now_ts()],
            )
            .unwrap();
    }
    db.write_batch(&[record("journal.k", now_ts(), "{}")])
        .unwrap();
    db.conn
        .execute("UPDATE diag_journal SET session_id='kept'", [])
        .unwrap();
    db.drop_orphan_sessions().unwrap();
    let ids: Vec<String> = db
        .conn
        .prepare("SELECT session_id FROM diag_session")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(ids.contains(&"kept".to_string()));
    assert!(!ids.contains(&"orphan".to_string()));
    assert_eq!(ids.len(), 2, "current session is always kept");
}

#[test]
fn write_failure_marks_once_drops_and_never_panics() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let shared = layer.shared.clone();
    // Break the destination from outside: the writer's inserts will fail.
    let sabotage = Connection::open(dir.path().join(DB_FILE)).unwrap();
    sabotage
        .execute_batch("DROP VIEW diag_timeline; DROP TABLE diag_other;")
        .unwrap();
    drop(sabotage);
    let subscriber = tracing_subscriber::registry().with(layer);
    scoped(subscriber, || {
        for _ in 0..5 {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "test.fail",
                json!({"credits":42})
            );
        }
    });
    drop(guard);
    assert_eq!(shared.disk_errors.load(Ordering::Acquire), 1);
    assert!(shared.dropped.load(Ordering::Acquire) >= 5);
}

#[test]
fn session_row_tracks_start_and_clean_end_and_restart_adds_another() {
    let dir = tempfile::tempdir().unwrap();
    let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    let session = layer.shared.session.clone();
    let probe = || -> (Option<String>, i64, i64) {
        let conn = Connection::open(dir.path().join(DB_FILE)).unwrap();
        let (ended, clean) = conn
            .query_row(
                "SELECT ended_ts, clean_exit FROM diag_session WHERE session_id=?1",
                [&session],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let sessions = conn
            .query_row("SELECT COUNT(*) FROM diag_session", [], |r| r.get(0))
            .unwrap();
        (ended, clean, sessions)
    };
    assert_eq!(probe(), (None, 0, 1));
    drop(guard);
    let (ended, clean, _) = probe();
    assert!(ended.is_some());
    assert_eq!(clean, 1);
    let (_layer, guard2) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    assert_eq!(probe().2, 2);
    drop(guard2);
}

#[test]
fn legacy_own_jsonl_files_are_removed_once_and_foreign_kept() {
    let dir = tempfile::tempdir().unwrap();
    let own = format!(
        "tmc-debug-{}-{}-0000000001.jsonl",
        Utc::now().format("%Y%m%dT%H%M%S%.9fZ"),
        uuid::Uuid::new_v4()
    );
    fs::write(dir.path().join(&own), "old").unwrap();
    fs::write(dir.path().join("foreign.jsonl"), "foreign").unwrap();
    fs::write(dir.path().join("tmc-debug-foreign.jsonl"), "foreign").unwrap();
    let (_layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
    drop(guard);
    assert!(!dir.path().join(own).exists());
    assert!(dir.path().join("foreign.jsonl").exists());
    assert!(dir.path().join("tmc-debug-foreign.jsonl").exists());
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
        .try_send(record("test.inflight", now_ts(), "{}"))
        .unwrap();
    shared.active.fetch_sub(1, Ordering::AcqRel);
    done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    handle.join().unwrap();
    let records = read_records(dir.path());
    assert!(records.iter().any(|r| r["code"] == "test.inflight"));
    assert_eq!(records.last().unwrap()["code"], "diagnostics.stop");
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
        scoped(subscriber, || {
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

#[test]
fn producer_entry_racing_guard_close_never_loses_an_accepted_event() {
    // Entry and close start together. Neither thread signals completion/entry to the other.
    for _ in 0..64 {
        let dir = tempfile::tempdir().unwrap();
        let (layer, guard) = start_writer(dir.path().to_owned(), Limits::default()).unwrap();
        let shared = layer.shared.clone();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let producer_barrier = barrier.clone();
        let producer = std::thread::spawn(move || {
            let subscriber = tracing_subscriber::registry().with(layer);
            scoped(subscriber, || {
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
    scoped(subscriber, f);
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
    scoped(subscriber, || {
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

/// Ejecuta `f` con `subscriber` como suscriptor del hilo, de una en una entre pruebas.
///
/// El interés de los callsites se cachea globalmente: dos suscriptores simultáneos o un
/// callsite registrado antes sin suscriptor lo dejan en «nunca». Se serializa y se recalcula.
fn scoped<S, R>(subscriber: S, f: impl FnOnce() -> R) -> R
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        f()
    })
}

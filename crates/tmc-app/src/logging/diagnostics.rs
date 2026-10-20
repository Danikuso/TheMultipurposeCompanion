//! Bounded debug writer backed by a separate SQLite database.
//! This entire module is absent from release builds. The writer never uses
//! `tmc-store` and never emits diagnostics about itself.
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, Transaction, params};
use serde_json::{Value, json};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tracing::{
    Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context};

const TARGET: &str = "tmc::diagnostics";
const DB_FILE: &str = "diagnostics.sqlite";
const PAYLOAD_MAX_BYTES: usize = 128 * 1024;
const QUEUE_RECORDS: usize = 2048;
const PRUNE_BATCH: usize = 5000;
/// Backoff between retries of a batch that hit a transient SQLITE_BUSY/LOCKED.
const BUSY_BACKOFF: [Duration; 3] = [
    Duration::from_millis(20),
    Duration::from_millis(50),
    Duration::from_millis(100),
];
/// Probe: timestamp of the oldest row, found through the rowid index (no table scan).
const OLDEST_TS_SQL: &str = "SELECT ts FROM {t} WHERE id = (SELECT MIN(id) FROM {t})";
/// Probe: first row at or after an id, used for the binary search of the expiry boundary.
const ROW_FROM_SQL: &str = "SELECT id, ts FROM {t} WHERE id >= ?1 ORDER BY id LIMIT 1";

fn is_transient(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(e, _)
            if matches!(e.code, rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
}
fn retry_busy<T>(mut op: impl FnMut() -> rusqlite::Result<T>) -> rusqlite::Result<T> {
    let mut attempt = 0;
    loop {
        match op() {
            Err(e) if is_transient(&e) && attempt < 3 => {
                attempt += 1;
                thread::sleep(Duration::from_millis(100));
            }
            other => return other,
        }
    }
}
fn optional<T>(result: rusqlite::Result<T>) -> rusqlite::Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

const FLAG_TRUNCATED: i64 = 1;
const FLAG_DROPPED: i64 = 2;
const FLAG_INCOMPLETE: i64 = 4;
const FLAG_BAD_PAYLOAD: i64 = 8;

const TABLES: [&str; 10] = [
    "diag_journal",
    "diag_store",
    "diag_config",
    "diag_actor",
    "diag_module",
    "diag_topic",
    "diag_net",
    "diag_server",
    "diag_ui",
    "diag_other",
];
const MODULES: [&str; 8] = [
    "combat",
    "mining",
    "trade",
    "missions",
    "navigation",
    "exploration",
    "exobiology",
    "dashboard",
];

#[derive(Clone, Copy)]
pub(super) struct Limits {
    max_bytes: u64,
    retention_days: i64,
    batch_records: usize,
    batch_interval: Duration,
    prune_interval: Duration,
    age_interval: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            retention_days: 7,
            batch_records: 200,
            batch_interval: Duration::from_millis(250),
            prune_interval: Duration::from_secs(30),
            age_interval: Duration::from_secs(600),
        }
    }
}

pub(super) fn directory_for_executable(exe: &Path) -> io::Result<PathBuf> {
    exe.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.join("debug_logs"))
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

/// Destination of a diagnostic code: table plus the module column for `diag_module`.
#[derive(Debug, PartialEq, Eq)]
struct Route {
    table: &'static str,
    module: Option<&'static str>,
}
fn route(code: &str) -> Route {
    let prefix = code.split('.').next().unwrap_or("");
    let table = match prefix {
        "journal" | "source" => "diag_journal",
        "store" => "diag_store",
        "config" => "diag_config",
        "core" | "actor" | "app" => "diag_actor",
        "topic" => "diag_topic",
        "net" | "eddn" => "diag_net",
        "http" | "ws" | "auth" | "action" | "clipboard" => "diag_server",
        "ui" => "diag_ui",
        other => {
            if let Some(module) = MODULES.iter().find(|m| **m == other) {
                return Route {
                    table: "diag_module",
                    module: Some(module),
                };
            }
            "diag_other"
        }
    };
    Route {
        table,
        module: None,
    }
}

struct Shared {
    closing: AtomicBool,
    active: AtomicUsize,
    dropped: AtomicU64,
    incomplete: AtomicU64,
    last_incomplete: AtomicU64,
    disk_errors: AtomicU64,
    next_event: AtomicU64,
    session: String,
    started: Instant,
}

pub(super) struct DiagnosticGuard {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}
impl Drop for DiagnosticGuard {
    fn drop(&mut self) {
        self.shared.closing.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            eprintln!("TMC diagnostics: worker_failed");
        }
    }
}

/// One queued row. The payload is already redacted JSON from D1.
struct Record {
    mono_ns: u64,
    ts: String,
    context: String,
    schema: String,
    code: String,
    payload: Option<String>,
    flags: i64,
}
impl Record {
    fn marker(shared: &Shared, code: &str, details: Value, flags: i64) -> Self {
        Self {
            mono_ns: shared.started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
            ts: now_ts(),
            context: "state".into(),
            schema: "diagnostics".into(),
            code: code.into(),
            payload: Some(details.to_string()),
            flags,
        }
    }
}
fn ts_of(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn now_ts() -> String {
    ts_of(Utc::now())
}

pub(super) struct DiagnosticLayer {
    sender: SyncSender<Record>,
    shared: Arc<Shared>,
}
#[derive(Default)]
struct Fields {
    context: Option<String>,
    schema: Option<String>,
    code: Option<String>,
    payload: Option<String>,
}
impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "context" => self.context = Some(value.into()),
            "schema" => self.schema = Some(value.into()),
            "code" => self.code = Some(value.into()),
            "payload" => self.payload = Some(value.into()),
            _ => {}
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        // Only D1's sanitized Display payload crosses this boundary. Extra event fields are ignored.
        if field.name() == "payload" {
            self.payload = Some(format!("{value:?}"));
        }
    }
}
impl<S: Subscriber> Layer<S> for DiagnosticLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() != TARGET || *event.metadata().level() != tracing::Level::TRACE
        {
            return;
        }
        let mut fields = Fields::default();
        event.record(&mut fields);
        let (Some(context), Some(schema), Some(code), Some(payload)) =
            (fields.context, fields.schema, fields.code, fields.payload)
        else {
            return;
        };
        // All entry/close operations share one total order: an entrant that sees
        // closing=false precedes the close store, whose subsequent active load
        // must see that entrant until its RAII decrement.
        self.shared.active.fetch_add(1, Ordering::SeqCst);
        let _active = ActiveProducer(&self.shared.active);
        if self.shared.closing.load(Ordering::SeqCst) {
            return;
        }
        if context.len() > 32 || schema.len() > 128 || code.len() > 128 {
            return;
        }
        let id = self.shared.next_event.fetch_add(1, Ordering::Relaxed);
        let (payload, flags) = if payload.len() > PAYLOAD_MAX_BYTES {
            (
                Some(json!({"truncated":true,"payload_bytes":payload.len()}).to_string()),
                FLAG_TRUNCATED,
            )
        } else if serde_json::from_str::<Value>(&payload).is_ok() {
            (Some(payload), 0)
        } else {
            (None, FLAG_BAD_PAYLOAD)
        };
        let record = Record {
            mono_ns: self
                .shared
                .started
                .elapsed()
                .as_nanos()
                .min(u64::MAX as u128) as u64,
            ts: now_ts(),
            context,
            schema,
            code,
            payload,
            flags,
        };
        if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) =
            self.sender.try_send(record)
        {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            self.shared.incomplete.fetch_add(1, Ordering::Relaxed);
            self.shared.last_incomplete.store(id, Ordering::Relaxed);
        }
    }
}

struct ActiveProducer<'a>(&'a AtomicUsize);
impl Drop for ActiveProducer<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn db_err(error: rusqlite::Error) -> io::Error {
    io::Error::other(error)
}

fn schema_sql() -> String {
    let mut sql = String::from(
        "CREATE TABLE IF NOT EXISTS diag_session(
            id INTEGER PRIMARY KEY, session_id TEXT NOT NULL UNIQUE, started_ts TEXT NOT NULL,
            ended_ts TEXT, version TEXT NOT NULL, profile TEXT NOT NULL,
            dropped INTEGER NOT NULL DEFAULT 0, incomplete INTEGER NOT NULL DEFAULT 0,
            disk_errors INTEGER NOT NULL DEFAULT 0, clean_exit INTEGER NOT NULL DEFAULT 0);\n",
    );
    let mut view = Vec::new();
    for table in TABLES {
        let module_col = if table == "diag_module" {
            ", module TEXT NOT NULL"
        } else {
            ""
        };
        sql.push_str(&format!(
            "CREATE TABLE IF NOT EXISTS {table}(
                id INTEGER PRIMARY KEY, session_id TEXT NOT NULL, seq INTEGER NOT NULL,
                mono_ns INTEGER NOT NULL, ts TEXT NOT NULL, code TEXT NOT NULL,
                context TEXT NOT NULL, schema TEXT, payload TEXT,
                flags INTEGER NOT NULL DEFAULT 0{module_col});
            CREATE INDEX IF NOT EXISTS {table}_session_seq ON {table}(session_id, seq);
            CREATE INDEX IF NOT EXISTS {table}_code ON {table}(code);\n"
        ));
        let module = if table == "diag_module" {
            "module"
        } else {
            "NULL"
        };
        view.push(format!(
            "SELECT '{table}' AS tbl, id, session_id, seq, mono_ns, ts, code, context, schema, payload, flags, {module} AS module FROM {table}"
        ));
    }
    sql.push_str(&format!(
        "CREATE VIEW IF NOT EXISTS diag_timeline AS {} ORDER BY session_id, seq;",
        view.join(" UNION ALL ")
    ));
    sql
}

struct Db {
    conn: Connection,
    session: String,
    limits: Limits,
    seq: i64,
}
impl Db {
    fn open(dir: &Path, limits: Limits, session: String) -> io::Result<Self> {
        if let Ok(meta) = fs::symlink_metadata(dir)
            && (meta.file_type().is_symlink() || !meta.is_dir())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        fs::create_dir_all(dir)?;
        remove_legacy_jsonl(dir);
        let file = dir.join(DB_FILE);
        if fs::symlink_metadata(&file).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let conn = Connection::open(file).map_err(db_err)?;
        conn.busy_timeout(Duration::from_secs(2)).map_err(db_err)?;
        let auto: i64 = conn
            .pragma_query_value(None, "auto_vacuum", |r| r.get(0))
            .map_err(db_err)?;
        if auto != 2 {
            // Startup only: switching mode needs a VACUUM on an existing database.
            // If another process (a viewer) keeps the file busy, degrade: no one-off VACUUM.
            let _ = retry_busy(|| conn.pragma_update(None, "auto_vacuum", 2));
            let _ = retry_busy(|| conn.execute_batch("VACUUM"));
        }
        // WAL is an optimisation: a busy database keeps its current journal mode.
        let _ = retry_busy(|| conn.execute_batch("PRAGMA journal_mode=WAL;"));
        conn.execute_batch("PRAGMA synchronous=NORMAL; PRAGMA journal_size_limit=16777216;")
            .map_err(db_err)?;
        retry_busy(|| conn.execute_batch(&schema_sql())).map_err(db_err)?;
        conn.execute(
            "INSERT INTO diag_session(session_id, started_ts, version, profile) VALUES(?1, ?2, ?3, 'debug')",
            params![session, now_ts(), env!("CARGO_PKG_VERSION")],
        )
        .map_err(db_err)?;
        let mut db = Self {
            conn,
            session,
            limits,
            seq: 0,
        };
        let _ = db.prune(Utc::now());
        Ok(db)
    }

    /// Insert records in one transaction. `seq` advances only on commit.
    fn write_batch(&mut self, batch: &[Record]) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        let mut seq = self.seq;
        for record in batch {
            seq += 1;
            insert(&tx, &self.session, seq, record)?;
        }
        tx.commit()?;
        self.seq = seq;
        Ok(())
    }

    /// Retry a batch on transient BUSY/LOCKED; the failed attempt rolled back, so `seq` stays contiguous.
    fn write_batch_retrying(&mut self, batch: &[Record]) -> rusqlite::Result<()> {
        let mut backoff = BUSY_BACKOFF.iter();
        loop {
            match self.write_batch(batch) {
                Err(e) if is_transient(&e) => match backoff.next() {
                    Some(pause) => thread::sleep(*pause),
                    None => return Err(e),
                },
                other => return other,
            }
        }
    }

    fn end_session(&self, shared: &Shared) {
        let _ = self.conn.execute(
            "UPDATE diag_session SET ended_ts=?1, dropped=?2, incomplete=?3, disk_errors=?4, clean_exit=1 WHERE session_id=?5",
            params![
                now_ts(),
                shared.dropped.load(Ordering::Acquire) as i64,
                shared.incomplete.load(Ordering::Acquire) as i64,
                shared.disk_errors.load(Ordering::Acquire) as i64,
                self.session
            ],
        );
    }

    fn live_bytes(&self) -> rusqlite::Result<u64> {
        let pragma = |name: &str| -> rusqlite::Result<u64> {
            self.conn
                .pragma_query_value(None, name, |r| r.get::<_, i64>(0))
                .map(|v| v.max(0) as u64)
        };
        let pages = pragma("page_count")?.saturating_sub(pragma("freelist_count")?);
        Ok(pages * pragma("page_size")?)
    }

    fn prune(&mut self, now: DateTime<Utc>) -> rusqlite::Result<()> {
        self.prune_by_age(now - chrono::Duration::days(self.limits.retention_days))?;
        self.prune_by_size()
    }

    fn prune_by_age(&mut self, cutoff: DateTime<Utc>) -> rusqlite::Result<()> {
        let cutoff = ts_of(cutoff);
        for table in TABLES {
            let oldest: Option<String> = optional(self.conn.query_row(
                &OLDEST_TS_SQL.replace("{t}", table),
                [],
                |r| r.get(0),
            ))?;
            // Nothing expired (or empty): no further reads of this table.
            if oldest.is_none_or(|ts| ts >= cutoff) {
                continue;
            }
            let boundary = self.expiry_boundary(table, &cutoff)?;
            loop {
                let deleted = self.conn.execute(
                    &format!(
                        "DELETE FROM {table} WHERE id IN (SELECT id FROM {table} WHERE id <= ?1 ORDER BY id LIMIT {PRUNE_BATCH})"
                    ),
                    params![boundary],
                )?;
                if deleted < PRUNE_BATCH {
                    break;
                }
            }
        }
        self.conn.execute(
            "DELETE FROM diag_session WHERE session_id <> ?1 AND COALESCE(ended_ts, started_ts) < ?2",
            params![self.session, cutoff],
        )?;
        Ok(())
    }

    /// Largest id whose ts is older than the cutoff. ts grows with id, so binary search over ids.
    fn expiry_boundary(&self, table: &str, cutoff: &str) -> rusqlite::Result<i64> {
        let bound = |f: &str| -> rusqlite::Result<i64> {
            self.conn
                .query_row(&format!("SELECT {f}(id) FROM {table}"), [], |r| r.get(0))
        };
        let (mut lo, mut hi) = (bound("MIN")?, bound("MAX")? + 1);
        let sql = ROW_FROM_SQL.replace("{t}", table);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            let row: Option<(i64, String)> = optional(
                self.conn
                    .query_row(&sql, [mid], |r| Ok((r.get(0)?, r.get(1)?))),
            )?;
            match row {
                Some((id, ts)) if ts.as_str() < cutoff => lo = id,
                _ => hi = mid,
            }
        }
        Ok(lo)
    }

    fn drop_orphan_sessions(&self) -> rusqlite::Result<()> {
        let absent: Vec<String> = TABLES
            .iter()
            .map(|t| {
                format!(
                    "NOT EXISTS (SELECT 1 FROM {t} WHERE {t}.session_id = diag_session.session_id)"
                )
            })
            .collect();
        self.conn.execute(
            &format!(
                "DELETE FROM diag_session WHERE session_id <> ?1 AND {}",
                absent.join(" AND ")
            ),
            params![self.session],
        )?;
        Ok(())
    }

    /// Drop the oldest tenth of every table per round until the live size fits.
    fn prune_by_size(&mut self) -> rusqlite::Result<()> {
        if self.live_bytes()? <= self.limits.max_bytes {
            return Ok(());
        }
        for _ in 0..200 {
            if self.live_bytes()? <= self.limits.max_bytes {
                break;
            }
            let mut removed = 0;
            for table in TABLES {
                let count: i64 =
                    self.conn
                        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
                let batch = count / 10 + 1;
                removed += self.conn.execute(
                    &format!(
                        "DELETE FROM {table} WHERE id IN (SELECT id FROM {table} ORDER BY id LIMIT {batch})"
                    ),
                    [],
                )?;
            }
            if removed == 0 {
                break;
            }
        }
        self.drop_orphan_sessions()?;
        // incremental_vacuum frees one page per step: drain every row.
        {
            let mut stmt = self.conn.prepare("PRAGMA incremental_vacuum")?;
            let mut rows = stmt.query([])?;
            while rows.next()?.is_some() {}
        }
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }
}

fn insert(tx: &Transaction, session: &str, seq: i64, r: &Record) -> rusqlite::Result<()> {
    let route = route(&r.code);
    let mono = r.mono_ns.min(i64::MAX as u64) as i64;
    let common = "session_id, seq, mono_ns, ts, code, context, schema, payload, flags";
    match route.module {
        Some(module) => tx
            .prepare_cached(&format!(
                "INSERT INTO diag_module({common}, module) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"
            ))?
            .execute(params![
                session, seq, mono, r.ts, r.code, r.context, r.schema, r.payload, r.flags, module
            ]),
        None => tx
            .prepare_cached(&format!(
                "INSERT INTO {}({common}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                route.table
            ))?
            .execute(params![
                session, seq, mono, r.ts, r.code, r.context, r.schema, r.payload, r.flags
            ]),
    }
    .map(|_| ())
}

pub(super) fn start_writer(
    dir: PathBuf,
    limits: Limits,
) -> io::Result<(DiagnosticLayer, DiagnosticGuard)> {
    let shared = Arc::new(Shared {
        closing: AtomicBool::new(false),
        active: AtomicUsize::new(0),
        dropped: AtomicU64::new(0),
        incomplete: AtomicU64::new(0),
        last_incomplete: AtomicU64::new(0),
        disk_errors: AtomicU64::new(0),
        next_event: AtomicU64::new(1),
        session: uuid::Uuid::new_v4().to_string(),
        started: Instant::now(),
    });
    let db = Db::open(&dir, limits, shared.session.clone())?;
    let (sender, receiver) = mpsc::sync_channel(QUEUE_RECORDS);
    let state = shared.clone();
    let worker = thread::Builder::new()
        .name("tmc-diagnostics".into())
        .spawn(move || run_worker(db, receiver, state))?;
    Ok((
        DiagnosticLayer {
            sender,
            shared: shared.clone(),
        },
        DiagnosticGuard {
            shared,
            worker: Some(worker),
        },
    ))
}

struct Output {
    db: Db,
    shared: Arc<Shared>,
    failed: bool,
}
impl Output {
    /// Write the batch plus pending loss counters; on failure mark once and drop everything.
    fn flush(&mut self, batch: &mut Vec<Record>) {
        if self.failed {
            // Counters stay as the only evidence; there is no alternative destination.
            self.shared
                .dropped
                .fetch_add(batch.len() as u64, Ordering::Relaxed);
            batch.clear();
            return;
        }
        let dropped = self.shared.dropped.swap(0, Ordering::AcqRel);
        let incomplete = self.shared.incomplete.swap(0, Ordering::AcqRel);
        if dropped != 0 || incomplete != 0 {
            batch.push(Record::marker(
                &self.shared,
                "diagnostics.incomplete",
                json!({"dropped_records":dropped,"incomplete_events":incomplete,"last_event_id":self.shared.last_incomplete.load(Ordering::Acquire)}),
                FLAG_DROPPED | FLAG_INCOMPLETE,
            ));
        }
        if batch.is_empty() {
            return;
        }
        if self.db.write_batch_retrying(batch).is_err() {
            self.failed = true;
            self.shared.disk_errors.fetch_add(1, Ordering::Relaxed);
            self.shared
                .dropped
                .fetch_add(batch.len() as u64, Ordering::Relaxed);
            eprintln!("TMC diagnostics: disk_write_failed");
        }
        batch.clear();
    }
}

fn run_worker(db: Db, receiver: Receiver<Record>, shared: Arc<Shared>) {
    let interval = db.limits.batch_interval;
    let capacity = db.limits.batch_records;
    let prune_every = db.limits.prune_interval;
    let age_every = db.limits.age_interval;
    let mut output = Output {
        db,
        shared,
        failed: false,
    };
    let mut batch = Vec::with_capacity(capacity);
    batch.push(Record::marker(
        &output.shared,
        "diagnostics.start",
        json!({"version":env!("CARGO_PKG_VERSION"),"profile":"debug"}),
        0,
    ));
    let mut flushed = Instant::now();
    let mut pruned = Instant::now();
    let mut aged = Instant::now();
    loop {
        if output.shared.closing.load(Ordering::SeqCst)
            && output.shared.active.load(Ordering::SeqCst) == 0
        {
            while let Ok(record) = receiver.try_recv() {
                batch.push(record);
                if batch.len() >= capacity {
                    output.flush(&mut batch);
                }
            }
            break;
        }
        match receiver.recv_timeout(interval) {
            Ok(record) => {
                batch.push(record);
                if batch.len() >= capacity {
                    output.flush(&mut batch);
                    flushed = Instant::now();
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if flushed.elapsed() >= interval {
            output.flush(&mut batch);
            flushed = Instant::now();
        }
        if !output.failed && aged.elapsed() >= age_every {
            let cutoff = Utc::now() - chrono::Duration::days(output.db.limits.retention_days);
            let _ = output.db.prune_by_age(cutoff);
            aged = Instant::now();
        }
        if !output.failed && pruned.elapsed() >= prune_every {
            // O(1) when under the limit: only page_count/freelist_count are read.
            let _ = output.db.prune_by_size();
            pruned = Instant::now();
        }
    }
    output.flush(&mut batch);
    batch.push(Record::marker(
        &output.shared,
        "diagnostics.stop",
        json!({"drained":true}),
        0,
    ));
    output.flush(&mut batch);
    output.db.end_session(&output.shared);
    let errors = output.shared.disk_errors.load(Ordering::Acquire);
    if errors != 0 {
        eprintln!("TMC diagnostics: shutdown_disk_errors count={errors}");
    }
}

/// One-time migration: remove only our own legacy circular JSONL files.
fn remove_legacy_jsonl(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_legacy_name(&name) && entry.file_type().is_ok_and(|t| t.is_file()) {
            let _ = fs::remove_file(entry.path());
        }
    }
}
fn is_legacy_name(name: &str) -> bool {
    let Some(rest) = name
        .strip_prefix("tmc-debug-")
        .and_then(|s| s.strip_suffix(".jsonl"))
    else {
        return false;
    };
    let Some((date, tail)) = rest.split_once('-') else {
        return false;
    };
    let Some((session, segment)) = tail.rsplit_once('-') else {
        return false;
    };
    uuid::Uuid::parse_str(session).is_ok()
        && segment.len() == 10
        && segment.bytes().all(|c| c.is_ascii_digit())
        && chrono::NaiveDateTime::parse_from_str(date, "%Y%m%dT%H%M%S%.9fZ").is_ok()
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;

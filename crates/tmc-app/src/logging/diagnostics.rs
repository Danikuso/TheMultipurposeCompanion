//! Bounded debug writer. This entire module is absent from release builds.
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Write},
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
const RECORD_BYTES: usize = 2048;
const MAX_EVENT_BYTES: usize = 1024 * 1024;
const QUEUE_RECORDS: usize = 2048;

#[derive(Clone, Copy)]
pub(super) struct Limits {
    segment_bytes: usize,
    segments: usize,
    retention_days: i64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            segment_bytes: 8 * 1024 * 1024,
            segments: 32,
            retention_days: 7,
        }
    }
}

pub(super) fn directory_for_executable(exe: &Path) -> io::Result<PathBuf> {
    exe.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.join("debug_logs"))
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
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

pub(super) struct DiagnosticLayer {
    sender: SyncSender<Vec<u8>>,
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
        // must see that entrant until its RAII decrement. Acquire/Release on two
        // independent atomics would permit store buffering and an early drain.
        self.shared.active.fetch_add(1, Ordering::SeqCst);
        let _active = ActiveProducer(&self.shared.active);
        if self.shared.closing.load(Ordering::SeqCst) {
            return;
        }
        // D1 alone supplies these validated schema labels and literal codes.
        if context.len() > 32 || schema.len() > 128 || code.len() > 128 {
            return;
        }
        let id = self.shared.next_event.fetch_add(1, Ordering::Relaxed);
        let base = json!({"session":self.shared.session,"utc":Utc::now().to_rfc3339(),"monotonic_us":self.shared.started.elapsed().as_micros() as u64,"event_id":id,"context":context,"schema":schema,"code":code});
        let records = encode_event(base, &payload);
        let mut dropped = 0;
        for record in records {
            if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) =
                self.sender.try_send(record)
            {
                dropped += 1;
            }
        }
        if dropped != 0 {
            self.shared.dropped.fetch_add(dropped, Ordering::Relaxed);
            self.shared.incomplete.fetch_add(1, Ordering::Relaxed);
            self.shared.last_incomplete.store(id, Ordering::Relaxed);
        }
    }
}

// Leave room for worker ordinal (u64) and newline, measured on fully escaped JSON.
const ORDINAL_RESERVE: usize = 40;
fn encode_event(mut base: Value, payload: &str) -> Vec<Vec<u8>> {
    if payload.len() > MAX_EVENT_BYTES {
        base["oversized"] = json!(true);
        base["payload_bytes"] = json!(payload.len());
        return vec![base.to_string().into_bytes()];
    }
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        base["payload"] = value;
        let normal = base.to_string().into_bytes();
        if normal.len() + ORDINAL_RESERVE <= RECORD_BYTES {
            return vec![normal];
        }
        if let Some(object) = base.as_object_mut() {
            object.remove("payload");
        }
    } else {
        base["serialization_failed"] = json!(true);
        return vec![base.to_string().into_bytes()];
    }
    // Measure the complete escaped record, including worst-case chunk metadata.
    let mut template = base.clone();
    template["chunk_index"] = json!(MAX_EVENT_BYTES);
    template["chunk_count"] = json!(MAX_EVENT_BYTES);
    template["start"] = json!(false);
    template["end"] = json!(false);
    let mut fragments = Vec::new();
    let mut offset = 0;
    while offset < payload.len() {
        let mut end = (offset + 1024).min(payload.len());
        loop {
            while !payload.is_char_boundary(end) {
                end -= 1;
            }
            template["chunk"] = json!(&payload[offset..end]);
            if template.to_string().len() + ORDINAL_RESERVE <= RECORD_BYTES {
                break;
            }
            end = offset + (end - offset) / 2;
            if end == offset {
                return vec![
                    json!({"code":"diagnostics.invalid_metadata"})
                        .to_string()
                        .into_bytes(),
                ];
            }
        }
        fragments.push(&payload[offset..end]);
        offset = end;
    }
    let count = fragments.len();
    fragments
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let mut record = base.clone();
            record["chunk"] = json!(chunk);
            record["chunk_index"] = json!(index);
            record["chunk_count"] = json!(count);
            record["start"] = json!(index == 0);
            record["end"] = json!(index + 1 == count);
            record.to_string().into_bytes()
        })
        .collect()
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
    let ring = Ring::new(dir, limits, shared.session.clone())?;
    let (sender, receiver) = mpsc::sync_channel(QUEUE_RECORDS);
    let state = shared.clone();
    let worker = thread::Builder::new()
        .name("tmc-diagnostics".into())
        .spawn(move || run_worker(ring, receiver, state))?;
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

struct ActiveProducer<'a>(&'a AtomicUsize);
impl Drop for ActiveProducer<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

struct Output {
    ring: Ring,
    shared: Arc<Shared>,
    ordinal: u64,
}
impl Output {
    fn write(&mut self, mut bytes: Vec<u8>) -> bool {
        self.ordinal += 1;
        if bytes.pop() != Some(b'}') {
            return false;
        }
        bytes.extend_from_slice(format!(",\"ordinal\":{}}}\n", self.ordinal).as_bytes());
        if self.ring.write(&bytes, Utc::now()).is_err() {
            self.shared.disk_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("TMC diagnostics: disk_write_failed");
            return false;
        }
        true
    }
    fn marker(&mut self, code: &str, details: Value) -> bool {
        self.write(json!({"session":self.shared.session,"utc":Utc::now().to_rfc3339(),"monotonic_us":self.shared.started.elapsed().as_micros() as u64,"code":code,"details":details}).to_string().into_bytes())
    }
    fn losses(&mut self) {
        let dropped = self.shared.dropped.swap(0, Ordering::AcqRel);
        let incomplete = self.shared.incomplete.swap(0, Ordering::AcqRel);
        let disk = self.shared.disk_errors.swap(0, Ordering::AcqRel);
        if (dropped != 0 || incomplete != 0 || disk != 0)
            && !self.marker("diagnostics.incomplete", json!({"dropped_records":dropped,"incomplete_events":incomplete,"last_event_id":self.shared.last_incomplete.load(Ordering::Acquire),"disk_errors":disk})) {
                self.shared.dropped.fetch_add(dropped, Ordering::Relaxed);
                self.shared.incomplete.fetch_add(incomplete, Ordering::Relaxed);
                self.shared.disk_errors.fetch_add(disk, Ordering::Relaxed);
        }
    }
    fn flush(&mut self) {
        if self.ring.flush().is_err() {
            self.shared.disk_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("TMC diagnostics: disk_flush_failed");
        }
    }
}
fn run_worker(ring: Ring, receiver: Receiver<Vec<u8>>, shared: Arc<Shared>) {
    let mut output = Output {
        ring,
        shared,
        ordinal: 0,
    };
    output.marker("diagnostics.start", json!({"version":env!("CARGO_PKG_VERSION"),"profile":"debug","segment_bytes":output.ring.limits.segment_bytes,"segments":output.ring.limits.segments}));
    let mut flushed = Instant::now();
    loop {
        if output.shared.closing.load(Ordering::SeqCst)
            && output.shared.active.load(Ordering::SeqCst) == 0
        {
            while let Ok(record) = receiver.try_recv() {
                output.write(record);
            }
            break;
        }
        match receiver.recv_timeout(Duration::from_millis(250)) {
            Ok(record) => {
                output.write(record);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if flushed.elapsed() >= Duration::from_millis(250) {
            output.losses();
            output.flush();
            flushed = Instant::now();
        }
    }
    output.losses();
    output.marker("diagnostics.stop", json!({"drained":true}));
    output.flush();
    let errors = output.shared.disk_errors.load(Ordering::Acquire);
    if errors != 0 {
        eprintln!("TMC diagnostics: shutdown_disk_errors count={errors}");
    }
}

struct Ring {
    dir: PathBuf,
    limits: Limits,
    session: String,
    segment: u64,
    current: Option<BufWriter<File>>,
    bytes: usize,
    day: String,
}
impl Ring {
    fn new(dir: PathBuf, limits: Limits, session: String) -> io::Result<Self> {
        if let Ok(meta) = fs::symlink_metadata(&dir)
            && (meta.file_type().is_symlink() || !meta.is_dir())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        fs::create_dir_all(&dir)?;
        let mut ring = Self {
            dir,
            limits,
            session,
            segment: 0,
            current: None,
            bytes: 0,
            day: String::new(),
        };
        ring.rotate(Utc::now())?;
        Ok(ring)
    }
    fn rotate(&mut self, now: DateTime<Utc>) -> io::Result<()> {
        self.flush()?;
        self.current = None;
        // Reserve the next slot before creating it. If an old file is locked,
        // leave no current segment and retry cleanup on the next write.
        self.cleanup(now)?;
        self.day = now.format("%Y%m%d").to_string();
        self.segment += 1;
        let name = format!(
            "tmc-debug-{}-{}-{:010}.jsonl",
            now.format("%Y%m%dT%H%M%S%.9fZ"),
            self.session,
            self.segment
        );
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.dir.join(name))?;
        self.current = Some(BufWriter::with_capacity(32 * 1024, file));
        self.bytes = 0;
        Ok(())
    }
    fn cleanup(&self, now: DateTime<Utc>) -> io::Result<()> {
        let mut files = owned_files(&self.dir)?;
        files.sort_by(|a, b| a.0.cmp(&b.0));
        let mut remaining = files.len();
        for (_, path, date) in files {
            if remaining >= self.limits.segments
                || now.signed_duration_since(date).num_days() >= self.limits.retention_days
            {
                fs::remove_file(path)?;
                remaining -= 1;
            }
        }
        Ok(())
    }
    fn write(&mut self, bytes: &[u8], now: DateTime<Utc>) -> io::Result<()> {
        if self.current.is_none()
            || self.day != now.format("%Y%m%d").to_string()
            || self.bytes + bytes.len() > self.limits.segment_bytes
        {
            self.rotate(now)?;
        }
        let result = self
            .current
            .as_mut()
            .ok_or(io::ErrorKind::NotConnected)?
            .write_all(bytes);
        if result.is_err() {
            self.current = None;
        } else {
            self.bytes += bytes.len();
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        let result = match self.current.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        };
        if result.is_err() {
            self.current = None;
        }
        result
    }
}
impl Drop for Ring {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
fn owned_files(dir: &Path) -> io::Result<Vec<(String, PathBuf, DateTime<Utc>)>> {
    let mut owned = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name
            .strip_prefix("tmc-debug-")
            .and_then(|s| s.strip_suffix(".jsonl"))
        else {
            continue;
        };
        let Some((date, tail)) = rest.split_once('-') else {
            continue;
        };
        let Some((session, segment)) = tail.rsplit_once('-') else {
            continue;
        };
        if uuid::Uuid::parse_str(session).is_err()
            || segment.len() != 10
            || !segment.bytes().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        let Ok(date) = chrono::NaiveDateTime::parse_from_str(date, "%Y%m%dT%H%M%S%.9fZ") else {
            continue;
        };
        owned.push((name, entry.path(), date.and_utc()));
    }
    Ok(owned)
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;

use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, params};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub file_name: String,
    pub line: u64,
    pub ts: DateTime<Utc>,
    pub name: String,
    pub cmdr: Option<String>,
    pub json: String,
}

enum Command {
    Event(StoredEvent),
    Flush(Sender<Result<(), StoreError>>),
    Shutdown,
}

pub struct EventWriter {
    sender: Sender<Command>,
    worker: Option<JoinHandle<()>>,
}

impl EventWriter {
    pub fn spawn(store: &Store) -> Self {
        let (sender, receiver) = mpsc::channel();
        let store = store.clone();
        let worker = thread::Builder::new()
            .name("tmc-event-writer".into())
            .spawn(move || run_writer(store, receiver))
            .map_err(|error| tracing::error!(failure_class = ?error.kind(), "failed to spawn event writer"))
            .ok();
        Self { sender, worker }
    }

    pub fn send(&self, ev: StoredEvent) {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.writer.enqueue",
            serde_json::json!({"file":tmc_model::diagnostics::source_file(std::path::Path::new(&ev.file_name)),"line":ev.line,"event":ev.name,"bytes":ev.json.len()})
        );
        if self.sender.send(Command::Event(ev)).is_err() {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.writer.enqueue_failed",
                serde_json::json!({"failure_class":"broken_pipe"})
            );
            tracing::error!("event writer is no longer running");
        }
    }

    pub fn flush_blocking(&self) -> Result<(), StoreError> {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.writer.flush_requested",
            serde_json::json!({"requested":true})
        );
        let (sender, receiver) = mpsc::channel();
        self.sender.send(Command::Flush(sender)).map_err(|_| {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.writer.flush_send_failed",
                serde_json::json!({"failure_class":"broken_pipe"})
            );
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "event writer is no longer running",
            )
        })?;
        let result = receiver.recv().map_err(|_| {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.writer.flush_ack_failed",
                serde_json::json!({"failure_class":"broken_pipe"})
            );
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "event writer closed before acknowledging flush",
            )
        })?;
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.writer.flush_confirmed",
            serde_json::json!({"succeeded":result.is_ok()})
        );
        result
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.writer.shutdown_requested",
            serde_json::json!({"requested":true})
        );
        let _ = self.sender.send(Command::Shutdown);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            tracing::error!("event writer thread panicked");
        }
    }
}

pub(crate) fn insert_event(conn: &Connection, ev: &StoredEvent) -> Result<usize, StoreError> {
    let result: Result<usize, StoreError> = (|| {
        let line = i64::try_from(ev.line)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        conn.execute(
            "INSERT OR IGNORE INTO journal_file (name, size, offset) VALUES (?1, 0, 0)",
            [&ev.file_name],
        )?;
        let file_id: i64 = conn.query_row(
            "SELECT id FROM journal_file WHERE name = ?1",
            [&ev.file_name],
            |row| row.get(0),
        )?;
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO event (file_id, line, ts, name, cmdr, json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file_id,
                line,
                ev.ts.to_rfc3339_opts(SecondsFormat::Nanos, true),
                ev.name,
                ev.cmdr,
                ev.json
            ],
        )?;
        #[cfg(debug_assertions)]
        if inserted > 0 {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.event.inserted",
                serde_json::json!({"line":ev.line,"event":ev.name,"file":tmc_model::diagnostics::source_file(std::path::Path::new(&ev.file_name)),"bytes":ev.json.len()})
            );
        } else {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.event.deduplicated",
                serde_json::json!({"line":ev.line,"event":ev.name,"file":tmc_model::diagnostics::source_file(std::path::Path::new(&ev.file_name))})
            );
        }
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Journal { event: &ev.name },
            "store.event.payload",
            tmc_model::diagnostics::decoded_json(&ev.json)
        );
        Ok(inserted)
    })();
    #[cfg(debug_assertions)]
    if let Err(error) = &result {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.event.failed",
            serde_json::json!({"line":ev.line,"failure_class":error.diagnostic_class()})
        );
    }
    result
}

fn commit_batch(store: &Store, pending: &mut Vec<StoredEvent>) -> Result<(), StoreError> {
    if pending.is_empty() {
        return Ok(());
    }
    #[cfg(debug_assertions)]
    let diagnostic_started = Instant::now();
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Sql,
        "store.writer.batch_started",
        serde_json::json!({"events":pending.len()})
    );
    let result = (|| -> Result<(), StoreError> {
        let mut conn = store.lock()?;
        let transaction = conn.transaction()?;
        for event in pending.iter() {
            insert_event(&transaction, event)?;
        }
        transaction.commit()?;
        Ok(())
    })();
    match &result {
        Ok(()) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.writer.committed",
                serde_json::json!({"events":pending.len(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
            );
            pending.clear();
        }
        Err(error) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.writer.rollback",
                serde_json::json!({"events":pending.len(),"failure_class":error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
            );
            tracing::error!(
                failure_class = error.diagnostic_class(),
                "failed to commit event batch"
            );
        }
    }
    result
}

fn run_writer(store: Store, receiver: Receiver<Command>) {
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Sql,
        "store.writer.started",
        serde_json::json!({"batch_limit":500,"interval_ms":500})
    );
    let interval = Duration::from_millis(500);
    let mut pending = Vec::with_capacity(500);
    let mut deadline = Instant::now() + interval;
    loop {
        let command = if pending.is_empty() {
            receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            receiver.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        };
        match command {
            Ok(Command::Event(event)) => {
                if pending.is_empty() {
                    deadline = Instant::now() + interval;
                }
                pending.push(event);
                if pending.len() >= 500 || Instant::now() >= deadline {
                    let _ = commit_batch(&store, &mut pending);
                    deadline = Instant::now() + interval;
                }
            }
            Ok(Command::Flush(acknowledge)) => {
                let result = commit_batch(&store, &mut pending);
                deadline = Instant::now() + interval;
                let _ = acknowledge.send(result);
            }
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                if let Err(error) = commit_batch(&store, &mut pending) {
                    tracing::error!(
                        failure_class = error.diagnostic_class(),
                        pending_events = pending.len(),
                        "event writer stopped with uncommitted events; original journals and unchanged import offsets allow recovery"
                    );
                }
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.writer.stopped",
                    serde_json::json!({"pending_events":pending.len()})
                );
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = commit_batch(&store, &mut pending);
                deadline = Instant::now() + interval;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(line: u64) -> StoredEvent {
        StoredEvent {
            file_name: "Journal.synthetic.log".into(),
            line,
            ts: "2026-10-17T12:00:00Z".parse().unwrap(),
            name: "Test".into(),
            cmdr: None,
            json: "{}".into(),
        }
    }

    #[test]
    fn dropping_writer_drains_pending_events() {
        let store = Store::open_in_memory().unwrap();
        {
            let writer = EventWriter::spawn(&store);
            writer.send(event(1));
            writer.send(event(2));
        }
        let count: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM event", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn writer_commits_on_deadline_without_explicit_flush() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        writer.send(event(1));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let count: i64 = store
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM event", [], |row| row.get(0))
                .unwrap();
            if count == 1 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "writer never committed pending batch"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn flush_reports_sqlite_busy_and_preserves_batch_for_retry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store.sqlite");
        let store = Store::open(&path).unwrap();
        store
            .lock()
            .unwrap()
            .busy_timeout(Duration::from_millis(20))
            .unwrap();
        let blocker = Connection::open(&path).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
        let writer = EventWriter::spawn(&store);
        writer.send(event(1));
        let started = Instant::now();
        assert!(writer.flush_blocking().is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        let count: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM event", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
        blocker.execute_batch("COMMIT").unwrap();
        writer.flush_blocking().unwrap();
        let count: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM event", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let offset: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT offset FROM journal_file", [], |row| row.get(0))
            .unwrap();
        assert_eq!(offset, 0);
    }
}

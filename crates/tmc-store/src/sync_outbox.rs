//! Cola de salida de la sincronización (`sync_outbox`) y cursor del router, en una sola transacción.

use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use tmc_model::sync::{ErrorClass, OutboxState, RowOrigin, SyncService};

/// Fila nueva para la bandeja de salida.
#[derive(Debug, Clone)]
pub struct NewOutboxRow {
    pub service: SyncService,
    pub cmdr_fid: String,
    pub idem_key: String,
    pub kind: String,
    pub coalesce_key: Option<String>,
    pub event_ts: DateTime<Utc>,
    pub payload: String,
    pub origin: RowOrigin,
    /// Pending, Omitted, Discarded o Expired.
    pub state: OutboxState,
    pub last_class: Option<ErrorClass>,
}

/// Avance del cursor de un archivo de diario (líneas 1-based; ausente = 0).
#[derive(Debug, Clone)]
pub struct CursorAdvance {
    pub cmdr_fid: String,
    pub journal_file: String,
    pub last_line: u64,
}

/// Resultado de encolar un lote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnqueueOutcome {
    pub inserted: u64,
    pub ignored: u64,
    /// Filas realmente insertadas con origen recuperado.
    pub recovered: usize,
}

fn ts(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn class_str(class: ErrorClass) -> Result<String, StoreError> {
    match serde_json::to_value(class) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        _ => Err(StoreError::Unsupported("error class".into())),
    }
}

impl Store {
    /// Encola las filas (`INSERT OR IGNORE`) y avanza los cursores en una única transacción.
    /// Un cursor con `last_line` 0 reinicia el de ese archivo (truncado); si no, solo avanza.
    pub fn enqueue_batch_with_cursor(
        &self,
        rows: &[NewOutboxRow],
        cursors: &[CursorAdvance],
        now: DateTime<Utc>,
    ) -> Result<EnqueueOutcome, StoreError> {
        let now_s = ts(now);
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let mut outcome = EnqueueOutcome::default();
        for row in rows {
            let class = row.last_class.map(class_str).transpose()?;
            let changed = tx.execute(
                "INSERT OR IGNORE INTO sync_outbox (service, cmdr_fid, idem_key, kind, coalesce_key, event_ts, payload, origin, state, next_attempt_at, last_class, created_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?10)",
                rusqlite::params![
                    row.service.as_str(),
                    row.cmdr_fid,
                    row.idem_key,
                    row.kind,
                    row.coalesce_key,
                    ts(row.event_ts),
                    row.payload,
                    row.origin.as_str(),
                    row.state.as_str(),
                    now_s,
                    class
                ],
            )?;
            if changed == 0 {
                outcome.ignored += 1;
                continue;
            }
            outcome.inserted += 1;
            if row.origin == RowOrigin::Recovered {
                outcome.recovered += 1;
            }
            let column = match row.state {
                OutboxState::Omitted => Some("omitted"),
                OutboxState::Expired => Some("expired"),
                _ => None,
            };
            if let Some(column) = column {
                tx.execute(
                    &format!(
                        "INSERT INTO sync_counter (service, cmdr_fid, {column}) VALUES (?1,?2,1) \
                         ON CONFLICT(service, cmdr_fid) DO UPDATE SET {column} = {column} + 1"
                    ),
                    rusqlite::params![row.service.as_str(), row.cmdr_fid],
                )?;
            }
        }
        for cursor in cursors {
            tx.execute(
                "INSERT INTO sync_cursor (cmdr_fid, journal_file, last_line, updated_at) VALUES (?1,?2,?3,?4) \
                 ON CONFLICT(cmdr_fid, journal_file) DO UPDATE SET \
                 last_line = CASE WHEN excluded.last_line = 0 THEN 0 ELSE max(last_line, excluded.last_line) END, \
                 updated_at = excluded.updated_at",
                rusqlite::params![
                    cursor.cmdr_fid,
                    cursor.journal_file,
                    i64::try_from(cursor.last_line)?,
                    now_s
                ],
            )?;
        }
        tx.commit()?;
        Ok(outcome)
    }

    /// Última línea procesada de un archivo para un comandante (`None` = sin cursor).
    pub fn sync_cursor(
        &self,
        cmdr_fid: &str,
        journal_file: &str,
    ) -> Result<Option<u64>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT last_line FROM sync_cursor WHERE cmdr_fid=?1 AND journal_file=?2")?;
        let mut rows = stmt.query(rusqlite::params![cmdr_fid, journal_file])?;
        match rows.next()? {
            Some(row) => Ok(Some(u64::try_from(row.get::<_, i64>(0)?.max(0))?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const FILE: &str = "Journal.2026-11-08T100000.01.log";

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 8, 10, 0, 0).unwrap()
    }

    fn row(service: SyncService, idem: &str, state: OutboxState) -> NewOutboxRow {
        NewOutboxRow {
            service,
            cmdr_fid: "F0000001".into(),
            idem_key: idem.into(),
            kind: "test".into(),
            coalesce_key: None,
            event_ts: now(),
            payload: "{}".into(),
            origin: RowOrigin::Live,
            state,
            last_class: None,
        }
    }

    fn cursor(line: u64) -> CursorAdvance {
        CursorAdvance {
            cmdr_fid: "F0000001".into(),
            journal_file: FILE.into(),
            last_line: line,
        }
    }

    fn count(store: &Store, sql: &str) -> i64 {
        store
            .lock()
            .unwrap()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn migration_creates_sync_tables_idempotently() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        drop(Store::open(&path).unwrap());
        let store = Store::open(&path).unwrap();
        assert_eq!(
            count(
                &store,
                "SELECT COUNT(*) FROM schema_version WHERE id='f17_sync'"
            ),
            1
        );
        assert_eq!(
            count(
                &store,
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN \
                 ('sync_outbox','sync_credential','sync_activation','sync_cursor','sync_lease','sync_counter','sync_known_cmdr','sync_unassigned_key')"
            ),
            8
        );
    }

    #[test]
    fn same_idem_key_is_ignored() {
        let store = Store::open_in_memory().unwrap();
        let r = row(SyncService::Eddn, "k1", OutboxState::Pending);
        let a = store
            .enqueue_batch_with_cursor(std::slice::from_ref(&r), &[], now())
            .unwrap();
        let b = store
            .enqueue_batch_with_cursor(std::slice::from_ref(&r), &[], now())
            .unwrap();
        assert_eq!((a.inserted, a.ignored), (1, 0));
        assert_eq!((b.inserted, b.ignored), (0, 1));
        assert_eq!(count(&store, "SELECT COUNT(*) FROM sync_outbox"), 1);
    }

    #[test]
    fn cursor_advances_with_rows() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.sync_cursor("F0000001", FILE).unwrap(), None);
        store
            .enqueue_batch_with_cursor(&[], &[cursor(42)], now())
            .unwrap();
        assert_eq!(store.sync_cursor("F0000001", FILE).unwrap(), Some(42));
        store
            .enqueue_batch_with_cursor(&[], &[cursor(40)], now())
            .unwrap();
        assert_eq!(store.sync_cursor("F0000001", FILE).unwrap(), Some(42));
        // Truncado del archivo: line 0 reinicia el cursor.
        store
            .enqueue_batch_with_cursor(&[], &[cursor(0)], now())
            .unwrap();
        assert_eq!(store.sync_cursor("F0000001", FILE).unwrap(), Some(0));
    }

    #[test]
    fn cursor_and_rows_commit_atomically() {
        let store = Store::open_in_memory().unwrap();
        store
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_cursor BEFORE INSERT ON sync_cursor BEGIN SELECT RAISE(ABORT,'boom'); END;",
            )
            .unwrap();
        let r = row(SyncService::Eddn, "k1", OutboxState::Pending);
        let result = store.enqueue_batch_with_cursor(&[r], &[cursor(5)], now());
        assert!(result.is_err());
        assert_eq!(count(&store, "SELECT COUNT(*) FROM sync_outbox"), 0);
    }

    #[test]
    fn check_constraints_reject_unknown_state_and_service() {
        let store = Store::open_in_memory().unwrap();
        let conn = store.lock().unwrap();
        let insert = |service: &str, state: &str| {
            conn.execute(
                "INSERT INTO sync_outbox (service,cmdr_fid,idem_key,kind,event_ts,payload,origin,state,next_attempt_at,created_at) \
                 VALUES (?1,'F1','k','t','x','{}','live',?2,'x','x')",
                rusqlite::params![service, state],
            )
        };
        assert!(insert("eddn", "foo").is_err());
        assert!(insert("nope", "pending").is_err());
        assert!(insert("eddn", "pending").is_ok());
    }

    #[test]
    fn omitted_rows_count_in_counter() {
        let store = Store::open_in_memory().unwrap();
        let mut recovered = row(SyncService::Eddn, "k2", OutboxState::Expired);
        recovered.origin = RowOrigin::Recovered;
        let out = store
            .enqueue_batch_with_cursor(
                &[
                    row(SyncService::Eddn, "k1", OutboxState::Omitted),
                    row(SyncService::Eddn, "k3", OutboxState::Discarded),
                    recovered,
                ],
                &[],
                now(),
            )
            .unwrap();
        assert_eq!((out.inserted, out.recovered), (3, 1));
        assert_eq!(
            count(
                &store,
                "SELECT omitted FROM sync_counter WHERE service='eddn' AND cmdr_fid='F0000001'"
            ),
            1
        );
        assert_eq!(
            count(
                &store,
                "SELECT expired FROM sync_counter WHERE service='eddn' AND cmdr_fid='F0000001'"
            ),
            1
        );
    }
}

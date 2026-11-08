//! Cola de salida de la sincronización (`sync_outbox`) y cursor del router, en una sola transacción.

use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use tmc_model::sync::{
    ErrorClass, OutboxState, RECENT_REJECTIONS_MAX, RecentRejection, RowOrigin, SyncService,
};

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

/// Máximo de filas pendientes por servicio antes de caducar las más antiguas.
pub const SYNC_PENDING_CAP: u64 = 20_000;
/// Días que se conservan las filas cerradas y los cursores inactivos.
pub const SYNC_RETENTION_DAYS: i64 = 7;

/// Fila reclamada para su envío.
#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub id: i64,
    pub service: SyncService,
    pub cmdr_fid: String,
    pub kind: String,
    pub coalesce_key: Option<String>,
    pub event_ts: DateTime<Utc>,
    pub payload: String,
    pub origin: RowOrigin,
    pub attempts: u32,
}

/// Contadores y volumen pendiente de un servicio para un comandante.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncCounts {
    /// Filas pendientes o en vuelo.
    pub pending: u64,
    pub sent_total: u64,
    pub rejected_total: u64,
    pub expired_total: u64,
    pub omitted_total: u64,
    /// Filas descartadas todavía no purgadas.
    pub discarded: u64,
    pub last_confirmed_at: Option<String>,
}

fn parse_ts(value: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| StoreError::Unsupported("timestamp".into()))
}

fn parse_origin(value: &str) -> Result<RowOrigin, StoreError> {
    match value {
        "live" => Ok(RowOrigin::Live),
        "recovered" => Ok(RowOrigin::Recovered),
        "snapshot" => Ok(RowOrigin::Snapshot),
        _ => Err(StoreError::Unsupported("origin".into())),
    }
}

fn parse_class(value: &str) -> Result<ErrorClass, StoreError> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|_| StoreError::Unsupported("error class".into()))
}

fn bump(
    tx: &rusqlite::Transaction<'_>,
    service: &str,
    cmdr_fid: &str,
    column: &str,
    confirmed_at: Option<&str>,
) -> Result<(), StoreError> {
    tx.execute(
        &format!(
            "INSERT INTO sync_counter (service, cmdr_fid, {column}, last_confirmed_at) VALUES (?1,?2,1,?3) \
             ON CONFLICT(service, cmdr_fid) DO UPDATE SET {column} = {column} + 1, \
             last_confirmed_at = COALESCE(?3, last_confirmed_at)"
        ),
        rusqlite::params![service, cmdr_fid, confirmed_at],
    )?;
    Ok(())
}

impl Store {
    /// Reclama filas listas (pendientes con reintento vencido), por `event_ts, id`, y las pasa a `inflight`.
    pub fn claim_ready(
        &self,
        service: SyncService,
        cmdr_fid: Option<&str>,
        limit: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<OutboxRow>, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let mut out = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT id, cmdr_fid, kind, coalesce_key, event_ts, payload, origin, attempts FROM sync_outbox \
                 WHERE service=?1 AND state='pending' AND next_attempt_at <= ?2 AND (?3 IS NULL OR cmdr_fid=?3) \
                 ORDER BY event_ts, id LIMIT ?4",
            )?;
            let mut rows = stmt.query(rusqlite::params![
                service.as_str(),
                ts(now),
                cmdr_fid,
                i64::try_from(limit)?
            ])?;
            while let Some(r) = rows.next()? {
                out.push(OutboxRow {
                    id: r.get(0)?,
                    service,
                    cmdr_fid: r.get(1)?,
                    kind: r.get(2)?,
                    coalesce_key: r.get(3)?,
                    event_ts: parse_ts(&r.get::<_, String>(4)?)?,
                    payload: r.get(5)?,
                    origin: parse_origin(&r.get::<_, String>(6)?)?,
                    attempts: u32::try_from(r.get::<_, i64>(7)?.max(0))?,
                });
            }
        }
        for row in &out {
            tx.execute(
                "UPDATE sync_outbox SET state='inflight' WHERE id=?1",
                [row.id],
            )?;
        }
        tx.commit()?;
        Ok(out)
    }

    /// Cierra filas con un estado final y suma el contador indicado (solo si estaban abiertas).
    fn close_rows(
        &self,
        ids: &[i64],
        state: OutboxState,
        code: Option<&str>,
        class: Option<ErrorClass>,
        counter: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let now_s = ts(now);
        let class = class.map(class_str).transpose()?;
        let confirmed = (state == OutboxState::Sent).then_some(now_s.as_str());
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        for id in ids {
            let key: Option<(String, String)> = tx
                .query_row(
                    "SELECT service, cmdr_fid FROM sync_outbox WHERE id=?1 AND state IN ('pending','inflight')",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map(Some)
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    e => Err(e),
                })?;
            let Some((service, fid)) = key else { continue };
            tx.execute(
                "UPDATE sync_outbox SET state=?2, last_code=?3, last_class=?4, done_at=?5 WHERE id=?1",
                rusqlite::params![id, state.as_str(), code, class, now_s],
            )?;
            if let Some(column) = counter {
                bump(&tx, &service, &fid, column, confirmed)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Marca filas como enviadas (cuenta `sent` y fija `last_confirmed_at`).
    pub fn mark_sent(&self, ids: &[i64], code: &str, now: DateTime<Utc>) -> Result<(), StoreError> {
        self.close_rows(ids, OutboxState::Sent, Some(code), None, Some("sent"), now)
    }

    /// Marca filas como rechazadas definitivamente por el servicio.
    pub fn mark_rejected(
        &self,
        ids: &[i64],
        code: &str,
        class: ErrorClass,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.close_rows(
            ids,
            OutboxState::Rejected,
            Some(code),
            Some(class),
            Some("rejected"),
            now,
        )
    }

    /// Cierra filas como `discarded`, `omitted` o `superseded` (solo `omitted` cuenta).
    pub fn mark_closed(
        &self,
        ids: &[i64],
        state: OutboxState,
        code: Option<&str>,
        class: Option<ErrorClass>,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        if !matches!(
            state,
            OutboxState::Discarded | OutboxState::Omitted | OutboxState::Superseded
        ) {
            return Err(StoreError::Unsupported("closing state".into()));
        }
        let counter = (state == OutboxState::Omitted).then_some("omitted");
        self.close_rows(ids, state, code, class, counter, now)
    }

    fn reopen_rows(
        &self,
        ids: &[i64],
        next_attempt_at: Option<DateTime<Utc>>,
        code: &str,
        class: ErrorClass,
    ) -> Result<(), StoreError> {
        let class = class_str(class)?;
        let next = next_attempt_at.map(ts);
        let step = i64::from(next.is_some());
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        for id in ids {
            tx.execute(
                "UPDATE sync_outbox SET state='pending', attempts = attempts + ?2, \
                 next_attempt_at = COALESCE(?3, next_attempt_at), last_code=?4, last_class=?5 \
                 WHERE id=?1 AND state IN ('pending','inflight')",
                rusqlite::params![id, step, next, code, class],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Devuelve las filas a `pending` con un nuevo reintento (`attempts + 1`).
    pub fn schedule_retry(
        &self,
        ids: &[i64],
        next_attempt_at: DateTime<Utc>,
        code: &str,
        class: ErrorClass,
    ) -> Result<(), StoreError> {
        self.reopen_rows(ids, Some(next_attempt_at), code, class)
    }

    /// Devuelve las filas a `pending` sin consumir intentos (p. ej. credenciales).
    pub fn hold_pending(
        &self,
        ids: &[i64],
        code: &str,
        class: ErrorClass,
    ) -> Result<(), StoreError> {
        self.reopen_rows(ids, None, code, class)
    }

    /// Recupera al arrancar las filas que quedaron `inflight`.
    pub fn release_inflight(&self) -> Result<u64, StoreError> {
        let conn = self.lock()?;
        let n = conn.execute(
            "UPDATE sync_outbox SET state='pending' WHERE state='inflight'",
            [],
        )?;
        Ok(n as u64)
    }

    /// Cierra como `expired` (y cuenta) las filas pendientes ya demasiado antiguas.
    fn expire_where(
        tx: &rusqlite::Transaction<'_>,
        ids: &[(i64, String, String)],
        now_s: &str,
    ) -> Result<u64, StoreError> {
        for (id, service, fid) in ids {
            tx.execute(
                "UPDATE sync_outbox SET state='expired', done_at=?2 WHERE id=?1",
                rusqlite::params![id, now_s],
            )?;
            bump(tx, service, fid, "expired", None)?;
        }
        Ok(ids.len() as u64)
    }

    /// Caduca las filas pendientes más antiguas que `service.max_age()`.
    pub fn expire_old(&self, service: SyncService, now: DateTime<Utc>) -> Result<u64, StoreError> {
        let limit = ts(now - service.max_age());
        let now_s = ts(now);
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let ids = {
            let mut stmt = tx.prepare(
                "SELECT id, service, cmdr_fid FROM sync_outbox \
                 WHERE service=?1 AND state='pending' AND event_ts < ?2",
            )?;
            stmt.query_map(rusqlite::params![service.as_str(), limit], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        let n = Self::expire_where(&tx, &ids, &now_s)?;
        tx.commit()?;
        Ok(n)
    }

    /// Si hay más de `cap` pendientes, caduca las más antiguas hasta quedar en `cap`.
    pub fn enforce_pending_cap(
        &self,
        service: SyncService,
        cap: u64,
        now: DateTime<Utc>,
    ) -> Result<u64, StoreError> {
        let now_s = ts(now);
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let total: i64 = tx.query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE service=?1 AND state='pending'",
            [service.as_str()],
            |r| r.get(0),
        )?;
        let excess = u64::try_from(total.max(0))?.saturating_sub(cap);
        if excess == 0 {
            return Ok(0);
        }
        let ids = {
            let mut stmt = tx.prepare(
                "SELECT id, service, cmdr_fid FROM sync_outbox \
                 WHERE service=?1 AND state='pending' ORDER BY event_ts, id LIMIT ?2",
            )?;
            stmt.query_map(
                rusqlite::params![service.as_str(), i64::try_from(excess)?],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?
            .collect::<Result<Vec<_>, _>>()?
        };
        let n = Self::expire_where(&tx, &ids, &now_s)?;
        tx.commit()?;
        Ok(n)
    }

    /// Borra filas cerradas y cursores con más de `SYNC_RETENTION_DAYS` días (los contadores se conservan).
    pub fn purge_done(&self, now: DateTime<Utc>) -> Result<u64, StoreError> {
        let limit = ts(now - chrono::Duration::days(SYNC_RETENTION_DAYS));
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let rows = tx.execute(
            "DELETE FROM sync_outbox WHERE state NOT IN ('pending','inflight') \
             AND COALESCE(done_at, created_at) < ?1",
            [&limit],
        )?;
        tx.execute("DELETE FROM sync_cursor WHERE updated_at < ?1", [&limit])?;
        tx.commit()?;
        Ok(rows as u64)
    }

    /// Descarta las filas pendientes de un servicio (y comandante, si se indica).
    pub fn discard_pending(
        &self,
        service: SyncService,
        cmdr_fid: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<u64, StoreError> {
        let conn = self.lock()?;
        let n = conn.execute(
            "UPDATE sync_outbox SET state='discarded', done_at=?3 \
             WHERE service=?1 AND state='pending' AND (?2 IS NULL OR cmdr_fid=?2)",
            rusqlite::params![service.as_str(), cmdr_fid, ts(now)],
        )?;
        Ok(n as u64)
    }

    /// Próximo reintento programado entre las filas pendientes del servicio.
    pub fn next_attempt_at(
        &self,
        service: SyncService,
    ) -> Result<Option<DateTime<Utc>>, StoreError> {
        let conn = self.lock()?;
        let value: Option<String> = conn.query_row(
            "SELECT MIN(next_attempt_at) FROM sync_outbox WHERE service=?1 AND state='pending'",
            [service.as_str()],
            |r| r.get(0),
        )?;
        value.as_deref().map(parse_ts).transpose()
    }

    /// Contadores acumulados y filas abiertas/descartadas de un comandante en un servicio.
    pub fn sync_counts(
        &self,
        service: SyncService,
        cmdr_fid: &str,
    ) -> Result<SyncCounts, StoreError> {
        let conn = self.lock()?;
        let by_state = |states: &str| -> Result<u64, StoreError> {
            let n: i64 = conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM sync_outbox WHERE service=?1 AND cmdr_fid=?2 AND state IN ({states})"
                ),
                rusqlite::params![service.as_str(), cmdr_fid],
                |r| r.get(0),
            )?;
            Ok(u64::try_from(n.max(0))?)
        };
        let mut counts = SyncCounts {
            pending: by_state("'pending','inflight'")?,
            discarded: by_state("'discarded'")?,
            ..SyncCounts::default()
        };
        let mut stmt = conn.prepare(
            "SELECT sent, rejected, expired, omitted, last_confirmed_at FROM sync_counter WHERE service=?1 AND cmdr_fid=?2",
        )?;
        let mut rows = stmt.query(rusqlite::params![service.as_str(), cmdr_fid])?;
        if let Some(r) = rows.next()? {
            let n =
                |i| -> Result<u64, StoreError> { Ok(u64::try_from(r.get::<_, i64>(i)?.max(0))?) };
            counts.sent_total = n(0)?;
            counts.rejected_total = n(1)?;
            counts.expired_total = n(2)?;
            counts.omitted_total = n(3)?;
            counts.last_confirmed_at = r.get(4)?;
        }
        Ok(counts)
    }

    /// Pendientes por comandante: (fid, cantidad, `event_ts` más antiguo).
    pub fn pending_by_cmdr(
        &self,
        service: SyncService,
    ) -> Result<Vec<(String, u64, DateTime<Utc>)>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT cmdr_fid, COUNT(*), MIN(event_ts) FROM sync_outbox \
             WHERE service=?1 AND state='pending' GROUP BY cmdr_fid ORDER BY cmdr_fid",
        )?;
        let mut rows = stmt.query([service.as_str()])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push((
                r.get::<_, String>(0)?,
                u64::try_from(r.get::<_, i64>(1)?.max(0))?,
                parse_ts(&r.get::<_, String>(2)?)?,
            ));
        }
        Ok(out)
    }

    /// Últimos rechazos del servicio para el comandante (máximo `RECENT_REJECTIONS_MAX`).
    pub fn recent_rejections(
        &self,
        service: SyncService,
        cmdr_fid: &str,
    ) -> Result<Vec<RecentRejection>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT kind, last_class, last_code, done_at FROM sync_outbox \
             WHERE service=?1 AND cmdr_fid=?2 AND state='rejected' \
             ORDER BY done_at DESC, id DESC LIMIT ?3",
        )?;
        let mut rows = stmt.query(rusqlite::params![
            service.as_str(),
            cmdr_fid,
            i64::try_from(RECENT_REJECTIONS_MAX)?
        ])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(RecentRejection {
                kind: r.get(0)?,
                class: parse_class(&r.get::<_, String>(1)?)?,
                code: r.get(2)?,
                at: r.get(3)?,
                hint: None,
            });
        }
        Ok(out)
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

    fn at(service: SyncService, idem: &str, fid: &str, event_ts: DateTime<Utc>) -> NewOutboxRow {
        let mut r = row(service, idem, OutboxState::Pending);
        r.cmdr_fid = fid.into();
        r.event_ts = event_ts;
        r
    }

    fn put(store: &Store, rows: &[NewOutboxRow]) {
        store.enqueue_batch_with_cursor(rows, &[], now()).unwrap();
    }

    fn state_of(store: &Store, idem: &str) -> String {
        store
            .lock()
            .unwrap()
            .query_row(
                "SELECT state FROM sync_outbox WHERE idem_key=?1",
                [idem],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn ids(rows: &[OutboxRow]) -> Vec<i64> {
        rows.iter().map(|r| r.id).collect()
    }

    const F1: &str = "F0000001";
    const F2: &str = "F0000002";

    #[test]
    fn claim_orders_by_event_ts_and_skips_future_retry() {
        let store = Store::open_in_memory().unwrap();
        let h = |hh| Utc.with_ymd_and_hms(2026, 11, 8, hh, 0, 0).unwrap();
        put(
            &store,
            &[
                at(SyncService::Inara, "a", F1, h(10)),
                at(SyncService::Inara, "b", F1, h(9)),
                at(SyncService::Inara, "c", F1, h(11)),
            ],
        );
        let c = store
            .claim_ready(SyncService::Inara, None, 10, now())
            .unwrap();
        assert_eq!(c.len(), 3);
        assert_eq!(
            c.iter().map(|r| r.event_ts).collect::<Vec<_>>(),
            vec![h(9), h(10), h(11)]
        );
        let all = ids(&c);
        store.release_inflight().unwrap();
        // c (11:00) se pospone 60 s.
        store
            .schedule_retry(
                &[all[2]],
                now() + chrono::Duration::seconds(60),
                "x",
                ErrorClass::Network,
            )
            .unwrap();
        let c = store
            .claim_ready(SyncService::Inara, Some(F1), 10, now())
            .unwrap();
        assert_eq!(
            c.iter().map(|r| r.event_ts).collect::<Vec<_>>(),
            vec![h(9), h(10)]
        );
        assert_eq!(state_of(&store, "a"), "inflight");
        assert_eq!(state_of(&store, "c"), "pending");
        assert_eq!(c[0].attempts, 0);
        assert_eq!(
            store.next_attempt_at(SyncService::Inara).unwrap(),
            Some(now() + chrono::Duration::seconds(60))
        );
    }

    #[test]
    fn release_inflight_returns_rows_to_pending() {
        let store = Store::open_in_memory().unwrap();
        put(
            &store,
            &[
                at(SyncService::Eddn, "a", F1, now()),
                at(SyncService::Eddn, "b", F1, now()),
            ],
        );
        assert_eq!(
            store
                .claim_ready(SyncService::Eddn, None, 10, now())
                .unwrap()
                .len(),
            2
        );
        assert_eq!(store.release_inflight().unwrap(), 2);
        assert_eq!(
            store
                .claim_ready(SyncService::Eddn, None, 10, now())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn sent_and_rejected_update_counters() {
        let store = Store::open_in_memory().unwrap();
        put(
            &store,
            &[
                at(SyncService::Edsm, "a", F1, now()),
                at(SyncService::Edsm, "b", F1, now()),
                at(SyncService::Edsm, "c", F1, now()),
            ],
        );
        let c = store
            .claim_ready(SyncService::Edsm, None, 10, now())
            .unwrap();
        let v = ids(&c);
        store.mark_sent(&v[..2], "100", now()).unwrap();
        store
            .mark_rejected(&v[2..], "bad", ErrorClass::InvalidEvent, now())
            .unwrap();
        let n = store.sync_counts(SyncService::Edsm, F1).unwrap();
        assert_eq!((n.sent_total, n.rejected_total, n.pending), (2, 1, 0));
        assert_eq!(n.last_confirmed_at, Some("2026-11-08T10:00:00Z".into()));
        let rr = store.recent_rejections(SyncService::Edsm, F1).unwrap();
        assert_eq!(rr.len(), 1);
        assert_eq!((rr[0].kind.as_str(), rr[0].code.as_str()), ("test", "bad"));
        assert_eq!(rr[0].class, ErrorClass::InvalidEvent);
    }

    #[test]
    fn expire_uses_service_max_age() {
        let store = Store::open_in_memory().unwrap();
        let ago = |d: chrono::Duration| now() - d;
        put(
            &store,
            &[
                at(SyncService::Eddn, "e", F1, ago(chrono::Duration::hours(25))),
                at(SyncService::Edsm, "s", F1, ago(chrono::Duration::days(6))),
                at(SyncService::Inara, "i", F1, ago(chrono::Duration::days(8))),
            ],
        );
        assert_eq!(store.expire_old(SyncService::Eddn, now()).unwrap(), 1);
        assert_eq!(store.expire_old(SyncService::Edsm, now()).unwrap(), 0);
        assert_eq!(store.expire_old(SyncService::Inara, now()).unwrap(), 1);
        assert_eq!(state_of(&store, "e"), "expired");
        assert_eq!(state_of(&store, "s"), "pending");
        assert_eq!(state_of(&store, "i"), "expired");
        assert_eq!(
            store
                .sync_counts(SyncService::Eddn, F1)
                .unwrap()
                .expired_total,
            1
        );
        assert_eq!(
            store
                .sync_counts(SyncService::Inara, F1)
                .unwrap()
                .expired_total,
            1
        );
    }

    #[test]
    fn pending_cap_expires_oldest() {
        let store = Store::open_in_memory().unwrap();
        let rows: Vec<_> = (0..5)
            .map(|i| {
                at(
                    SyncService::Inara,
                    &format!("k{i}"),
                    F1,
                    now() - chrono::Duration::minutes(10 - i),
                )
            })
            .collect();
        put(&store, &rows);
        assert_eq!(
            store
                .enforce_pending_cap(SyncService::Inara, 3, now())
                .unwrap(),
            2
        );
        assert_eq!(state_of(&store, "k0"), "expired");
        assert_eq!(state_of(&store, "k1"), "expired");
        assert_eq!(state_of(&store, "k2"), "pending");
        let n = store.sync_counts(SyncService::Inara, F1).unwrap();
        assert_eq!((n.pending, n.expired_total), (3, 2));
        assert_eq!(
            store
                .enforce_pending_cap(SyncService::Inara, 3, now())
                .unwrap(),
            0
        );
    }

    #[test]
    fn purge_removes_closed_rows_after_7_days_but_keeps_counters() {
        let store = Store::open_in_memory().unwrap();
        put(
            &store,
            &[
                at(SyncService::Inara, "old", F1, now()),
                at(SyncService::Inara, "new", F1, now()),
                at(SyncService::Inara, "pend", F1, now()),
            ],
        );
        let c = store
            .claim_ready(SyncService::Inara, None, 2, now())
            .unwrap();
        let v = ids(&c);
        store
            .mark_sent(&v[..1], "ok", now() - chrono::Duration::days(8))
            .unwrap();
        store.mark_sent(&v[1..], "ok", now()).unwrap();
        store
            .enqueue_batch_with_cursor(&[], &[cursor(7)], now() - chrono::Duration::days(8))
            .unwrap();
        assert_eq!(store.purge_done(now()).unwrap(), 1);
        assert_eq!(count(&store, "SELECT COUNT(*) FROM sync_outbox"), 2);
        assert_eq!(state_of(&store, "pend"), "pending");
        assert_eq!(store.sync_cursor(F1, FILE).unwrap(), None);
        assert_eq!(
            store
                .sync_counts(SyncService::Inara, F1)
                .unwrap()
                .sent_total,
            2
        );
    }

    #[test]
    fn discard_pending_only_for_that_cmdr_and_service() {
        let store = Store::open_in_memory().unwrap();
        put(
            &store,
            &[
                at(SyncService::Inara, "i1", F1, now()),
                at(SyncService::Inara, "i2", F2, now()),
                at(SyncService::Edsm, "e1", F1, now()),
            ],
        );
        assert_eq!(
            store
                .discard_pending(SyncService::Inara, Some(F1), now())
                .unwrap(),
            1
        );
        assert_eq!(state_of(&store, "i1"), "discarded");
        assert_eq!(state_of(&store, "i2"), "pending");
        assert_eq!(state_of(&store, "e1"), "pending");
        assert_eq!(
            store.sync_counts(SyncService::Inara, F1).unwrap().discarded,
            1
        );
        let by = store.pending_by_cmdr(SyncService::Inara).unwrap();
        assert_eq!(by, vec![(F2.to_string(), 1, now())]);
    }

    #[test]
    fn hold_pending_does_not_increment_attempts() {
        let store = Store::open_in_memory().unwrap();
        put(&store, &[at(SyncService::Inara, "a", F1, now())]);
        let c = store
            .claim_ready(SyncService::Inara, None, 10, now())
            .unwrap();
        store
            .hold_pending(&ids(&c), "401", ErrorClass::Auth)
            .unwrap();
        assert_eq!(state_of(&store, "a"), "pending");
        let (attempts, class): (i64, String) = store
            .lock()
            .unwrap()
            .query_row(
                "SELECT attempts,last_class FROM sync_outbox WHERE idem_key='a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((attempts, class.as_str()), (0, "auth"));
        store
            .schedule_retry(&ids(&c), now(), "x", ErrorClass::Timeout)
            .unwrap();
        let c = store
            .claim_ready(SyncService::Inara, None, 10, now())
            .unwrap();
        assert_eq!(c[0].attempts, 1);
    }

    #[test]
    fn mark_closed_omitted_counts_and_superseded_does_not() {
        let store = Store::open_in_memory().unwrap();
        put(
            &store,
            &[
                at(SyncService::Inara, "a", F1, now()),
                at(SyncService::Inara, "b", F1, now()),
            ],
        );
        let c = store
            .claim_ready(SyncService::Inara, None, 10, now())
            .unwrap();
        let v = ids(&c);
        store
            .mark_closed(&v[..1], OutboxState::Omitted, Some("dup"), None, now())
            .unwrap();
        store
            .mark_closed(&v[1..], OutboxState::Superseded, None, None, now())
            .unwrap();
        let n = store.sync_counts(SyncService::Inara, F1).unwrap();
        assert_eq!((n.omitted_total, n.pending), (1, 0));
        assert_eq!(state_of(&store, "b"), "superseded");
    }

    #[test]
    fn ignored_row_does_not_bump_counter_and_expired_accumulates() {
        let store = Store::open_in_memory().unwrap();
        let o = row(SyncService::Eddn, "o", OutboxState::Omitted);
        put(&store, std::slice::from_ref(&o));
        put(&store, std::slice::from_ref(&o));
        assert_eq!(
            store
                .sync_counts(SyncService::Eddn, F1)
                .unwrap()
                .omitted_total,
            1
        );
        put(
            &store,
            &[
                row(SyncService::Eddn, "x1", OutboxState::Expired),
                row(SyncService::Eddn, "x2", OutboxState::Expired),
            ],
        );
        assert_eq!(
            store
                .sync_counts(SyncService::Eddn, F1)
                .unwrap()
                .expired_total,
            2
        );
    }
}

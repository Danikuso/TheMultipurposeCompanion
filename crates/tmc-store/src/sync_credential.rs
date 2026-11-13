//! Credenciales de Inara/EDSM por comandante, migración desde `accounts_credential`,
//! activación por servicio y comandantes conocidos (spec F17 §6.2, DR-32).

use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, Transaction};
use std::fmt;
use tmc_model::sync::{KeyAssignment, SyncService};

/// Clave de un servicio asignada a un comandante.
#[derive(Clone, PartialEq, Eq)]
pub struct SyncCredential {
    pub service: SyncService,
    pub cmdr_fid: String,
    pub cmdr_name: String,
    pub api_key: String,
    pub created_at: String,
    pub verified_at: Option<String>,
    pub assignment: KeyAssignment,
}

impl fmt::Debug for SyncCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncCredential")
            .field("service", &self.service)
            .field("cmdr_fid", &self.cmdr_fid)
            .field("cmdr_name", &self.cmdr_name)
            .field("api_key", &"••••")
            .field("created_at", &self.created_at)
            .field("verified_at", &self.verified_at)
            .field("assignment", &self.assignment)
            .finish()
    }
}

/// Motivo por el que una clave migrada sigue sin comandante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnassignedReason {
    WaitingFirstCmdr,
    Ambiguous,
}

impl UnassignedReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::WaitingFirstCmdr => "waiting_first_cmdr",
            Self::Ambiguous => "ambiguous",
        }
    }

    fn parse(value: &str) -> Result<Self, StoreError> {
        match value {
            "waiting_first_cmdr" => Ok(Self::WaitingFirstCmdr),
            "ambiguous" => Ok(Self::Ambiguous),
            _ => Err(StoreError::Unsupported("unassigned reason".into())),
        }
    }
}

/// Clave sin comandante (nunca expone el valor de la clave).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnassignedKey {
    pub service: SyncService,
    pub cmdr_name: Option<String>,
    pub reason: UnassignedReason,
}

/// Resultado de migrar la clave heredada de un servicio.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub assigned: Vec<SyncService>,
    pub waiting: Vec<SyncService>,
    pub ambiguous: Vec<SyncService>,
}

fn ts(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_ts(value: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| StoreError::Unsupported("timestamp".into()))
}

fn assignment_str(value: KeyAssignment) -> &'static str {
    match value {
        KeyAssignment::Confirmed => "confirmed",
        KeyAssignment::AutoPendingConfirmation => "auto_pending_confirmation",
    }
}

fn parse_assignment(value: &str) -> Result<KeyAssignment, StoreError> {
    match value {
        "confirmed" => Ok(KeyAssignment::Confirmed),
        "auto_pending_confirmation" => Ok(KeyAssignment::AutoPendingConfirmation),
        _ => Err(StoreError::Unsupported("key assignment".into())),
    }
}

fn parse_service(value: &str) -> Result<SyncService, StoreError> {
    SyncService::parse(value).ok_or_else(|| StoreError::Unsupported("service".into()))
}

const CREDENTIAL_COLUMNS: &str =
    "service, cmdr_fid, cmdr_name, api_key, created_at, verified_at, assignment";

fn credential_from_row(row: &rusqlite::Row<'_>) -> Result<SyncCredential, StoreError> {
    Ok(SyncCredential {
        service: parse_service(&row.get::<_, String>(0)?)?,
        cmdr_fid: row.get(1)?,
        cmdr_name: row.get(2)?,
        api_key: row.get(3)?,
        created_at: row.get(4)?,
        verified_at: row.get(5)?,
        assignment: parse_assignment(&row.get::<_, String>(6)?)?,
    })
}

fn insert_credential(
    tx: &Transaction<'_>,
    c: &SyncCredential,
    replace: bool,
) -> Result<(), StoreError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT OR IGNORE"
    };
    tx.execute(
        &format!(
            "{verb} INTO sync_credential ({CREDENTIAL_COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7)"
        ),
        rusqlite::params![
            c.service.as_str(),
            c.cmdr_fid,
            c.cmdr_name,
            c.api_key,
            c.created_at,
            c.verified_at,
            assignment_str(c.assignment)
        ],
    )?;
    Ok(())
}

fn set_activation_tx(
    tx: &Transaction<'_>,
    service: SyncService,
    fid: &str,
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO sync_activation (service, cmdr_fid, activated_at) VALUES (?1,?2,?3) \
         ON CONFLICT(service, cmdr_fid) DO UPDATE SET activated_at=excluded.activated_at",
        rusqlite::params![service.as_str(), fid, ts(at)],
    )?;
    Ok(())
}

fn discard_pending_tx(
    tx: &Transaction<'_>,
    service: Option<SyncService>,
    fid: &str,
    now: DateTime<Utc>,
) -> Result<u64, StoreError> {
    let n = tx.execute(
        "UPDATE sync_outbox SET state='discarded', done_at=?3 \
         WHERE state='pending' AND cmdr_fid=?2 AND (?1 IS NULL OR service=?1)",
        rusqlite::params![service.map(SyncService::as_str), fid, ts(now)],
    )?;
    Ok(n as u64)
}

/// Siembra `sync_known_cmdr` con los (FID, nombre) de `LoadGame`/`Commander` del histórico.
fn seed_known_from_events(tx: &Transaction<'_>) -> Result<(), StoreError> {
    let mut stmt = tx.prepare(
        "SELECT json_extract(json,'$.FID'), \
                COALESCE(json_extract(json,'$.Commander'), json_extract(json,'$.Name')), ts \
         FROM event WHERE name IN ('LoadGame','Commander') ORDER BY id",
    )?;
    let seen: Vec<(Option<String>, Option<String>, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    for (fid, name, seen_at) in seen {
        let (Some(fid), Some(name)) = (fid, name) else {
            continue;
        };
        if fid.is_empty() || name.is_empty() {
            continue;
        }
        tx.execute(
            "INSERT INTO sync_known_cmdr (cmdr_fid, cmdr_name, last_seen) VALUES (?1,?2,?3) \
             ON CONFLICT(cmdr_fid) DO UPDATE SET cmdr_name=excluded.cmdr_name, last_seen=excluded.last_seen",
            rusqlite::params![fid, name, seen_at],
        )?;
    }
    Ok(())
}

impl Store {
    /// Guarda o sustituye la clave de un comandante.
    pub fn put_sync_credential(&self, c: &SyncCredential) -> Result<(), StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        insert_credential(&tx, c, true)?;
        tx.commit()?;
        Ok(())
    }

    pub fn sync_credential(
        &self,
        service: SyncService,
        cmdr_fid: &str,
    ) -> Result<Option<SyncCredential>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {CREDENTIAL_COLUMNS} FROM sync_credential WHERE service=?1 AND cmdr_fid=?2"
        ))?;
        let mut rows = stmt.query(rusqlite::params![service.as_str(), cmdr_fid])?;
        rows.next()?.map(credential_from_row).transpose()
    }

    pub fn sync_credentials(
        &self,
        service: SyncService,
    ) -> Result<Vec<SyncCredential>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {CREDENTIAL_COLUMNS} FROM sync_credential WHERE service=?1 ORDER BY cmdr_fid"
        ))?;
        let mut rows = stmt.query([service.as_str()])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(credential_from_row(row)?);
        }
        Ok(out)
    }

    /// Borra la clave, su activación y descarta los pendientes del par servicio/comandante.
    /// Devuelve las filas descartadas.
    pub fn delete_sync_credential(
        &self,
        service: SyncService,
        cmdr_fid: &str,
        now: DateTime<Utc>,
    ) -> Result<u64, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM sync_credential WHERE service=?1 AND cmdr_fid=?2",
            rusqlite::params![service.as_str(), cmdr_fid],
        )?;
        tx.execute(
            "DELETE FROM sync_activation WHERE service=?1 AND cmdr_fid=?2",
            rusqlite::params![service.as_str(), cmdr_fid],
        )?;
        let discarded = discard_pending_tx(&tx, Some(service), cmdr_fid, now)?;
        tx.commit()?;
        Ok(discarded)
    }

    /// Mueve la clave heredada de `accounts_credential` a la nueva estructura (idempotente).
    /// Solo aplica a Inara y EDSM; la sal EDDN y CAPI no se tocan.
    pub fn migrate_legacy_credentials(
        &self,
        service: SyncService,
        now: DateTime<Utc>,
    ) -> Result<MigrationReport, StoreError> {
        let mut report = MigrationReport::default();
        if service == SyncService::Eddn {
            return Ok(report);
        }
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let legacy: Option<(Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT api_key, cmdr_name FROM accounts_credential WHERE service=?1",
                [service.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((Some(key), name)) = legacy else {
            return Ok(report);
        };
        tx.execute(
            "UPDATE accounts_credential SET api_key=NULL WHERE service=?1",
            [service.as_str()],
        )?;
        if key.is_empty() {
            tx.commit()?;
            return Ok(report);
        }
        seed_known_from_events(&tx)?;
        let known: Vec<(String, String)> = {
            let mut stmt =
                tx.prepare("SELECT cmdr_fid, cmdr_name FROM sync_known_cmdr ORDER BY cmdr_fid")?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?
        };
        let wanted = name.as_deref().map(str::trim).filter(|n| !n.is_empty());
        let matches: Vec<&(String, String)> = known
            .iter()
            .filter(|(_, n)| wanted.is_some_and(|w| n.trim().eq_ignore_ascii_case(w)))
            .collect();
        if let [(fid, cmdr_name)] = matches.as_slice() {
            let c = SyncCredential {
                service,
                cmdr_fid: fid.clone(),
                cmdr_name: cmdr_name.clone(),
                api_key: key,
                created_at: ts(now),
                verified_at: None,
                assignment: KeyAssignment::Confirmed,
            };
            insert_credential(&tx, &c, false)?;
            set_activation_tx(&tx, service, fid, now)?;
            report.assigned.push(service);
        } else {
            let reason = if known.is_empty() {
                report.waiting.push(service);
                UnassignedReason::WaitingFirstCmdr
            } else {
                report.ambiguous.push(service);
                UnassignedReason::Ambiguous
            };
            tx.execute(
                "INSERT OR REPLACE INTO sync_unassigned_key (service, cmdr_name, api_key, reason, created_at) \
                 VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![service.as_str(), name, key, reason.as_str(), ts(now)],
            )?;
        }
        tx.commit()?;
        Ok(report)
    }

    pub fn unassigned_keys(&self) -> Result<Vec<UnassignedKey>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT service, cmdr_name, reason FROM sync_unassigned_key ORDER BY service",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(UnassignedKey {
                service: parse_service(&row.get::<_, String>(0)?)?,
                cmdr_name: row.get(1)?,
                reason: UnassignedReason::parse(&row.get::<_, String>(2)?)?,
            });
        }
        Ok(out)
    }

    /// Asigna la clave sin comandante al FID indicado y fija la activación en `now`.
    /// `false` si no había clave sin asignar para el servicio.
    pub fn assign_unassigned_key(
        &self,
        service: SyncService,
        cmdr_fid: &str,
        cmdr_name: &str,
        assignment: KeyAssignment,
        now: DateTime<Utc>,
    ) -> Result<bool, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let key: Option<String> = tx
            .query_row(
                "SELECT api_key FROM sync_unassigned_key WHERE service=?1",
                [service.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        let Some(api_key) = key else {
            return Ok(false);
        };
        let c = SyncCredential {
            service,
            cmdr_fid: cmdr_fid.into(),
            cmdr_name: cmdr_name.into(),
            api_key,
            created_at: ts(now),
            verified_at: None,
            assignment,
        };
        insert_credential(&tx, &c, true)?;
        tx.execute(
            "DELETE FROM sync_unassigned_key WHERE service=?1",
            [service.as_str()],
        )?;
        set_activation_tx(&tx, service, cmdr_fid, now)?;
        tx.commit()?;
        Ok(true)
    }

    /// Pasa una asignación automática a confirmada. `false` si no estaba pendiente.
    pub fn confirm_assignment(
        &self,
        service: SyncService,
        cmdr_fid: &str,
    ) -> Result<bool, StoreError> {
        let conn = self.lock()?;
        let n = conn.execute(
            "UPDATE sync_credential SET assignment='confirmed' \
             WHERE service=?1 AND cmdr_fid=?2 AND assignment='auto_pending_confirmation'",
            rusqlite::params![service.as_str(), cmdr_fid],
        )?;
        Ok(n > 0)
    }

    pub fn set_activation(
        &self,
        service: SyncService,
        cmdr_fid: &str,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        set_activation_tx(&tx, service, cmdr_fid, at)?;
        tx.commit()?;
        Ok(())
    }

    pub fn activation(
        &self,
        service: SyncService,
        cmdr_fid: &str,
    ) -> Result<Option<DateTime<Utc>>, StoreError> {
        let conn = self.lock()?;
        let value: Option<String> = conn
            .query_row(
                "SELECT activated_at FROM sync_activation WHERE service=?1 AND cmdr_fid=?2",
                rusqlite::params![service.as_str(), cmdr_fid],
                |r| r.get(0),
            )
            .optional()?;
        value.as_deref().map(parse_ts).transpose()
    }

    /// Elimina todas las activaciones registradas para un servicio (p. ej. al apagar EDDN).
    pub fn clear_sync_activations(&self, service: SyncService) -> Result<(), StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM sync_activation WHERE service=?1",
            [service.as_str()],
        )?;
        Ok(())
    }

    /// Registra o actualiza un comandante visto en el diario. `true` si es nuevo.
    pub fn upsert_known_cmdr(
        &self,
        fid: &str,
        name: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, StoreError> {
        let conn = self.lock()?;
        let existed: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_known_cmdr WHERE cmdr_fid=?1)",
            [fid],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO sync_known_cmdr (cmdr_fid, cmdr_name, last_seen) VALUES (?1,?2,?3) \
             ON CONFLICT(cmdr_fid) DO UPDATE SET cmdr_name=excluded.cmdr_name, last_seen=excluded.last_seen",
            rusqlite::params![fid, name, ts(now)],
        )?;
        Ok(!existed)
    }

    /// Comandantes conocidos como (FID, nombre), ordenados por FID.
    pub fn known_cmdrs(&self) -> Result<Vec<(String, String)>, StoreError> {
        let conn = self.lock()?;
        let mut stmt =
            conn.prepare("SELECT cmdr_fid, cmdr_name FROM sync_known_cmdr ORDER BY cmdr_fid")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Olvida un comandante: claves, activaciones, contadores y conocido; pendientes a `discarded`.
    pub fn forget_cmdr(&self, fid: &str, now: DateTime<Utc>) -> Result<(), StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        discard_pending_tx(&tx, None, fid, now)?;
        for table in [
            "sync_credential",
            "sync_activation",
            "sync_counter",
            "sync_known_cmdr",
        ] {
            tx.execute(&format!("DELETE FROM {table} WHERE cmdr_fid=?1"), [fid])?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod sync_credential_migration {
    use crate::Store;
    use crate::sync_credential::*;
    use chrono::{DateTime, TimeZone, Utc};
    use tmc_model::sync::{KeyAssignment, SyncService};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 8, 10, 0, 0).unwrap()
    }

    fn put_legacy(store: &Store, service: &str, key: Option<&str>, name: Option<&str>) {
        store
            .lock()
            .unwrap()
            .execute(
                "INSERT OR REPLACE INTO accounts_credential (service, api_key, cmdr_name, token_json, salt, updated_at) \
                 VALUES (?1, ?2, ?3, NULL, NULL, '2026-01-01T00:00:00Z')",
                rusqlite::params![service, key, name],
            )
            .unwrap();
    }

    fn put_load_game(store: &Store, line: i64, fid: &str, name: &str) {
        let conn = store.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO journal_file (id, name, size, offset) VALUES (1, 'J.log', 0, 0)",
            [],
        )
        .unwrap();
        let json = format!(r#"{{"event":"LoadGame","FID":"{fid}","Commander":"{name}"}}"#);
        conn.execute(
            "INSERT INTO event (file_id, line, ts, name, cmdr, json) \
             VALUES (1, ?1, '2026-11-08T09:00:00Z', 'LoadGame', ?2, ?3)",
            rusqlite::params![line, name, json],
        )
        .unwrap();
    }

    fn legacy_key(store: &Store, service: &str) -> Option<String> {
        store
            .lock()
            .unwrap()
            .query_row(
                "SELECT api_key FROM accounts_credential WHERE service=?1",
                [service],
                |r| r.get(0),
            )
            .unwrap()
    }

    #[test]
    fn case_a_single_key_matching_single_known_cmdr_is_confirmed() {
        let store = Store::open_in_memory().unwrap();
        put_legacy(&store, "edsm", Some("K1"), Some("CMDR TESTER"));
        put_load_game(&store, 1, "F0000001", "CMDR TESTER");
        let report = store
            .migrate_legacy_credentials(SyncService::Edsm, now())
            .unwrap();
        assert_eq!(report.assigned, vec![SyncService::Edsm]);
        let c = store
            .sync_credential(SyncService::Edsm, "F0000001")
            .unwrap()
            .unwrap();
        assert_eq!(c.assignment, KeyAssignment::Confirmed);
        assert_eq!(c.api_key, "K1");
        assert_eq!(
            store.activation(SyncService::Edsm, "F0000001").unwrap(),
            Some(now())
        );
        assert_eq!(legacy_key(&store, "edsm"), None);
        assert_eq!(
            store.known_cmdrs().unwrap(),
            vec![("F0000001".to_string(), "CMDR TESTER".to_string())]
        );
    }

    #[test]
    fn case_b_no_known_cmdrs_waits_for_first() {
        let store = Store::open_in_memory().unwrap();
        put_legacy(&store, "inara", Some("K2"), Some("CMDR TESTER"));
        let report = store
            .migrate_legacy_credentials(SyncService::Inara, now())
            .unwrap();
        assert_eq!(report.waiting, vec![SyncService::Inara]);
        let keys = store.unassigned_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].reason, UnassignedReason::WaitingFirstCmdr);
        assert_eq!(legacy_key(&store, "inara"), None);
        assert!(
            store
                .assign_unassigned_key(
                    SyncService::Inara,
                    "F0000001",
                    "CMDR TESTER",
                    KeyAssignment::AutoPendingConfirmation,
                    now()
                )
                .unwrap()
        );
        let c = store
            .sync_credential(SyncService::Inara, "F0000001")
            .unwrap()
            .unwrap();
        assert_eq!(c.assignment, KeyAssignment::AutoPendingConfirmation);
        assert_eq!(c.api_key, "K2");
        assert!(store.unassigned_keys().unwrap().is_empty());
        assert!(
            store
                .confirm_assignment(SyncService::Inara, "F0000001")
                .unwrap()
        );
        assert_eq!(
            store
                .sync_credential(SyncService::Inara, "F0000001")
                .unwrap()
                .unwrap()
                .assignment,
            KeyAssignment::Confirmed
        );
    }

    #[test]
    fn case_c_ambiguous_stays_unassigned_and_sends_nothing() {
        let store = Store::open_in_memory().unwrap();
        put_legacy(&store, "edsm", Some("K3"), Some("OTRO"));
        put_load_game(&store, 1, "F0000001", "CMDR ONE");
        put_load_game(&store, 2, "F0000002", "CMDR TWO");
        let report = store
            .migrate_legacy_credentials(SyncService::Edsm, now())
            .unwrap();
        assert_eq!(report.ambiguous, vec![SyncService::Edsm]);
        assert!(
            store
                .sync_credentials(SyncService::Edsm)
                .unwrap()
                .is_empty()
        );
        let keys = store.unassigned_keys().unwrap();
        assert_eq!(keys[0].reason, UnassignedReason::Ambiguous);
        assert_eq!(keys[0].cmdr_name.as_deref(), Some("OTRO"));
    }

    #[test]
    fn migration_is_idempotent_and_keeps_eddn_salt_and_capi() {
        let store = Store::open_in_memory().unwrap();
        put_legacy(&store, "edsm", Some("K1"), Some("CMDR TESTER"));
        put_legacy(&store, "inara", Some("K2"), None);
        put_load_game(&store, 1, "F0000001", "CMDR TESTER");
        store
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO accounts_credential (service, api_key, cmdr_name, token_json, salt, updated_at) \
                 VALUES ('eddn', NULL, NULL, NULL, 'SALT', 'x'), ('capi', NULL, NULL, 'TOKEN', NULL, 'x')",
                [],
            )
            .unwrap();
        for s in [SyncService::Inara, SyncService::Edsm] {
            store.migrate_legacy_credentials(s, now()).unwrap();
        }
        for s in [SyncService::Inara, SyncService::Edsm, SyncService::Eddn] {
            assert_eq!(
                store.migrate_legacy_credentials(s, now()).unwrap(),
                MigrationReport::default()
            );
        }
        assert_eq!(store.sync_credentials(SyncService::Edsm).unwrap().len(), 1);
        assert_eq!(store.unassigned_keys().unwrap().len(), 1);
        assert_eq!(
            store.get_account_credential("eddn").unwrap().unwrap().salt,
            Some("SALT".into())
        );
        assert_eq!(
            store
                .get_account_credential("capi")
                .unwrap()
                .unwrap()
                .token_json,
            Some("TOKEN".into())
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use crate::sync_credential::*;
    use crate::sync_outbox::NewOutboxRow;
    use chrono::{DateTime, TimeZone, Utc};
    use tmc_model::sync::{KeyAssignment, OutboxState, RowOrigin, SyncService};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 8, 10, 0, 0).unwrap()
    }

    fn cred(service: SyncService, fid: &str) -> SyncCredential {
        SyncCredential {
            service,
            cmdr_fid: fid.into(),
            cmdr_name: "CMDR TESTER".into(),
            api_key: "K1".into(),
            created_at: "2026-11-08T10:00:00Z".into(),
            verified_at: None,
            assignment: KeyAssignment::Confirmed,
        }
    }

    fn enqueue(store: &Store, service: SyncService, fid: &str, idem: &str) {
        let row = NewOutboxRow {
            service,
            cmdr_fid: fid.into(),
            idem_key: idem.into(),
            kind: "test".into(),
            coalesce_key: None,
            event_ts: now(),
            payload: "{}".into(),
            origin: RowOrigin::Live,
            state: OutboxState::Pending,
            last_class: None,
        };
        store.enqueue_batch_with_cursor(&[row], &[], now()).unwrap();
    }

    fn count(store: &Store, sql: &str) -> i64 {
        store
            .lock()
            .unwrap()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn debug_never_prints_api_key() {
        let c = cred(SyncService::Edsm, "F0000001");
        assert!(!format!("{c:?}").contains("K1"));
    }

    #[test]
    fn delete_credential_discards_pending_and_activation() {
        let store = Store::open_in_memory().unwrap();
        store
            .put_sync_credential(&cred(SyncService::Edsm, "F0000001"))
            .unwrap();
        store
            .set_activation(SyncService::Edsm, "F0000001", now())
            .unwrap();
        enqueue(&store, SyncService::Edsm, "F0000001", "a");
        enqueue(&store, SyncService::Edsm, "F0000001", "b");
        enqueue(&store, SyncService::Edsm, "F0000002", "c");
        let n = store
            .delete_sync_credential(SyncService::Edsm, "F0000001", now())
            .unwrap();
        assert_eq!(n, 2);
        assert!(
            store
                .sync_credential(SyncService::Edsm, "F0000001")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store.activation(SyncService::Edsm, "F0000001").unwrap(),
            None
        );
        assert_eq!(
            count(
                &store,
                "SELECT COUNT(*) FROM sync_outbox WHERE state='pending'"
            ),
            1
        );
    }

    #[test]
    fn forget_cmdr_removes_counters_and_activation() {
        let store = Store::open_in_memory().unwrap();
        assert!(
            store
                .upsert_known_cmdr("F0000001", "CMDR A", now())
                .unwrap()
        );
        assert!(
            !store
                .upsert_known_cmdr("F0000001", "CMDR A2", now())
                .unwrap()
        );
        assert_eq!(
            store.known_cmdrs().unwrap(),
            vec![("F0000001".to_string(), "CMDR A2".to_string())]
        );
        store
            .put_sync_credential(&cred(SyncService::Inara, "F0000001"))
            .unwrap();
        store
            .set_activation(SyncService::Eddn, "F0000001", now())
            .unwrap();
        enqueue(&store, SyncService::Eddn, "F0000001", "a");
        store
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO sync_counter (service, cmdr_fid, sent) VALUES ('eddn','F0000001',5)",
                [],
            )
            .unwrap();
        store.forget_cmdr("F0000001", now()).unwrap();
        assert!(store.known_cmdrs().unwrap().is_empty());
        assert!(
            store
                .sync_credentials(SyncService::Inara)
                .unwrap()
                .is_empty()
        );
        assert_eq!(count(&store, "SELECT COUNT(*) FROM sync_counter"), 0);
        assert_eq!(count(&store, "SELECT COUNT(*) FROM sync_activation"), 0);
        assert_eq!(
            count(
                &store,
                "SELECT COUNT(*) FROM sync_outbox WHERE state='pending'"
            ),
            0
        );
    }
}

//! Lease exclusiva en SQLite para garantizar una única instancia emisora (C3c).

use crate::{Store, StoreError};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, TransactionBehavior};

fn parse_ts(value: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| StoreError::Unsupported("lease timestamp".into()))
}

impl Store {
    /// Adquiere o renueva el lease exclusivo (`id = 1`) si está libre, caducado o ya pertenece a `owner`.
    pub fn try_acquire_lease(
        &self,
        owner: &str,
        now: DateTime<Utc>,
        ttl: Duration,
    ) -> Result<bool, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(String, String)> = tx
            .query_row(
                "SELECT owner, expires_at FROM sync_lease WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let can_take = match current {
            None => true,
            Some((cur_owner, cur_exp)) => cur_owner == owner || parse_ts(&cur_exp)? <= now,
        };
        if !can_take {
            tx.commit()?;
            return Ok(false);
        }
        let new_exp = (now + ttl).to_rfc3339_opts(SecondsFormat::Millis, true);
        tx.execute(
            "INSERT INTO sync_lease (id, owner, expires_at) VALUES (1, ?1, ?2) \
             ON CONFLICT(id) DO UPDATE SET owner = excluded.owner, expires_at = excluded.expires_at",
            rusqlite::params![owner, new_exp],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Titular y fecha de expiración del lease actual (`None` = libre).
    pub fn lease_owner(&self) -> Result<Option<(String, DateTime<Utc>)>, StoreError> {
        let conn = self.lock()?;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT owner, expires_at FROM sync_lease WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(owner, exp)| Ok((owner, parse_ts(&exp)?)))
            .transpose()
    }

    /// Libera el lease en un cierre ordenado solo si sigue perteneciendo a `owner`.
    pub fn release_lease(&self, owner: &str) -> Result<(), StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM sync_lease WHERE id = 1 AND owner = ?1",
            [owner],
        )?;
        Ok(())
    }
}

use crate::{Store, StoreError};

pub(crate) const BASE_SCHEMA: &str = "
CREATE TABLE journal_file (
  id INTEGER PRIMARY KEY, name TEXT UNIQUE NOT NULL, size INTEGER NOT NULL,
  offset INTEGER NOT NULL, cmdr TEXT, gameversion TEXT, galaxy TEXT, complete INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE event (
  id INTEGER PRIMARY KEY, file_id INTEGER NOT NULL REFERENCES journal_file(id),
  line INTEGER NOT NULL, ts TEXT NOT NULL, name TEXT NOT NULL, cmdr TEXT, json TEXT NOT NULL,
  UNIQUE(file_id, line)
);
CREATE INDEX event_name_ts ON event(name, ts);
CREATE INDEX event_cmdr_ts ON event(cmdr, ts);
CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE api_cache (key TEXT PRIMARY KEY, fetched_at TEXT NOT NULL, ttl_s INTEGER NOT NULL, body BLOB NOT NULL);
";

pub(crate) const PRICES_SCHEMA: &str = "
CREATE TABLE market_price (
  market_id INTEGER NOT NULL, commodity TEXT NOT NULL, name_localised TEXT,
  buy_price INTEGER NOT NULL, sell_price INTEGER NOT NULL, stock INTEGER NOT NULL, demand INTEGER NOT NULL,
  updated_at TEXT NOT NULL, origin TEXT NOT NULL,
  PRIMARY KEY (market_id, commodity)
);
CREATE INDEX market_price_commodity_updated ON market_price(commodity, updated_at);
CREATE TABLE market_station (
  market_id INTEGER PRIMARY KEY, station_name TEXT NOT NULL, system_name TEXT NOT NULL, system_address INTEGER,
  x REAL, y REAL, z REAL, max_pad INTEGER, station_type TEXT, dist_ls REAL, updated_at TEXT NOT NULL
);
CREATE INDEX market_station_updated ON market_station(updated_at);
CREATE TABLE system_coords (
  name_key TEXT PRIMARY KEY, system_address INTEGER, x REAL NOT NULL, y REAL NOT NULL, z REAL NOT NULL, seen_at TEXT NOT NULL
);
CREATE INDEX system_coords_seen ON system_coords(seen_at);
";

/// El esquema anterior solo deja evidencia de snapshots completos en precios
/// Local/EDDN aún presentes. No permite reconstruir catálogos vacíos ni snapshots
/// cuyas filas fueron sustituidas por parciales; el backfill es necesariamente parcial.
pub(crate) const FULL_SNAPSHOTS_SCHEMA: &str = "
CREATE TABLE market_full_snapshot (
  market_id INTEGER PRIMARY KEY, updated_at TEXT NOT NULL, origin TEXT NOT NULL
);
CREATE INDEX market_full_snapshot_updated ON market_full_snapshot(updated_at);
INSERT INTO market_full_snapshot(market_id,updated_at,origin)
SELECT market_id,updated_at,origin FROM (
  SELECT market_id,updated_at,origin,
    ROW_NUMBER() OVER(PARTITION BY market_id ORDER BY updated_at DESC, CASE origin WHEN 'local' THEN 1 ELSE 0 END DESC) AS position
  FROM market_price WHERE origin IN ('local','eddn')
) WHERE position=1;
";

pub struct Migration {
    pub id: &'static str,
    pub sql: &'static str,
}

impl Store {
    pub fn apply_migrations(&self, migrations: &[Migration]) -> Result<(), StoreError> {
        let mut connection = self.lock()?;
        for migration in migrations {
            let transaction = connection.transaction()?;
            let applied: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_version WHERE id=?)",
                [migration.id],
                |r| r.get(0),
            )?;
            if !applied {
                transaction.execute_batch(migration.sql)?;
                transaction.execute(
                    "INSERT INTO schema_version(id,applied_at) VALUES(?1,?2)",
                    [migration.id, &chrono::Utc::now().to_rfc3339()],
                )?;
            }
            transaction.commit()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_snapshot_migration_is_registered_on_fresh_database_and_idempotent() {
        let store = Store::open_in_memory().unwrap();
        let versions = store
            .query_json(
                "SELECT id FROM schema_version WHERE id='f3_market_full_snapshots'",
                &[],
            )
            .unwrap();
        assert_eq!(versions.len(), 1);
        store
            .apply_migrations(&[Migration {
                id: "f3_market_full_snapshots",
                sql: "CREATE TABLE must_not_create(id INTEGER)",
            }])
            .unwrap();
        assert!(
            store
                .query_json(
                    "SELECT market_id,updated_at,origin FROM market_full_snapshot",
                    &[]
                )
                .is_ok()
        );
        assert!(
            store
                .query_json("SELECT id FROM must_not_create", &[])
                .is_err()
        );
    }
    #[test]
    fn existing_price_schema_backfills_local_and_eddn_evidence_only() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(BASE_SCHEMA).unwrap();
        conn.execute_batch(PRICES_SCHEMA).unwrap();
        conn.execute_batch("CREATE TABLE schema_version(id TEXT PRIMARY KEY,applied_at TEXT NOT NULL); INSERT INTO schema_version VALUES('core-f0-1','now'),('f3_prices','now'); INSERT INTO market_price VALUES(1,'gold',NULL,1,1,1,1,'2026-10-19T10:00:00.000Z','local'),(1,'silver',NULL,1,1,1,1,'2026-10-19T11:00:00.000Z','eddn'),(2,'gold',NULL,1,1,1,1,'2026-10-19T12:00:00.000Z','ardent');").unwrap();
        let store = Store::initialize(conn).unwrap();
        let rows = store
            .query_json(
                "SELECT market_id,updated_at,origin FROM market_full_snapshot ORDER BY market_id",
                &[],
            )
            .unwrap();
        assert_eq!(
            rows,
            vec![serde_json::json!([1, "2026-10-19T11:00:00.000Z", "eddn"])]
        );
    }
}

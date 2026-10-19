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

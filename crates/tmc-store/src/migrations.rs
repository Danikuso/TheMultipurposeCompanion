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

pub struct Migration {
    pub id: &'static str,
    pub sql: &'static str,
}

impl Store {
    pub fn apply_migrations(&self, migrations: &[Migration]) -> Result<(), StoreError> {
        let mut connection = self.lock()?;
        for migration in migrations {
            #[cfg(debug_assertions)]
            let diagnostic_started = std::time::Instant::now();
            let result: Result<(), StoreError> = (|| {
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
                #[cfg(debug_assertions)]
                if applied {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Sql,
                        "store.migration.skipped",
                        serde_json::json!({"index":migrations.iter().position(|m|std::ptr::eq(m,migration)),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                    );
                } else {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Sql,
                        "store.migration.applied",
                        serde_json::json!({"index":migrations.iter().position(|m|std::ptr::eq(m,migration)),"sql_bytes":migration.sql.len(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                    );
                }
                Ok(())
            })();
            #[cfg(debug_assertions)]
            if let Err(_error) = &result {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.migration.failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
            result?;
        }
        Ok(())
    }
}

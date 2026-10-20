//! Persistencia SQLite y acceso al historial de diarios.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

pub mod import;
pub mod migrations;
pub mod prices;
pub mod query;
pub mod writer;

pub use import::{ImportOptions, ImportReport, import_history};
pub use migrations::Migration;
pub use query::{EventQuery, EventRow};
pub use writer::{EventWriter, StoredEvent};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("database connection lock poisoned")]
    Poisoned,
    #[error("journal metadata exceeds SQLite integer range")]
    IntegerRange(#[from] std::num::TryFromIntError),
    #[error("unsupported value: {0}")]
    Unsupported(String),
}

/// Entrada de la caché de respuestas de API (`api_cache`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedEntry {
    pub value: String,
    pub fetched_at: DateTime<Utc>,
}

impl StoreError {
    /// Safe for logs: Display/Debug may contain parameters, paths or trigger text.
    pub fn diagnostic_class(&self) -> &'static str {
        match self {
            Self::Sql(_) => "sqlite",
            Self::Io(_) => "io",
            Self::Poisoned => "poisoned",
            Self::IntegerRange(_) => "integer_range",
            Self::Unsupported(_) => "unsupported",
        }
    }
}

fn to_sql(value: &serde_json::Value) -> Result<rusqlite::types::Value, StoreError> {
    use rusqlite::types::Value as Sql;
    Ok(match value {
        serde_json::Value::Null => Sql::Null,
        serde_json::Value::Bool(b) => Sql::Integer(i64::from(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Sql::Integer(i)
            } else if n.is_u64() {
                return Err(StoreError::Unsupported(format!(
                    "integer {n} exceeds SQLite range"
                )));
            } else if let Some(f) = n.as_f64() {
                Sql::Real(f)
            } else {
                return Err(StoreError::Unsupported(format!("number {n}")));
            }
        }
        serde_json::Value::String(s) => Sql::Text(s.clone()),
        other => {
            return Err(StoreError::Unsupported(format!(
                "non-scalar parameter {other}"
            )));
        }
    })
}

fn from_sql(value: rusqlite::types::ValueRef<'_>) -> Result<serde_json::Value, StoreError> {
    use rusqlite::types::ValueRef as R;
    Ok(match value {
        R::Null => serde_json::Value::Null,
        R::Integer(i) => i.into(),
        R::Real(f) => serde_json::Number::from_f64(f).map_or(serde_json::Value::Null, Into::into),
        R::Text(t) => String::from_utf8_lossy(t).into_owned().into(),
        R::Blob(_) => return Err(StoreError::Unsupported("BLOB column".into())),
    })
}

#[derive(Clone)]
pub struct Store {
    // Serializa transacciones de importación y escritor, también sobre la misma BD en memoria.
    connection: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::initialize(Connection::open(path).inspect_err(|_| {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.connection.failed",
                serde_json::json!({"failure_class":"sqlite","in_memory":false})
            );
        })?)
    }
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::initialize(Connection::open_in_memory().inspect_err(|_| {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.connection.failed",
                serde_json::json!({"failure_class":"sqlite","in_memory":true})
            );
        })?)
    }
    fn initialize(connection: Connection) -> Result<Self, StoreError> {
        #[cfg(debug_assertions)]
        let diagnostic_started = std::time::Instant::now();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.initialize.started",
            serde_json::json!({"started":true})
        );
        let result: Result<_, StoreError> = (|| {
            connection.busy_timeout(std::time::Duration::from_secs(5))?;
            connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS schema_version (id TEXT PRIMARY KEY, applied_at TEXT NOT NULL);")?;
            let store = Self {
                connection: Arc::new(Mutex::new(connection)),
            };
            store.apply_migrations(&[
                Migration {
                    id: "core-f0-1",
                    sql: migrations::BASE_SCHEMA,
                },
                Migration {
                    id: "f3_prices",
                    sql: migrations::PRICES_SCHEMA,
                },
                Migration {
                    id: "f3_market_full_snapshots",
                    sql: migrations::FULL_SNAPSHOTS_SCHEMA,
                },
            ])?;
            Ok(store)
        })();
        #[cfg(debug_assertions)]
        match &result {
            Ok(_value) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.initialize.completed",
                    serde_json::json!({"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.initialize.failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
        }
        result
    }
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.connection.poisoned",
                serde_json::json!({"failure_class":"poisoned"})
            );
            StoreError::Poisoned
        })
    }
    /// Ejecuta una consulta y devuelve cada fila como array JSON.
    /// INTEGER/REAL -> número, TEXT -> cadena, NULL -> null; una columna BLOB es un error.
    /// Los parámetros deben ser escalares JSON (bool se guarda como 0/1).
    pub fn query_json(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        #[cfg(debug_assertions)]
        let diagnostic_started = std::time::Instant::now();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.query.started",
            serde_json::json!({"sql_bytes":sql.len(),"sql_fingerprint":tmc_model::diagnostics::sql_fingerprint(sql),"sql_template":tmc_model::diagnostics::sql_template(sql),"parameter_types":tmc_model::diagnostics::value_types(params)})
        );
        let result: Result<_, StoreError> = (|| {
            let params = params.iter().map(to_sql).collect::<Result<Vec<_>, _>>()?;
            let conn = self.lock()?;
            let mut stmt = conn.prepare(sql)?;
            let columns = stmt.column_count();
            let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let cells = (0..columns)
                    .map(|i| from_sql(row.get_ref(i)?))
                    .collect::<Result<Vec<_>, _>>()?;
                out.push(serde_json::Value::Array(cells));
            }
            Ok(out)
        })();
        #[cfg(debug_assertions)]
        match &result {
            Ok(_value) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.query.completed",
                    serde_json::json!({"rows":_value.len(),"bytes":_value.iter().map(tmc_model::diagnostics::value_bytes).sum::<usize>(),"columns":_value.first().and_then(|r|r.as_array()).map_or(0,Vec::len),"cell_types":_value.first().and_then(|r|r.as_array()).map(|r|tmc_model::diagnostics::value_types(r)),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.query.failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
        }
        result
    }
    /// Ejecuta una sentencia y devuelve las filas afectadas.
    pub fn exec_json(&self, sql: &str, params: &[serde_json::Value]) -> Result<usize, StoreError> {
        #[cfg(debug_assertions)]
        let diagnostic_started = std::time::Instant::now();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.exec.started",
            serde_json::json!({"sql_bytes":sql.len(),"sql_fingerprint":tmc_model::diagnostics::sql_fingerprint(sql),"sql_template":tmc_model::diagnostics::sql_template(sql),"parameter_types":tmc_model::diagnostics::value_types(params)})
        );
        let result: Result<_, StoreError> = (|| {
            let params = params.iter().map(to_sql).collect::<Result<Vec<_>, _>>()?;
            let conn = self.lock()?;
            Ok(conn.execute(sql, rusqlite::params_from_iter(params))?)
        })();
        #[cfg(debug_assertions)]
        match &result {
            Ok(_value) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.exec.completed",
                    serde_json::json!({"rows":_value,"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.exec.failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
        }
        result
    }
    pub fn api_cache_get(&self, key: &str) -> Result<Option<CachedEntry>, StoreError> {
        #[cfg(debug_assertions)]
        let diagnostic_started = std::time::Instant::now();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.cache.get",
            serde_json::json!({"key_bytes":key.len()})
        );
        let result: Result<_, StoreError> = (|| {
            let conn = self.lock()?;
            let row = conn
                .query_row(
                    "SELECT body, fetched_at FROM api_cache WHERE key=?1",
                    [key],
                    |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?;
            let Some((body, fetched_at)) = row else {
                return Ok(None);
            };
            let fetched_at = DateTime::parse_from_rfc3339(&fetched_at)
                .map_err(|e| StoreError::Unsupported(format!("bad fetched_at: {e}")))?
                .with_timezone(&Utc);
            Ok(Some(CachedEntry {
                value: String::from_utf8_lossy(&body).into_owned(),
                fetched_at,
            }))
        })();
        #[cfg(debug_assertions)]
        match &result {
            Ok(_value) => {
                if let Some(_entry) = _value {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Sql,
                        "store.cache.hit",
                        serde_json::json!({"bytes":_entry.value.len(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                    );
                } else {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Sql,
                        "store.cache.miss",
                        serde_json::json!({"elapsed_us":diagnostic_started.elapsed().as_micros()})
                    );
                }
            }
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.cache.get_failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
        }
        result
    }
    /// Guarda el cuerpo con `ttl_s = 0`; la política de frescura es de la capa de red.
    pub fn api_cache_put(
        &self,
        key: &str,
        value: &str,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        #[cfg(debug_assertions)]
        let diagnostic_started = std::time::Instant::now();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.cache.put",
            serde_json::json!({"key_bytes":key.len(),"bytes":value.len()})
        );
        let result: Result<_, StoreError> = (|| {
            let conn = self.lock()?;
            conn.execute(
            "INSERT INTO api_cache(key,fetched_at,ttl_s,body) VALUES(?1,?2,0,?3) ON CONFLICT(key) DO UPDATE SET fetched_at=excluded.fetched_at, ttl_s=0, body=excluded.body",
            rusqlite::params![key, fetched_at.to_rfc3339(), value.as_bytes()],
        )?;
            Ok(())
        })();
        #[cfg(debug_assertions)]
        match &result {
            Ok(_value) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.cache.put_completed",
                    serde_json::json!({"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.cache.put_failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
        }
        result
    }
    pub fn kv_get(&self, key: &str) -> Option<String> {
        let result = self.lock().and_then(|conn| {
            Ok(conn
                .query_row("SELECT value FROM kv WHERE key=?", [key], |row| row.get(0))
                .optional()?)
        });
        match result {
            Ok(value) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.kv.get",
                    serde_json::json!({"found":value.is_some(),"bytes":value.as_ref().map_or(0,String::len)})
                );
                value
            }
            Err(error) => {
                tracing::warn!(
                    failure_class = error.diagnostic_class(),
                    "could not read stored setting"
                );
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.kv.get_failed",
                    serde_json::json!({"failure_class":error.diagnostic_class()})
                );
                None
            }
        }
    }
    pub fn kv_set(&self, key: &str, value: &str) {
        let result = self.lock().and_then(|conn| {
            conn.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [key,value])?;
            Ok(())
        });
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.kv.set",
            serde_json::json!({"succeeded":result.is_ok(),"bytes":value.len()})
        );
        if let Err(error) = result {
            tracing::warn!(
                failure_class = error.diagnostic_class(),
                "could not write stored setting"
            );
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.kv.set_failed",
                serde_json::json!({"failure_class":error.diagnostic_class()})
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    const A: &str = "Journal.2026-10-17T180000.01.log";
    const B: &str = "Journal.2026-10-17T190000.01.log";
    fn options() -> ImportOptions {
        ImportOptions {
            throttle: Arc::new(AtomicBool::new(false)),
        }
    }
    fn line(name: &str) -> String {
        format!("{{\"timestamp\":\"2026-10-17T18:00:00Z\",\"event\":\"{name}\"}}\n")
    }
    fn commander(name: &str) -> String {
        format!(
            "{{\"timestamp\":\"2026-10-17T18:00:00Z\",\"event\":\"Commander\",\"Name\":\"{name}\"}}\n"
        )
    }
    fn stored(line: u64, name: &str, cmdr: Option<&str>) -> StoredEvent {
        StoredEvent {
            file_name: A.into(),
            line,
            ts: Utc.with_ymd_and_hms(2026, 10, 17, 18, 0, 0).unwrap(),
            name: name.into(),
            cmdr: cmdr.map(str::to_owned),
            json: self::line(name).trim_end().to_owned(),
        }
    }
    fn count(store: &Store) -> i64 {
        store
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn store_query_json_returns_rows() {
        let store = Store::open_in_memory().unwrap();
        store
            .exec_json("CREATE TABLE t (id INTEGER, name TEXT)", &[])
            .unwrap();
        assert_eq!(
            store
                .exec_json("INSERT INTO t VALUES (1,'a'),(2,'b')", &[])
                .unwrap(),
            2
        );
        assert_eq!(
            store
                .query_json("SELECT id, name FROM t ORDER BY id", &[])
                .unwrap(),
            vec![serde_json::json!([1, "a"]), serde_json::json!([2, "b"])]
        );
        assert_eq!(
            store
                .query_json(
                    "SELECT id, name FROM t WHERE id=?1",
                    &[serde_json::json!(2)]
                )
                .unwrap(),
            vec![serde_json::json!([2, "b"])]
        );
    }

    #[test]
    fn query_json_rejects_u64_beyond_i64() {
        let store = Store::open_in_memory().unwrap();
        let big = serde_json::json!(u64::MAX);
        assert!(matches!(
            store.query_json("SELECT ?1", &[big]),
            Err(StoreError::Unsupported(_))
        ));
        assert_eq!(
            store
                .query_json("SELECT ?1", &[serde_json::json!(i64::MAX)])
                .unwrap(),
            vec![serde_json::json!([i64::MAX])]
        );
    }

    #[test]
    fn api_cache_put_then_get_roundtrips() {
        let store = Store::open_in_memory().unwrap();
        let t = Utc.with_ymd_and_hms(2026, 10, 17, 18, 0, 0).unwrap();
        store.api_cache_put("k", "{\"a\":1}", t).unwrap();
        assert_eq!(
            store.api_cache_get("k").unwrap(),
            Some(CachedEntry {
                value: "{\"a\":1}".into(),
                fetched_at: t
            })
        );
        assert_eq!(store.api_cache_get("zz").unwrap(), None);
        let t2 = Utc.with_ymd_and_hms(2026, 10, 18, 0, 0, 0).unwrap();
        store.api_cache_put("k", "2", t2).unwrap();
        assert_eq!(store.api_cache_get("k").unwrap().unwrap().value, "2");
    }

    #[test]
    fn schema_is_created_and_wal_enabled() {
        let dir = tempdir().unwrap();
        let store = Store::open(&dir.path().join("tmc.db")).unwrap();
        let conn = store.lock().unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        assert_eq!(synchronous, 1);
        for table in ["journal_file", "event", "kv", "api_cache", "schema_version"] {
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let store = Store::open_in_memory().unwrap();
        let migrations = [Migration {
            id: "test-1",
            sql: "CREATE TABLE test_data (value INTEGER); INSERT INTO test_data VALUES(1);",
        }];
        store.apply_migrations(&migrations).unwrap();
        store.apply_migrations(&migrations).unwrap();
        assert_eq!(
            store
                .lock()
                .unwrap()
                .query_row("SELECT count(*) FROM test_data", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn writer_batches_and_is_idempotent() {
        let store = Store::open_in_memory().unwrap();
        store
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO journal_file(name,size,offset) VALUES(?,200,123)",
                [A],
            )
            .unwrap();
        let writer = EventWriter::spawn(&store);
        writer.send(stored(1, "Test", Some("TEST_A")));
        writer.send(stored(1, "Test", Some("TEST_A")));
        writer.flush_blocking().unwrap();
        assert_eq!(count(&store), 1);
        assert_eq!(
            store
                .lock()
                .unwrap()
                .query_row("SELECT offset FROM journal_file WHERE name=?", [A], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            123
        );
        assert_eq!(count(&store.clone()), 1);
    }

    #[test]
    fn import_is_resumable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(A);
        fs::write(&path, [commander("TEST_A"), line("Test1")].concat()).unwrap();
        let store = Store::open_in_memory().unwrap();
        assert_eq!(
            import_history(&store, dir.path(), &options(), &mut |_, _| {})
                .unwrap()
                .events,
            2
        );
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(line("Test2").as_bytes())
            .unwrap();
        assert_eq!(
            import_history(&store, dir.path(), &options(), &mut |_, _| {})
                .unwrap()
                .events,
            1
        );
        assert_eq!(
            import_history(&store, dir.path(), &options(), &mut |_, _| {})
                .unwrap()
                .events,
            0
        );
        assert_eq!(count(&store), 3);
    }

    #[test]
    fn import_tags_cmdr_per_file() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(A),
            [commander("TEST_A"), line("Test")].concat(),
        )
        .unwrap();
        fs::write(
            dir.path().join(B),
            [commander("TEST_B"), line("Test")].concat(),
        )
        .unwrap();
        let store = Store::open_in_memory().unwrap();
        import_history(&store, dir.path(), &options(), &mut |_, _| {}).unwrap();
        for name in ["TEST_A", "TEST_B"] {
            let rows = store
                .query_events(&EventQuery {
                    cmdr: Some(name.into()),
                    limit: 100,
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| r.cmdr.as_deref() == Some(name)));
        }
        let complete: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT complete FROM journal_file WHERE name=?", [A], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(complete, 1);
    }

    #[test]
    fn import_reports_progress_monotonic() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), line("A")).unwrap();
        fs::write(dir.path().join(B), line("B")).unwrap();
        let store = Store::open_in_memory().unwrap();
        let mut progress = Vec::new();
        import_history(&store, dir.path(), &options(), &mut |done, total| {
            progress.push((done, total))
        })
        .unwrap();
        assert_eq!(progress.first(), Some(&(0, 2)));
        assert_eq!(progress.last(), Some(&(2, 2)));
        assert!(
            progress
                .windows(2)
                .all(|p| p[0].0 <= p[1].0 && p[0].1 == p[1].1)
        );
    }

    #[test]
    fn import_skips_invalid_lines_and_counts_them() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), format!("{{broken\n{}", line("Test"))).unwrap();
        let store = Store::open_in_memory().unwrap();
        let report = import_history(&store, dir.path(), &options(), &mut |_, _| {}).unwrap();
        assert_eq!((report.events, report.skipped_lines), (1, 1));
        assert_eq!(count(&store), 1);
        let report = import_history(&store, dir.path(), &options(), &mut |_, _| {}).unwrap();
        assert_eq!((report.events, report.skipped_lines), (0, 0));
    }

    #[test]
    fn query_events_filters_and_paginates() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        for line in 1..=5 {
            writer.send(stored(
                line,
                if line == 1 { "Other" } else { "Test" },
                Some("TEST_A"),
            ));
        }
        writer.flush_blocking().unwrap();
        let query = EventQuery {
            name: Some("Test".into()),
            cmdr: Some("TEST_A".into()),
            limit: 2,
            ..Default::default()
        };
        let first = store.query_events(&query).unwrap();
        assert_eq!(first.len(), 2);
        assert!(first[0].id > first[1].id);
        let last = store
            .query_events(&EventQuery {
                before_id: Some(first[1].id),
                ..query
            })
            .unwrap();
        assert_eq!(last.len(), 2);
        assert!(last.iter().all(|r| r.id < first[1].id));
    }

    #[test]
    fn resume_preserves_commander_and_partial_line_with_verbatim_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(A);
        let prefix = commander("TEST_A");
        fs::write(
            &path,
            format!("{prefix}  {{\"timestamp\":\"2026-10-17T18:00:00Z\", \"event\":"),
        )
        .unwrap();
        let store = Store::open_in_memory().unwrap();
        import_history(&store, dir.path(), &options(), &mut |_, _| {}).unwrap();
        let offset: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT offset FROM journal_file WHERE name=?", [A], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(offset as usize, prefix.len());
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"\"Partial\"}  \n")
            .unwrap();
        let report = import_history(&store, dir.path(), &options(), &mut |_, _| {}).unwrap();
        assert_eq!(report.events, 1);
        let rows = store
            .query_events(&EventQuery {
                name: Some("Partial".into()),
                limit: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows[0].cmdr.as_deref(), Some("TEST_A"));
        assert_eq!(
            rows[0].json,
            "  {\"timestamp\":\"2026-10-17T18:00:00Z\", \"event\":\"Partial\"}  "
        );
    }

    #[test]
    fn writer_and_import_share_database_without_duplicate_rows_or_offset_regression() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(A),
            (1..=600).map(|_| line("Test")).collect::<String>(),
        )
        .unwrap();
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        for number in 1..=600 {
            writer.send(stored(number, "Test", None));
        }
        import_history(&store, dir.path(), &options(), &mut |_, _| {}).unwrap();
        writer.flush_blocking().unwrap();
        assert_eq!(count(&store), 600);
        let offset: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT offset FROM journal_file WHERE name=?", [A], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            offset as u64,
            fs::metadata(dir.path().join(A)).unwrap().len()
        );
    }

    #[test]
    fn kv_is_shared_by_store_clones() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.kv_get("locale"), None);
        store.clone().kv_set("locale", "es");
        assert_eq!(store.kv_get("locale").as_deref(), Some("es"));
        store.kv_set("locale", "en");
        assert_eq!(store.kv_get("locale").as_deref(), Some("en"));
    }

    #[test]
    fn import_failure_after_committed_batch_does_not_advance_checkpoint() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(A);
        fs::write(&path, (0..5001).map(|_| line("Test")).collect::<String>()).unwrap();
        let store = Store::open_in_memory().unwrap();
        store.lock().unwrap().execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON event WHEN NEW.line=5001 BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        assert!(import_history(&store, dir.path(), &options(), &mut |_, _| {}).is_err());
        assert_eq!(count(&store), 5000);
        let offset: i64 = store
            .lock()
            .unwrap()
            .query_row("SELECT offset FROM journal_file WHERE name=?", [A], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(offset, 0);
        store
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_import")
            .unwrap();
        assert_eq!(
            import_history(&store, dir.path(), &options(), &mut |_, _| {})
                .unwrap()
                .events,
            1
        );
        assert_eq!(count(&store), 5001);
    }

    #[test]
    fn failed_migration_rolls_back_schema_and_version() {
        let store = Store::open_in_memory().unwrap();
        assert!(
            store
                .apply_migrations(&[Migration {
                    id: "broken",
                    sql: "CREATE TABLE partial (value TEXT); INVALID SQL;"
                }])
                .is_err()
        );
        let conn = store.lock().unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='partial'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let versions: i64 = conn
            .query_row(
                "SELECT count(*) FROM schema_version WHERE id='broken'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!((tables, versions), (0, 0));
    }
}

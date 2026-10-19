use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, types::Type};

#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    pub name: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cmdr: Option<String>,
    pub limit: u32,
    pub before_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct EventRow {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub name: String,
    pub cmdr: Option<String>,
    pub json: String,
}

impl Store {
    pub fn query_events(&self, q: &EventQuery) -> Result<Vec<EventRow>, StoreError> {
        #[cfg(debug_assertions)]
        let diagnostic_started = std::time::Instant::now();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.events.query_started",
            serde_json::json!({"event":q.name,"from":q.from.is_some(),"to":q.to.is_some(),"commander_filter":q.cmdr.is_some(),"before_id":q.before_id,"limit":q.limit.min(1000)})
        );
        let result: Result<Vec<EventRow>, StoreError> = (|| {
            let conn = self.lock()?;
            let mut statement = conn.prepare(
                "SELECT id, ts, name, cmdr, json FROM event
             WHERE (?1 IS NULL OR name = ?1)
               AND (?2 IS NULL OR ts >= ?2)
               AND (?3 IS NULL OR ts <= ?3)
               AND (?4 IS NULL OR cmdr = ?4)
               AND (?5 IS NULL OR id < ?5)
             ORDER BY id DESC LIMIT ?6",
            )?;
            let from = q
                .from
                .map(|time| time.to_rfc3339_opts(SecondsFormat::Nanos, true));
            let to =
                q.to.map(|time| time.to_rfc3339_opts(SecondsFormat::Nanos, true));
            let rows = statement.query_map(
                params![q.name, from, to, q.cmdr, q.before_id, q.limit.min(1000)],
                |row| {
                    let timestamp: String = row.get(1)?;
                    let ts = DateTime::parse_from_rfc3339(&timestamp)
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                Type::Text,
                                Box::new(error),
                            )
                        })?
                        .with_timezone(&Utc);
                    Ok(EventRow {
                        id: row.get(0)?,
                        ts,
                        name: row.get(2)?,
                        cmdr: row.get(3)?,
                        json: row.get(4)?,
                    })
                },
            )?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })();
        #[cfg(debug_assertions)]
        match &result {
            Ok(_rows) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.events.query_completed",
                    serde_json::json!({"rows":_rows.len(),"bytes":_rows.iter().map(|r|r.json.len()).sum::<usize>(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
                #[cfg(debug_assertions)]
                for row in _rows {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Journal { event: &row.name },
                        "store.events.row_payload",
                        tmc_model::diagnostics::decoded_json(&row.json)
                    );
                }
            }
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.events.query_failed",
                    serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
                );
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventWriter, StoredEvent};

    #[test]
    fn query_preserves_fractional_seconds_and_utc_bounds() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        for (line, ts) in [
            (1, "2026-10-17T12:00:00.123456788Z"),
            (2, "2026-10-17T12:00:00.123456789Z"),
            (3, "2026-10-17T12:00:00.123456790Z"),
        ] {
            writer.send(StoredEvent {
                file_name: "synthetic.log".into(),
                line,
                ts: ts.parse().unwrap(),
                name: "Test".into(),
                cmdr: None,
                json: "{}".into(),
            });
        }
        writer.flush_blocking().unwrap();
        let bound = DateTime::parse_from_rfc3339("2026-10-17T14:00:00.123456789+02:00")
            .unwrap()
            .with_timezone(&Utc);
        let rows = store
            .query_events(&EventQuery {
                from: Some(bound),
                to: Some(bound),
                limit: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ts, bound);
    }

    #[test]
    fn query_caps_requested_limit_at_one_thousand() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        for line in 1..=1001 {
            writer.send(StoredEvent {
                file_name: "synthetic.log".into(),
                line,
                ts: "2026-10-17T12:00:00Z".parse().unwrap(),
                name: "Test".into(),
                cmdr: None,
                json: "{}".into(),
            });
        }
        writer.flush_blocking().unwrap();
        let rows = store
            .query_events(&EventQuery {
                limit: u32::MAX,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 1000);
        assert!(rows.windows(2).all(|rows| rows[0].id > rows[1].id));
    }
}

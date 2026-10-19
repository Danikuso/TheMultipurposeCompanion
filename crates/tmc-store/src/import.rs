use crate::{Store, StoreError, StoredEvent};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tmc_journal::{LineTailer, list_journals};
use tmc_model::events::{EventKind, parse_line};
use tmc_model::state::Galaxy;

/// Lineas por archivo con diagnostico completo en importacion; el resto se resume por lote.
const IMPORT_FULL_DIAGNOSTIC_LINES: u64 = 32;

pub struct ImportOptions {
    pub throttle: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub files: u64,
    pub events: u64,
    pub skipped_lines: u64,
}

pub fn import_history(
    store: &Store,
    dir: &Path,
    opts: &ImportOptions,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<ImportReport, StoreError> {
    #[cfg(debug_assertions)]
    let diagnostic_started = std::time::Instant::now();
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Sql,
        "store.import.started",
        serde_json::json!({"started":true})
    );
    let result = (|| -> Result<ImportReport, StoreError> {
        let paths = list_journals(dir)?;
        let total = paths.len() as u64;
        let mut report = ImportReport::default();
        progress(0, total);
        for (index, path) in paths.iter().enumerate() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let (offset, mut cmdr, mut version, mut galaxy, complete): (
                i64,
                Option<String>,
                Option<String>,
                Option<String>,
                bool,
            ) = {
                let conn = store.lock()?;
                conn.execute(
                    "INSERT OR IGNORE INTO journal_file(name,size,offset) VALUES(?,0,0)",
                    [&name],
                )?;
                conn.query_row(
                    "SELECT offset,cmdr,gameversion,galaxy,complete FROM journal_file WHERE name=?",
                    [&name],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )?
            };
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.import.file",
                serde_json::json!({"file":tmc_model::diagnostics::source_file(path),"offset":offset,"complete":complete,"has_commander":cmdr.is_some(),"galaxy":galaxy})
            );
            if complete {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Sql,
                    "store.import.skipped_complete",
                    serde_json::json!({"file":tmc_model::diagnostics::source_file(path)})
                );
                progress(index as u64 + 1, total);
                continue;
            }
            let mut tail = LineTailer::open(path, u64::try_from(offset)?)?;
            if tail.offset() == 0 {
                // Offset inválido tras truncado: vuelve a reconstruir el contexto del archivo.
                cmdr = None;
                version = None;
                galaxy = None;
            }
            tail.limit_line_diagnostics(IMPORT_FULL_DIAGNOSTIC_LINES);
            #[cfg(debug_assertions)]
            let mut invalid_logged = 0u64;
            let mut batch = Vec::with_capacity(5000);
            for (line, json) in tail.read_new_lines()? {
                let event = match parse_line(&json) {
                    Ok(event) => event,
                    Err(_error) => {
                        report.skipped_lines += 1;
                        tracing::warn!(
                            file = file_label(path, &name),
                            line,
                            "invalid history line skipped"
                        );
                        #[cfg(debug_assertions)]
                        if invalid_logged < IMPORT_FULL_DIAGNOSTIC_LINES {
                            invalid_logged += 1;
                            tmc_model::diagnostic!(
                                tmc_model::diagnostics::DiagnosticContext::Sql,
                                "store.import.invalid_line",
                                serde_json::json!({"file":tmc_model::diagnostics::source_file(path),"line":line,"bytes":json.len(),"failure_class":match _error {tmc_model::ParseError::Empty=>"empty",tmc_model::ParseError::Json(_)=>"json",tmc_model::ParseError::MissingField(_)=>"missing_field"}})
                            );
                        }
                        continue;
                    }
                };
                match &event.kind {
                    EventKind::Commander(commander) => {
                        if let Some(name) = &commander.name {
                            cmdr = Some(name.clone());
                        }
                    }
                    EventKind::LoadGame(load) => {
                        if let Some(name) = &load.commander {
                            cmdr = Some(name.clone());
                        }
                        if let Some(gameversion) = &load.gameversion {
                            version = Some(gameversion.clone());
                            galaxy = Some(galaxy_name(gameversion).into());
                        }
                    }
                    EventKind::Fileheader(header) => {
                        if let Some(gameversion) = &header.gameversion {
                            version = Some(gameversion.clone());
                            galaxy = Some(galaxy_name(gameversion).into());
                        }
                    }
                    _ => {}
                }
                #[cfg(debug_assertions)]
                if line <= IMPORT_FULL_DIAGNOSTIC_LINES {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Sql,
                        "store.import.context",
                        serde_json::json!({"line":line,"has_commander":cmdr.is_some(),"galaxy":galaxy})
                    );
                }
                batch.push(StoredEvent {
                    file_name: name.clone(),
                    line,
                    ts: event.timestamp,
                    name: event.name,
                    cmdr: cmdr.clone(),
                    json,
                });
                if batch.len() == 5000 {
                    report.events += write_batch(store, &batch)?;
                    batch.clear();
                    if opts.throttle.load(Ordering::Relaxed) {
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::Sql,
                            "store.import.throttled",
                            serde_json::json!({"delay_ms":50})
                        );
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
            if !batch.is_empty() {
                report.events += write_batch(store, &batch)?;
            }
            // El offset del tailer solo cubre LF completos. Se confirma DESPUÉS de todos los lotes;
            // si hay un corte entre lotes, se releen filas y UNIQUE(file_id,line) las deduplica.
            let size = i64::try_from(std::fs::metadata(path)?.len())?;
            let offset = i64::try_from(tail.offset())?;
            let complete = index + 1 < paths.len() && offset == size;
            {
                let mut conn = store.lock()?;
                let transaction = conn.transaction()?;
                transaction.execute("UPDATE journal_file SET size=?1,offset=?2,cmdr=?3,gameversion=?4,galaxy=?5,complete=?6 WHERE name=?7", rusqlite::params![size,offset,cmdr,version,galaxy,complete,name])?;
                transaction.commit()?;
            }
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.import.checkpoint",
                serde_json::json!({"file":tmc_model::diagnostics::source_file(path),"offset":offset,"size":size,"complete":complete})
            );
            report.files += 1;
            progress(index as u64 + 1, total);
        }
        Ok(report)
    })();
    #[cfg(debug_assertions)]
    match &result {
        Ok(report) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.import.completed",
                serde_json::json!({"files":report.files,"events":report.events,"skipped_lines":report.skipped_lines,"elapsed_us":diagnostic_started.elapsed().as_micros()})
            );
        }
        Err(_error) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Sql,
                "store.import.failed",
                serde_json::json!({"failure_class":_error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
            );
        }
    }
    result
}

fn write_batch(store: &Store, batch: &[StoredEvent]) -> Result<u64, StoreError> {
    #[cfg(debug_assertions)]
    let diagnostic_started = std::time::Instant::now();
    let result: Result<u64, StoreError> = (|| {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.import.batch_started",
            serde_json::json!({"events":batch.len()})
        );
        let mut conn = store.lock()?;
        let transaction = conn.transaction()?;
        let mut inserted = 0;
        for event in batch {
            inserted += crate::writer::insert_event_quiet(&transaction, event)? as u64;
        }
        transaction.commit()?;
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.import.batch_committed",
            serde_json::json!({"events":batch.len(),"inserted":inserted,"deduplicated":batch.len() as u64-inserted,"elapsed_us":diagnostic_started.elapsed().as_micros()})
        );
        Ok(inserted)
    })();
    #[cfg(debug_assertions)]
    if let Err(error) = &result {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.import.batch_rollback",
            serde_json::json!({"events":batch.len(),"failure_class":error.diagnostic_class(),"elapsed_us":diagnostic_started.elapsed().as_micros()})
        );
    }
    result
}

fn galaxy_name(version: &str) -> &'static str {
    match Galaxy::from_game_version(version) {
        Galaxy::Live => "Live",
        Galaxy::Legacy => "Legacy",
        Galaxy::Unknown => "Unknown",
    }
}

/// Nombre de diario seguro para logs generales (los nombres vienen de `list_journals`).
fn file_label<'a>(path: &'a Path, name: &'a str) -> &'a str {
    #[cfg(debug_assertions)]
    {
        let _ = name;
        tmc_model::diagnostics::source_file(path)
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = path;
        name
    }
}

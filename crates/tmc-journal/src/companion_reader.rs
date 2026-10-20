use std::path::Path;
use tmc_model::companion::{CompanionFile, CompanionKind};

pub fn read_companion(dir: &Path, kind: CompanionKind) -> Option<CompanionFile> {
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::State,
        "source.companion.selected",
        tmc_model::diagnostics::json!({"file":tmc_model::diagnostics::source_file(Path::new(kind.file_name()))})
    );
    read_with_retry(&dir.join(kind.file_name()), |text| {
        tmc_model::companion::parse(kind, text)
    })
    .map(|(_, parsed)| parsed)
}

/// Blocking read; the watcher runs it in Tokio's I/O pool.
pub(crate) fn read_with_retry<T>(
    path: &Path,
    parse: impl Fn(&str) -> Result<T, tmc_model::ParseError>,
) -> Option<(String, T)> {
    for attempt in 1..=10 {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.companion.attempt",
            tmc_model::diagnostics::json!({"file":tmc_model::diagnostics::source_file(path),"attempts":attempt})
        );
        match std::fs::read_to_string(path) {
            Ok(text) if text.trim().is_empty() => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "source.companion.empty",
                    tmc_model::diagnostics::json!({"bytes":text.len(),"attempts":attempt})
                );
            }
            Ok(text) => match parse(&text) {
                Ok(value) => {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::State,
                        "source.companion.accepted",
                        tmc_model::diagnostics::json!({"file":tmc_model::diagnostics::source_file(path),"bytes":text.len(),"attempts":attempt})
                    );
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::Journal {
                            event: path.file_stem().and_then(|x| x.to_str()).unwrap_or("")
                        },
                        "source.companion.payload",
                        tmc_model::diagnostics::decoded_json(&text)
                    );
                    return Some((text, value));
                }
                Err(_error) => {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::State,
                        "source.companion.invalid",
                        tmc_model::diagnostics::json!({"bytes":text.len(),"attempts":attempt,"failure_class":match _error {tmc_model::ParseError::Empty=>"empty",tmc_model::ParseError::Json(_)=>"json",tmc_model::ParseError::MissingField(_)=>"missing_field"}})
                    );
                }
            },
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "source.companion.read_failed",
                    tmc_model::diagnostics::json!({"attempts":attempt,"failure_class":tmc_model::diagnostics::io_class(&_error)})
                );
            }
        }
        if attempt < 10 {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "source.companion.retry",
                tmc_model::diagnostics::json!({"attempts":attempt,"delay_ms":50})
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::State,
        "source.companion.exhausted",
        tmc_model::diagnostics::json!({"file":tmc_model::diagnostics::source_file(path),"attempts":10})
    );
    tracing::warn!("could not read complete companion data after retries");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn read_companion_returns_none_after_retries_on_garbage() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.json"), "{broken").unwrap();
        assert!(read_companion(dir.path(), CompanionKind::Cargo).is_none());
    }

    #[test]
    fn read_companion_retries_until_writer_finishes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Cargo.json");
        fs::write(&path, "").unwrap();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(80));
            fs::write(path, r#"{"Count":7,"Inventory":[]}"#).unwrap();
        });
        let result = read_companion(dir.path(), CompanionKind::Cargo);
        writer.join().unwrap();
        assert!(matches!(result, Some(CompanionFile::Cargo(c)) if c.count == 7));
    }
}

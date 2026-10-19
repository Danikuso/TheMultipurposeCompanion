use std::path::PathBuf;
use tmc_model::status::Status;

pub struct StatusPoller {
    path: PathBuf,
    accepted_metadata: Option<(u64, std::time::SystemTime)>,
    accepted_text: Option<String>,
    #[cfg(debug_assertions)]
    unchanged_polls: u64,
}
impl StatusPoller {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            accepted_metadata: None,
            accepted_text: None,
            #[cfg(debug_assertions)]
            unchanged_polls: 0,
        }
    }
    pub fn poll(&mut self) -> Option<Status> {
        let metadata = match std::fs::metadata(&self.path) {
            Ok(value) => value,
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "source.status.metadata_failed",
                    tmc_model::diagnostics::json!({"failure_class":tmc_model::diagnostics::io_class(&_error)})
                );
                return None;
            }
        };
        let modified = match metadata.modified() {
            Ok(value) => value,
            Err(_error) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "source.status.modified_failed",
                    tmc_model::diagnostics::json!({"failure_class":tmc_model::diagnostics::io_class(&_error)})
                );
                return None;
            }
        };
        let signature = (metadata.len(), modified);
        if self.accepted_metadata == Some(signature) {
            #[cfg(debug_assertions)]
            {
                self.unchanged_polls = self.unchanged_polls.saturating_add(1);
                if self.unchanged_polls == 1 || self.unchanged_polls.is_multiple_of(1024) {
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::State,
                        "source.status.metadata_unchanged",
                        tmc_model::diagnostics::json!({"unchanged_polls":self.unchanged_polls,"bytes":metadata.len()})
                    );
                }
            }
            return None;
        }
        #[cfg(debug_assertions)]
        {
            self.unchanged_polls = 0;
        }
        let (text, status) = crate::companion_reader::read_with_retry(&self.path, Status::parse)?;
        self.accepted_metadata = Some(signature);
        if self.accepted_text.as_ref() == Some(&text) {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "source.status.content_unchanged",
                tmc_model::diagnostics::json!({"bytes":text.len()})
            );
            return None;
        }
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "source.status.accepted",
            &status
        );
        self.accepted_text = Some(text);
        Some(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn status_poller_ignores_empty_and_truncated_reads() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Status.json");
        let first = r#"{"timestamp":"2026-03-05T18:00:00Z","event":"Status","Flags":1}"#;
        fs::write(&path, first).unwrap();
        let mut poller = StatusPoller::new(path.clone());
        assert!(poller.poll().is_some());
        assert!(poller.poll().is_none());
        fs::write(&path, "").unwrap();
        assert!(poller.poll().is_none());
        fs::write(&path, "{\"Flags\":").unwrap();
        assert!(poller.poll().is_none());
        fs::write(&path, first).unwrap();
        assert!(poller.poll().is_none());
        fs::write(&path, first.replace("\"Flags\":1", "\"Flags\":16")).unwrap();
        assert!(poller.poll().is_some());
    }
}

use std::path::PathBuf;
use tmc_model::status::Status;

pub struct StatusPoller {
    path: PathBuf,
    accepted_metadata: Option<(u64, std::time::SystemTime)>,
    accepted_text: Option<String>,
}
impl StatusPoller {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            accepted_metadata: None,
            accepted_text: None,
        }
    }
    pub fn poll(&mut self) -> Option<Status> {
        let metadata = std::fs::metadata(&self.path).ok()?;
        let signature = (metadata.len(), metadata.modified().ok()?);
        if self.accepted_metadata == Some(signature) {
            return None;
        }
        let (text, status) =
            crate::companion_reader::read_with_retry(&self.path, |text| Status::parse(text).ok())?;
        self.accepted_metadata = Some(signature);
        if self.accepted_text.as_ref() == Some(&text) {
            return None;
        }
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

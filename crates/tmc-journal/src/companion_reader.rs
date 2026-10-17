use std::path::Path;
use tmc_model::companion::{CompanionFile, CompanionKind};

pub fn read_companion(dir: &Path, kind: CompanionKind) -> Option<CompanionFile> {
    read_with_retry(&dir.join(kind.file_name()), |text| {
        tmc_model::companion::parse(kind, text).ok()
    })
    .map(|(_, parsed)| parsed)
}

/// Lectura bloqueante; el watcher la ejecuta en el pool de E/S de Tokio.
pub(crate) fn read_with_retry<T>(
    path: &Path,
    parse: impl Fn(&str) -> Option<T>,
) -> Option<(String, T)> {
    for attempt in 0..10 {
        if let Ok(text) = std::fs::read_to_string(path)
            && !text.trim().is_empty()
            && let Some(value) = parse(&text)
        {
            return Some((text, value));
        }
        if attempt < 9 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    tracing::warn!(path = %path.display(), "could not read complete companion data after retries");
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

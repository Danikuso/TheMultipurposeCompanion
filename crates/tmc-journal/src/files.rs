//! Reconocimiento y listado de archivos de diario.

use std::io;
use std::path::{Path, PathBuf};

pub fn is_journal_name(name: &str) -> bool {
    let Some(body) = name
        .strip_prefix("Journal.")
        .and_then(|s| s.strip_suffix(".log"))
    else {
        return false;
    };
    let Some((date, part)) = body.rsplit_once('.') else {
        return false;
    };
    if part.len() != 2 || !part.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match date.len() {
        12 => date.bytes().all(|b| b.is_ascii_digit()),
        17 => date.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            10 => b == b'T',
            _ => b.is_ascii_digit(),
        }),
        _ => false,
    }
}

pub fn list_journals(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let result: io::Result<Vec<PathBuf>> = (|| {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry.file_name().to_str().is_some_and(is_journal_name)
            {
                files.push(entry.path());
            }
        }
        // La fecha de dos dígitos del formato antiguo no es comparable con YYYY-MM-DD.
        files.sort_by(|a, b| {
            let a = a.file_name().unwrap();
            let b = b.file_name().unwrap();
            a.len().cmp(&b.len()).then_with(|| a.cmp(b))
        });
        Ok(files)
    })();
    #[cfg(debug_assertions)]
    match &result {
        Ok(_files) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "source.files.listed",
                tmc_model::diagnostics::json!({"files":_files.len()})
            );
        }
        Err(_error) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "source.files.list_failed",
                tmc_model::diagnostics::json!({"failure_class":tmc_model::diagnostics::io_class(_error)})
            );
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn recognises_new_and_old_journal_names() {
        for name in [
            "Journal.2026-10-17T182233.01.log",
            "Journal.2026-10-17T182233.02.log",
            "Journal.161018142304.01.log",
        ] {
            assert!(is_journal_name(name), "{name}");
        }
    }

    #[test]
    fn ignores_non_journal_files() {
        for name in [
            "Status.json",
            "Journal.txt",
            "JournalBeta.2026-10-17T182233.01.log",
            "Journal.2026-10-17T182233.01.log.bak",
            "Journal.2026-10-17T182233.1.log",
            "Journal.2026-10-17T1822330.01.log",
            "Journal.2026_10_17T182233.01.log",
            "Journal.2026-10-17T1822ab.01.log",
            "Journal.16101814230.01.log",
            "Journal.161018142304.ab.log",
            "journal.161018142304.01.log",
            "Journal.é61018142304.01.log",
            "Journal.2026-10-17T182233.01.LOG",
        ] {
            assert!(!is_journal_name(name), "{name}");
        }
    }

    #[test]
    fn lists_journals_sorted_old_format_first() {
        let dir = tempdir().unwrap();
        let expected = [
            "Journal.161018142304.01.log",
            "Journal.261018142304.01.log",
            "Journal.2026-10-16T182233.01.log",
            "Journal.2026-10-17T182233.01.log",
            "Journal.2026-10-17T182233.02.log",
        ];
        for name in expected.iter().rev().chain(["Status.json"].iter()) {
            fs::write(dir.path().join(name), "").unwrap();
        }
        fs::create_dir(dir.path().join("Journal.2026-10-18T182233.01.log")).unwrap();
        let paths: Vec<_> = expected.iter().map(|n| dir.path().join(n)).collect();
        assert_eq!(list_journals(dir.path()).unwrap(), paths);
    }

    #[test]
    fn reads_journal_from_path_with_spaces_and_accents() {
        let dir = tempdir().unwrap();
        let folder = dir.path().join("Partidas guardadas ñ é");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("Journal.2026-10-17T182233.01.log"), "").unwrap();
        fs::write(folder.join("Journal.2026-10-17T182233.02.log"), "").unwrap();
        assert_eq!(list_journals(&folder).unwrap().len(), 2);
    }

    #[test]
    fn list_errors_for_missing_folder_and_accepts_empty_folder() {
        let dir = tempdir().unwrap();
        assert!(list_journals(dir.path()).unwrap().is_empty());
        assert_eq!(
            list_journals(&dir.path().join("missing"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
}

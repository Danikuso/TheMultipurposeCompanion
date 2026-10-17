//! Lectura incremental de líneas completas con offset persistible de bytes.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub struct LineTailer {
    path: PathBuf,
    file: File,
    offset: u64,
    line_no: u64,
    read_pos: u64,
    pending: Vec<u8>,
}

impl LineTailer {
    pub fn open(path: &Path, offset: u64) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            const FILE_SHARE_READ: u32 = 1;
            const FILE_SHARE_WRITE: u32 = 2;
            const FILE_SHARE_DELETE: u32 = 4;
            options.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
        }
        let mut file = options.open(path)?;
        let offset = if offset <= file.metadata()?.len() {
            offset
        } else {
            0
        };
        let mut line_no = 0;
        let mut prefix = (&mut file).take(offset);
        let mut buffer = [0; 8192];
        loop {
            let count = prefix.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            line_no += buffer[..count].iter().filter(|&&b| b == b'\n').count() as u64;
        }
        file.seek(SeekFrom::Start(offset))?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            offset,
            line_no,
            read_pos: offset,
            pending: Vec::new(),
        })
    }

    pub fn read_new_lines(&mut self) -> io::Result<Vec<(u64, String)>> {
        if self.file.metadata()?.len() < self.read_pos {
            self.file.seek(SeekFrom::Start(0))?;
            self.offset = 0;
            self.line_no = 0;
            self.read_pos = 0;
            self.pending.clear();
        }
        let read = self.file.read_to_end(&mut self.pending);
        // Incluso un error puede haber añadido bytes: conservamos buffer y cursor para reintentar.
        self.read_pos = self.file.stream_position()?;
        read?;
        let mut lines = Vec::new();
        let mut start = 0;
        for (end, &byte) in self.pending.iter().enumerate() {
            if byte != b'\n' {
                continue;
            }
            let mut line = &self.pending[start..end];
            if self.offset == 0 && start == 0 {
                let nuls = line.iter().take_while(|&&b| b == 0).count();
                line = &line[nuls..];
            }
            if let Some(without_cr) = line.strip_suffix(b"\r") {
                line = without_cr;
            }
            self.line_no += 1;
            lines.push((self.line_no, String::from_utf8_lossy(line).into_owned()));
            start = end + 1;
        }
        self.offset += start as u64;
        if start > 0 {
            self.pending = self.pending.split_off(start);
        }
        Ok(lines)
    }

    /// Offset de bytes tras el último LF entregado. Excluye la cola parcial para poder reabrir.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Número de la última línea completa entregada (0 antes de la primera).
    pub fn line_no(&self) -> u64 {
        self.line_no
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;

    fn append(path: &Path, bytes: &[u8]) {
        OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }

    #[test]
    fn reads_complete_lines_and_advances_offset() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, "uno\ndos\n").unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert_eq!(tail.path(), path);
        assert_eq!(tail.line_no(), 0);
        assert_eq!(
            tail.read_new_lines().unwrap(),
            vec![(1, "uno".into()), (2, "dos".into())]
        );
        assert_eq!((tail.offset(), tail.line_no()), (8, 2));
        assert!(tail.read_new_lines().unwrap().is_empty());
        append(&path, "tres\n".as_bytes());
        assert_eq!(tail.read_new_lines().unwrap(), vec![(3, "tres".into())]);
        assert_eq!(tail.offset(), 13);
    }

    #[test]
    fn tailer_holds_partial_line_until_newline() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, br#"{"a":1"#).unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert!(tail.read_new_lines().unwrap().is_empty());
        assert!(tail.read_new_lines().unwrap().is_empty());
        assert_eq!(tail.offset(), 0);
        append(&path, b"}\n");
        assert_eq!(
            tail.read_new_lines().unwrap(),
            vec![(1, r#"{"a":1}"#.into())]
        );
        assert_eq!(tail.offset(), 8);
    }

    #[test]
    fn strips_crlf() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, b"one\r\ntwo\r").unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "one".into())]);
        assert_eq!(tail.offset(), 5);
        append(&path, b"\n");
        assert_eq!(tail.read_new_lines().unwrap(), vec![(2, "two".into())]);
        assert_eq!(tail.offset(), 10);
    }

    #[test]
    fn resumes_from_offset_with_correct_line_numbers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, "ñ\r\nsecond\nthird\n").unwrap();
        let mut tail = LineTailer::open(&path, 4).unwrap();
        assert_eq!(tail.line_no(), 1);
        assert_eq!(
            tail.read_new_lines().unwrap(),
            vec![(2, "second".into()), (3, "third".into())]
        );
        assert_eq!((tail.offset(), tail.line_no()), (17, 3));
    }

    #[test]
    fn handles_40kb_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        let text = "x".repeat(40 * 1024);
        fs::write(&path, &text).unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert!(tail.read_new_lines().unwrap().is_empty());
        append(&path, b"\n");
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, text)]);
        assert_eq!(tail.offset(), 40 * 1024 + 1);
    }

    #[test]
    fn reads_while_writer_keeps_file_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        let mut writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        writer.write_all(b"one\n").unwrap();
        writer.flush().unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "one".into())]);
        writer.write_all(b"two\n").unwrap();
        writer.flush().unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(2, "two".into())]);
    }

    #[test]
    fn skips_leading_nul_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, b"\0\0").unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert!(tail.read_new_lines().unwrap().is_empty());
        append(&path, b"\0one\n\0two\n");
        assert_eq!(
            tail.read_new_lines().unwrap(),
            vec![(1, "one".into()), (2, "\0two".into())]
        );
        assert_eq!(tail.offset(), 12);
    }

    #[test]
    fn persisted_offset_replays_buffered_partial_line_on_reopen() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, b"one\npar").unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "one".into())]);
        let offset = tail.offset();
        assert_eq!(offset, 4);
        drop(tail);
        append(&path, b"tial\n");
        let mut resumed = LineTailer::open(&path, offset).unwrap();
        assert_eq!(
            resumed.read_new_lines().unwrap(),
            vec![(2, "partial".into())]
        );
        assert_eq!(resumed.offset(), 12);
    }

    #[test]
    fn truncation_resets_offset_line_numbers_and_pending_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, b"old\nunfinished").unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "old".into())]);
        fs::write(&path, b"new\n").unwrap();
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "new".into())]);
        assert_eq!((tail.offset(), tail.line_no()), (4, 1));
    }

    #[test]
    fn offset_beyond_eof_resets_to_start() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, b"one\n").unwrap();
        let mut tail = LineTailer::open(&path, 99).unwrap();
        assert_eq!((tail.offset(), tail.line_no()), (0, 0));
        assert_eq!(tail.read_new_lines().unwrap(), vec![(1, "one".into())]);
    }

    #[test]
    fn invalid_utf8_is_lossy_without_changing_byte_offsets() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.log");
        fs::write(&path, b"bad\xff\n\xc3").unwrap();
        let mut tail = LineTailer::open(&path, 0).unwrap();
        assert_eq!(
            tail.read_new_lines().unwrap(),
            vec![(1, "bad\u{fffd}".into())]
        );
        assert_eq!(tail.offset(), 5);
        append(&path, b"\xb1\n");
        assert_eq!(tail.read_new_lines().unwrap(), vec![(2, "ñ".into())]);
        assert_eq!(tail.offset(), 8);
    }
}

//! Recuperación del diario anterior de la misma sesión (§5.4, rulings T0-4, T0-5 y PF-6).
//!
//! El archivo vigente lo cubre la relectura del watcher (`CatchUp`); aquí solo se decide cuál es el
//! vigente y si el inmediatamente anterior pertenece a la misma sesión, y se lee ese anterior línea a
//! línea (sin cargarlo entero) para entregarlo al router antes de la relectura del vigente.

use crate::tracker::TransientTracker;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tmc_journal::{is_journal_name, list_journals};
use tmc_model::events::parse_line;
use tmc_model::sync::{FeedOrigin, SyncFeed, SyncFeedEvent};

/// Bytes finales del archivo anterior que se leen para buscar el `Continued`.
const TAIL_BYTES: u64 = 4096;

fn part_of(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    if !is_journal_name(name) {
        return None;
    }
    let body = name.strip_suffix(".log")?;
    body.rsplit_once('.')?.1.parse().ok()
}

/// La última línea completa y no vacía del archivo es un evento `Continued`.
fn ends_with_continued(path: &Path) -> bool {
    let read_tail = || -> io::Result<String> {
        let mut file = File::open(path)?;
        let len = file.metadata()?.len();
        file.seek(SeekFrom::Start(len.saturating_sub(TAIL_BYTES)))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    };
    let Ok(tail) = read_tail() else {
        return false;
    };
    tail.lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .and_then(|l| parse_line(l).ok())
        .is_some_and(|ev| ev.name == "Continued")
}

/// Regla de misma sesión (ruling T0-4): `part` consecutivo en el nombre o `Continued` al final de `prev`.
pub fn same_session(prev: &Path, cur: &Path) -> bool {
    let (Some(prev_part), Some(cur_part)) = (part_of(prev), part_of(cur)) else {
        return false;
    };
    (cur_part >= 2 && prev_part + 1 == cur_part) || ends_with_continued(prev)
}

/// Qué recuperar en este arranque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPlan {
    /// Nombre del archivo vigente al arrancar (el último de `list_journals`).
    pub current: String,
    /// Ruta del inmediatamente anterior, solo si es de la misma sesión.
    pub previous: Option<PathBuf>,
}

/// Lector del diario anterior de la misma sesión.
pub struct Recovery {
    journal_dir: PathBuf,
}

impl Recovery {
    pub fn new(journal_dir: PathBuf) -> Self {
        Self { journal_dir }
    }

    /// Vigente y anterior de la misma sesión (`None` = no hay diarios). Nunca más atrás.
    pub fn plan(&self) -> io::Result<Option<RecoveryPlan>> {
        let journals = list_journals(&self.journal_dir)?;
        let Some(current) = journals.last() else {
            return Ok(None);
        };
        let previous = journals
            .len()
            .checked_sub(2)
            .map(|i| journals[i].clone())
            .filter(|prev| same_session(prev, current));
        Ok(Some(RecoveryPlan {
            current: current
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            previous,
        }))
    }

    /// Entrega las líneas de `path` en orden como `CatchUp`, una a una, con la galaxia del propio
    /// archivo y `game_running = false`. Numeración 1-based como `LineTailer`; una cola sin salto de
    /// línea no se entrega y las líneas que no se pueden analizar se saltan sin romper la numeración.
    pub fn stream_feeds<E: From<io::Error>>(
        path: &Path,
        mut deliver: impl FnMut(SyncFeed) -> Result<(), E>,
    ) -> Result<(), E> {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut reader = BufReader::new(File::open(path)?);
        let mut tracker = TransientTracker::new();
        let mut buf = Vec::new();
        let mut line_no = 0u64;
        loop {
            buf.clear();
            if reader.read_until(b'\n', &mut buf)? == 0 || buf.last() != Some(&b'\n') {
                return Ok(());
            }
            line_no += 1;
            let mut bytes = &buf[..buf.len() - 1];
            if line_no == 1 {
                let nuls = bytes.iter().take_while(|&&b| b == 0).count();
                bytes = &bytes[nuls..];
            }
            let text = String::from_utf8_lossy(bytes.strip_suffix(b"\r").unwrap_or(bytes));
            let Ok(ev) = parse_line(&text) else {
                continue;
            };
            tracker.apply(&ev);
            deliver(SyncFeed {
                event: SyncFeedEvent::Journal {
                    ev,
                    file: file_name.clone(),
                    line: line_no,
                },
                origin: FeedOrigin::CatchUp,
                galaxy: tracker.galaxy(),
                game_running: false,
            })?;
        }
    }
}

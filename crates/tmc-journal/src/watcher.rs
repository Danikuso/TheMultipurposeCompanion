//! Fuentes locales por sondeo; notify solo acelera la detección de diarios nuevos.

use crate::{
    LineTailer, StatusPoller, graphics_override_path, is_game_running, list_journals,
    read_companion, read_hud_theme,
};
use notify::{EventKind as NotifyKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tmc_model::companion::CompanionKind;
use tmc_model::events::{EventKind, parse_line};
use tmc_model::source::SourceEvent;
use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, MissedTickBehavior};

pub struct WatchOptions {
    pub journal_dir: PathBuf,
    pub poll_active: Duration,
    pub poll_idle: Duration,
    pub new_file_check: Duration,
    pub process_check: Duration,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            journal_dir: PathBuf::new(),
            poll_active: Duration::from_millis(250),
            poll_idle: Duration::from_secs(1),
            new_file_check: Duration::from_secs(1),
            process_check: Duration::from_secs(10),
        }
    }
}

pub fn spawn_watcher(
    opts: WatchOptions,
    tx: mpsc::Sender<SourceEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run(opts, tx))
}

struct JournalReader {
    dir: PathBuf,
    tail: Option<LineTailer>,
}

impl JournalReader {
    fn startup(dir: PathBuf) -> Self {
        let tail = match list_journals(&dir) {
            Ok(paths) => paths.last().and_then(|p| Self::open(p)),
            Err(error) => {
                tracing::warn!(path = %dir.display(), %error, "could not list journals");
                None
            }
        };
        Self { dir, tail }
    }

    fn open(path: &Path) -> Option<LineTailer> {
        match LineTailer::open(path, 0) {
            Ok(tail) => Some(tail),
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "could not open journal");
                None
            }
        }
    }

    fn drain(&mut self, replaying: bool, events: &mut Vec<SourceEvent>) -> bool {
        let Some(tail) = &mut self.tail else {
            return true;
        };
        let lines = match tail.read_new_lines() {
            Ok(lines) => lines,
            Err(error) => {
                tracing::warn!(path = %tail.path().display(), %error, "could not read journal");
                return false;
            }
        };
        let file = tail
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for (line, text) in lines {
            match parse_line(&text) {
                Ok(ev) => events.push(SourceEvent::Journal {
                    ev,
                    file: file.clone(),
                    line,
                    replaying,
                }),
                Err(error) => tracing::warn!(%file, line, %error, "invalid journal line skipped"),
            }
        }
        true
    }

    fn poll(&mut self, scan: bool, replaying: bool) -> Vec<SourceEvent> {
        let mut events = Vec::new();
        if !self.drain(replaying, &mut events) || !scan {
            return events;
        }
        let paths = match list_journals(&self.dir) {
            Ok(paths) => paths,
            Err(error) => {
                tracing::warn!(path = %self.dir.display(), %error, "could not list journals");
                return events;
            }
        };
        let candidate = if let Some(tail) = &self.tail {
            paths
                .into_iter()
                .find(|p| journal_key(p) > journal_key(tail.path()))
        } else {
            paths.into_iter().next()
        };
        if let Some(path) = candidate
            && let Some(tail) = Self::open(&path)
        {
            self.tail = Some(tail);
            self.drain(false, &mut events);
        }
        events
    }
}

fn journal_key(path: &Path) -> (usize, &std::ffi::OsStr) {
    let name = path.file_name().unwrap_or_default();
    (name.len(), name)
}

async fn emit_journals(
    events: Vec<SourceEvent>,
    dir: &Path,
    tx: &mpsc::Sender<SourceEvent>,
    running: &mut bool,
) -> bool {
    for event in events {
        let companion = if let SourceEvent::Journal { ev, .. } = &event {
            match ev.kind {
                EventKind::Fileheader(_) | EventKind::LoadGame(_) => *running = true,
                EventKind::Shutdown => *running = false,
                _ => {}
            }
            CompanionKind::for_event(&ev.name)
        } else {
            None
        };
        if tx.send(event).await.is_err() {
            return false;
        }
        if let Some(kind) = companion {
            let dir = dir.to_path_buf();
            let result = tokio::select! {
                biased;
                _ = tx.closed() => return false,
                result = tokio::task::spawn_blocking(move || read_companion(&dir, kind)) => result,
            };
            if let Ok(Some(file)) = result
                && tx.send(SourceEvent::Companion(file)).await.is_err()
            {
                return false;
            }
        }
    }
    true
}

async fn poll_status(mut poller: StatusPoller, tx: mpsc::Sender<SourceEvent>) {
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! { biased; _ = tx.closed() => return, _ = tick.tick() => {} }
        let result = tokio::select! {
            biased;
            _ = tx.closed() => return,
            result = tokio::task::spawn_blocking(move || {
                let status = poller.poll();
                (poller, status)
            }) => result,
        };
        let Ok((returned, status)) = result else {
            return;
        };
        poller = returned;
        if let Some(status) = status
            && tx.send(SourceEvent::Status(status)).await.is_err()
        {
            return;
        }
    }
}

async fn poll_process(
    period: Duration,
    tx: mpsc::Sender<SourceEvent>,
    running: watch::Sender<bool>,
) {
    let mut tick = tokio::time::interval(nonzero(period));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last = None;
    loop {
        tokio::select! { biased; _ = tx.closed() => return, _ = tick.tick() => {} }
        let result = tokio::select! {
            biased;
            _ = tx.closed() => return,
            result = tokio::task::spawn_blocking(is_game_running) => result,
        };
        let Ok(value) = result else {
            return;
        };
        if last != Some(value) {
            last = Some(value);
            let _ = running.send(value);
            if tx
                .send(SourceEvent::GameProcess { running: value })
                .await
                .is_err()
            {
                return;
            }
        }
    }
}

fn nonzero(duration: Duration) -> Duration {
    duration.max(Duration::from_millis(1))
}

async fn run(opts: WatchOptions, tx: mpsc::Sender<SourceEvent>) {
    let (notify_tx, mut notify_rx) = mpsc::channel(1);
    let mut notifier = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result
            && matches!(event.kind, NotifyKind::Create(_))
        {
            let _ = notify_tx.try_send(());
        }
    })
    .ok();
    if let Some(watcher) = &mut notifier
        && let Err(error) = watcher.watch(&opts.journal_dir, RecursiveMode::NonRecursive)
    {
        tracing::warn!(%error, "notify unavailable; using journal polling");
    }

    let dir = opts.journal_dir.clone();
    let startup = tokio::select! {
        biased;
        _ = tx.closed() => return,
        result = tokio::task::spawn_blocking(move || {
            let mut reader = JournalReader::startup(dir);
            let events = reader.poll(false, true);
            (reader, events)
        }) => result,
    };
    let Ok((mut reader, events)) = startup else {
        return;
    };
    let mut running = false;
    if !emit_journals(events, &opts.journal_dir, &tx, &mut running).await {
        return;
    }
    let theme = tokio::select! {
        biased;
        _ = tx.closed() => return,
        result = tokio::task::spawn_blocking(|| read_hud_theme(graphics_override_path().as_deref())) => result,
    };
    if let Ok(theme) = theme
        && tx.send(SourceEvent::Theme(theme)).await.is_err()
    {
        return;
    }

    let status_path = opts.journal_dir.join("Status.json");
    let initial_status = tokio::select! {
        biased;
        _ = tx.closed() => return,
        result = tokio::task::spawn_blocking(move || {
            let mut poller = StatusPoller::new(status_path);
            let initial = poller.poll();
            (poller, initial)
        }) => result,
    };
    let Ok((poller, initial_status)) = initial_status else {
        return;
    };
    if let Some(status) = initial_status
        && tx.send(SourceEvent::Status(status)).await.is_err()
    {
        return;
    }

    let (running_tx, mut running_rx) = watch::channel(running);
    let mut sources = tokio::task::JoinSet::new();
    sources.spawn(poll_status(poller, tx.clone()));
    sources.spawn(poll_process(opts.process_check, tx.clone(), running_tx));
    let mut scan = tokio::time::interval(nonzero(opts.new_file_check));
    scan.set_missed_tick_behavior(MissedTickBehavior::Skip);
    scan.tick().await;
    let mut next_read = Instant::now();
    let mut notify_open = true;
    let mut process_open = true;
    loop {
        let check_new = tokio::select! {
            biased;
            _ = tx.closed() => return,
            changed = running_rx.changed(), if process_open => {
                if changed.is_ok() { running = *running_rx.borrow_and_update(); }
                else { process_open = false; }
                next_read = Instant::now();
                continue;
            }
            notification = notify_rx.recv(), if notify_open => {
                if notification.is_none() { notify_open = false; continue; }
                true
            }
            _ = scan.tick() => true,
            _ = tokio::time::sleep_until(next_read) => false,
        };
        let result = tokio::select! {
            biased;
            _ = tx.closed() => return,
            result = tokio::task::spawn_blocking(move || {
                let events = reader.poll(check_new, false);
                (reader, events)
            }) => result,
        };
        let Ok((returned, events)) = result else {
            return;
        };
        reader = returned;
        if !emit_journals(events, &opts.journal_dir, &tx, &mut running).await {
            return;
        }
        next_read = Instant::now()
            + nonzero(if running {
                opts.poll_active
            } else {
                opts.poll_idle
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;
    use tmc_model::companion::CompanionFile;
    use tokio::time::timeout;

    const A: &str = "Journal.2026-10-17T180000.01.log";
    const B: &str = "Journal.2026-10-17T180100.01.log";
    fn line(id: u32) -> String {
        format!("{{\"timestamp\":\"2026-10-17T18:00:00Z\",\"event\":\"Test{id}\"}}\n")
    }
    fn start(dir: &std::path::Path) -> (mpsc::Receiver<SourceEvent>, tokio::task::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel(128);
        let opts = WatchOptions {
            journal_dir: dir.to_path_buf(),
            poll_active: Duration::from_millis(20),
            poll_idle: Duration::from_millis(20),
            new_file_check: Duration::from_millis(20),
            process_check: Duration::from_secs(60),
        };
        (rx, spawn_watcher(opts, tx))
    }
    async fn next(rx: &mut mpsc::Receiver<SourceEvent>) -> SourceEvent {
        timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .expect("watcher stopped")
    }
    async fn journal(rx: &mut mpsc::Receiver<SourceEvent>) -> (String, String, u64, bool) {
        loop {
            if let SourceEvent::Journal {
                ev,
                file,
                line,
                replaying,
            } = next(rx).await
            {
                return (ev.name, file, line, replaying);
            }
        }
    }
    async fn theme(rx: &mut mpsc::Receiver<SourceEvent>) {
        loop {
            if matches!(next(rx).await, SourceEvent::Theme(_)) {
                return;
            }
        }
    }
    async fn stop(rx: mpsc::Receiver<SourceEvent>, handle: tokio::task::JoinHandle<()>) {
        drop(rx);
        timeout(Duration::from_secs(2), handle)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn watcher_replays_current_journal_then_goes_live() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), line(1)).unwrap();
        let (mut rx, handle) = start(dir.path());
        assert_eq!(journal(&mut rx).await, ("Test1".into(), A.into(), 1, true));
        theme(&mut rx).await;
        OpenOptions::new()
            .append(true)
            .open(dir.path().join(A))
            .unwrap()
            .write_all(line(2).as_bytes())
            .unwrap();
        assert_eq!(journal(&mut rx).await, ("Test2".into(), A.into(), 2, false));
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_switches_to_new_journal_without_losing_lines() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), [line(1), line(2), line(3)].concat()).unwrap();
        let (mut rx, handle) = start(dir.path());
        for id in 1..=3 {
            assert_eq!(journal(&mut rx).await.0, format!("Test{id}"));
        }
        theme(&mut rx).await;
        // Sin await entre ambas escrituras: el watcher debe drenar A antes de leer B.
        fs::write(dir.path().join(B), [line(5), line(6)].concat()).unwrap();
        OpenOptions::new()
            .append(true)
            .open(dir.path().join(A))
            .unwrap()
            .write_all(line(4).as_bytes())
            .unwrap();
        for (name, file, number) in [("Test4", A, 4), ("Test5", B, 1), ("Test6", B, 2)] {
            assert_eq!(
                journal(&mut rx).await,
                (name.into(), file.into(), number, false)
            );
        }
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_emits_companion_after_trigger_event() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.json"),
            r#"{"Count":7,"Inventory":[]}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join(A),
            "{\"timestamp\":\"2026-10-17T18:00:00Z\",\"event\":\"Cargo\"}\n",
        )
        .unwrap();
        let (mut rx, handle) = start(dir.path());
        assert_eq!(journal(&mut rx).await.0, "Cargo");
        assert!(
            matches!(next(&mut rx).await, SourceEvent::Companion(CompanionFile::Cargo(c)) if c.count == 7)
        );
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn invalid_line_is_skipped_not_fatal() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), format!("{{broken\n{}", line(2))).unwrap();
        let (mut rx, handle) = start(dir.path());
        assert_eq!(journal(&mut rx).await, ("Test2".into(), A.into(), 2, true));
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_stops_when_channel_closed_with_no_journal() {
        let dir = tempdir().unwrap();
        let (rx, handle) = start(dir.path());
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_keeps_reading_while_status_is_truncated() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), line(1)).unwrap();
        let status_path = dir.path().join("Status.json");
        fs::write(
            &status_path,
            r#"{"timestamp":"2026-10-17T18:00:00Z","Flags":1}"#,
        )
        .unwrap();
        let (mut rx, handle) = start(dir.path());
        assert_eq!(journal(&mut rx).await.0, "Test1");
        theme(&mut rx).await;
        loop {
            if matches!(next(&mut rx).await, SourceEvent::Status(_)) {
                break;
            }
        }
        // Corrupción durante el seguimiento: el snapshot inicial ya se ha publicado.
        fs::write(status_path, "{broken").unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        OpenOptions::new()
            .append(true)
            .open(dir.path().join(A))
            .unwrap()
            .write_all(line(2).as_bytes())
            .unwrap();
        assert_eq!(
            timeout(Duration::from_millis(400), journal(&mut rx))
                .await
                .unwrap()
                .0,
            "Test2"
        );
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_initially_replays_only_latest_journal() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), line(1)).unwrap();
        fs::write(dir.path().join(B), line(2)).unwrap();
        let (mut rx, handle) = start(dir.path());
        assert_eq!(journal(&mut rx).await, ("Test2".into(), B.into(), 1, true));
        theme(&mut rx).await;
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_finds_journal_created_after_startup() {
        let dir = tempdir().unwrap();
        let (mut rx, handle) = start(dir.path());
        theme(&mut rx).await;
        fs::write(dir.path().join(A), line(1)).unwrap();
        assert_eq!(journal(&mut rx).await, ("Test1".into(), A.into(), 1, false));
        stop(rx, handle).await;
    }

    #[tokio::test]
    async fn watcher_initial_status_precedes_live_journal_even_during_rewrite() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(A), line(1)).unwrap();
        let status_path = dir.path().join("Status.json");
        fs::write(&status_path, "{partial").unwrap();
        let (mut rx, handle) = start(dir.path());
        assert_eq!(journal(&mut rx).await.0, "Test1");
        theme(&mut rx).await;
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = fs::write(
                status_path,
                r#"{"timestamp":"2026-10-17T18:00:00Z","Flags":1}"#,
            );
        });
        OpenOptions::new()
            .append(true)
            .open(dir.path().join(A))
            .unwrap()
            .write_all(line(2).as_bytes())
            .unwrap();
        loop {
            match next(&mut rx).await {
                SourceEvent::Status(_) => break,
                SourceEvent::Journal {
                    replaying: false, ..
                } => panic!("live journal preceded initial Status"),
                _ => {}
            }
        }
        writer.join().unwrap();
        assert_eq!(journal(&mut rx).await.0, "Test2");
        stop(rx, handle).await;
    }
}

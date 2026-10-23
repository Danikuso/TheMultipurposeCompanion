//! Reproducción de archivos de diario a velocidad configurable.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PlayOptions {
    pub from: PathBuf,
    pub to: PathBuf,
    pub speed: f64,
    pub status_dir: Option<PathBuf>,
}

fn companion_filename_for_event(event: &str) -> Option<&'static str> {
    match event {
        "Market" => Some("Market.json"),
        "Outfitting" => Some("Outfitting.json"),
        "Shipyard" => Some("Shipyard.json"),
        "Cargo" => Some("Cargo.json"),
        "NavRoute" | "FSDTarget" => Some("NavRoute.json"),
        "Backpack" => Some("Backpack.json"),
        "ShipLocker" => Some("ShipLocker.json"),
        "ModuleInfo" => Some("ModulesInfo.json"),
        _ => None,
    }
}

fn parse_timestamp(line: &str) -> Option<(DateTime<Utc>, String)> {
    let val: Value = serde_json::from_str(line).ok()?;
    let ts_str = val.get("timestamp")?.as_str()?;
    let ts = DateTime::parse_from_rfc3339(ts_str)
        .ok()?
        .with_timezone(&Utc);
    let event = val
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    Some((ts, event))
}

fn load_status_snapshots(status_dir: &Path) -> Vec<(DateTime<Utc>, String)> {
    let Ok(entries) = std::fs::read_dir(status_dir) else {
        return Vec::new();
    };
    let mut snapshots = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some((ts, _)) = parse_timestamp(&content) {
            snapshots.push((ts, content));
        }
    }
    snapshots.sort_by_key(|(ts, _)| *ts);
    snapshots
}

fn collect_source_files(from: &Path) -> anyhow::Result<(PathBuf, Vec<PathBuf>)> {
    if from.is_file() {
        let parent = from
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        return Ok((parent, vec![from.to_path_buf()]));
    }
    if from.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(from)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("Journal.") && n.ends_with(".log"))
            })
            .collect();
        files.sort();
        return Ok((from.to_path_buf(), files));
    }
    anyhow::bail!("source path does not exist: {}", from.display())
}

fn output_journal_name(src_file: &Path, first_ts: Option<DateTime<Utc>>, index: usize) -> String {
    if let Some(name) = src_file.file_name().and_then(|n| n.to_str())
        && name.starts_with("Journal.")
        && name.ends_with(".log")
    {
        return name.to_owned();
    }
    let date_part = first_ts
        .map(|ts| ts.format("%Y-%m-%dT%H%M%S").to_string())
        .unwrap_or_else(|| "2026-10-16T200000".to_owned());
    format!("Journal.{date_part}.{:02}.log", index + 1)
}

pub async fn play(opts: PlayOptions) -> anyhow::Result<()> {
    std::fs::create_dir_all(&opts.to)?;
    let (src_dir, files) = collect_source_files(&opts.from)?;
    let status_snapshots = opts
        .status_dir
        .as_deref()
        .map(load_status_snapshots)
        .unwrap_or_default();
    let speed = if opts.speed <= 0.0 { 10.0 } else { opts.speed };

    for (idx, file_path) in files.iter().enumerate() {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        let first_ts = lines
            .first()
            .and_then(|l| parse_timestamp(l).map(|(ts, _)| ts));
        let out_name = output_journal_name(file_path, first_ts, idx);
        let out_path = opts.to.join(out_name);
        let mut out_file = std::fs::File::create(&out_path)?;

        let mut prev_ts: Option<DateTime<Utc>> = None;
        for line in lines {
            if let Some((ts, event)) = parse_timestamp(line) {
                if let Some(prev) = prev_ts {
                    let delta = (ts - prev).num_milliseconds().max(0) as f64 / 1000.0;
                    let wait_secs = (delta / speed).min(2.0);
                    if wait_secs > 0.0 {
                        tokio::time::sleep(Duration::from_secs_f64(wait_secs)).await;
                    }
                }
                prev_ts = Some(ts);

                if let Some(companion) = companion_filename_for_event(&event) {
                    let src_companion = src_dir.join(companion);
                    if src_companion.exists() {
                        let _ = std::fs::copy(&src_companion, opts.to.join(companion));
                    }
                }

                if !status_snapshots.is_empty()
                    && let Some((_, snapshot_body)) =
                        status_snapshots.iter().rev().find(|(sts, _)| *sts <= ts)
                {
                    let _ = std::fs::write(opts.to.join("Status.json"), snapshot_body);
                }
            }

            writeln!(out_file, "{line}")?;
            out_file.flush()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::companion::CompanionKind;

    #[test]
    fn replay_maps_ship_locker_to_the_file_the_watcher_reads() {
        let name = companion_filename_for_event("ShipLocker").unwrap();
        assert_eq!(name, CompanionKind::ShipLocker.file_name());
        assert_eq!(
            CompanionKind::for_event("ShipLocker"),
            Some(CompanionKind::ShipLocker)
        );
    }
}

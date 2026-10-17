//! Reproductor y anonimizador de diarios de Elite Dangerous.
#![forbid(unsafe_code)]

pub mod anonymize;
pub mod replay;

pub use anonymize::{Pseudonyms, anonymize_line};
pub use replay::{PlayOptions, play};

use std::io::Write;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        anyhow::bail!(
            "usage: tmc-replay <play|anonymize> --from <path> --to <path> [--speed <n>] [--status <dir>]"
        );
    };

    let mut from: Option<PathBuf> = None;
    let mut to: Option<PathBuf> = None;
    let mut speed: f64 = 10.0;
    let mut status_dir: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => from = args.next().map(PathBuf::from),
            "--to" => to = args.next().map(PathBuf::from),
            "--speed" => {
                if let Some(val) = args.next() {
                    speed = val.parse::<f64>().unwrap_or(10.0);
                }
            }
            "--status" => status_dir = args.next().map(PathBuf::from),
            _ => {}
        }
    }

    let from = from.ok_or_else(|| anyhow::anyhow!("missing --from"))?;
    let to = to.ok_or_else(|| anyhow::anyhow!("missing --to"))?;

    match cmd.as_str() {
        "play" => {
            play(PlayOptions {
                from,
                to,
                speed,
                status_dir,
            })
            .await?;
        }
        "anonymize" => {
            let content = std::fs::read_to_string(&from)?;
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&to)?;
            let mut map = Pseudonyms::new();
            for line in content.lines() {
                if let Some(anon) = anonymize_line(line, &mut map) {
                    writeln!(out, "{anon}")?;
                }
            }
            out.flush()?;
        }
        other => anyhow::bail!("unknown subcommand: {other}"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::tempdir;

    #[test]
    fn anonymize_replaces_identity_fields_consistently() {
        let mut map = Pseudonyms::new();
        let line1 = r#"{"timestamp":"2026-10-16T20:00:01Z","event":"Commander","FID":"F9876543","Name":"RealCmdrName"}"#;
        let line2 = r#"{"timestamp":"2026-10-16T20:00:02Z","event":"LoadGame","FID":"F9876543","Commander":"RealCmdrName","Ship":"SideWinder","ShipID":1,"ShipName":"My Secret Ship","ShipIdent":"SECRET1","Credits":1000}"#;
        let line3 = r#"{"timestamp":"2026-10-16T20:00:03Z","event":"Loadout","Ship":"SideWinder","ShipID":1,"ShipName":"My Secret Ship","ShipIdent":"SECRET1","Modules":[]}"#;

        let out1 = anonymize_line(line1, &mut map).expect("Commander line kept");
        let out2 = anonymize_line(line2, &mut map).expect("LoadGame line kept");
        let out3 = anonymize_line(line3, &mut map).expect("Loadout line kept");

        assert!(!out1.contains("RealCmdrName"));
        assert!(!out1.contains("F9876543"));
        assert!(!out2.contains("RealCmdrName"));
        assert!(!out2.contains("F9876543"));
        assert!(!out2.contains("My Secret Ship"));
        assert!(!out2.contains("SECRET1"));

        let v1: Value = serde_json::from_str(&out1).unwrap();
        let v2: Value = serde_json::from_str(&out2).unwrap();
        let v3: Value = serde_json::from_str(&out3).unwrap();

        assert_eq!(v1["Name"], "CMDR TESTER");
        assert_eq!(v1["FID"], "F0000001");
        assert_eq!(v2["Commander"], "CMDR TESTER");
        assert_eq!(v2["FID"], "F0000001");
        assert_eq!(v2["ShipName"], v3["ShipName"]);
        assert_eq!(v2["ShipIdent"], v3["ShipIdent"]);
    }

    #[test]
    fn anonymize_drops_friends_and_chat_text() {
        let mut map = Pseudonyms::new();
        let friends = r#"{"timestamp":"2026-10-16T20:01:00Z","event":"Friends","Status":"Online","Name":"BestBuddy"}"#;
        assert!(anonymize_line(friends, &mut map).is_none());

        let recv = r#"{"timestamp":"2026-10-16T20:01:05Z","event":"ReceiveText","From":"SomePlayer","Message":"Hello secret world","Channel":"player"}"#;
        let send = r#"{"timestamp":"2026-10-16T20:01:06Z","event":"SendText","To":"SomePlayer","Message":"My private reply","Sent":true}"#;

        let out_recv = anonymize_line(recv, &mut map).expect("ReceiveText anonymized");
        let out_send = anonymize_line(send, &mut map).expect("SendText anonymized");

        assert!(!out_recv.contains("SomePlayer"));
        assert!(!out_recv.contains("Hello secret world"));
        assert!(!out_send.contains("SomePlayer"));
        assert!(!out_send.contains("My private reply"));

        let vr: Value = serde_json::from_str(&out_recv).unwrap();
        let vs: Value = serde_json::from_str(&out_send).unwrap();
        assert_eq!(vr["From"], vs["To"]);
        assert_eq!(vr["Message"], "[REDACTED]");
        assert_eq!(vs["Message"], "[REDACTED]");
    }

    #[tokio::test]
    async fn play_writes_all_lines_in_order_at_speed() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let fixture = include_str!("../../../tests/fixtures/journal/basic_session.log");
        let src_file = src_dir.path().join("Journal.2026-10-16T200000.01.log");
        std::fs::write(&src_file, fixture).unwrap();

        let nav_route = r#"{"timestamp":"2026-10-16T20:00:30Z","event":"NavRoute","Route":[]}"#;
        std::fs::write(src_dir.path().join("NavRoute.json"), nav_route).unwrap();

        play(PlayOptions {
            from: src_file,
            to: dst_dir.path().to_path_buf(),
            speed: 1000.0,
            status_dir: None,
        })
        .await
        .unwrap();

        let journals: Vec<_> = std::fs::read_dir(dst_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with("Journal.") && name.ends_with(".log")
            })
            .collect();
        assert_eq!(journals.len(), 1);

        let written = std::fs::read_to_string(journals[0].path()).unwrap();
        let expected_lines: Vec<_> = fixture.lines().filter(|l| !l.trim().is_empty()).collect();
        let actual_lines: Vec<_> = written.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(actual_lines, expected_lines);
        assert!(dst_dir.path().join("NavRoute.json").exists());
    }
}

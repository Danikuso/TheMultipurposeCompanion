//! Composición del núcleo, configuración, logs y portapapeles de TMC.
#![forbid(unsafe_code)]

pub mod clipboard;
pub mod config;
pub mod logging;
pub mod wiring;

pub use clipboard::{AppActions, Clipboard};
pub use config::{Config, ServerSection, config_path, data_dir};
pub use logging::init_logging;
pub use wiring::{Running, start};

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode, header};
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tempfile::tempdir;
    use tmc_server::{ActionError, ActionHost, ServerConfig, build_router};
    use tower::ServiceExt;

    #[test]
    fn config_roundtrip_and_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = Config::load_from_or_default(&path);
        assert_eq!(cfg.server.port, 47320);
        assert_eq!(cfg.language, "es");
        assert!(cfg.auto_copy_next_system);
        assert!(cfg.alerts_enabled);
        assert!(!cfg.server.lan);
        assert!(path.exists());

        let mut modified = cfg.clone();
        modified.language = "en".into();
        modified.server.port = 48000;
        modified.save_to(&path).unwrap();

        let reloaded = Config::load_from_or_default(&path);
        assert_eq!(reloaded.language, "en");
        assert_eq!(reloaded.server.port, 48000);
    }

    #[test]
    fn invalid_config_is_backed_up_and_defaulted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "invalid = [toml syntax").unwrap();

        let cfg = Config::load_from_or_default(&path);
        assert_eq!(cfg.server.port, 47320);
        assert_eq!(cfg.language, "es");
        assert!(dir.path().join("config.toml.bak").exists());
    }

    #[test]
    fn lan_token_is_128_bit_hex_and_stable() {
        let mut cfg = Config::default();
        let first = cfg.ensure_lan_token();
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        let second = cfg.ensure_lan_token();
        assert_eq!(first, second);
    }

    #[test]
    fn copy_next_system_action_without_route_fails() {
        let actions = AppActions::new();
        let mut snapshot = BTreeMap::new();
        snapshot.insert("core.route".into(), json!({ "next_system": null }));
        let err = actions
            .run_action("clipboard.copy-next-system", &snapshot)
            .unwrap_err();
        assert_eq!(err, ActionError::Failed("no-next-system".into()));
    }

    #[tokio::test]
    async fn corrupt_database_is_renamed_and_rebuilt() {
        let jdir = tempdir().unwrap();
        let ddir = tempdir().unwrap();
        let fixture = include_str!("../../../tests/fixtures/journal/basic_session.log");
        std::fs::write(
            jdir.path().join("Journal.2026-10-16T200000.01.log"),
            fixture,
        )
        .unwrap();
        let db_path = ddir.path().join("tmc.db");
        std::fs::write(&db_path, b"this is not a valid sqlite file").unwrap();

        let cfg = Config {
            journal_dir: Some(jdir.path().to_path_buf()),
            data_dir: Some(ddir.path().to_path_buf()),
            server: ServerSection {
                port: 0,
                ..Default::default()
            },
            auto_copy_next_system: false,
            ..Default::default()
        };

        let running = start(cfg).await.unwrap();
        let entries: Vec<_> = std::fs::read_dir(ddir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            entries
                .iter()
                .any(|name| name.starts_with("tmc.db.corrupt-")),
            "expected corrupt backup in {entries:?}"
        );

        // Wait until history import populates events in the rebuilt database.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let rows = running
                    .store
                    .query_events(&tmc_store::EventQuery {
                        limit: 100,
                        ..Default::default()
                    })
                    .unwrap();
                if !rows.is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        running.server.shutdown();
    }

    #[tokio::test]
    async fn start_end_to_end_with_temp_journal_dir() {
        let jdir = tempdir().unwrap();
        let ddir = tempdir().unwrap();
        let fixture = include_str!("../../../tests/fixtures/journal/basic_session.log");
        std::fs::write(
            jdir.path().join("Journal.2026-10-16T200000.01.log"),
            fixture,
        )
        .unwrap();
        let status_fixture = include_str!("../../../tests/fixtures/status/docked.json");
        std::fs::write(jdir.path().join("Status.json"), status_fixture).unwrap();

        let cfg = Config {
            journal_dir: Some(jdir.path().to_path_buf()),
            data_dir: Some(ddir.path().to_path_buf()),
            server: ServerSection {
                port: 0,
                ..Default::default()
            },
            auto_copy_next_system: false,
            ..Default::default()
        };

        let running = start(cfg).await.unwrap();

        // Wait for watcher initial replay to populate core.location.system
        let snap = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let s = running.core.snapshot().await;
                if s.get("core.location")
                    .and_then(|l| l.get("system"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|sys| sys == "Sampleton")
                {
                    break s;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();

        assert_eq!(snap["core.location"]["system"], "Sampleton");

        let router = build_router(
            ServerConfig {
                port: running.server.port,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            "127.0.0.1".into(),
            running.core.clone(),
            running.store.clone(),
            std::sync::Arc::new(AppActions::new()),
        );
        let mut req = Request::builder()
            .uri("/api/health")
            .header(header::HOST, format!("127.0.0.1:{}", running.server.port))
            .body(Body::empty())
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:55000".parse::<SocketAddr>().unwrap(),
        ));
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let health: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(health["ok"], true);

        running.server.shutdown();
    }
}

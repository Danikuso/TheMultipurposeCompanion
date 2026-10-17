//! Composición del núcleo, configuración, logs y portapapeles de TMC.
#![forbid(unsafe_code)]

pub mod clipboard;
pub mod config;
pub mod logging;
pub mod net_service;
pub mod wiring;

pub use clipboard::{AppActions, Clipboard};
pub use config::{Config, ServerSection, config_path, data_dir};
pub use logging::init_logging;
pub use net_service::{NetServiceHandle, spawn_net_service};
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
    fn old_config_without_f1_fields_loads_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "language = \"en\"
auto_copy_next_system = false
alerts_enabled = true
[server]
port = 48001
lan = false
",
        )
        .unwrap();
        let cfg = Config::load_from_or_default(&path);
        assert!(!dir.path().join("config.toml.bak").exists());
        assert_eq!(cfg.language, "en");
        assert_eq!(cfg.server.port, 48001);
        assert!(cfg.online_services);
        assert_eq!(cfg.map_value_threshold, 500_000);
        assert_eq!(cfg.default_neutron_efficiency, 60);
        assert!(!cfg.auto_copy_next_system);
        let settings = cfg.module_settings();
        assert_eq!(settings.auto_copy_next_system, cfg.auto_copy_next_system);
        assert_eq!(settings.map_value_threshold, 500_000);
        assert!(settings.alerts_enabled && settings.online_services);
    }

    struct SettingsProbe(Option<tmc_model::settings::ModuleSettings>);
    impl tmc_core::Module for SettingsProbe {
        fn id(&self) -> &'static str {
            "probe"
        }
        fn on_event(
            &mut self,
            ev: &tmc_model::source::SourceEvent,
            _: &tmc_model::state::GameState,
            _: &mut tmc_core::ModuleCtx,
        ) -> tmc_core::TopicSet {
            if let tmc_model::source::SourceEvent::Settings(s) = ev {
                self.0 = Some(s.clone());
            }
            ["probe.settings".into()].into()
        }
        fn topic_value(&self, _: &str) -> Option<Value> {
            Some(serde_json::to_value(&self.0).unwrap())
        }
        fn topics(&self) -> Vec<String> {
            vec!["probe.settings".into()]
        }
    }

    fn runtime_actions(
        dir: &std::path::Path,
    ) -> (
        AppActions,
        tmc_core::CoreHandle,
        std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        let core = tmc_core::spawn_core(
            vec![Box::new(SettingsProbe(None))],
            None,
            Duration::from_millis(100),
        );
        let online = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (events, _rx) = tokio::sync::mpsc::channel(8);
        let net = spawn_net_service(
            std::sync::Arc::new(tmc_net::NetClient::new(std::sync::Arc::new(
                tmc_net::ReqwestTransport::new().unwrap(),
            ))),
            tmc_core::Store::open_in_memory().unwrap(),
            events,
            online.clone(),
        );
        let actions =
            AppActions::with_runtime(Config::default(), core.events.clone(), online.clone(), net)
                .with_save_path(dir.join("config.toml"));
        (actions, core, online)
    }

    #[tokio::test]
    async fn put_config_propagates_settings_to_core() {
        let dir = tempdir().unwrap();
        let (actions, core, online) = runtime_actions(dir.path());
        let mut cfg = actions.get_config();
        cfg["online_services"] = json!(false);
        cfg["map_value_threshold"] = json!(750_000);
        let saved = actions.put_config(cfg).unwrap();
        assert_eq!(saved["online_services"], false);
        assert!(!online.load(std::sync::atomic::Ordering::Relaxed));
        let snap = core.snapshot().await;
        assert_eq!(snap["probe.settings"]["online_services"], false);
        assert_eq!(snap["probe.settings"]["map_value_threshold"], 750_000);
        assert_eq!(snap["probe.settings"]["alerts_enabled"], true);
        // Con servicios en línea desactivados, el autocompletado no consulta nada.
        let out = actions.autocomplete("Sol", &snap).await.unwrap();
        assert_eq!(out, json!({ "items": [], "page": 0, "more": false }));
        let reloaded = Config::load_from_or_default(&dir.path().join("config.toml"));
        assert!(!reloaded.online_services);
    }

    #[tokio::test]
    async fn autocomplete_is_empty_for_legacy_galaxy() {
        let dir = tempdir().unwrap();
        let (actions, _core, _online) = runtime_actions(dir.path());
        let mut snap = BTreeMap::new();
        snap.insert("core.game".into(), json!({ "galaxy": "Legacy" }));
        let out = actions.autocomplete("Sol", &snap).await.unwrap();
        assert_eq!(out, json!({ "items": [], "page": 0, "more": false }));
    }

    #[test]
    fn module_action_is_forwarded_as_source_event() {
        let (events, mut rx) = tokio::sync::mpsc::channel(1);
        let (net_events, _nrx) = tokio::sync::mpsc::channel(1);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let online = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let net = spawn_net_service(
            std::sync::Arc::new(tmc_net::NetClient::new(std::sync::Arc::new(
                tmc_net::ReqwestTransport::new().unwrap(),
            ))),
            tmc_core::Store::open_in_memory().unwrap(),
            net_events,
            online.clone(),
        );
        let actions = AppActions::with_runtime(Config::default(), events, online, net);
        let snap = BTreeMap::new();
        let args = json!({ "to": "X" });
        let out = actions
            .run_action_with("navigation.plan", &args, &snap)
            .unwrap();
        assert_eq!(out, json!({ "accepted": true }));
        match rx.try_recv().unwrap() {
            tmc_model::source::SourceEvent::Action { name, args } => {
                assert_eq!(name, "navigation.plan");
                assert_eq!(args, json!({ "to": "X" }));
            }
            other => panic!("unexpected event {other:?}"),
        }
        // Cola de capacidad 1 llena: la segunda acción falla sin bloquear.
        actions
            .run_action_with("exploration.refresh", &json!(null), &snap)
            .unwrap();
        let err = actions
            .run_action_with("exobiology.x", &json!(null), &snap)
            .unwrap_err();
        assert_eq!(err, ActionError::Failed("busy".into()));
        // Prefijo desconocido: no existe.
        assert_eq!(
            actions
                .run_action_with("other.thing", &json!(null), &snap)
                .unwrap_err(),
            ActionError::NotFound
        );
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

    #[test]
    fn config_host_roundtrips_and_validates_language() {
        let actions = AppActions::with_config(Config::default());
        let current = actions.get_config();
        assert_eq!(current["language"], "es");

        let mut invalid = current.clone();
        invalid["language"] = json!("xx");
        assert!(actions.put_config(invalid).is_err());

        let mut valid = current;
        valid["language"] = json!("en");
        let saved = actions.put_config(valid).unwrap();
        assert_eq!(saved["language"], "en");
        assert_eq!(actions.get_config()["language"], "en");
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

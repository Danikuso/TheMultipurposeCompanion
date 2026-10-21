//! Composición del núcleo, configuración, logs y portapapeles de TMC.
#![forbid(unsafe_code)]

pub mod clipboard;
pub mod config;
pub mod eddn_service;
mod eddn_wiring;
pub mod logging;
pub mod net_service;
pub mod wiring;

pub use clipboard::{AppActions, Clipboard};
pub use config::{Config, ServerSection, config_path, data_dir};
pub use logging::{LoggingGuards, init_logging};
pub use net_service::{NetServiceHandle, spawn_net_service};
pub use wiring::{Running, start, start_custom, start_full, start_with_transport};

#[cfg(all(test, debug_assertions))]
mod e2e_tests;

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

    #[test]
    fn auto_companion_defaults_true_and_old_config_keeps_it() {
        assert!(Config::default().auto_companion);
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "language = \"en\"
",
        )
        .unwrap();
        let cfg = Config::load_from_or_default(&path);
        assert!(cfg.auto_companion);
        let mut off = cfg.clone();
        off.auto_companion = false;
        off.save_to(&path).unwrap();
        assert!(!Config::load_from_or_default(&path).auto_companion);
    }

    #[tokio::test]
    async fn put_config_persists_auto_companion_and_rejects_non_boolean() {
        let dir = tempdir().unwrap();
        let (actions, _core, _online) = runtime_actions(dir.path());
        let before = actions.get_config();
        assert_eq!(before["auto_companion"], true);
        let mut bad = before.clone();
        bad["auto_companion"] = json!("no");
        assert!(matches!(
            actions.put_config(bad),
            Err(ActionError::Failed(_))
        ));
        assert_eq!(actions.get_config(), before);
        let mut off = before.clone();
        off["auto_companion"] = json!(false);
        assert_eq!(actions.put_config(off).unwrap()["auto_companion"], false);
        assert_eq!(actions.get_config()["auto_companion"], false);
    }

    #[test]
    fn f1_config_without_mining_fields_loads_with_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "language = \"en\"
online_services = true
map_value_threshold = 600000
default_neutron_efficiency = 60
",
        )
        .unwrap();
        let cfg = Config::load_from_or_default(&path);
        assert!(!dir.path().join("config.toml.bak").exists());
        assert_eq!(cfg.map_value_threshold, 600_000);
        assert_eq!(cfg.mining_threshold_cr, 1_000_000);
        assert_eq!(cfg.sell_search_radius_ly, 100);
        assert_eq!(cfg.mining_interest.len(), 11);
        assert_eq!(cfg.module_settings().mining_interest[0], "platinum");
    }

    #[test]
    fn invalid_f2_fields_in_file_fall_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            format!(
                "language = \"en\"
sell_search_radius_ly = 9999
mining_threshold_cr = 5000000000
mining_interest = [{}]
",
                vec!["\"a\""; 31].join(",")
            ),
        )
        .unwrap();
        let cfg = Config::load_from_or_default(&path);
        assert_eq!(cfg.language, "en");
        assert_eq!(cfg.sell_search_radius_ly, 100);
        assert_eq!(cfg.mining_threshold_cr, 1_000_000);
        assert_eq!(cfg.mining_interest.len(), 11);
    }

    #[test]
    fn module_settings_payload_without_mining_fields_deserializes() {
        let s: tmc_model::settings::ModuleSettings = serde_json::from_value(json!({
            "alerts_enabled": false,
            "online_services": true,
            "auto_copy_next_system": true,
            "map_value_threshold": 1,
            "default_neutron_efficiency": 70
        }))
        .unwrap();
        assert!(!s.alerts_enabled);
        assert_eq!(s.default_neutron_efficiency, 70);
        assert_eq!(s.mining_threshold_cr, 1_000_000);
        assert_eq!(s.sell_search_radius_ly, 100);
        assert_eq!(s.mining_interest.len(), 11);
    }

    #[tokio::test]
    async fn put_config_rejects_invalid_mining_values_and_keeps_config() {
        let dir = tempdir().unwrap();
        let (actions, _core, _online) = runtime_actions(dir.path());
        let before = actions.get_config();
        for (field, value) in [
            ("sell_search_radius_ly", json!(600)),
            ("sell_search_radius_ly", json!(9)),
            ("mining_threshold_cr", json!(1_000_000_001u64)),
            ("mining_interest", json!(vec!["a"; 31])),
            ("mining_interest", json!(["x".repeat(41)])),
        ] {
            let mut cfg = before.clone();
            cfg[field] = value;
            assert!(
                matches!(actions.put_config(cfg), Err(ActionError::Failed(_))),
                "{field}"
            );
        }
        assert_eq!(actions.get_config(), before);
        let mut ok = before.clone();
        ok["sell_search_radius_ly"] = json!(500);
        assert_eq!(
            actions.put_config(ok).unwrap()["sell_search_radius_ly"],
            500
        );
    }

    #[test]
    fn f2_config_without_f3_fields_loads_with_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "language = \"en\"
sell_search_radius_ly = 120
mining_threshold_cr = 2000000
",
        )
        .unwrap();
        let cfg = Config::load_from_or_default(&path);
        assert_eq!(cfg.sell_search_radius_ly, 120);
        assert_eq!(cfg.eddn_radius_ly, 200);
        assert_eq!(cfg.tourism_reference_cr_ly, 100_000);
    }

    #[tokio::test]
    async fn put_config_rejects_eddn_radius_out_of_range() {
        let dir = tempdir().unwrap();
        let (actions, _core, _online) = runtime_actions(dir.path());
        let before = actions.get_config();
        for (field, value) in [
            ("eddn_radius_ly", json!(600)),
            ("eddn_radius_ly", json!(19)),
            ("tourism_reference_cr_ly", json!(999)),
            ("tourism_reference_cr_ly", json!(10_000_001u64)),
        ] {
            let mut cfg = before.clone();
            cfg[field] = value;
            assert!(
                matches!(actions.put_config(cfg), Err(ActionError::Failed(_))),
                "{field}"
            );
        }
        assert_eq!(actions.get_config(), before);
        let mut ok = before.clone();
        ok["eddn_radius_ly"] = json!(500);
        assert_eq!(actions.put_config(ok).unwrap()["eddn_radius_ly"], 500);
    }

    #[test]
    fn f3_config_without_f4_fields_loads_with_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "language = \"en\"
eddn_radius_ly = 250
tourism_reference_cr_ly = 200000
",
        )
        .unwrap();
        let cfg = Config::load_from_or_default(&path);
        assert_eq!(cfg.eddn_radius_ly, 250);
        assert_eq!(cfg.engineering_rolls, [1, 1, 1, 1, 4]);
        assert_eq!(cfg.module_settings().engineering_rolls, [1, 1, 1, 1, 4]);
    }

    #[test]
    fn config_snapshot_includes_engineering_rolls() {
        assert_eq!(
            Config::default().diagnostic_snapshot()["engineering_rolls"],
            json!([1, 1, 1, 1, 4])
        );
    }

    #[tokio::test]
    async fn put_config_rejects_rolls_out_of_range() {
        let dir = tempdir().unwrap();
        let (actions, _core, _online) = runtime_actions(dir.path());
        let before = actions.get_config();
        for rolls in [json!([1, 1, 1, 1, 0]), json!([1, 1, 1, 1, 11])] {
            let mut cfg = before.clone();
            cfg["engineering_rolls"] = rolls;
            match actions.put_config(cfg) {
                Err(ActionError::Failed(m)) => assert_eq!(m, "invalid-engineering-rolls"),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(actions.get_config(), before);
        let mut ok = before.clone();
        ok["engineering_rolls"] = json!([2, 3, 4, 5, 10]);
        assert_eq!(
            actions.put_config(ok).unwrap()["engineering_rolls"],
            json!([2, 3, 4, 5, 10])
        );
    }

    #[tokio::test]
    async fn put_config_normalizes_interest_list() {
        let dir = tempdir().unwrap();
        let (actions, core, _online) = runtime_actions(dir.path());
        let mut cfg = actions.get_config();
        cfg["mining_interest"] = json!(["Platinum", "$painite_name;"]);
        let saved = actions.put_config(cfg).unwrap();
        assert_eq!(saved["mining_interest"], json!(["platinum", "painite"]));
        let snap = core.snapshot().await;
        assert_eq!(
            snap["probe.settings"]["mining_interest"],
            json!(["platinum", "painite"])
        );
        let reloaded = Config::load_from_or_default(&dir.path().join("config.toml"));
        assert_eq!(reloaded.mining_interest, vec!["platinum", "painite"]);
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

    struct ActionProbe(Vec<String>);
    impl tmc_core::Module for ActionProbe {
        fn id(&self) -> &'static str {
            "actionprobe"
        }
        fn on_event(
            &mut self,
            ev: &tmc_model::source::SourceEvent,
            _: &tmc_model::state::GameState,
            _: &mut tmc_core::ModuleCtx,
        ) -> tmc_core::TopicSet {
            if let tmc_model::source::SourceEvent::Action { name, .. } = ev {
                self.0.push(name.clone());
            }
            ["actionprobe.actions".into()].into()
        }
        fn topic_value(&self, _: &str) -> Option<Value> {
            Some(json!(self.0))
        }
        fn topics(&self) -> Vec<String> {
            vec!["actionprobe.actions".into()]
        }
    }

    #[tokio::test]
    async fn module_actions_for_combat_and_mining_reach_core() {
        let dir = tempdir().unwrap();
        let core = tmc_core::spawn_core(
            vec![Box::new(ActionProbe(Vec::new()))],
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
        let actions = AppActions::with_runtime(Config::default(), core.events.clone(), online, net)
            .with_save_path(dir.path().join("config.toml"));
        let snap = BTreeMap::new();
        for name in [
            "combat.reset_session",
            "mining.reset_session",
            "trade.reset_session",
            "trade.route_request",
            "trade.search",
            "missions.dismiss_alert",
            "missions.to_route",
            "missions.reset_session",
        ] {
            let out = actions.run_action_with(name, &json!({}), &snap);
            assert!(out.is_ok(), "{name}: {out:?}");
        }
        let mut seen = Value::Null;
        for _ in 0..50 {
            seen = core.snapshot().await["actionprobe.actions"].clone();
            if seen.as_array().is_some_and(|a| a.len() == 8) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(
            seen,
            json!([
                "combat.reset_session",
                "mining.reset_session",
                "trade.reset_session",
                "trade.route_request",
                "trade.search",
                "missions.dismiss_alert",
                "missions.to_route",
                "missions.reset_session"
            ])
        );
    }

    #[test]
    fn trade_route_copy_target_reaches_clipboard() {
        let actions = AppActions::new();
        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "trade.route".into(),
            json!({ "copy_target": "C", "copy_seq": 2 }),
        );
        let out = actions
            .run_action("clipboard.copy-next-system", &snapshot)
            .unwrap();
        assert_eq!(out["system"], "C");
        assert_eq!(actions.clipboard().last_copied().as_deref(), Some("C"));
        // Sin destino (ajuste desactivado o ruta acabada) no se copia nada.
        let actions = AppActions::new();
        snapshot.insert(
            "trade.route".into(),
            json!({ "copy_target": null, "copy_seq": 2 }),
        );
        assert!(
            actions
                .run_action("clipboard.copy-next-system", &snapshot)
                .is_err()
        );
        assert_eq!(actions.clipboard().last_copied(), None);
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
    fn manual_copy_prefers_active_plan_next_system() {
        let actions = AppActions::new();
        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "navigation.route".into(),
            json!({ "source": "plan", "copy_target": "B", "copy_seq": 0 }),
        );
        snapshot.insert(
            "core.route".into(),
            json!({ "next_system": "NavRouteNext" }),
        );
        let out = actions
            .run_action("clipboard.copy-next-system", &snapshot)
            .unwrap();
        assert_eq!(out["system"], "B");
        assert_eq!(actions.clipboard().last_copied().as_deref(), Some("B"));
    }

    #[test]
    fn manual_copy_falls_back_to_core_route() {
        let actions = AppActions::new();
        let mut snapshot = BTreeMap::new();
        snapshot.insert(
            "navigation.route".into(),
            json!({ "copy_target": null, "copy_seq": 0 }),
        );
        snapshot.insert("core.route".into(), json!({ "next_system": "Z" }));
        let out = actions
            .run_action("clipboard.copy-next-system", &snapshot)
            .unwrap();
        assert_eq!(out["system"], "Z");
        assert_eq!(actions.clipboard().last_copied().as_deref(), Some("Z"));
    }

    /// Plan PA -> PB -> PC -> PD persistido, sesión en PA y app arrancada.
    async fn plan_fixture() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        std::path::PathBuf,
        Running,
    ) {
        use tmc_model::net::PlannedHop;
        use tmc_modules::navigation::route::{ActivePlan, PlanMode};

        let jdir = tempdir().unwrap();
        let ddir = tempdir().unwrap();
        let journal = jdir.path().join("Journal.2026-10-18T100000.01.log");
        let head = concat!(
            r#"{"timestamp":"2026-10-18T10:00:00Z","event":"Fileheader","part":1,"language":"English/UK","Odyssey":true,"gameversion":"4.1.0.0","build":"r1"}"#,
            "
",
            r#"{"timestamp":"2026-10-18T10:00:01Z","event":"Commander","FID":"F0000001","Name":"TESTER"}"#,
            "
",
            r#"{"timestamp":"2026-10-18T10:00:02Z","event":"LoadGame","FID":"F0000001","Commander":"TESTER","Ship":"python","ShipID":7,"FuelLevel":20.0,"FuelCapacity":32.0,"GameMode":"Solo","Credits":1,"Loan":0}"#,
            "
",
            r#"{"timestamp":"2026-10-18T10:00:03Z","event":"Location","StarSystem":"PA","SystemAddress":101,"StarPos":[0,0,0],"Docked":false}"#,
            "
",
        );
        std::fs::write(&journal, head).unwrap();

        // Plan aceptado y persistido antes de arrancar: PA -> PB -> PC -> PD.
        {
            let store = tmc_store::Store::open(&ddir.path().join("tmc.db")).unwrap();
            for module in tmc_modules::all_modules() {
                store.apply_migrations(module.migrations()).unwrap();
            }
            let hops = [("PA", 101u64), ("PB", 102), ("PC", 103), ("PD", 104)]
                .iter()
                .enumerate()
                .map(|(i, (n, id))| PlannedHop {
                    name: (*n).into(),
                    id64: Some(*id),
                    star_pos: None,
                    distance_ly: 10.0 * i as f64,
                    jumps: 1,
                    neutron: false,
                    refuel: None,
                    star_class: None,
                })
                .collect();
            let plan = ActivePlan {
                created_at: "2026-10-18T09:00:00Z".parse().unwrap(),
                origin: "PA".into(),
                destination: "PD".into(),
                mode: PlanMode::Neutron,
                params: json!({}),
                hops,
                current_index: 0,
                off_route: false,
            };
            tmc_modules::navigation::persist::save_plan("TESTER".into(), plan)(&store).unwrap();
        }

        let cfg = Config {
            journal_dir: Some(jdir.path().to_path_buf()),
            data_dir: Some(ddir.path().to_path_buf()),
            server: ServerSection {
                port: 0,
                ..Default::default()
            },
            auto_copy_next_system: true,
            online_services: false,
            ..Default::default()
        };
        let running = start(cfg).await.unwrap();

        // El plan se carga con el comandante y el replay no copia nada.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let s = running.core.snapshot().await;
                if s.get("navigation.route")
                    .is_some_and(|r| r["source"] == "plan" && r["copy_target"] == "PB")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(running.clipboard.last_copied(), None);
        (jdir, ddir, journal, running)
    }

    fn append_jumps(journal: &std::path::Path, jumps: &[(&str, u64)], minute: u32) {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(journal)
            .unwrap();
        for (i, (name, id)) in jumps.iter().enumerate() {
            writeln!(
                file,
                r#"{{"timestamp":"2026-10-18T10:{minute:02}:0{i}Z","event":"FSDJump","StarSystem":"{name}","SystemAddress":{id},"StarPos":[10,0,0],"JumpDist":10.0}}"#
            )
            .unwrap();
        }
    }

    async fn route_topic(running: &Running) -> Value {
        running.core.snapshot().await["navigation.route"].clone()
    }

    async fn wait_copied(running: &Running, expected: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while running.clipboard.last_copied().as_deref() != Some(expected) {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("el portapapeles debe acabar con {expected}"));
    }

    fn drain_auto_copy_alerts(
        rx: &mut tokio::sync::broadcast::Receiver<tmc_core::CoreMessage>,
    ) -> Vec<Value> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let tmc_core::CoreMessage::Alert(a) = msg
                && a.key == "nav-auto-copied"
            {
                out.push(a.args["system"].clone());
            }
        }
        out
    }

    async fn send_auto_copy(running: &Running, on: bool) {
        let settings = tmc_model::settings::ModuleSettings {
            online_services: false,
            auto_copy_next_system: on,
            ..Default::default()
        };
        running
            .core
            .events
            .send(tmc_model::source::SourceEvent::Settings(settings))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn auto_copy_follows_plan_after_each_jump() {
        let (_j, _d, journal, running) = plan_fixture().await;
        let mut rx = running.core.subscribe();
        append_jumps(&journal, &[("PB", 102), ("PC", 103)], 1);
        wait_copied(&running, "PD").await;
        assert_eq!(drain_auto_copy_alerts(&mut rx).last(), Some(&json!("PD")));
        running.server.shutdown();
    }

    #[tokio::test]
    async fn auto_copy_setting_toggles_live_through_settings_events() {
        let (_j, _d, journal, running) = plan_fixture().await;
        let mut rx = running.core.subscribe();
        send_auto_copy(&running, false).await;
        // Con el ajuste apagado: ni copy_seq, ni alerta, ni copia; copy_target sigue.
        append_jumps(&journal, &[("PB", 102)], 1);
        tokio::time::timeout(Duration::from_secs(5), async {
            while route_topic(&running).await["current"]["name"] != "PB" {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        let route = route_topic(&running).await;
        assert_eq!(route["copy_seq"], 0);
        assert_eq!(route["copy_target"], "PC");
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(running.clipboard.last_copied(), None);
        assert!(drain_auto_copy_alerts(&mut rx).is_empty());
        // Encendido: el siguiente salto en vivo copia y avisa.
        send_auto_copy(&running, true).await;
        append_jumps(&journal, &[("PC", 103)], 2);
        wait_copied(&running, "PD").await;
        assert_eq!(route_topic(&running).await["copy_seq"], 1);
        assert_eq!(drain_auto_copy_alerts(&mut rx), vec![json!("PD")]);
        running.server.shutdown();
    }

    #[tokio::test]
    async fn exhausted_plan_neither_copies_nor_alerts() {
        let (_j, _d, journal, running) = plan_fixture().await;
        let mut rx = running.core.subscribe();
        // Se llega a PC sin copiar (ajuste apagado) y luego se salta al último hop.
        send_auto_copy(&running, false).await;
        append_jumps(&journal, &[("PB", 102), ("PC", 103)], 1);
        tokio::time::timeout(Duration::from_secs(5), async {
            while route_topic(&running).await["current"]["name"] != "PC" {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        send_auto_copy(&running, true).await;
        append_jumps(&journal, &[("PD", 104)], 2);
        tokio::time::timeout(Duration::from_secs(5), async {
            while route_topic(&running).await["current"]["name"] != "PD" {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        let route = route_topic(&running).await;
        assert!(route["copy_target"].is_null());
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(running.clipboard.last_copied(), None);
        assert!(drain_auto_copy_alerts(&mut rx).is_empty());
        running.server.shutdown();
    }

    #[test]
    fn config_host_roundtrips_and_validates_language() {
        let directory = tempdir().unwrap();
        let save_path = directory.path().join("config.toml");
        let actions = AppActions::with_config(Config::default()).with_save_path(save_path.clone());
        let current = actions.get_config();
        assert_eq!(current["language"], "es");

        let mut invalid = current.clone();
        invalid["language"] = json!("xx");
        assert!(actions.put_config(invalid).is_err());
        assert!(!save_path.exists());

        let mut valid = current;
        valid["language"] = json!("en");
        let saved = actions.put_config(valid).unwrap();
        assert_eq!(saved["language"], "en");
        assert_eq!(actions.get_config()["language"], "en");
        let persisted: Config =
            toml::from_str(&std::fs::read_to_string(&save_path).unwrap()).unwrap();
        assert_eq!(persisted.language, "en");
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

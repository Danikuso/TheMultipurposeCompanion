//! Arranque y cableado de todos los subsistemas.

use crate::{AppActions, Clipboard, Config, data_dir};
use chrono::Utc;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tmc_core::{CoreHandle, CoreMessage, spawn_core};
use tmc_journal::{WatchOptions, is_game_running, resolve_journal_dir, spawn_watcher};
use tmc_model::source::SourceEvent;
use tmc_modules::all_modules;
use tmc_server::{ServerConfig, ServerHandle, serve};
use tmc_store::{EventWriter, ImportOptions, Store, import_history};

pub struct Running {
    pub server: ServerHandle,
    pub core: CoreHandle,
    pub store: Store,
}

fn open_or_rebuild_store(db_dir: &Path) -> anyhow::Result<Store> {
    std::fs::create_dir_all(db_dir)?;
    let db_path = db_dir.join("tmc.db");
    match Store::open(&db_path) {
        Ok(store) => Ok(store),
        Err(err) => {
            tracing::warn!(%err, path = %db_path.display(), "corrupt SQLite database; renaming and rebuilding");
            let timestamp = Utc::now().format("%Y%m%d%H%M%S");
            let corrupt_path = db_dir.join(format!("tmc.db.corrupt-{timestamp}"));
            let _ = std::fs::rename(&db_path, &corrupt_path);
            let _ = std::fs::remove_file(db_dir.join("tmc.db-wal"));
            let _ = std::fs::remove_file(db_dir.join("tmc.db-shm"));
            Ok(Store::open(&db_path)?)
        }
    }
}

pub async fn start(mut config: Config) -> anyhow::Result<Running> {
    let journal_dir = resolve_journal_dir(config.journal_dir.as_deref())?;
    let db_dir = config.data_dir.clone().unwrap_or_else(data_dir);
    let store = open_or_rebuild_store(&db_dir)?;
    store.kv_set("locale", &config.language);

    let writer = EventWriter::spawn(&store);
    let modules = if config.alerts_enabled {
        all_modules()
    } else {
        Vec::new()
    };
    let core = spawn_core(modules, Some(writer), Duration::from_millis(100));

    let watch_opts = WatchOptions {
        journal_dir: journal_dir.clone(),
        ..Default::default()
    };
    spawn_watcher(watch_opts, core.events.clone());

    let throttle = Arc::new(AtomicBool::new(is_game_running()));
    let import_throttle = throttle.clone();
    let import_store = store.clone();
    let import_dir = journal_dir.clone();
    let import_tx = core.events.clone();
    std::thread::spawn(move || {
        let opts = ImportOptions {
            throttle: import_throttle,
        };
        match import_history(&import_store, &import_dir, &opts, &mut |done, total| {
            let _ = import_tx.blocking_send(SourceEvent::ImportProgress { done, total });
        }) {
            Ok(report) => {
                tracing::info!(
                    files = report.files,
                    events = report.events,
                    skipped_lines = report.skipped_lines,
                    "history import completed"
                );
            }
            Err(err) => {
                tracing::warn!(%err, "background history import failed");
            }
        }
    });

    if config.auto_copy_next_system {
        let mut rx = core.subscribe();
        let clipboard = Clipboard::new();
        let throttle_flag = throttle.clone();
        tokio::spawn(async move {
            let mut last_next: Option<String> = None;
            let mut initialized = false;
            while let Ok(msg) = rx.recv().await {
                if let CoreMessage::Update { topics, .. } = msg {
                    if let Some(game) = topics.get("core.game")
                        && let Some(running) = game.get("running").and_then(|v| v.as_bool())
                    {
                        throttle_flag.store(running, Ordering::Relaxed);
                    }
                    if let Some(route) = topics.get("core.route") {
                        let current = route
                            .get("next_system")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned);
                        if initialized
                            && current != last_next
                            && let Some(ref sys) = current
                        {
                            let _ = clipboard.set_text(sys);
                        }
                        last_next = current;
                        initialized = true;
                    }
                }
            }
        });
    }

    let token = config.ensure_lan_token();
    let server_cfg = ServerConfig {
        port: config.server.port,
        lan: config.server.lan,
        token,
        ui_dir: config.ui_dir.clone(),
    };
    let server = serve(
        server_cfg,
        core.clone(),
        store.clone(),
        Arc::new(AppActions::new()),
    )
    .await?;

    Ok(Running {
        server,
        core,
        store,
    })
}

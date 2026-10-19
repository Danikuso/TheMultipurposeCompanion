//! Arranque y cableado de todos los subsistemas.

use crate::eddn_wiring::spawn_eddn_wiring;
use crate::{AppActions, Clipboard, Config, data_dir, spawn_net_service};
use chrono::Utc;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tmc_core::{CoreHandle, CoreMessage, CoreOptions, spawn_core_with};
use tmc_eddn::RELAY_URL;
use tmc_journal::{WatchOptions, is_game_running, resolve_journal_dir, spawn_watcher};
use tmc_model::source::SourceEvent;
use tmc_modules::all_modules;
use tmc_net::{NetClient, ReqwestTransport, Transport};
use tmc_server::{ServerConfig, ServerHandle, serve};
use tmc_store::{EventWriter, ImportOptions, Store, import_history};
use tokio::sync::broadcast::error::RecvError;

pub struct Running {
    pub server: ServerHandle,
    pub core: CoreHandle,
    pub store: Store,
    pub clipboard: Arc<Clipboard>,
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

/// `codex/ref` compacto de Canonn en `api_cache` (7 días, lo aplica el cliente).
struct CodexRefStore(tmc_core::Store);

const CODEX_REF_KEY: &str = "canonn:codex_ref:biology";

impl tmc_net::client::RefPersist for CodexRefStore {
    fn load(&self) -> Option<(chrono::DateTime<chrono::Utc>, String)> {
        let entry = self.0.api_cache_get(CODEX_REF_KEY).ok()??;
        Some((entry.fetched_at, entry.value))
    }
    fn save(&self, value: &str, at: chrono::DateTime<chrono::Utc>) {
        if let Err(error) = self.0.api_cache_put(CODEX_REF_KEY, value, at) {
            tracing::warn!(%error, "api_cache write failed");
        }
    }
}

pub async fn start(config: Config) -> anyhow::Result<Running> {
    let transport = ReqwestTransport::new()
        .map_err(|err| anyhow::anyhow!("could not build HTTP transport: {err:?}"))?;
    start_custom(config, Arc::new(transport), None).await
}

/// Igual que [`start`] pero con el transporte HTTP inyectado (tests de extremo a extremo).
pub async fn start_with_transport(
    config: Config,
    transport: Arc<dyn Transport>,
) -> anyhow::Result<Running> {
    start_custom(config, transport, None).await
}

/// Como [`start_with_transport`], y con `poll_idle` del vigilante del diario
/// sustituible (los tests emulan «juego en marcha», que sondea cada 250 ms).
pub async fn start_custom(
    config: Config,
    transport: Arc<dyn Transport>,
    poll_idle: Option<Duration>,
) -> anyhow::Result<Running> {
    start_full(config, transport, poll_idle, None).await
}

/// Como [`start_custom`], con la URL del relay EDDN sustituible (solo e2e en depuración).
pub async fn start_full(
    mut config: Config,
    transport: Arc<dyn Transport>,
    poll_idle: Option<Duration>,
    eddn_url: Option<String>,
) -> anyhow::Result<Running> {
    let journal_dir = resolve_journal_dir(config.journal_dir.as_deref())?;
    let db_dir = config.data_dir.clone().unwrap_or_else(data_dir);
    let store = open_or_rebuild_store(&db_dir)?;
    store.kv_set("locale", &config.language);

    let writer = EventWriter::spawn(&store);
    // Los módulos se registran siempre; cada uno respeta los ajustes que recibe
    // por `SourceEvent::Settings` (p. ej. `Dashboard` y `alerts_enabled`).
    let modules = all_modules();
    for module in &modules {
        store.apply_migrations(module.migrations())?;
    }

    let online = Arc::new(AtomicBool::new(config.online_services));
    // El servicio de red necesita el canal de eventos del núcleo, que a su vez
    // necesita el emisor de peticiones: se enlazan con un canal intermedio.
    let (net_events, mut net_events_rx) = tokio::sync::mpsc::channel::<SourceEvent>(64);
    let net = spawn_net_service(
        Arc::new(
            NetClient::new(transport).with_codex_persist(Arc::new(CodexRefStore(store.clone()))),
        ),
        store.clone(),
        net_events,
        online.clone(),
    );
    let core = spawn_core_with(
        modules,
        CoreOptions {
            writer: Some(writer),
            flush_every: Duration::from_millis(100),
            store: Some(store.clone()),
            net: Some(net.tx.clone()),
        },
    );
    let forward = core.events.clone();
    tokio::spawn(async move {
        while let Some(event) = net_events_rx.recv().await {
            if forward.send(event).await.is_err() {
                break;
            }
        }
    });
    // Suscripción previa al watcher para no perder la primera actualización.
    let mut rx = core.subscribe();
    let (settings_tx, settings_rx) = tokio::sync::watch::channel(config.module_settings());
    spawn_eddn_wiring(
        store.clone(),
        core.events.clone(),
        core.clone(),
        settings_rx,
        eddn_url.unwrap_or_else(|| RELAY_URL.to_owned()),
    );
    let _ = core
        .events
        .send(SourceEvent::Settings(config.module_settings()))
        .await;

    let watch_opts = WatchOptions {
        journal_dir: journal_dir.clone(),
        poll_idle: poll_idle.unwrap_or(WatchOptions::default().poll_idle),
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
                crate::eddn_service::seed_own_jumps_blocking(&import_store);
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

    // Navigation decide (sube `copy_seq` solo con el ajuste activo y en vivo);
    // aquí se copia cada vez que `copy_seq` aumenta.
    let clipboard = Arc::new(Clipboard::new());
    {
        let task_clipboard = clipboard.clone();
        let throttle_flag = throttle.clone();
        tokio::spawn(async move {
            // El replay nunca incrementa `copy_seq`: arranca en 0 y solo suben los saltos en vivo.
            let mut last_seq: u64 = 0;
            loop {
                // Si el canal se retrasa se descartan mensajes pero se sigue:
                // `copy_seq` es acumulativo, la siguiente actualización lo recupera.
                let msg = match rx.recv().await {
                    Ok(msg) => msg,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                };
                if let CoreMessage::Update { topics, .. } = msg {
                    if let Some(game) = topics.get("core.game")
                        && let Some(running) = game.get("running").and_then(|v| v.as_bool())
                    {
                        throttle_flag.store(running, Ordering::Relaxed);
                    }
                    if let Some(route) = topics.get("navigation.route")
                        && let Some(seq) = route.get("copy_seq").and_then(|v| v.as_u64())
                    {
                        if seq > last_seq
                            && let Some(target) = route
                                .get("copy_target")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                        {
                            let _ = task_clipboard.set_text(target);
                        }
                        last_seq = seq;
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
        Arc::new(
            AppActions::with_runtime(config, core.events.clone(), online, net)
                .with_clipboard(clipboard.clone())
                .with_settings_watch(settings_tx),
        ),
    )
    .await?;

    Ok(Running {
        server,
        core,
        store,
        clipboard,
    })
}

use crate::{Config, NetServiceHandle};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tmc_model::source::SourceEvent;
use tmc_server::{ActionError, ActionFut, ActionHost, ConfigHost};
use tmc_tts::TtsEngine;
use tokio::sync::mpsc;

/// Por dónde acabó la copia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod {
    System,
    MemoryFallback,
}

/// Escritura en el portapapeles del sistema (sustituible en pruebas).
type Backend = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

#[cfg(not(test))]
fn system_backend(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text.to_owned()))
        .map_err(|e| e.to_string())
}

/// En pruebas nunca se toca el portapapeles real.
#[cfg(test)]
fn system_backend(_text: &str) -> Result<(), String> {
    Ok(())
}

pub struct Clipboard {
    last: Mutex<Option<String>>,
    warned: AtomicBool,
    backend: Backend,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            last: Mutex::new(None),
            warned: AtomicBool::new(false),
            backend: Box::new(system_backend),
        }
    }

    /// Portapapeles con un backend propio (pruebas).
    #[cfg(all(test, debug_assertions))]
    fn with_backend(backend: Backend) -> Self {
        Self {
            backend,
            ..Self::new()
        }
    }

    /// Copia `text`. Devuelve si se usó el portapapeles del sistema o el
    /// respaldo en memoria; en ese caso avisa (`warn!`) una sola vez por
    /// sesión. El aviso refleja la petición de copia: la alerta
    /// `nav-auto-copied` no distingue si el sistema aceptó el texto.
    pub fn set_text(&self, text: &str) -> Result<CopyMethod, String> {
        // Solo longitudes: el texto (nombre de sistema, dato público) no se registra.
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "clipboard.copy.requested",
            serde_json::json!({"chars": text.chars().count(), "bytes": text.len()})
        );
        if let Ok(mut guard) = self.last.lock() {
            *guard = Some(text.to_owned());
        }
        match (self.backend)(text) {
            Ok(()) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "clipboard.copy.succeeded",
                    serde_json::json!({"system": true})
                );
                Ok(CopyMethod::System)
            }
            Err(err) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "clipboard.copy.fallback",
                    serde_json::json!({"system": false, "first_warning":
                        !self.warned.load(Ordering::Relaxed)})
                );
                if !self.warned.swap(true, Ordering::Relaxed) {
                    tracing::warn!(%err, "system clipboard unavailable; stored in memory fallback");
                }
                Ok(CopyMethod::MemoryFallback)
            }
        }
    }

    pub fn last_copied(&self) -> Option<String> {
        self.last.lock().ok().and_then(|g| g.clone())
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Conexión de `AppActions` con el núcleo y la capa de red en ejecución.
struct Runtime {
    events: mpsc::Sender<SourceEvent>,
    online: Arc<AtomicBool>,
    net: NetServiceHandle,
    sync_waker: Option<tmc_sync::SyncWaker>,
    store: Option<tmc_store::Store>,
}

pub struct AppActions {
    clipboard: Arc<Clipboard>,
    config: Mutex<Config>,
    runtime: Option<Runtime>,
    save_path: Option<PathBuf>,
    settings_watch: Option<tokio::sync::watch::Sender<tmc_model::settings::ModuleSettings>>,
    capi_service: Option<Arc<crate::capi_service::CapiService>>,
    store: Option<tmc_store::Store>,
    sync_waker: Option<tmc_sync::SyncWaker>,
    transport: Option<Arc<dyn tmc_net::Transport>>,
    native_engine: Mutex<Option<tmc_tts::NativeEngine>>,
    kokoro_engine: Mutex<Option<tmc_tts::KokoroEngine>>,
    tts_queue: Mutex<tmc_tts::TtsQueue>,
    pub update_state: Arc<tokio::sync::RwLock<crate::updater::UpdateStatus>>,
}

impl AppActions {
    /// Publica los ajustes tras cada `put_config` (el cableado de EDDN los observa).
    pub fn with_settings_watch(
        mut self,
        tx: tokio::sync::watch::Sender<tmc_model::settings::ModuleSettings>,
    ) -> Self {
        self.settings_watch = Some(tx);
        self
    }

    pub fn new() -> Self {
        Self {
            clipboard: Arc::new(Clipboard::new()),
            config: Mutex::new(Config::default()),
            runtime: None,
            save_path: None,
            settings_watch: None,
            capi_service: None,
            store: None,
            sync_waker: None,
            transport: None,
            native_engine: Mutex::new(None),
            kokoro_engine: Mutex::new(None),
            tts_queue: Mutex::new(tmc_tts::TtsQueue::default()),
            update_state: Arc::new(tokio::sync::RwLock::new(
                crate::updater::UpdateStatus::default(),
            )),
        }
    }

    pub fn with_config(cfg: Config) -> Self {
        Self {
            clipboard: Arc::new(Clipboard::new()),
            save_path: cfg.config_file.clone(),
            config: Mutex::new(cfg),
            runtime: None,
            settings_watch: None,
            capi_service: None,
            store: None,
            sync_waker: None,
            transport: None,
            native_engine: Mutex::new(None),
            kokoro_engine: Mutex::new(None),
            tts_queue: Mutex::new(tmc_tts::TtsQueue::default()),
            update_state: Arc::new(tokio::sync::RwLock::new(
                crate::updater::UpdateStatus::default(),
            )),
        }
    }

    /// Como `with_config`, y además `put_config` propaga los ajustes al núcleo
    /// y actualiza el interruptor de servicios en línea.
    pub fn with_runtime(
        cfg: Config,
        events: mpsc::Sender<SourceEvent>,
        online: Arc<AtomicBool>,
        net: NetServiceHandle,
    ) -> Self {
        let transport = net.transport();
        Self {
            runtime: Some(Runtime {
                events,
                online,
                net,
                sync_waker: None,
                store: None,
            }),
            transport: Some(transport),
            ..Self::with_config(cfg)
        }
    }

    /// Conecta el notificador de sincronización (`accounts.sync_now` y cambios de `online_services`).
    pub fn with_sync_waker(mut self, waker: tmc_sync::SyncWaker) -> Self {
        if let Some(runtime) = &mut self.runtime {
            runtime.sync_waker = Some(waker.clone());
        }
        self.sync_waker = Some(waker);
        self
    }

    /// Conecta la base de datos SQLite para acciones de cuentas y credenciales (`accounts.eddn.set_enabled`, etc.).
    pub fn with_store(mut self, store: tmc_store::Store) -> Self {
        if let Some(runtime) = &mut self.runtime {
            runtime.store = Some(store.clone());
        }
        self.store = Some(store);
        self
    }

    /// Conecta un transporte HTTP para verificación de credenciales en pruebas o headless.
    pub fn with_transport(mut self, transport: Arc<dyn tmc_net::Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Comparte el portapapeles con la tarea de copia automática.
    pub fn with_clipboard(mut self, clipboard: Arc<Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// Guarda la configuración en `path` en vez de en la ruta de usuario (tests).
    pub fn with_save_path(mut self, path: PathBuf) -> Self {
        self.save_path = Some(path);
        self
    }

    /// Conecta el servicio de autenticación y flota de Frontier CAPI.
    pub fn with_capi_service(
        mut self,
        capi_service: Arc<crate::capi_service::CapiService>,
    ) -> Self {
        self.capi_service = Some(capi_service);
        self
    }

    /// Autocompletado de sistemas para REST: `{items, page, more}`.
    ///
    /// Vacío en la galaxia Legacy (nunca se consulta) o con servicios en línea
    /// desactivados. `snapshot` es la instantánea del núcleo (`core.game.galaxy`).
    pub async fn autocomplete(
        &self,
        prefix: &str,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError> {
        let empty = json!({ "items": [], "page": 0, "more": false });
        let legacy = snapshot
            .get("core.game")
            .and_then(|g| g.get("galaxy"))
            .and_then(Value::as_str)
            == Some("Legacy");
        let Some(runtime) = &self.runtime else {
            return Ok(empty);
        };
        if legacy || !runtime.online.load(Ordering::Relaxed) {
            return Ok(empty);
        }
        match runtime.net.autocomplete(prefix).await {
            Ok(items) => Ok(json!({ "items": items, "page": 0, "more": false })),
            Err(failure) => Err(ActionError::Failed(format!("net-{failure:?}"))),
        }
    }

    pub fn clipboard(&self) -> &Clipboard {
        &self.clipboard
    }
}

impl Default for AppActions {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigHost for AppActions {
    fn get_config(&self) -> Value {
        let guard = self.config.lock().unwrap();
        serde_json::to_value(&*guard).unwrap_or(Value::Null)
    }

    fn put_config(&self, v: Value) -> Result<Value, ActionError> {
        let mut updated: Config =
            serde_json::from_value(v).map_err(|e| ActionError::Failed(e.to_string()))?;
        if !matches!(updated.language.as_str(), "es" | "en" | "fr" | "de" | "pt") {
            return Err(ActionError::Failed("invalid-language".into()));
        }
        updated
            .validate_f2()
            .map_err(|e| ActionError::Failed(e.into()))?;
        updated
            .validate_f3()
            .map_err(|e| ActionError::Failed(e.into()))?;
        updated
            .validate_f4()
            .map_err(|e| ActionError::Failed(e.into()))?;
        updated
            .validate_auto_companion()
            .map_err(|e| ActionError::Failed(e.into()))?;

        // F2 & F19: Preservar rutas del sistema y configuración de infraestructura del servidor
        {
            let current = self.config.lock().unwrap();
            updated.ui_dir = current.ui_dir.clone();
            updated.data_dir = current.data_dir.clone();
            updated.journal_dir = current.journal_dir.clone();
            updated.server.port = current.server.port;
            updated.server.lan_token = current.server.lan_token.clone();
            updated.desktop_device_id = current.desktop_device_id.clone();
        }

        if updated.server.lan {
            updated.ensure_lan_token();
        }
        let _ = match &self.save_path {
            Some(path) => updated.save_to(path),
            None => updated.save(),
        };
        if let Some(tx) = &self.settings_watch {
            tx.send_replace(updated.module_settings());
        }
        if let Some(runtime) = &self.runtime {
            runtime
                .online
                .store(updated.online_services, Ordering::Relaxed);
            if let Some(waker) = &runtime.sync_waker {
                waker.wake_all();
            }
            let event = SourceEvent::Settings(updated.module_settings());
            match runtime.events.try_send(event) {
                Ok(()) => {}
                // Cola llena: se reenvía sin bloquear a quien llama.
                Err(mpsc::error::TrySendError::Full(event)) => {
                    let events = runtime.events.clone();
                    match tokio::runtime::Handle::try_current() {
                        Ok(handle) => {
                            handle.spawn(async move {
                                let _ = events.send(event).await;
                            });
                        }
                        Err(_) => tracing::warn!("settings update dropped: no runtime"),
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    tracing::warn!("settings update dropped: core closed");
                }
            }
        }
        let out = serde_json::to_value(&updated).map_err(|e| ActionError::Failed(e.to_string()))?;
        *self.config.lock().unwrap() = updated;
        Ok(out)
    }
}

impl ActionHost for AppActions {
    fn desktop_device_id(&self) -> Option<String> {
        let mut guard = self.config.lock().unwrap();
        let before = guard.desktop_device_id.clone();
        let id = guard.ensure_desktop_device_id();
        if before.as_deref() != Some(id.as_str()) {
            // Solo se persiste el id sobre la copia del disco: la configuración en
            // memoria incluye sustituciones de la línea de órdenes que no se guardan.
            let path = self
                .save_path
                .clone()
                .unwrap_or_else(crate::config::config_path);
            let saved = Config::load_strict(&path).and_then(|mut on_disk| {
                on_disk.desktop_device_id = Some(id.clone());
                on_disk.save_to(&path)
            });
            if saved.is_err() {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "config.desktop_device_id.save_failed",
                    serde_json::json!({})
                );
            }
        }
        Some(id)
    }

    fn run_action(
        &self,
        name: &str,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError> {
        match name {
            "clipboard.copy-next-system" => {
                // Primero el siguiente de la ruta activa (plan o NavRoute) y,
                // si no hay, el de la ruta del juego.
                let pick = |topic: &str, field: &str| {
                    snapshot
                        .get(topic)
                        .and_then(|r| r.get(field))
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                };
                let next = pick("navigation.route", "copy_target")
                    .or_else(|| pick("trade.route", "copy_target"))
                    .or_else(|| pick("core.route", "next_system"))
                    .ok_or_else(|| {
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::State,
                            "clipboard.copy.no_target",
                            serde_json::json!({})
                        );
                        ActionError::Failed("no-next-system".into())
                    })?;
                self.clipboard.set_text(next).map_err(ActionError::Failed)?;
                Ok(json!({ "system": next }))
            }
            "frontier.logout" => {
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
                capi.clear_session()
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                Ok(json!({ "ok": true }))
            }
            "app.rotate_lan_token" | "server.rotate_lan_token" => {
                let mut guard = self.config.lock().unwrap();
                let new_token = guard.rotate_lan_token();
                let _ = match &self.save_path {
                    Some(path) => guard.save_to(path),
                    None => guard.save(),
                };
                Ok(json!({ "ok": true, "lan_token": new_token }))
            }
            _ => Err(ActionError::NotFound),
        }
    }

    fn run_action_with(
        &self,
        name: &str,
        args: &Value,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError> {
        let sync_capi_fid = |capi: &crate::capi_service::CapiService| {
            let fid = args.get("fid").and_then(Value::as_str).or_else(|| {
                snapshot
                    .get("core.cmdr")
                    .and_then(|c| c.get("fid"))
                    .and_then(Value::as_str)
            });
            if let Some(f) = fid {
                capi.set_active_fid(Some(f));
            }
        };

        match name {
            "frontier.login" => {
                let redirect_uri = args
                    .get("redirect_uri")
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| std::env::var("TMC_CAPI_REDIRECT_URI").ok())
                    .unwrap_or_else(|| {
                        "https://localhost:47320/api/auth/frontier/callback".to_string()
                    });
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
                sync_capi_fid(capi);
                let (url, state) = capi.start_login(&redirect_uri);
                return Ok(json!({ "url": url, "state": state }));
            }
            "frontier.logout" => {
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
                sync_capi_fid(capi);
                capi.clear_session()
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                if let Some(runtime) = &self.runtime {
                    let _ = runtime.events.try_send(SourceEvent::Action {
                        name: "accounts.set_settings".to_string(),
                        args: json!({
                            "capi_connected": false,
                            "capi_enabled": false,
                            "capi_remote_ships_count": 0,
                        }),
                    });
                }
                return Ok(json!({ "ok": true }));
            }
            "frontier.submit_callback" => {
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
                sync_capi_fid(capi);
                let raw_input = args
                    .get("url")
                    .or_else(|| args.get("code"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ActionError::Failed("missing-url-or-code".into()))?;

                let (code, state) = if raw_input.contains("code=") {
                    let parsed_code = raw_input
                        .split("code=")
                        .nth(1)
                        .and_then(|s| s.split('&').next())
                        .unwrap_or("");
                    let parsed_state = raw_input
                        .split("state=")
                        .nth(1)
                        .and_then(|s| s.split('&').next())
                        .unwrap_or("");
                    (parsed_code.to_string(), parsed_state.to_string())
                } else {
                    let code = raw_input.trim().to_string();
                    let state = args
                        .get("state")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    (code, state)
                };

                if code.is_empty() {
                    return Err(ActionError::Failed("missing-authorization-code".into()));
                }

                let detected_uri =
                    if raw_input.starts_with("http://") || raw_input.starts_with("https://") {
                        raw_input.split('?').next().map(|s| s.to_string())
                    } else {
                        None
                    };
                let redirect_uri = args
                    .get("redirect_uri")
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string())
                    .or(detected_uri)
                    .or_else(|| std::env::var("TMC_CAPI_REDIRECT_URI").ok())
                    .unwrap_or_else(|| {
                        "https://localhost:47320/api/auth/frontier/callback".to_string()
                    });

                let capi_cloned = capi.clone();
                let events = self.runtime.as_ref().map(|r| r.events.clone());

                let result = {
                    let handle = tokio::runtime::Handle::try_current();
                    match handle {
                        Ok(h)
                            if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread =>
                        {
                            tokio::task::block_in_place(|| {
                                h.block_on(async {
                                    capi_cloned
                                        .handle_callback(&code, &state, &redirect_uri)
                                        .await
                                })
                            })
                        }
                        _ => std::thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async {
                                capi_cloned
                                    .handle_callback(&code, &state, &redirect_uri)
                                    .await
                            })
                        })
                        .join()
                        .unwrap_or_else(|_| {
                            Err(crate::capi_service::CapiError::Malformed(
                                "thread-panicked".into(),
                            ))
                        }),
                    }
                };

                match result {
                    Ok(token) => {
                        if let Some(events) = events {
                            let _ = events.try_send(SourceEvent::Action {
                                name: "accounts.set_settings".to_string(),
                                args: serde_json::json!({
                                    "capi_connected": true,
                                }),
                            });
                        }
                        return Ok(json!({
                            "ok": true,
                            "token_type": token.token_type,
                            "expires_in": token.expires_in,
                        }));
                    }
                    Err(e) => return Err(ActionError::Failed(e.to_string())),
                }
            }
            "frontier.fleet" => {
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
                sync_capi_fid(capi);
                let capi_cloned = capi.clone();
                let events = self.runtime.as_ref().map(|r| r.events.clone());

                let result = {
                    let handle = tokio::runtime::Handle::try_current();
                    match handle {
                        Ok(h)
                            if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread =>
                        {
                            tokio::task::block_in_place(|| {
                                h.block_on(async { capi_cloned.fetch_profile().await })
                            })
                        }
                        _ => std::thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async { capi_cloned.fetch_profile().await })
                        })
                        .join()
                        .unwrap_or_else(|_| {
                            Err(crate::capi_service::CapiError::Malformed(
                                "thread-panicked".into(),
                            ))
                        }),
                    }
                };

                match result {
                    Ok(profile) => {
                        let count = profile.remote_ships.len();
                        if let Some(events) = events {
                            let _ = events.try_send(SourceEvent::Action {
                                name: "accounts.set_settings".to_string(),
                                args: serde_json::json!({
                                    "capi_remote_ships_count": count,
                                    "capi_commander_name": profile.commander_name,
                                }),
                            });
                        }
                        return Ok(json!({
                            "ok": true,
                            "ships": profile.remote_ships,
                            "count": count,
                            "commander_name": profile.commander_name,
                            "credits": profile.credits,
                        }));
                    }
                    Err(e) => return Err(ActionError::Failed(e.to_string())),
                }
            }
            "tts.voices" => {
                let engine_kind = args
                    .get("engine")
                    .and_then(|e| e.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| match s {
                        "kokoro" => tmc_model::tts::TtsEngineKind::Kokoro,
                        _ => tmc_model::tts::TtsEngineKind::Native,
                    })
                    .unwrap_or_else(|| {
                        let cfg = self.config.lock().unwrap();
                        cfg.tts.engine
                    });

                let voices = match engine_kind {
                    tmc_model::tts::TtsEngineKind::Kokoro => {
                        let mut guard = self.kokoro_engine.lock().unwrap();
                        if guard.is_none() {
                            *guard = tmc_tts::KokoroEngine::new().ok();
                        }
                        guard
                            .as_ref()
                            .map(|e| e.available_voices())
                            .unwrap_or_default()
                    }
                    tmc_model::tts::TtsEngineKind::Native => {
                        let mut guard = self.native_engine.lock().unwrap();
                        if guard.is_none() {
                            *guard = tmc_tts::NativeEngine::new().ok();
                        }
                        guard
                            .as_ref()
                            .map(|e| e.available_voices())
                            .unwrap_or_default()
                    }
                };

                return Ok(json!({ "voices": voices }));
            }
            "tts.test" => {
                let (engine_kind, default_voice, default_vol, default_rate, lang) = {
                    let cfg = self.config.lock().unwrap();
                    (
                        cfg.tts.engine,
                        cfg.tts.voice.clone(),
                        cfg.tts.volume,
                        cfg.tts.rate,
                        cfg.language.clone(),
                    )
                };

                let chosen_engine = args
                    .get("engine")
                    .and_then(|e| e.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| match s {
                        "kokoro" => tmc_model::tts::TtsEngineKind::Kokoro,
                        _ => tmc_model::tts::TtsEngineKind::Native,
                    })
                    .unwrap_or(engine_kind);

                let voice = args
                    .get("voice")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .or(default_voice);

                let volume = args
                    .get("volume")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.min(100) as u8)
                    .unwrap_or(default_vol);

                let rate = args
                    .get("rate")
                    .and_then(|r| r.as_u64())
                    .map(|r| r.clamp(80, 150) as u8)
                    .unwrap_or(default_rate);

                let default_phrase = tmc_i18n::I18n::new(&lang).t("tts-test-phrase", &[]);
                let text = args
                    .get("text")
                    .and_then(|t| t.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(&default_phrase);

                let rate_f32 = rate as f32 / 100.0;
                self.speak_with_engine(chosen_engine, text, voice.as_deref(), volume, rate_f32)?;

                return Ok(json!({ "ok": true }));
            }
            "tts.stop" | "voice.stop" => {
                self.stop_tts();
                return Ok(json!({ "ok": true, "stopped": true }));
            }
            "voice.status" => {
                let status = crate::voiceattack::detect_voiceattack();
                return serde_json::to_value(&status)
                    .map_err(|e| ActionError::Failed(e.to_string()));
            }
            "voice.profile" => {
                let port = args
                    .get("port")
                    .and_then(|p| p.as_u64())
                    .map(|p| p as u16)
                    .unwrap_or(47320);
                let xml = crate::voiceattack::generate_vap_profile(port);
                return Ok(json!({ "xml": xml }));
            }
            "clipboard.copy" => {
                let text = args
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                if !text.is_empty() {
                    self.clipboard.set_text(text).map_err(ActionError::Failed)?;
                    return Ok(json!({ "copied": text }));
                }
                return Err(ActionError::Failed("empty-text".into()));
            }
            "accounts.eddn.set_enabled" => {
                let enabled = args
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| ActionError::Failed("missing-enabled".into()))?;
                let store = self.active_store()?;
                let now = chrono::Utc::now();
                if enabled {
                    if store.kv_get(tmc_sync::router::EDDN_ENABLED_KEY).is_none() {
                        store.kv_set(
                            tmc_sync::router::EDDN_ENABLED_KEY,
                            &now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        );
                    }
                } else {
                    store.kv_delete(tmc_sync::router::EDDN_ENABLED_KEY);
                    let _ = store.discard_pending(tmc_model::sync::SyncService::Eddn, None, now);
                    let _ = store.clear_sync_activations(tmc_model::sync::SyncService::Eddn);
                }
                if let Some(waker) = self.active_sync_waker() {
                    waker.wake_all();
                }
                return Ok(json!({ "ok": true, "enabled": enabled }));
            }
            "accounts.key.save" => {
                let service = parse_key_service(args)?;
                let cmdr_ref = args
                    .get("cmdr_ref")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("missing-cmdr-ref".into()))?;
                let api_key = args
                    .get("api_key")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("empty-api-key".into()))?;
                let store = self.active_store()?;
                let (fid, cmdr_name) = resolve_cmdr_ref(&store, cmdr_ref)?;
                let is_online = self
                    .runtime
                    .as_ref()
                    .map(|r| r.online.load(Ordering::Acquire))
                    .unwrap_or_else(|| self.config.lock().unwrap().online_services);

                let check = if !is_online {
                    tmc_net::KeyCheck::Unreachable
                } else if let Some(transport) = self.transport.clone() {
                    let fid_owned = fid.clone();
                    let name_owned = cmdr_name.clone();
                    let key_owned = api_key.to_owned();
                    let inara_limiter = self
                        .active_sync_waker()
                        .and_then(|w| w.inara_limiter().cloned());
                    block_on_sync(async move {
                        tmc_sync::check_service_key_limited(
                            transport.as_ref(),
                            service,
                            &name_owned,
                            Some(&fid_owned),
                            &key_owned,
                            inara_limiter.as_ref(),
                        )
                        .await
                    })
                } else {
                    tmc_net::KeyCheck::Unreachable
                };

                let now = chrono::Utc::now();
                let now_str = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                match check {
                    tmc_net::KeyCheck::Valid => {
                        let cred = tmc_store::SyncCredential {
                            service,
                            cmdr_fid: fid.clone(),
                            cmdr_name,
                            api_key: api_key.to_owned(),
                            created_at: now_str.clone(),
                            verified_at: Some(now_str),
                            assignment: tmc_model::sync::KeyAssignment::Confirmed,
                        };
                        store
                            .put_sync_credential(&cred)
                            .map_err(|e| ActionError::Failed(e.to_string()))?;
                        if store.activation(service, &fid).ok().flatten().is_none() {
                            store
                                .set_activation(service, &fid, now)
                                .map_err(|e| ActionError::Failed(e.to_string()))?;
                        }
                        if let Some(waker) = self.active_sync_waker() {
                            waker.report_verify(tmc_sync::VerifyUpdate {
                                service,
                                fid,
                                check: Some(tmc_net::KeyCheck::Valid),
                                next_retry: None,
                            });
                            waker.wake_all();
                        }
                        return Ok(json!({
                            "ok": true,
                            "verified": true,
                            "state": "active",
                        }));
                    }
                    tmc_net::KeyCheck::Rejected => {
                        return Ok(json!({
                            "ok": false,
                            "verified": false,
                            "state": "rejected",
                        }));
                    }
                    tmc_net::KeyCheck::Unreachable => {
                        let cred = tmc_store::SyncCredential {
                            service,
                            cmdr_fid: fid.clone(),
                            cmdr_name,
                            api_key: api_key.to_owned(),
                            created_at: now_str,
                            verified_at: None,
                            assignment: tmc_model::sync::KeyAssignment::Confirmed,
                        };
                        store
                            .put_sync_credential(&cred)
                            .map_err(|e| ActionError::Failed(e.to_string()))?;
                        let next_retry = now + chrono::Duration::seconds(60);
                        if let Some(waker) = self.active_sync_waker() {
                            waker.report_verify(tmc_sync::VerifyUpdate {
                                service,
                                fid,
                                check: Some(tmc_net::KeyCheck::Unreachable),
                                next_retry: Some(next_retry),
                            });
                            waker.wake_all();
                        }
                        return Ok(json!({
                            "ok": true,
                            "verified": false,
                            "state": "saved_unverified",
                        }));
                    }
                }
            }
            "accounts.key.delete" => {
                let service = parse_key_service(args)?;
                let cmdr_ref = args
                    .get("cmdr_ref")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("missing-cmdr-ref".into()))?;
                let store = self.active_store()?;
                let (fid, _) = resolve_cmdr_ref(&store, cmdr_ref)?;
                let discarded = store
                    .delete_sync_credential(service, &fid, chrono::Utc::now())
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                if let Some(waker) = self.active_sync_waker() {
                    waker.report_verify(tmc_sync::VerifyUpdate {
                        service,
                        fid,
                        check: None,
                        next_retry: None,
                    });
                    waker.wake_all();
                }
                return Ok(json!({ "ok": true, "discarded": discarded }));
            }
            "accounts.key.confirm" => {
                let service = parse_key_service(args)?;
                let cmdr_ref = args
                    .get("cmdr_ref")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("missing-cmdr-ref".into()))?;
                let store = self.active_store()?;
                let (fid, _) = resolve_cmdr_ref(&store, cmdr_ref)?;
                let _ = store
                    .confirm_assignment(service, &fid)
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                if let Some(waker) = self.active_sync_waker() {
                    waker.wake_all();
                }
                return Ok(json!({ "ok": true }));
            }
            "accounts.key.assign" => {
                let service = parse_key_service(args)?;
                let cmdr_ref = args
                    .get("cmdr_ref")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("missing-cmdr-ref".into()))?;
                let store = self.active_store()?;
                let (fid, cmdr_name) = resolve_cmdr_ref(&store, cmdr_ref)?;
                let now = chrono::Utc::now();
                let mut assigned = store
                    .assign_unassigned_key(
                        service,
                        &fid,
                        &cmdr_name,
                        tmc_model::sync::KeyAssignment::Confirmed,
                        now,
                    )
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                if !assigned {
                    // Reasignación desde una clave auto-asignada pendiente de confirmación.
                    let known = store
                        .known_cmdrs()
                        .map_err(|e| ActionError::Failed(e.to_string()))?;
                    for (other_fid, _) in known {
                        if let Ok(Some(other_cred)) = store.sync_credential(service, &other_fid)
                            && other_cred.assignment
                                == tmc_model::sync::KeyAssignment::AutoPendingConfirmation
                        {
                            let migrated = tmc_store::SyncCredential {
                                service,
                                cmdr_fid: fid.clone(),
                                cmdr_name: cmdr_name.clone(),
                                api_key: other_cred.api_key,
                                created_at: other_cred.created_at,
                                verified_at: other_cred.verified_at,
                                assignment: tmc_model::sync::KeyAssignment::Confirmed,
                            };
                            store
                                .put_sync_credential(&migrated)
                                .map_err(|e| ActionError::Failed(e.to_string()))?;
                            if store.activation(service, &fid).ok().flatten().is_none() {
                                store
                                    .set_activation(service, &fid, now)
                                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                            }
                            if other_fid != fid {
                                let _ = store.delete_sync_credential(service, &other_fid, now);
                            }
                            assigned = true;
                            break;
                        }
                    }
                }
                if assigned {
                    if store.activation(service, &fid).ok().flatten().is_none() {
                        let _ = store.set_activation(service, &fid, now);
                    }
                    if let Some(waker) = self.active_sync_waker() {
                        waker.wake_verify();
                        waker.wake_all();
                    }
                    return Ok(json!({ "ok": true }));
                }
                return Err(ActionError::Failed("no-unassigned-key".into()));
            }
            "accounts.cmdr.forget" => {
                let cmdr_ref = args
                    .get("cmdr_ref")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("missing-cmdr-ref".into()))?;
                let store = self.active_store()?;
                let (fid, _) = resolve_cmdr_ref(&store, cmdr_ref)?;
                store
                    .forget_cmdr(&fid, chrono::Utc::now())
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                if let Some(waker) = self.active_sync_waker() {
                    for service in [
                        tmc_model::sync::SyncService::Inara,
                        tmc_model::sync::SyncService::Edsm,
                    ] {
                        waker.report_verify(tmc_sync::VerifyUpdate {
                            service,
                            fid: fid.clone(),
                            check: None,
                            next_retry: None,
                        });
                    }
                    waker.wake_all();
                }
                return Ok(json!({ "ok": true }));
            }
            "voice.speak" => {
                let (engine_kind, default_voice, default_vol, default_rate) = {
                    let cfg = self.config.lock().unwrap();
                    (
                        cfg.tts.engine,
                        cfg.tts.voice.clone(),
                        cfg.tts.volume,
                        cfg.tts.rate,
                    )
                };

                let text = args
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                if text.trim().is_empty() {
                    return Ok(json!({ "ok": true, "spoken": false }));
                }

                let rate_f32 = default_rate as f32 / 100.0;
                let spoken = self
                    .speak_with_engine(
                        engine_kind,
                        text,
                        default_voice.as_deref(),
                        default_vol,
                        rate_f32,
                    )
                    .is_ok();
                return Ok(json!({ "ok": true, "spoken": spoken }));
            }
            _ => {}
        }
        if !MODULE_PREFIXES.iter().any(|p| name.starts_with(p)) {
            return self.run_action(name, snapshot);
        }
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| ActionError::Failed("no-runtime".into()))?;
        if name == "accounts.sync_now"
            && let Some(waker) = &runtime.sync_waker
        {
            waker.wake_all();
        }
        let event = SourceEvent::Action {
            name: name.to_owned(),
            args: args.clone(),
        };
        match runtime.events.try_send(event) {
            Ok(()) => Ok(json!({ "accepted": true })),
            Err(mpsc::error::TrySendError::Full(_)) => Err(ActionError::Failed("busy".into())),
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(ActionError::Failed("core-closed".into()))
            }
        }
    }

    fn autocomplete(&self, q: String, snapshot: &BTreeMap<String, Value>) -> ActionFut {
        let empty = json!({ "items": [], "page": 0, "more": false });
        let legacy = snapshot
            .get("core.game")
            .and_then(|g| g.get("galaxy"))
            .and_then(Value::as_str)
            == Some("Legacy");
        let Some(runtime) = &self.runtime else {
            return Box::pin(async move { Ok(empty) });
        };
        if legacy || !runtime.online.load(Ordering::Relaxed) {
            return Box::pin(async move { Ok(empty) });
        }
        let net = runtime.net.clone();
        Box::pin(async move {
            match net.autocomplete(&q).await {
                Ok(items) => Ok(json!({ "items": items })),
                Err(failure) => Err(ActionError::Failed(format!("net-{failure:?}"))),
            }
        })
    }

    fn get_config(&self) -> Value {
        ConfigHost::get_config(self)
    }

    fn put_config(&self, v: Value) -> Result<Value, ActionError> {
        ConfigHost::put_config(self, v)
    }

    fn frontier_login_url(&self, redirect_uri: &str) -> ActionFut {
        let capi = self.capi_service.clone();
        let redirect_uri = redirect_uri.to_owned();
        Box::pin(async move {
            let Some(capi) = capi else {
                return Err(ActionError::NotFound);
            };
            let (url, state) = capi.start_login(&redirect_uri);
            Ok(json!({ "url": url, "state": state }))
        })
    }

    fn frontier_exchange_code(&self, code: &str, state: &str, redirect_uri: &str) -> ActionFut {
        let capi = self.capi_service.clone();
        let events = self.runtime.as_ref().map(|r| r.events.clone());
        let code = code.to_owned();
        let state = state.to_owned();
        let redirect_uri = redirect_uri.to_owned();
        Box::pin(async move {
            let Some(capi) = capi else {
                return Err(ActionError::NotFound);
            };
            let token = capi
                .handle_callback(&code, &state, &redirect_uri)
                .await
                .map_err(|e| ActionError::Failed(e.to_string()))?;
            if let Some(events) = events {
                let _ = events.try_send(SourceEvent::Action {
                    name: "accounts.set_settings".to_string(),
                    args: serde_json::json!({
                        "capi_connected": true,
                        "capi_enabled": true,
                    }),
                });
            }
            Ok(json!({
                "ok": true,
                "token_type": token.token_type,
                "expires_in": token.expires_in,
            }))
        })
    }

    fn update_status(&self) -> ActionFut {
        let state = self.update_state.clone();
        Box::pin(async move {
            let status = state.read().await.clone();
            Ok(serde_json::to_value(status).unwrap_or_default())
        })
    }

    fn check_for_updates(&self) -> ActionFut {
        let state = self.update_state.clone();
        Box::pin(async move {
            match crate::updater::check_for_updates(state).await {
                Ok(status) => Ok(serde_json::to_value(status).unwrap_or_default()),
                Err(err) => Err(ActionError::Failed(err)),
            }
        })
    }

    fn install_update(&self) -> ActionFut {
        let state = self.update_state.clone();
        Box::pin(async move {
            match crate::updater::download_and_install_update(state).await {
                Ok(msg) => Ok(serde_json::json!({ "ok": true, "message": msg })),
                Err(err) => Err(ActionError::Failed(err)),
            }
        })
    }
}

const MODULE_PREFIXES: [&str; 13] = [
    "navigation.",
    "exploration.",
    "exobiology.",
    "combat.",
    "mining.",
    "trade.",
    "missions.",
    "engineering.",
    "carrier.",
    "colonisation.",
    "powerplay.",
    "bgs.",
    "accounts.",
];

#[cfg(test)]
pub(crate) fn module_prefixes() -> &'static [&'static str] {
    &MODULE_PREFIXES
}

#[cfg(test)]
pub(crate) fn has_module_prefix(name: &str) -> bool {
    MODULE_PREFIXES.iter().any(|p| name.starts_with(p))
}

fn parse_key_service(args: &Value) -> Result<tmc_model::sync::SyncService, ActionError> {
    match args.get("service").and_then(Value::as_str) {
        Some("edsm") => Ok(tmc_model::sync::SyncService::Edsm),
        Some("inara") => Ok(tmc_model::sync::SyncService::Inara),
        _ => Err(ActionError::Failed("invalid-service".into())),
    }
}

fn resolve_cmdr_ref(
    store: &tmc_store::Store,
    cmdr_ref: &str,
) -> Result<(String, String), ActionError> {
    let salt = store
        .get_or_create_eddn_salt()
        .map_err(|e| ActionError::Failed(e.to_string()))?;
    let known = store
        .known_cmdrs()
        .map_err(|e| ActionError::Failed(e.to_string()))?;
    known
        .into_iter()
        .find(|(fid, _)| tmc_sync::cmdr_ref(&salt, fid) == cmdr_ref)
        .ok_or_else(|| ActionError::Failed("unknown-commander".into()))
}

fn block_on_sync<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(h) if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| h.block_on(fut))
        }
        _ => std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio current_thread runtime");
            rt.block_on(fut)
        })
        .join()
        .expect("verification thread panicked"),
    }
}

impl AppActions {
    fn active_store(&self) -> Result<tmc_store::Store, ActionError> {
        self.store
            .clone()
            .or_else(|| self.runtime.as_ref().and_then(|r| r.store.clone()))
            .ok_or_else(|| ActionError::Failed("no-store".into()))
    }

    fn active_sync_waker(&self) -> Option<tmc_sync::SyncWaker> {
        self.sync_waker
            .clone()
            .or_else(|| self.runtime.as_ref().and_then(|r| r.sync_waker.clone()))
    }

    /// Reutiliza las instancias persistentes de los motores TTS para evitar cortar `SPF_ASYNC`
    /// o recrear subprocesos en cada locución.
    fn speak_with_engine(
        &self,
        engine_kind: tmc_model::tts::TtsEngineKind,
        text: &str,
        voice: Option<&str>,
        volume: u8,
        rate_f32: f32,
    ) -> Result<(), ActionError> {
        match engine_kind {
            tmc_model::tts::TtsEngineKind::Kokoro => {
                let mut guard = self.kokoro_engine.lock().unwrap();
                if guard.is_none() {
                    *guard = tmc_tts::KokoroEngine::new().ok();
                }
                if let Some(ref mut engine) = *guard {
                    let _ = engine.check_idle_timeout();
                    engine
                        .speak(text, voice, volume, rate_f32)
                        .map_err(|e| ActionError::Failed(e.to_string()))
                } else {
                    Err(ActionError::Failed("KokoroEngine unavailable".into()))
                }
            }
            tmc_model::tts::TtsEngineKind::Native => {
                let mut guard = self.native_engine.lock().unwrap();
                if guard.is_none() {
                    *guard = tmc_tts::NativeEngine::new().ok();
                }
                if let Some(ref mut engine) = *guard {
                    engine
                        .speak(text, voice, volume, rate_f32)
                        .map_err(|e| ActionError::Failed(e.to_string()))
                } else {
                    Err(ActionError::Failed("NativeEngine unavailable".into()))
                }
            }
        }
    }

    /// Detiene de inmediato la locución en curso y vacía la cola TTS pendiente.
    pub fn stop_tts(&self) {
        if let Ok(mut q) = self.tts_queue.lock() {
            q.clear();
        }
        if let Ok(mut guard) = self.kokoro_engine.lock()
            && let Some(ref mut engine) = *guard
        {
            let _ = engine.stop();
        }
        if let Ok(mut guard) = self.native_engine.lock()
            && let Some(ref mut engine) = *guard
        {
            let _ = engine.stop();
        }
    }

    /// Encola y reproduce un aviso táctico de cabina (`VoiceCue`) respetando los ajustes de canal,
    /// cooldown y prioridades del copiloto. Devuelve `true` si se despachó a síntesis.
    pub fn handle_voice_cue(&self, cue: &tmc_model::tts::VoiceCue) -> bool {
        let (tts_cfg, lang) = {
            let cfg = self.config.lock().unwrap();
            (cfg.tts.clone(), cfg.language.clone())
        };

        if !tts_cfg.is_channel_enabled(cue.channel) {
            return false;
        }

        let spoken_text = tmc_i18n::I18n::new(&lang).t_map(&cue.text_key, &cue.args);
        if spoken_text.trim().is_empty() || spoken_text == cue.text_key {
            return false;
        }

        let next_item = {
            let mut queue = self.tts_queue.lock().unwrap();
            let accepted = queue.enqueue(
                cue.priority,
                spoken_text,
                tts_cfg.voice.clone(),
                tts_cfg.volume,
                tts_cfg.rate_multiplier(),
                cue.cooldown_key.clone(),
            );
            if !accepted {
                return false;
            }
            queue.pop_next()
        };

        if let Some(item) = next_item {
            self.speak_with_engine(
                tts_cfg.engine,
                &item.text,
                item.voice.as_deref(),
                item.volume,
                item.rate,
            )
            .is_ok()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {

    #[cfg(debug_assertions)]
    #[test]
    fn copy_diagnostics_never_include_the_text() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            let ok = super::Clipboard::with_backend(Box::new(|_| Ok(())));
            let _ = ok.set_text("CANARYCOPY");
            let failing = super::Clipboard::with_backend(Box::new(|_| Err("CANARYERR".into())));
            let _ = failing.set_text("CANARYCOPY");
        });
        let all: String = records
            .iter()
            .map(|r| {
                format!(
                    "{} {}
",
                    r.code, r.payload
                )
            })
            .collect();
        assert!(!all.contains("CANARYCOPY"), "{all}");
        assert!(!all.contains("CANARYERR"), "{all}");
        let codes: Vec<&str> = records.iter().map(|r| r.code.as_str()).collect();
        assert_eq!(
            codes,
            [
                "clipboard.copy.requested",
                "clipboard.copy.succeeded",
                "clipboard.copy.requested",
                "clipboard.copy.fallback"
            ]
        );
        assert!(all.contains("\"chars\":10"));
    }

    #[test]
    fn voice_cue_respects_enabled_channels_and_cooldown_and_stop() {
        use tmc_model::tts::{TtsChannel, TtsPriority, VoiceCue};
        use tmc_server::ActionHost;

        let actions = super::AppActions::new();
        let cue = VoiceCue::new(
            TtsPriority::Tactical,
            TtsChannel::Navigation,
            "tts-nav-jump-scoopable",
        )
        .with_arg("system", "Sol")
        .with_cooldown("nav_jump");

        // Con TTS desactivado por defecto no debe despachar nada
        assert!(!actions.handle_voice_cue(&cue));

        // Activar TTS con volumen 0 para prueba silenciosa
        {
            let mut cfg = actions.config.lock().unwrap();
            cfg.tts.enabled = true;
            cfg.tts.volume = 0;
        }
        assert!(actions.handle_voice_cue(&cue));
        // Segunda emisión inmediata con la misma clave de cooldown debe suprimirse
        assert!(!actions.handle_voice_cue(&cue));

        // Canal deshabilitado tampoco debe emitir
        {
            let mut cfg = actions.config.lock().unwrap();
            cfg.tts.channels.insert(TtsChannel::Combat, false);
        }
        let combat_cue = VoiceCue::new(
            TtsPriority::Emergency,
            TtsChannel::Combat,
            "tts-combat-shields-down",
        );
        assert!(!actions.handle_voice_cue(&combat_cue));

        // tts.stop limpia y detiene sin errores
        let res = actions
            .run_action_with("tts.stop", &serde_json::Value::Null, &Default::default())
            .unwrap();
        assert_eq!(res["ok"], true);
        assert_eq!(res["stopped"], true);
    }
}

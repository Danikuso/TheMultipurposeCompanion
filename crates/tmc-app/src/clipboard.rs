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
}

pub struct AppActions {
    clipboard: Arc<Clipboard>,
    config: Mutex<Config>,
    runtime: Option<Runtime>,
    save_path: Option<PathBuf>,
    settings_watch: Option<tokio::sync::watch::Sender<tmc_model::settings::ModuleSettings>>,
    capi_service: Option<Arc<crate::capi_service::CapiService>>,
    edsm_service: Option<Arc<crate::edsm_service::EdsmService>>,
    inara_service: Option<Arc<crate::inara_service::InaraService>>,
    kokoro_engine: Mutex<Option<tmc_tts::KokoroEngine>>,
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
            edsm_service: None,
            inara_service: None,
            kokoro_engine: Mutex::new(None),
        }
    }

    pub fn with_config(cfg: Config) -> Self {
        Self {
            clipboard: Arc::new(Clipboard::new()),
            config: Mutex::new(cfg),
            runtime: None,
            save_path: None,
            settings_watch: None,
            capi_service: None,
            edsm_service: None,
            inara_service: None,
            kokoro_engine: Mutex::new(None),
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
        Self {
            runtime: Some(Runtime {
                events,
                online,
                net,
            }),
            ..Self::with_config(cfg)
        }
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

    /// Conecta el servicio de persistencia y sincronización de EDSM.
    pub fn with_edsm_service(
        mut self,
        edsm_service: Arc<crate::edsm_service::EdsmService>,
    ) -> Self {
        self.edsm_service = Some(edsm_service);
        self
    }

    /// Conecta el servicio de integración y sincronización de Inara InAPI v1.
    pub fn with_inara_service(
        mut self,
        inara_service: Arc<crate::inara_service::InaraService>,
    ) -> Self {
        self.inara_service = Some(inara_service);
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
            _ => Err(ActionError::NotFound),
        }
    }

    fn run_action_with(
        &self,
        name: &str,
        args: &Value,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError> {
        match name {
            "frontier.login" => {
                let redirect_uri = args
                    .get("redirect_uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("https://localhost:47320/api/auth/frontier/callback");
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
                let (url, state) = capi.start_login(redirect_uri);
                return Ok(json!({ "url": url, "state": state }));
            }
            "frontier.logout" => {
                let capi = self.capi_service.as_ref().ok_or(ActionError::NotFound)?;
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

                let redirect_uri = args
                    .get("redirect_uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("https://localhost:47320/api/auth/frontier/callback")
                    .to_string();

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
            "inara.test_connection" => {
                let inara = self.inara_service.as_ref().ok_or(ActionError::NotFound)?;
                let api_key = args.get("api_key").and_then(|k| k.as_str());
                let cmdr = args
                    .get("commander_name")
                    .and_then(|c| c.as_str())
                    .or_else(|| {
                        snapshot
                            .get("accounts.status")
                            .and_then(|s| s.get("inara"))
                            .and_then(|i| i.get("commander_name"))
                            .and_then(|c| c.as_str())
                    })
                    .or_else(|| {
                        snapshot
                            .get("core.cmdr")
                            .and_then(|c| c.get("name"))
                            .and_then(|n| n.as_str())
                    });

                let inara_cloned = inara.clone();
                let key_owned = api_key.map(|k| k.to_string());
                let cmdr_owned = cmdr.map(|c| c.to_string());
                let events = self.runtime.as_ref().map(|r| r.events.clone());

                let result = {
                    let handle = tokio::runtime::Handle::try_current();
                    match handle {
                        Ok(h)
                            if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread =>
                        {
                            tokio::task::block_in_place(|| {
                                h.block_on(async {
                                    inara_cloned
                                        .test_connection(
                                            key_owned.as_deref(),
                                            cmdr_owned.as_deref(),
                                        )
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
                                inara_cloned
                                    .test_connection(key_owned.as_deref(), cmdr_owned.as_deref())
                                    .await
                            })
                        })
                        .join()
                        .unwrap_or_else(|_| Err("Thread panicked".to_string())),
                    }
                };

                match result {
                    Ok(val) => {
                        if let Some(events) = events {
                            let inara_url = val
                                .get("inara_url")
                                .and_then(|u| u.as_str())
                                .map(ToString::to_string);
                            let cmdr_name = val
                                .get("commander_name")
                                .and_then(|c| c.as_str())
                                .map(ToString::to_string);
                            let _ = events.try_send(SourceEvent::Action {
                                name: "accounts.set_settings".to_string(),
                                args: json!({
                                    "inara_has_key": true,
                                    "inara_enabled": true,
                                    "inara_url": inara_url,
                                    "inara_commander_name": cmdr_name,
                                    "inara_verified": true,
                                }),
                            });
                        }
                        return Ok(val);
                    }
                    Err(err) => {
                        if let Some(events) = events {
                            let _ = events.try_send(SourceEvent::Action {
                                name: "accounts.set_settings".to_string(),
                                args: json!({
                                    "inara_last_error": err.clone(),
                                }),
                            });
                        }
                        return Err(ActionError::Failed(err));
                    }
                }
            }
            "inara.save_key" => {
                let inara = self.inara_service.as_ref().ok_or(ActionError::NotFound)?;
                let api_key = args
                    .get("api_key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| ActionError::Failed("missing-api-key".into()))?;
                let cmdr = args
                    .get("commander_name")
                    .and_then(|c| c.as_str())
                    .or_else(|| {
                        snapshot
                            .get("core.cmdr")
                            .and_then(|c| c.get("name"))
                            .and_then(|n| n.as_str())
                    });
                inara.save_key(api_key, cmdr).map_err(ActionError::Failed)?;

                if let Some(runtime) = &self.runtime {
                    let _ = runtime.events.try_send(SourceEvent::Action {
                        name: "accounts.set_settings".to_string(),
                        args: json!({
                            "inara_has_key": true,
                            "inara_enabled": true,
                        }),
                    });
                }
                return Ok(json!({ "ok": true }));
            }
            "inara.clear_key" => {
                let inara = self.inara_service.as_ref().ok_or(ActionError::NotFound)?;
                inara.clear_key().map_err(ActionError::Failed)?;
                if let Some(runtime) = &self.runtime {
                    let _ = runtime.events.try_send(SourceEvent::Action {
                        name: "accounts.set_settings".to_string(),
                        args: json!({
                            "inara_has_key": false,
                            "inara_verified": false,
                        }),
                    });
                }
                return Ok(json!({ "ok": true }));
            }
            "edsm.save_key" => {
                let edsm = self.edsm_service.as_ref().ok_or(ActionError::NotFound)?;
                let api_key = args
                    .get("api_key")
                    .and_then(|k| k.as_str())
                    .ok_or_else(|| ActionError::Failed("missing-api-key".into()))?;
                let cmdr = args
                    .get("commander_name")
                    .and_then(|c| c.as_str())
                    .or_else(|| {
                        snapshot
                            .get("core.cmdr")
                            .and_then(|c| c.get("name"))
                            .and_then(|n| n.as_str())
                    });
                edsm.save_key(api_key, cmdr).map_err(ActionError::Failed)?;

                if let Some(runtime) = &self.runtime {
                    let _ = runtime.events.try_send(SourceEvent::Action {
                        name: "accounts.set_settings".to_string(),
                        args: json!({
                            "edsm_has_key": true,
                            "edsm_enabled": true,
                        }),
                    });
                }
                return Ok(json!({ "ok": true }));
            }
            "edsm.clear_key" => {
                let edsm = self.edsm_service.as_ref().ok_or(ActionError::NotFound)?;
                edsm.clear_key().map_err(ActionError::Failed)?;
                if let Some(runtime) = &self.runtime {
                    let _ = runtime.events.try_send(SourceEvent::Action {
                        name: "accounts.set_settings".to_string(),
                        args: json!({
                            "edsm_has_key": false,
                            "edsm_enabled": false,
                        }),
                    });
                }
                return Ok(json!({ "ok": true }));
            }
            "tts.voices" => {
                let engine_kind = args
                    .get("engine")
                    .and_then(|e| e.as_str())
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
                    tmc_model::tts::TtsEngineKind::Native => tmc_tts::NativeEngine::new()
                        .map(|e| e.available_voices())
                        .unwrap_or_default(),
                };

                return Ok(json!({ "voices": voices }));
            }
            "tts.test" => {
                let (engine_kind, default_voice, default_vol, default_rate) = {
                    let cfg = self.config.lock().unwrap();
                    (
                        cfg.tts.engine,
                        cfg.tts.voice.clone(),
                        cfg.tts.volume,
                        cfg.tts.rate,
                    )
                };

                let chosen_engine = args
                    .get("engine")
                    .and_then(|e| e.as_str())
                    .map(|s| match s {
                        "kokoro" => tmc_model::tts::TtsEngineKind::Kokoro,
                        _ => tmc_model::tts::TtsEngineKind::Native,
                    })
                    .unwrap_or(engine_kind);

                let voice = args
                    .get("voice")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .or(default_voice);

                let volume = args
                    .get("volume")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u8)
                    .unwrap_or(default_vol);

                let rate = args
                    .get("rate")
                    .and_then(|r| r.as_u64())
                    .map(|r| r as u8)
                    .unwrap_or(default_rate);

                let text = args
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("Copiloto de TMC activo y listo para la navegación.");

                let rate_f32 = rate as f32 / 100.0;

                match chosen_engine {
                    tmc_model::tts::TtsEngineKind::Kokoro => {
                        let mut guard = self.kokoro_engine.lock().unwrap();
                        if guard.is_none() {
                            *guard = tmc_tts::KokoroEngine::new().ok();
                        }
                        if let Some(ref mut engine) = *guard {
                            engine
                                .speak(text, voice.as_deref(), volume, rate_f32)
                                .map_err(|e| ActionError::Failed(e.to_string()))?;
                        } else {
                            return Err(ActionError::Failed("KokoroEngine unavailable".into()));
                        }
                    }
                    tmc_model::tts::TtsEngineKind::Native => {
                        let mut engine = tmc_tts::NativeEngine::new()
                            .map_err(|e| ActionError::Failed(e.to_string()))?;
                        engine
                            .speak(text, voice.as_deref(), volume, rate_f32)
                            .map_err(|e| ActionError::Failed(e.to_string()))?;
                    }
                }

                return Ok(json!({ "ok": true }));
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
                if text.is_empty() {
                    return Ok(json!({ "ok": true, "spoken": false }));
                }

                let rate_f32 = default_rate as f32 / 100.0;
                match engine_kind {
                    tmc_model::tts::TtsEngineKind::Kokoro => {
                        if let Ok(mut engine) = tmc_tts::KokoroEngine::new() {
                            let _ =
                                engine.speak(text, default_voice.as_deref(), default_vol, rate_f32);
                        }
                    }
                    tmc_model::tts::TtsEngineKind::Native => {
                        if let Ok(mut engine) = tmc_tts::NativeEngine::new() {
                            let _ =
                                engine.speak(text, default_voice.as_deref(), default_vol, rate_f32);
                        }
                    }
                }
                return Ok(json!({ "ok": true, "spoken": true }));
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
            Ok(json!({
                "ok": true,
                "token_type": token.token_type,
                "expires_in": token.expires_in,
            }))
        })
    }
}

const MODULE_PREFIXES: [&str; 15] = [
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
    "inara.",
    "edsm.",
];

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
}

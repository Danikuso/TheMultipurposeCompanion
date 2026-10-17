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
use tokio::sync::mpsc;

pub struct Clipboard {
    last: Mutex<Option<String>>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            last: Mutex::new(None),
        }
    }

    pub fn set_text(&self, text: &str) -> Result<(), String> {
        if let Ok(mut guard) = self.last.lock() {
            *guard = Some(text.to_owned());
        }
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.to_owned())) {
            Ok(()) => Ok(()),
            Err(err) => {
                tracing::warn!(%err, "system clipboard unavailable; stored in memory fallback");
                Ok(())
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
    /// Interruptor vivo de la copia automática (lo lee la tarea de copia).
    auto_copy: Option<Arc<AtomicBool>>,
}

pub struct AppActions {
    clipboard: Clipboard,
    config: Mutex<Config>,
    runtime: Option<Runtime>,
    save_path: Option<PathBuf>,
}

impl AppActions {
    pub fn new() -> Self {
        Self {
            clipboard: Clipboard::new(),
            config: Mutex::new(Config::default()),
            runtime: None,
            save_path: None,
        }
    }

    pub fn with_config(cfg: Config) -> Self {
        Self {
            clipboard: Clipboard::new(),
            config: Mutex::new(cfg),
            runtime: None,
            save_path: None,
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
                auto_copy: None,
            }),
            ..Self::with_config(cfg)
        }
    }

    /// Comparte el interruptor de la copia automática: `put_config` lo actualiza.
    pub fn with_auto_copy_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        if let Some(runtime) = &mut self.runtime {
            runtime.auto_copy = Some(flag);
        }
        self
    }

    /// Guarda la configuración en `path` en vez de en la ruta de usuario (tests).
    pub fn with_save_path(mut self, path: PathBuf) -> Self {
        self.save_path = Some(path);
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
        if updated.server.lan {
            updated.ensure_lan_token();
        }
        let _ = match &self.save_path {
            Some(path) => updated.save_to(path),
            None => updated.save(),
        };
        if let Some(runtime) = &self.runtime {
            runtime
                .online
                .store(updated.online_services, Ordering::Relaxed);
            if let Some(flag) = &runtime.auto_copy {
                flag.store(updated.auto_copy_next_system, Ordering::Relaxed);
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
                    .or_else(|| pick("core.route", "next_system"))
                    .ok_or_else(|| ActionError::Failed("no-next-system".into()))?;
                self.clipboard.set_text(next).map_err(ActionError::Failed)?;
                Ok(json!({ "system": next }))
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
}

const MODULE_PREFIXES: [&str; 3] = ["navigation.", "exploration.", "exobiology."];

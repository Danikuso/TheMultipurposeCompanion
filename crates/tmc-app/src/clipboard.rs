//! Portapapeles y acciones de aplicación (`ActionHost`).

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;
use tmc_server::{ActionError, ActionHost};

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

pub struct AppActions {
    clipboard: Clipboard,
}

impl AppActions {
    pub fn new() -> Self {
        Self {
            clipboard: Clipboard::new(),
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

impl ActionHost for AppActions {
    fn run_action(
        &self,
        name: &str,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError> {
        match name {
            "clipboard.copy-next-system" => {
                let next = snapshot
                    .get("core.route")
                    .and_then(|r| r.get("next_system"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| ActionError::Failed("no-next-system".into()))?;
                self.clipboard.set_text(next).map_err(ActionError::Failed)?;
                Ok(json!({ "system": next }))
            }
            _ => Err(ActionError::NotFound),
        }
    }
}

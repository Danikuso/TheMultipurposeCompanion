//! Servicio de persistencia y gestión de credenciales para EDSM.

use tmc_store::{AccountCredentialRecord, Store};

pub const EDSM_SERVICE_KEY: &str = "edsm";

pub struct EdsmService {
    store: Store,
}

impl EdsmService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// Obtiene la clave API guardada en SQLite si existe.
    pub fn get_saved_key(&self) -> Option<String> {
        self.store
            .get_account_credential(EDSM_SERVICE_KEY)
            .ok()
            .flatten()
            .and_then(|r| r.api_key)
            .filter(|k| !k.trim().is_empty())
    }

    /// Obtiene el nombre de comandante guardado si existe.
    pub fn get_saved_cmdr(&self) -> Option<String> {
        self.store
            .get_account_credential(EDSM_SERVICE_KEY)
            .ok()
            .flatten()
            .and_then(|r| r.cmdr_name)
            .filter(|c| !c.trim().is_empty())
    }

    /// Guarda o actualiza la clave API de EDSM en SQLite.
    pub fn save_key(&self, key: &str, cmdr_name: Option<&str>) -> Result<(), String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err("La clave API de EDSM no puede estar vacía".into());
        }
        let record = AccountCredentialRecord {
            service: EDSM_SERVICE_KEY.to_string(),
            api_key: Some(trimmed.to_string()),
            cmdr_name: cmdr_name.map(|n| n.trim().to_string()),
            token_json: None,
            salt: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.store
            .set_account_credential(&record)
            .map_err(|e| e.to_string())
    }

    /// Elimina las credenciales de EDSM guardadas en SQLite.
    pub fn clear_key(&self) -> Result<(), String> {
        self.store
            .delete_account_credential(EDSM_SERVICE_KEY)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_clears_edsm_key() {
        let store = Store::open_in_memory().unwrap();
        let service = EdsmService::new(store);

        assert_eq!(service.get_saved_key(), None);
        service
            .save_key("edsm_secret_123", Some("Danikuso"))
            .unwrap();
        assert_eq!(service.get_saved_key().as_deref(), Some("edsm_secret_123"));
        assert_eq!(service.get_saved_cmdr().as_deref(), Some("Danikuso"));

        service.clear_key().unwrap();
        assert_eq!(service.get_saved_key(), None);
        assert_eq!(service.get_saved_cmdr(), None);
    }
}

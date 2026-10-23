//! Almacenamiento seguro de credenciales y tokens en SQLite.

use crate::{Store, StoreError};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Registro persistente de credenciales de una cuenta o servicio externo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCredentialRecord {
    pub service: String,
    pub api_key: Option<String>,
    pub cmdr_name: Option<String>,
    pub token_json: Option<String>,
    pub salt: Option<String>,
    pub updated_at: String,
}

impl Store {
    /// Obtiene las credenciales almacenadas para un servicio específico.
    pub fn get_account_credential(
        &self,
        service: &str,
    ) -> Result<Option<AccountCredentialRecord>, StoreError> {
        let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
        let mut stmt = conn.prepare(
            "SELECT service, api_key, cmdr_name, token_json, salt, updated_at FROM accounts_credential WHERE service = ?1",
        )?;
        let mut rows = stmt.query([service])?;
        if let Some(row) = rows.next()? {
            Ok(Some(AccountCredentialRecord {
                service: row.get(0)?,
                api_key: row.get(1)?,
                cmdr_name: row.get(2)?,
                token_json: row.get(3)?,
                salt: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Guarda o actualiza las credenciales de un servicio.
    pub fn set_account_credential(
        &self,
        record: &AccountCredentialRecord,
    ) -> Result<(), StoreError> {
        let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
        conn.execute(
            "INSERT INTO accounts_credential (service, api_key, cmdr_name, token_json, salt, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(service) DO UPDATE SET
               api_key = excluded.api_key,
               cmdr_name = excluded.cmdr_name,
               token_json = excluded.token_json,
               salt = excluded.salt,
               updated_at = excluded.updated_at",
            rusqlite::params![
                record.service,
                record.api_key,
                record.cmdr_name,
                record.token_json,
                record.salt,
                record.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Elimina las credenciales de un servicio (p. ej. desvincular cuenta).
    pub fn delete_account_credential(&self, service: &str) -> Result<bool, StoreError> {
        let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
        let rows = conn.execute(
            "DELETE FROM accounts_credential WHERE service = ?1",
            [service],
        )?;
        Ok(rows > 0)
    }

    /// Obtiene la sal local para EDDN (`service = "eddn"`) o crea una nueva determinista y persistente.
    pub fn get_or_create_eddn_salt(&self) -> Result<String, StoreError> {
        if let Some(s) = self
            .get_account_credential("eddn")?
            .and_then(|r| r.salt)
            .filter(|s| !s.is_empty())
        {
            return Ok(s);
        }
        let new_salt = format!("{:x}", uuid::Uuid::new_v4().as_u128());
        let record = AccountCredentialRecord {
            service: "eddn".to_string(),
            api_key: None,
            cmdr_name: None,
            token_json: None,
            salt: Some(new_salt.clone()),
            updated_at: Utc::now().to_rfc3339(),
        };
        self.set_account_credential(&record)?;
        Ok(new_salt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_store() -> Store {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(crate::migrations::BASE_SCHEMA).unwrap();
        conn.execute_batch(crate::migrations::PRICES_SCHEMA)
            .unwrap();
        conn.execute_batch(crate::migrations::FULL_SNAPSHOTS_SCHEMA)
            .unwrap();
        conn.execute_batch(crate::migrations::ACCOUNTS_SCHEMA)
            .unwrap();
        Store {
            connection: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        }
    }

    #[test]
    fn persists_and_retrieves_credentials() {
        let store = in_memory_store();

        assert_eq!(store.get_account_credential("inara").unwrap(), None);

        let rec = AccountCredentialRecord {
            service: "inara".to_string(),
            api_key: Some("inara_key_123".to_string()),
            cmdr_name: Some("Danikuso".to_string()),
            token_json: None,
            salt: None,
            updated_at: "2026-10-23T12:00:00Z".to_string(),
        };
        store.set_account_credential(&rec).unwrap();

        let loaded = store.get_account_credential("inara").unwrap().unwrap();
        assert_eq!(loaded.api_key.as_deref(), Some("inara_key_123"));
        assert_eq!(loaded.cmdr_name.as_deref(), Some("Danikuso"));

        // Update
        let updated = AccountCredentialRecord {
            service: "inara".to_string(),
            api_key: Some("new_key".to_string()),
            cmdr_name: Some("Danikuso".to_string()),
            token_json: None,
            salt: None,
            updated_at: "2026-10-23T13:00:00Z".to_string(),
        };
        store.set_account_credential(&updated).unwrap();
        let loaded2 = store.get_account_credential("inara").unwrap().unwrap();
        assert_eq!(loaded2.api_key.as_deref(), Some("new_key"));

        // Delete
        assert!(store.delete_account_credential("inara").unwrap());
        assert_eq!(store.get_account_credential("inara").unwrap(), None);
    }

    #[test]
    fn get_or_create_eddn_salt_is_persistent() {
        let store = in_memory_store();

        let salt1 = store.get_or_create_eddn_salt().unwrap();
        assert!(!salt1.is_empty());

        let salt2 = store.get_or_create_eddn_salt().unwrap();
        assert_eq!(salt1, salt2); // Returns identical salt once created
    }
}

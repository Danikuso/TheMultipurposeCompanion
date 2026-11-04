//! Perfiles de disposición de tarjetas de la UI, uno por dispositivo.

use crate::{Store, StoreError};
use chrono::Utc;

/// Resumen de un perfil (sin el documento) para listados.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct UiLayoutSummary {
    pub device_id: String,
    pub name: String,
    pub updated_at: String,
}

/// Perfil completo con su documento de disposición.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct UiLayoutProfile {
    pub device_id: String,
    pub name: String,
    pub updated_at: String,
    pub doc: serde_json::Value,
}

impl Store {
    /// Lista los perfiles ordenados del más reciente al más antiguo, sin `doc`.
    pub fn ui_layout_list(&self) -> Result<Vec<UiLayoutSummary>, StoreError> {
        let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
        let mut stmt = conn.prepare(
            "SELECT device_id, name, updated_at FROM ui_layout_profile ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(UiLayoutSummary {
                device_id: row.get(0)?,
                name: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Obtiene un perfil; error si su documento almacenado está corrupto.
    pub fn ui_layout_get(&self, device_id: &str) -> Result<Option<UiLayoutProfile>, StoreError> {
        let row = {
            let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
            let mut stmt = conn.prepare(
                "SELECT device_id, name, updated_at, doc FROM ui_layout_profile WHERE device_id = ?1",
            )?;
            let mut rows = stmt.query([device_id])?;
            match rows.next()? {
                Some(r) => Some((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                )),
                None => None,
            }
        };
        let found = row.is_some();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.ui_layout.get",
            serde_json::json!({"found":found})
        );
        let Some((device_id, name, updated_at, doc)) = row else {
            return Ok(None);
        };
        let doc = serde_json::from_str(&doc)
            .map_err(|_| StoreError::Unsupported("corrupt ui layout document".into()))?;
        Ok(Some(UiLayoutProfile {
            device_id,
            name,
            updated_at,
            doc,
        }))
    }

    /// Crea o sobrescribe el perfil del dispositivo (upsert) y renueva `updated_at`.
    pub fn ui_layout_put(
        &self,
        device_id: &str,
        name: &str,
        doc: &serde_json::Value,
    ) -> Result<UiLayoutProfile, StoreError> {
        let text = serde_json::to_string(doc)
            .map_err(|_| StoreError::Unsupported("unserializable ui layout document".into()))?;
        let updated_at = Utc::now().to_rfc3339();
        {
            let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
            conn.execute(
                "INSERT INTO ui_layout_profile (device_id, name, updated_at, doc)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(device_id) DO UPDATE SET
                   name = excluded.name,
                   updated_at = excluded.updated_at,
                   doc = excluded.doc",
                rusqlite::params![device_id, name, updated_at, text],
            )?;
        }
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.ui_layout.put",
            serde_json::json!({"doc_bytes":text.len()})
        );
        Ok(UiLayoutProfile {
            device_id: device_id.to_string(),
            name: name.to_string(),
            updated_at,
            doc: doc.clone(),
        })
    }

    /// Renombra un perfil; `false` si no existe.
    pub fn ui_layout_rename(&self, device_id: &str, name: &str) -> Result<bool, StoreError> {
        let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
        let rows = conn.execute(
            "UPDATE ui_layout_profile SET name = ?2 WHERE device_id = ?1",
            rusqlite::params![device_id, name],
        )?;
        Ok(rows > 0)
    }

    /// Elimina un perfil; `false` si no existe.
    pub fn ui_layout_delete(&self, device_id: &str) -> Result<bool, StoreError> {
        let removed = {
            let conn = self.connection.lock().map_err(|_| StoreError::Poisoned)?;
            conn.execute(
                "DELETE FROM ui_layout_profile WHERE device_id = ?1",
                [device_id],
            )? > 0
        };
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Sql,
            "store.ui_layout.delete",
            serde_json::json!({"removed":removed})
        );
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use serde_json::json;

    const ID_A: &str = "6f1c0b9e-0000-4000-8000-000000000001";
    const ID_B: &str = "6f1c0b9e-0000-4000-8000-000000000002";

    #[test]
    fn ui_layout_migration_creates_table_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        drop(Store::open(&path).unwrap());
        let store = Store::open(&path).unwrap();
        assert!(store.ui_layout_list().unwrap().is_empty());
    }

    #[test]
    fn ui_layout_put_then_get_roundtrips() {
        let store = Store::open_in_memory().unwrap();
        let doc = json!({"version":1,"tabs":{}});
        store.ui_layout_put(ID_A, "PC", &doc).unwrap();
        let got = store.ui_layout_get(ID_A).unwrap().unwrap();
        assert_eq!(got.name, "PC");
        assert_eq!(got.doc, doc);
        assert!(chrono::DateTime::parse_from_rfc3339(&got.updated_at).is_ok());
        assert!(store.ui_layout_get(ID_B).unwrap().is_none());
    }

    #[test]
    fn ui_layout_put_overwrites_and_bumps_updated_at() {
        let store = Store::open_in_memory().unwrap();
        store.ui_layout_put(ID_A, "PC", &json!({"a":1})).unwrap();
        let second = store
            .ui_layout_put(ID_A, "Tablet", &json!({"a":2}))
            .unwrap();
        let list = store.ui_layout_list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Tablet");
        assert_eq!(list[0].updated_at, second.updated_at);
        assert_eq!(
            store.ui_layout_get(ID_A).unwrap().unwrap().doc,
            json!({"a":2})
        );
    }

    #[test]
    fn ui_layout_list_omits_doc_and_orders_by_recent() {
        let store = Store::open_in_memory().unwrap();
        store.ui_layout_put(ID_A, "PC", &json!({})).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        store.ui_layout_put(ID_B, "Tablet", &json!({})).unwrap();
        let list = store.ui_layout_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].device_id, ID_B);
        assert_eq!(list[1].device_id, ID_A);
    }

    #[test]
    fn ui_layout_rename_and_delete_report_existence() {
        let store = Store::open_in_memory().unwrap();
        store.ui_layout_put(ID_A, "PC", &json!({})).unwrap();
        assert!(store.ui_layout_rename(ID_A, "Sobremesa").unwrap());
        assert!(!store.ui_layout_rename(ID_B, "X").unwrap());
        assert_eq!(
            store.ui_layout_get(ID_A).unwrap().unwrap().name,
            "Sobremesa"
        );
        assert!(store.ui_layout_delete(ID_A).unwrap());
        assert!(!store.ui_layout_delete(ID_A).unwrap());
    }

    #[test]
    fn ui_layout_get_with_corrupt_doc_errors_without_panic() {
        let store = Store::open_in_memory().unwrap();
        store
            .connection
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO ui_layout_profile (device_id, name, updated_at, doc) VALUES (?1, 'PC', 'x', '{no json')",
                [ID_A],
            )
            .unwrap();
        assert!(store.ui_layout_get(ID_A).is_err());
    }
}

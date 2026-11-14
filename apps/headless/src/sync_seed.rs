//! Sembrado seguro de una base temporal para medir huella con servicios activos (PD-11).

use chrono::{SecondsFormat, Utc};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use tmc_model::sync::{KeyAssignment, SyncService};
use tmc_store::{Store, SyncCredential};
use tmc_sync::edsm::EDSM_DISCARD_CACHE_KEY;
use tmc_sync::router::EDDN_ENABLED_KEY;

const FOOTPRINT_DIR_PREFIX: &str = "tmc-footprint-data-";

/// Punto de entrada del subcomando `tmc-headless sync-seed --db <ruta> [--from <fixture>]`.
pub fn run<I>(mut args: I) -> anyhow::Result<()>
where
    I: Iterator<Item = String>,
{
    let mut db: Option<PathBuf> = None;
    let mut from: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = args.next().map(PathBuf::from),
            "--from" => from = args.next().map(PathBuf::from),
            other => anyhow::bail!("argumento no reconocido en sync-seed: {other}"),
        }
    }
    let db = db.ok_or_else(|| anyhow::anyhow!("falta --db <ruta> en sync-seed"))?;
    seed_sync_db(&db, from.as_deref())
}

/// Comprueba que `db_path` resida dentro de un directorio `tmc-footprint-data-*` en `%TEMP%`.
pub(crate) fn validate_footprint_db_path(db_path: &Path) -> anyhow::Result<PathBuf> {
    if db_path.as_os_str().is_empty() {
        anyhow::bail!("ruta de BD vacía");
    }
    if db_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        anyhow::bail!("sync-seed rechaza rutas con '..': {}", db_path.display());
    }
    let file_name = db_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("nombre de archivo de BD inválido"))?;
    if !file_name.ends_with(".db") || file_name == ".db" {
        anyhow::bail!("la base de sync-seed debe terminar en .db: {file_name}");
    }
    let parent = db_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| anyhow::anyhow!("la ruta de BD debe tener directorio padre"))?;
    let dir_name = parent
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("directorio padre inválido"))?;
    let suffix = dir_name.strip_prefix(FOOTPRINT_DIR_PREFIX).ok_or_else(|| {
        anyhow::anyhow!(
            "sync-seed solo acepta bases dentro de un directorio {FOOTPRINT_DIR_PREFIX}*"
        )
    })?;
    if suffix.is_empty()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        anyhow::bail!("sufijo de directorio de huella inválido: {dir_name}");
    }

    let canon_temp = std::fs::canonicalize(std::env::temp_dir())?;
    let mut existing = Some(parent);
    while let Some(cur) = existing {
        if cur.exists() {
            let meta = std::fs::symlink_metadata(cur)?;
            if meta.file_type().is_symlink() {
                anyhow::bail!("sync-seed rechaza enlaces simbólicos: {}", cur.display());
            }
            let canon_cur = std::fs::canonicalize(cur)?;
            if !canon_cur.starts_with(&canon_temp) {
                anyhow::bail!(
                    "sync-seed solo permite escribir dentro de %TEMP%: {}",
                    canon_cur.display()
                );
            }
            return Ok(db_path.to_path_buf());
        }
        existing = cur.parent().filter(|p| !p.as_os_str().is_empty());
    }
    anyhow::bail!(
        "sync-seed no puede verificar un ancestro existente bajo %TEMP%: {}",
        db_path.display()
    )
}

/// Puebla la BD temporal indicada con credenciales ficticias, EDDN encendido y caché `/discard`.
pub(crate) fn seed_sync_db(db_path: &Path, from_fixture: Option<&Path>) -> anyhow::Result<()> {
    let validated = validate_footprint_db_path(db_path)?;
    if let Some(parent) = validated.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let (fid, name) = match from_fixture {
        Some(path) => extract_cmdr_from_fixture(path)?,
        None => ("F0000001".to_string(), "CMDR TESTER".to_string()),
    };

    let store = Store::open(&validated)?;
    let now = Utc::now();
    let now_s = now.to_rfc3339_opts(SecondsFormat::Secs, true);

    store.upsert_known_cmdr(&fid, &name, now)?;
    store.get_or_create_eddn_salt()?;
    store.kv_set(EDDN_ENABLED_KEY, &now_s);
    store.set_activation(SyncService::Eddn, &fid, now)?;

    for service in [SyncService::Edsm, SyncService::Inara] {
        store.put_sync_credential(&SyncCredential {
            service,
            cmdr_fid: fid.clone(),
            cmdr_name: name.clone(),
            api_key: format!("FOOTPRINT_DUMMY_{}", service.as_str().to_uppercase()),
            created_at: now_s.clone(),
            verified_at: Some(now_s.clone()),
            assignment: KeyAssignment::Confirmed,
        })?;
        store.set_activation(service, &fid, now)?;
    }

    store.api_cache_put(EDSM_DISCARD_CACHE_KEY, r#"["Music"]"#, now)?;
    Ok(())
}

fn extract_cmdr_from_fixture(path: &Path) -> anyhow::Result<(String, String)> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut fallback = None;
    for line in reader.lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let event = v.get("event").and_then(Value::as_str);
        if matches!(event, Some("LoadGame" | "Commander")) {
            let fid = v.get("FID").and_then(Value::as_str).unwrap_or("");
            let name = v
                .get("Commander")
                .or_else(|| v.get("Name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if !fid.is_empty() && !name.is_empty() {
                return Ok((fid.to_string(), name.to_string()));
            }
            if fallback.is_none() && !fid.is_empty() {
                fallback = Some((fid.to_string(), "CMDR TESTER".to_string()));
            }
        }
    }
    Ok(fallback.unwrap_or_else(|| ("F0000001".to_string(), "CMDR TESTER".to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_seed_refuses_db_outside_footprint_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bad_plain = tmp.path().join("tmc.db");
        let bad_other = tmp.path().join("other-dir").join("tmc.db");
        let bad_traversal = tmp
            .path()
            .join("tmc-footprint-data-1234abcd")
            .join("..")
            .join("tmc.db");
        let bad_outside = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tmc-footprint-data-fake")
            .join("tmc.db");

        for bad in [&bad_plain, &bad_other, &bad_traversal, &bad_outside] {
            assert!(
                seed_sync_db(bad, None).is_err(),
                "expected refusal for {}",
                bad.display()
            );
        }

        let valid_dir = tmp
            .path()
            .join("tmc-footprint-data-0123456789abcdef0123456789abcdef");
        std::fs::create_dir_all(&valid_dir).unwrap();
        let valid = valid_dir.join("tmc.db");
        seed_sync_db(&valid, None).unwrap();

        let store = Store::open(&valid).unwrap();
        assert!(
            store
                .sync_credential(SyncService::Inara, "F0000001")
                .unwrap()
                .is_some()
        );
        assert!(
            store
                .sync_credential(SyncService::Edsm, "F0000001")
                .unwrap()
                .is_some()
        );
        assert!(store.kv_get(EDDN_ENABLED_KEY).is_some());
        assert!(
            store
                .activation(SyncService::Eddn, "F0000001")
                .unwrap()
                .is_some()
        );
        assert!(
            store
                .api_cache_get(EDSM_DISCARD_CACHE_KEY)
                .unwrap()
                .is_some()
        );
    }
}

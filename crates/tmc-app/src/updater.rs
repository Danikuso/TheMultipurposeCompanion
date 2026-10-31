//! Servicio de actualización automática de TMC.
//!
//! Comprueba si hay una nueva release en GitHub Releases, descarga el
//! instalador NSIS o binario oficial y permite ejecutarlo para actualizar la app.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/Danikuso/TheMultipurposeCompanion/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_name: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
    pub release_notes: String,
    pub published_at: Option<String>,
    pub is_checking: bool,
    pub is_downloading: bool,
    pub download_progress: f32, // 0.0 .. 1.0
    pub status_message: Option<String>,
    pub error: Option<String>,
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            latest_version: env!("CARGO_PKG_VERSION").to_string(),
            has_update: false,
            release_name: String::new(),
            release_url: String::new(),
            download_url: None,
            asset_name: None,
            asset_size: None,
            release_notes: String::new(),
            published_at: None,
            is_checking: false,
            is_downloading: false,
            download_progress: 0.0,
            status_message: None,
            error: None,
        }
    }
}

pub fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let clean = s.trim().trim_start_matches(['v', 'V']);
    let mut parts = clean.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .split('-')
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(curr), Some(lat)) => lat > curr,
        _ => false,
    }
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

pub async fn check_for_updates(state: Arc<RwLock<UpdateStatus>>) -> Result<UpdateStatus, String> {
    {
        let mut w = state.write().await;
        w.is_checking = true;
        w.error = None;
    }

    let client = match reqwest::Client::builder()
        .user_agent("The-Multipurpose-Companion-App")
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("Error al crear cliente HTTP: {e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    let resp = match client.get(GITHUB_RELEASES_API).send().await {
        Ok(r) => r,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("No se pudo conectar con GitHub Releases: {e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    if !resp.status().is_success() {
        let mut w = state.write().await;
        w.is_checking = false;
        let err = format!("GitHub Releases respondió con HTTP {}", resp.status());
        w.error = Some(err.clone());
        return Err(err);
    }

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("Error al leer respuesta de GitHub: {e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    let release: GithubRelease = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("Error al decodificar respuesta de GitHub: {e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    let current = env!("CARGO_PKG_VERSION");
    let has_update = is_newer_version(current, &release.tag_name);

    // Buscar el instalador: preferir TMC-Setup.exe o TMC_..._setup.exe
    let setup_asset = release
        .assets
        .iter()
        .find(|a| {
            let n = a.name.to_lowercase();
            n.ends_with(".exe") && (n.contains("setup") || n.contains("installer"))
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|a| a.name.to_lowercase().ends_with(".exe"))
        });

    let mut w = state.write().await;
    w.is_checking = false;
    w.current_version = current.to_string();
    w.latest_version = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    w.has_update = has_update;
    w.release_name = release.name.unwrap_or_else(|| release.tag_name.clone());
    w.release_url = release.html_url;
    w.release_notes = release.body.unwrap_or_default();
    w.published_at = release.published_at;
    if let Some(asset) = setup_asset {
        w.download_url = Some(asset.browser_download_url.clone());
        w.asset_name = Some(asset.name.clone());
        w.asset_size = Some(asset.size);
    }
    w.error = None;

    Ok(w.clone())
}

pub async fn download_and_install_update(
    state: Arc<RwLock<UpdateStatus>>,
) -> Result<String, String> {
    let (download_url, asset_name) = {
        let r = state.read().await;
        let Some(url) = r.download_url.clone() else {
            return Err("No hay URL de descarga disponible para la actualización".to_string());
        };
        (
            url,
            r.asset_name
                .clone()
                .unwrap_or_else(|| "TMC-Setup.exe".to_string()),
        )
    };

    {
        let mut w = state.write().await;
        w.is_downloading = true;
        w.download_progress = 0.0;
        w.status_message = Some("Iniciando descarga de la actualización...".to_string());
        w.error = None;
    }

    let client = reqwest::Client::builder()
        .user_agent("The-Multipurpose-Companion-App")
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let mut resp = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Error en descarga: {e}"))?;
    if !resp.status().is_success() {
        let mut w = state.write().await;
        w.is_downloading = false;
        let err = format!("HTTP {}", resp.status());
        w.error = Some(err.clone());
        return Err(err);
    }

    let total_size = resp.content_length().unwrap_or(0);
    let temp_dir = std::env::temp_dir();
    let target_file = temp_dir.join(&asset_name);

    let mut file = tokio::fs::File::create(&target_file)
        .await
        .map_err(|e| format!("No se pudo crear archivo temporal: {e}"))?;

    use tokio::io::AsyncWriteExt;
    let mut downloaded = 0u64;
    let mut last_update_time = std::time::Instant::now();

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("Error al recibir datos: {e}"))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Error al escribir archivo: {e}"))?;
        downloaded += chunk.len() as u64;
        if total_size > 0 && last_update_time.elapsed() >= std::time::Duration::from_millis(100) {
            last_update_time = std::time::Instant::now();
            let progress = (downloaded as f32 / total_size as f32).min(1.0);
            let mut w = state.write().await;
            w.download_progress = progress;
            w.status_message = Some(format!(
                "Descargando: {:.1} MB / {:.1} MB ({:.0}%)",
                downloaded as f64 / 1_048_576.0,
                total_size as f64 / 1_048_576.0,
                progress * 100.0
            ));
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Error al finalizar archivo: {e}"))?;
    drop(file);

    {
        let mut w = state.write().await;
        w.download_progress = 1.0;
        w.is_downloading = false;
        w.status_message = Some("Descarga completada. Ejecutando instalador...".to_string());
    }

    // Ejecutar el instalador de forma desacoplada
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        let spawn_res = std::process::Command::new(&target_file)
            .creation_flags(DETACHED_PROCESS)
            .spawn();

        if let Err(e) = spawn_res {
            let err = format!("No se pudo ejecutar el instalador: {e}");
            let mut w = state.write().await;
            w.error = Some(err.clone());
            return Err(err);
        }
    }

    #[cfg(not(windows))]
    {
        let spawn_res = std::process::Command::new(&target_file).spawn();
        if let Err(e) = spawn_res {
            let err = format!("No se pudo ejecutar el archivo: {e}");
            let mut w = state.write().await;
            w.error = Some(err.clone());
            return Err(err);
        }
    }

    // Programar cierre limpio de TMC en 1.5s para que el instalador pueda sobreescribir los archivos
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        std::process::exit(0);
    });

    Ok(
        "Instalador ejecutado con éxito. TMC se cerrará para completar la actualización."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parsing_and_comparison() {
        assert_eq!(parse_semver("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_semver("v1.0.1"), Some((1, 0, 1)));
        assert_eq!(parse_semver("V2.1.3-beta"), Some((2, 1, 3)));

        assert!(!is_newer_version("1.0.1", "1.0.1"));
        assert!(!is_newer_version("1.0.1", "v1.0.1"));
        assert!(is_newer_version("1.0.1", "v1.0.2"));
        assert!(is_newer_version("1.0.1", "1.1.0"));
        assert!(is_newer_version("1.0.1", "2.0.0"));
        assert!(!is_newer_version("1.0.2", "1.0.1"));
    }
}

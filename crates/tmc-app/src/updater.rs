//! Servicio de actualización automática de TMC.
//!
//! Comprueba si hay una nueva release en GitHub Releases, descarga el
//! instalador NSIS o binario oficial y permite ejecutarlo para actualizar la app.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/Danikuso/TheMultipurposeCompanion/releases/latest";

const ALLOWED_DOWNLOAD_PREFIXES: &[&str] = &[
    "https://github.com/Danikuso/TheMultipurposeCompanion/releases/download/",
    "https://objects.githubusercontent.com/",
    "https://release-assets.githubusercontent.com/",
];

const FIXED_INSTALLER_FILENAME: &str = "TMC-Setup.exe";

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
    #[serde(default)]
    pub expected_sha256: Option<String>,
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
            expected_sha256: None,
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

/// Comprueba que `url` pertenezca por HTTPS al repositorio o CDN oficial de GitHub Releases.
pub fn is_allowed_update_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty()
        || trimmed.chars().any(char::is_whitespace)
        || trimmed.contains("..")
        || trimmed.contains('\\')
    {
        return false;
    }
    let Some(rest) = trimmed.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    ALLOWED_DOWNLOAD_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn is_valid_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Extrae el hash SHA-256 esperado desde `asset.digest` (`"sha256:..."`) o las notas de la release.
pub fn extract_expected_sha256(
    asset_digest: Option<&str>,
    release_body: Option<&str>,
    asset_name: &str,
) -> Option<String> {
    if let Some(digest) = asset_digest {
        let hex = digest
            .trim()
            .strip_prefix("sha256:")
            .unwrap_or(digest.trim())
            .trim();
        if is_valid_sha256_hex(hex) {
            return Some(hex.to_ascii_lowercase());
        }
    }

    let body = release_body?;
    let target_lower = asset_name.trim().to_ascii_lowercase();
    let lines: Vec<&str> = body.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let lower = line.to_ascii_lowercase();
        let mentions_target = (!target_lower.is_empty() && lower.contains(&target_lower))
            || lower.contains("tmc-setup.exe");
        if !mentions_target {
            continue;
        }
        // Buscar un hash SHA-256 en la misma línea o en la inmediatamente siguiente.
        for candidate_line in lines.iter().skip(idx).take(2) {
            for token in candidate_line
                .split(|c: char| !c.is_ascii_hexdigit())
                .filter(|t| !t.is_empty())
            {
                if is_valid_sha256_hex(token) {
                    return Some(token.to_ascii_lowercase());
                }
            }
        }
    }
    None
}

/// Calcula el digest SHA-256 hexadecimal en minúsculas de un buffer de bytes.
pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Verifica la coincidencia exacta (insensible a mayúsculas) entre hash real y esperado.
pub fn verify_sha256_digest(actual_hex: &str, expected_hex: &str) -> Result<(), String> {
    if is_valid_sha256_hex(actual_hex)
        && is_valid_sha256_hex(expected_hex)
        && actual_hex.eq_ignore_ascii_case(expected_hex)
    {
        Ok(())
    } else {
        Err(format!(
            "update-failed\tError de integridad SHA-256: esperado {expected_hex}, obtenido {actual_hex}"
        ))
    }
}

/// Ruta local fija y saneada para el instalador descargado dentro de `%TEMP%`.
pub fn installer_target_path(temp_dir: &Path) -> PathBuf {
    temp_dir.join(FIXED_INSTALLER_FILENAME)
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
    #[serde(default)]
    digest: Option<String>,
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
            let err = format!("update-connect-failed\t{e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    let resp = match client.get(GITHUB_RELEASES_API).send().await {
        Ok(r) => r,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("update-connect-failed\t{e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    if !resp.status().is_success() {
        let mut w = state.write().await;
        w.is_checking = false;
        let err = format!("update-check-failed\tHTTP {}", resp.status());
        w.error = Some(err.clone());
        return Err(err);
    }

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("update-check-failed\t{e}");
            w.error = Some(err.clone());
            return Err(err);
        }
    };

    let release: GithubRelease = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            let mut w = state.write().await;
            w.is_checking = false;
            let err = format!("update-check-failed\t{e}");
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
    let release_body = release.body.unwrap_or_default();
    if let Some(asset) = setup_asset {
        if is_allowed_update_url(&asset.browser_download_url) {
            w.expected_sha256 =
                extract_expected_sha256(asset.digest.as_deref(), Some(&release_body), &asset.name);
            w.download_url = Some(asset.browser_download_url.clone());
            w.asset_name = Some(asset.name.clone());
            w.asset_size = Some(asset.size);
        } else {
            w.download_url = None;
            w.expected_sha256 = None;
        }
    }
    w.release_notes = release_body;
    w.published_at = release.published_at;
    w.error = None;

    Ok(w.clone())
}

async fn fail_download(state: &Arc<RwLock<UpdateStatus>>, err: String) -> Result<String, String> {
    let mut w = state.write().await;
    w.is_downloading = false;
    w.download_progress = 0.0;
    w.status_message = None;
    w.error = Some(err.clone());
    Err(err)
}

pub async fn download_and_install_update(
    state: Arc<RwLock<UpdateStatus>>,
) -> Result<String, String> {
    let (download_url, expected_sha256) = {
        let r = state.read().await;
        let Some(url) = r.download_url.clone() else {
            return Err("update-no-download".to_string());
        };
        (url, r.expected_sha256.clone())
    };

    if !is_allowed_update_url(&download_url) {
        return fail_download(
            &state,
            "update-failed\tURL de descarga no autorizada fuera del repositorio oficial."
                .to_string(),
        )
        .await;
    }

    {
        let mut w = state.write().await;
        w.is_downloading = true;
        w.download_progress = 0.0;
        w.status_message = Some("update-download-starting".to_string());
        w.error = None;
    }

    let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > 5 {
            attempt.error("demasiadas redirecciones")
        } else if is_allowed_update_url(attempt.url().as_str()) {
            attempt.follow()
        } else {
            attempt.error("redireccion fuera de origen oficial")
        }
    });

    let client = match reqwest::Client::builder()
        .user_agent("The-Multipurpose-Companion-App")
        .redirect(redirect_policy)
        .timeout(std::time::Duration::from_secs(300))
        .build()
    {
        Ok(c) => c,
        Err(e) => return fail_download(&state, format!("update-failed\t{e}")).await,
    };

    let mut resp = match client.get(&download_url).send().await {
        Ok(r) => r,
        Err(e) => return fail_download(&state, format!("update-failed\t{e}")).await,
    };
    if !resp.status().is_success() {
        return fail_download(&state, format!("update-failed\tHTTP {}", resp.status())).await;
    }
    if !is_allowed_update_url(resp.url().as_str()) {
        return fail_download(
            &state,
            "update-failed\tDestino de descarga no autorizado.".to_string(),
        )
        .await;
    }

    let total_size = resp.content_length().unwrap_or(0);
    let target_file = installer_target_path(&std::env::temp_dir());

    let mut file = match tokio::fs::File::create(&target_file).await {
        Ok(f) => f,
        Err(e) => return fail_download(&state, format!("update-failed\t{e}")).await,
    };

    use tokio::io::AsyncWriteExt;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut last_update_time = std::time::Instant::now();

    loop {
        let chunk = match resp.chunk().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(e) => {
                drop(file);
                let _ = tokio::fs::remove_file(&target_file).await;
                return fail_download(&state, format!("update-failed\t{e}")).await;
            }
        };
        hasher.update(&chunk);
        if let Err(e) = file.write_all(&chunk).await {
            drop(file);
            let _ = tokio::fs::remove_file(&target_file).await;
            return fail_download(&state, format!("update-failed\t{e}")).await;
        }
        downloaded += chunk.len() as u64;
        if total_size > 0 && last_update_time.elapsed() >= std::time::Duration::from_millis(100) {
            last_update_time = std::time::Instant::now();
            let progress = (downloaded as f32 / total_size as f32).min(1.0);
            let mut w = state.write().await;
            w.download_progress = progress;
            w.status_message = Some(format!(
                "update-download-progress\t{:.1}\t{:.1}\t{:.0}",
                downloaded as f64 / 1_048_576.0,
                total_size as f64 / 1_048_576.0,
                progress * 100.0
            ));
        }
    }

    if let Err(e) = file.flush().await {
        drop(file);
        let _ = tokio::fs::remove_file(&target_file).await;
        return fail_download(&state, format!("update-failed\t{e}")).await;
    }
    drop(file);

    let actual_sha256 = format!("{:x}", hasher.finalize());
    if let Some(expected) = expected_sha256.as_deref()
        && let Err(err) = verify_sha256_digest(&actual_sha256, expected)
    {
        let _ = tokio::fs::remove_file(&target_file).await;
        return fail_download(&state, err).await;
    }

    {
        let mut w = state.write().await;
        w.download_progress = 1.0;
        w.is_downloading = false;
        w.status_message = Some("update-installer-started".to_string());
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
            let err = format!("update-install-failed\t{e}");
            let mut w = state.write().await;
            w.error = Some(err.clone());
            return Err(err);
        }
    }

    #[cfg(not(windows))]
    {
        let spawn_res = std::process::Command::new(&target_file).spawn();
        if let Err(e) = spawn_res {
            let err = format!("update-install-failed\t{e}");
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

    Ok("update-installer-started".to_string())
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

    #[test]
    fn test_is_allowed_update_url_accepts_official_https_and_rejects_untrusted() {
        assert!(is_allowed_update_url(
            "https://github.com/Danikuso/TheMultipurposeCompanion/releases/download/v1.4.0/TMC-Setup.exe"
        ));
        assert!(is_allowed_update_url(
            "https://objects.githubusercontent.com/github-production-release-asset-2e65be/TMC-Setup.exe"
        ));
        assert!(is_allowed_update_url(
            "https://release-assets.githubusercontent.com/github-production-release-asset/123"
        ));

        assert!(!is_allowed_update_url(
            "http://github.com/Danikuso/TheMultipurposeCompanion/releases/download/v1.4.0/TMC-Setup.exe"
        ));
        assert!(!is_allowed_update_url(
            "https://evil.example.com/Danikuso/TheMultipurposeCompanion/releases/download/v1.4.0/TMC-Setup.exe"
        ));
        assert!(!is_allowed_update_url(
            "https://user@github.com/Danikuso/TheMultipurposeCompanion/releases/download/v1.4.0/TMC-Setup.exe"
        ));
        assert!(!is_allowed_update_url(
            "https://github.com/Danikuso/TheMultipurposeCompanion/releases/download/v1.4.0/../evil.exe"
        ));
        assert!(!is_allowed_update_url(""));
    }

    #[test]
    fn test_extract_expected_sha256_from_digest_and_release_notes() {
        let hex = "ca265407eaef5eb87f0eb8998c18381bb310d48c55bcba92a0c5434c50eb22e0";
        assert_eq!(
            extract_expected_sha256(
                Some(&format!("sha256:{}", hex.to_uppercase())),
                None,
                "TMC-Setup.exe"
            ),
            Some(hex.to_string())
        );

        let body = format!(
            "## Activos\n| `TMC-Setup.exe` | 13,3 MB | `{}` |\n",
            hex.to_uppercase()
        );
        assert_eq!(
            extract_expected_sha256(None, Some(&body), "TMC-Setup.exe"),
            Some(hex.to_string())
        );

        assert_eq!(
            extract_expected_sha256(Some("sha256:not-valid"), Some("Sin hash"), "TMC-Setup.exe"),
            None
        );
    }

    #[test]
    fn test_verify_sha256_digest_matches_and_rejects_mismatch() {
        let payload = b"tmc-official-installer";
        let digest = compute_sha256_hex(payload);
        assert!(verify_sha256_digest(&digest, &digest.to_uppercase()).is_ok());

        let wrong = "0".repeat(64);
        let err = verify_sha256_digest(&digest, &wrong).unwrap_err();
        assert!(err.starts_with("update-failed\t"));
    }

    #[test]
    fn test_installer_target_path_is_fixed_tmc_setup_exe() {
        let dir = Path::new("C:\\Users\\pilot\\AppData\\Local\\Temp");
        assert_eq!(installer_target_path(dir), dir.join("TMC-Setup.exe"));
    }
}

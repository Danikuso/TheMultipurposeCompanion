//! Configuración persistente en TOML.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tmc_model::events::commodity_key;
use tmc_model::settings::{DEFAULT_MINING_INTEREST, ModuleSettings};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub port: u16,
    pub lan: bool,
    pub lan_token: Option<String>,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            port: 47320,
            lan: false,
            lan_token: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub journal_dir: Option<PathBuf>,
    pub language: String,
    pub server: ServerSection,
    pub auto_copy_next_system: bool,
    pub alerts_enabled: bool,
    /// Consultas a servicios en línea (Spansh, EDSM, Canonn). Desactivarlo no envía nada.
    pub online_services: bool,
    pub map_value_threshold: u64,
    pub default_neutron_efficiency: u8,
    /// Claves `commodity_key` de las materias de minería que interesan.
    pub mining_interest: Vec<String>,
    pub mining_threshold_cr: u64,
    pub sell_search_radius_ly: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            journal_dir: None,
            language: "es".to_owned(),
            server: ServerSection::default(),
            auto_copy_next_system: true,
            alerts_enabled: true,
            online_services: true,
            map_value_threshold: 500_000,
            default_neutron_efficiency: 60,
            mining_interest: DEFAULT_MINING_INTEREST
                .iter()
                .map(|k| (*k).to_owned())
                .collect(),
            mining_threshold_cr: 1_000_000,
            sell_search_radius_ly: 100,
            theme_mode: None,
            data_dir: None,
            ui_dir: None,
        }
    }
}

pub fn config_path() -> PathBuf {
    ProjectDirs::from("", "", "TMC")
        .map(|proj| proj.config_dir().join("config.toml"))
        .unwrap_or_else(|| std::env::temp_dir().join("tmc").join("config.toml"))
}

pub fn data_dir() -> PathBuf {
    ProjectDirs::from("", "", "TMC")
        .map(|proj| proj.data_local_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("tmc"))
}

impl Config {
    pub fn load_or_default() -> Self {
        Self::load_from_or_default(&config_path())
    }

    pub fn load_from_or_default(path: &Path) -> Self {
        if !path.exists() {
            let cfg = Self::default();
            let _ = cfg.save_to(path);
            return cfg;
        }
        match std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str::<Self>(&text).ok())
        {
            Some(mut cfg) => {
                cfg.sanitize_f2();
                cfg
            }
            None => {
                let bak = path.with_extension("toml.bak");
                let _ = std::fs::rename(path, &bak);
                let cfg = Self::default();
                let _ = cfg.save_to(path);
                cfg
            }
        }
    }

    /// Valida y normaliza los campos de F2 (rango, umbral, lista de interés).
    pub fn validate_f2(&mut self) -> Result<(), &'static str> {
        if !(10..=500).contains(&self.sell_search_radius_ly) {
            return Err("invalid-sell-search-radius");
        }
        if self.mining_threshold_cr > 1_000_000_000 {
            return Err("invalid-mining-threshold");
        }
        self.mining_interest =
            normalize_interest(&self.mining_interest).ok_or("invalid-mining-interest")?;
        Ok(())
    }

    /// Al cargar: cada campo F2 inválido vuelve a su valor por defecto, sin fallar.
    fn sanitize_f2(&mut self) {
        let d = Self::default();
        if !(10..=500).contains(&self.sell_search_radius_ly) {
            tracing::warn!(
                value = self.sell_search_radius_ly,
                "sell_search_radius_ly inválido; se usa el valor por defecto"
            );
            self.sell_search_radius_ly = d.sell_search_radius_ly;
        }
        if self.mining_threshold_cr > 1_000_000_000 {
            tracing::warn!(
                value = self.mining_threshold_cr,
                "mining_threshold_cr inválido; se usa el valor por defecto"
            );
            self.mining_threshold_cr = d.mining_threshold_cr;
        }
        match normalize_interest(&self.mining_interest) {
            Some(list) => self.mining_interest = list,
            None => {
                tracing::warn!("mining_interest inválido; se usa la lista por defecto");
                self.mining_interest = d.mining_interest;
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&config_path())
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Ajustes que se propagan en caliente a los módulos.
    pub fn module_settings(&self) -> ModuleSettings {
        ModuleSettings {
            alerts_enabled: self.alerts_enabled,
            online_services: self.online_services,
            auto_copy_next_system: self.auto_copy_next_system,
            map_value_threshold: self.map_value_threshold,
            default_neutron_efficiency: self.default_neutron_efficiency,
            mining_interest: self.mining_interest.clone(),
            mining_threshold_cr: self.mining_threshold_cr,
            sell_search_radius_ly: self.sell_search_radius_ly,
        }
    }

    pub fn ensure_lan_token(&mut self) -> String {
        if let Some(existing) = &self.server.lan_token
            && existing.len() == 32
            && existing.chars().all(|c| c.is_ascii_hexdigit())
        {
            return existing.clone();
        }
        let token = uuid::Uuid::new_v4().as_simple().to_string();
        self.server.lan_token = Some(token.clone());
        token
    }
}

/// Lista de interés normalizada (claves únicas) o `None` si excede los límites.
fn normalize_interest(raw: &[String]) -> Option<Vec<String>> {
    if raw.len() > 30 {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    for r in raw {
        let key = commodity_key(r);
        if key.is_empty() || key.chars().count() > 40 {
            return None;
        }
        if !out.contains(&key) {
            out.push(key);
        }
    }
    Some(out)
}

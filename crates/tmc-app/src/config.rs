//! Configuración persistente en TOML.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
            Some(cfg) => cfg,
            None => {
                let bak = path.with_extension("toml.bak");
                let _ = std::fs::rename(path, &bak);
                let cfg = Self::default();
                let _ = cfg.save_to(path);
                cfg
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

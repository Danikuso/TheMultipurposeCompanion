//! Configuración persistente en TOML.

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tmc_model::events::commodity_key;
use tmc_model::settings::{DEFAULT_ENGINEERING_ROLLS, DEFAULT_MINING_INTEREST, ModuleSettings};

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
    /// Auto-companion: la UI cambia sola de disposición y de panel de contexto.
    pub auto_companion: bool,
    pub alerts_enabled: bool,
    /// Consultas a servicios en línea (Spansh, EDSM, Canonn). Desactivarlo no envía nada.
    pub online_services: bool,
    pub map_value_threshold: u64,
    pub default_neutron_efficiency: u8,
    /// Claves `commodity_key` de las materias de minería que interesan.
    pub mining_interest: Vec<String>,
    pub mining_threshold_cr: u64,
    pub sell_search_radius_ly: u32,
    pub eddn_radius_ly: u32,
    pub tourism_reference_cr_ly: u64,
    /// Tiradas por defecto de ingeniería por grado (G1..G5), cada una en 1..=10.
    pub engineering_rolls: [u8; 5],
    /// Configuración del copiloto auditivo y avisos de voz (F7).
    pub tts: tmc_model::tts::TtsSettings,
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
            auto_companion: true,
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
            eddn_radius_ly: 200,
            tourism_reference_cr_ly: 100_000,
            engineering_rolls: DEFAULT_ENGINEERING_ROLLS,
            tts: tmc_model::tts::TtsSettings::default(),
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

#[cfg(debug_assertions)]
// Primera carga de arranque (la primera gana), consumida tras iniciar el registro.
static BOOTSTRAP_REPORT: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

#[cfg(debug_assertions)]
pub(crate) fn emit_bootstrap_report() {
    let report = BOOTSTRAP_REPORT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    if let Some(report) = report {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "config.bootstrap",
            report
        );
    }
}

impl Config {
    pub fn load_or_default() -> Self {
        Self::load_from_or_default(&config_path())
    }

    pub fn load_from_or_default(path: &Path) -> Self {
        if !path.exists() {
            let cfg = Self::default();
            let _saved = cfg.save_to(path);
            #[cfg(debug_assertions)]
            cfg.record_bootstrap(
                serde_json::json!({"missing":true,"fallback":true,"save_succeeded":_saved.is_ok()}),
            );
            return cfg;
        }
        let text = std::fs::read_to_string(path);
        #[cfg(debug_assertions)]
        let read_failed = text.is_err();
        match text
            .ok()
            .and_then(|text| toml::from_str::<Self>(&text).ok())
        {
            Some(mut cfg) => {
                #[cfg(debug_assertions)]
                let normalized = !(10..=500).contains(&cfg.sell_search_radius_ly)
                    || cfg.mining_threshold_cr > 1_000_000_000
                    || normalize_interest(&cfg.mining_interest).is_none();
                cfg.sanitize_f2();
                cfg.sanitize_f3();
                cfg.sanitize_f4();
                #[cfg(debug_assertions)]
                cfg.record_bootstrap(
                    serde_json::json!({"fallback":false,"loaded":true,"normalized":normalized}),
                );
                cfg
            }
            None => {
                let bak = path.with_extension("toml.bak");
                let _backup = std::fs::rename(path, &bak);
                let cfg = Self::default();
                let _saved = cfg.save_to(path);
                #[cfg(debug_assertions)]
                cfg.record_bootstrap(serde_json::json!({"fallback":true,"read_failed":read_failed,"parse_failed":!read_failed,"backup_succeeded":_backup.is_ok(),"save_succeeded":_saved.is_ok()}));
                cfg
            }
        }
    }

    #[cfg(any(debug_assertions, test))]
    pub(crate) fn diagnostic_snapshot(&self) -> serde_json::Value {
        serde_json::json!({"language":self.language,"port":self.server.port,"lan":self.server.lan,"online_services":self.online_services,"alerts_enabled":self.alerts_enabled,"auto_copy_next_system":self.auto_copy_next_system,"auto_companion":self.auto_companion,"map_value_threshold":self.map_value_threshold,"default_neutron_efficiency":self.default_neutron_efficiency,"mining_interest_count":self.mining_interest.len(),"mining_threshold_cr":self.mining_threshold_cr,"sell_search_radius_ly":self.sell_search_radius_ly,"eddn_radius_ly":self.eddn_radius_ly,"tourism_reference_cr_ly":self.tourism_reference_cr_ly,"engineering_rolls":self.engineering_rolls,"journal_configured":self.journal_dir.is_some(),"data_configured":self.data_dir.is_some(),"ui_configured":self.ui_dir.is_some()})
    }

    #[cfg(debug_assertions)]
    fn record_bootstrap(&self, flags: serde_json::Value) {
        let mut report = self.diagnostic_snapshot();
        if let (Some(snapshot), Some(flags)) = (report.as_object_mut(), flags.as_object()) {
            snapshot.extend(flags.clone());
        }
        BOOTSTRAP_REPORT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_or_insert(report);
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

    /// Valida los campos de F3 (radio EDDN y referencia de turismo).
    pub fn validate_f3(&self) -> Result<(), &'static str> {
        if !(20..=500).contains(&self.eddn_radius_ly) {
            return Err("invalid-eddn-radius");
        }
        if !(1_000..=10_000_000).contains(&self.tourism_reference_cr_ly) {
            return Err("invalid-tourism-reference");
        }
        Ok(())
    }

    /// Valida los campos de F4 (tiradas de ingeniería por grado, 1..=10).
    pub fn validate_f4(&self) -> Result<(), &'static str> {
        if self.engineering_rolls.iter().any(|r| !(1..=10).contains(r)) {
            return Err("invalid-engineering-rolls");
        }
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

    /// Al cargar: cada campo F3 inválido vuelve a su valor por defecto, sin fallar.
    fn sanitize_f3(&mut self) {
        let d = Self::default();
        if !(20..=500).contains(&self.eddn_radius_ly) {
            tracing::warn!(
                value = self.eddn_radius_ly,
                "eddn_radius_ly inválido; se usa el valor por defecto"
            );
            self.eddn_radius_ly = d.eddn_radius_ly;
        }
        if !(1_000..=10_000_000).contains(&self.tourism_reference_cr_ly) {
            tracing::warn!(
                value = self.tourism_reference_cr_ly,
                "tourism_reference_cr_ly inválido; se usa el valor por defecto"
            );
            self.tourism_reference_cr_ly = d.tourism_reference_cr_ly;
        }
    }

    /// Al cargar: tiradas inválidas vuelven a su valor por defecto, sin fallar.
    fn sanitize_f4(&mut self) {
        if self.validate_f4().is_err() {
            tracing::warn!("engineering_rolls inválido; se usa el valor por defecto");
            self.engineering_rolls = DEFAULT_ENGINEERING_ROLLS;
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&config_path())
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "config.save_started",
            self.diagnostic_snapshot()
        );
        let result: anyhow::Result<()> = (|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(self)?;
            std::fs::write(path, content)?;
            Ok(())
        })();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "config.save_result",
            serde_json::json!({"succeeded":result.is_ok()})
        );
        result
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
            eddn_radius_ly: self.eddn_radius_ly,
            tourism_reference_cr_ly: self.tourism_reference_cr_ly,
            engineering_rolls: self.engineering_rolls,
            tts: self.tts.clone(),
        }
    }

    pub fn ensure_lan_token(&mut self) -> String {
        if let Some(existing) = &self.server.lan_token
            && existing.len() == 32
            && existing.chars().all(|c| c.is_ascii_hexdigit())
        {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "config.lan_token.reused",
                serde_json::json!({"reused":true})
            );
            return existing.clone();
        }
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "config.lan_token.generated",
            serde_json::json!({"generated":true})
        );
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

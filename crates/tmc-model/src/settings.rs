//! Ajustes compartidos por los módulos.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleSettings {
    pub alerts_enabled: bool,
    pub online_services: bool,
    pub auto_copy_next_system: bool,
    pub map_value_threshold: u64,
    pub default_neutron_efficiency: u8,
    /// Claves `commodity_key` de las materias de minería que interesan.
    pub mining_interest: Vec<String>,
    pub mining_threshold_cr: u64,
    pub sell_search_radius_ly: u32,
    /// Radio (al. luz) de mercados EDDN que se guardan alrededor del jugador.
    pub eddn_radius_ly: u32,
    /// Referencia de turismo en cr por ano luz.
    pub tourism_reference_cr_ly: u64,
    /// Tiradas por defecto de ingeniería por grado (G1..G5) al calcular materiales.
    pub engineering_rolls: [u8; 5],
    /// Configuración del copiloto auditivo y avisos de voz (F7).
    pub tts: crate::tts::TtsSettings,
    /// Lista ordenada de IDs de layouts según prioridad de conmutación de Auto-companion (F13).
    #[serde(default = "default_auto_priorities")]
    pub auto_companion_priorities: Vec<String>,
    /// Margen de retención tras cesar una actividad activa (0..=60 s).
    #[serde(default = "default_linger_seconds")]
    pub auto_companion_linger_seconds: u32,
    /// Avisos de voz TTS al cambiar de cabina.
    #[serde(default)]
    pub auto_companion_voice_cues: bool,
    /// Efectos sonoros de interfaz (bip Glass HUD) al cambiar de cabina.
    #[serde(default = "default_true")]
    pub auto_companion_sound_cues: bool,
    /// Anticipación predictiva (aproximación a anillos, atraque, interdicción).
    #[serde(default = "default_true")]
    pub auto_companion_predictive: bool,
}

/// Disposiciones automáticas y orden estándar por defecto.
pub const DEFAULT_AUTO_PRIORITIES: &[&str] = &[
    "carrier",
    "combat",
    "colonisation",
    "mining",
    "engineering",
    "missions",
    "trade",
    "powerplay",
    "exploration",
];

pub fn default_auto_priorities() -> Vec<String> {
    DEFAULT_AUTO_PRIORITIES
        .iter()
        .map(|&s| s.to_string())
        .collect()
}

pub fn default_linger_seconds() -> u32 {
    20
}

pub fn default_true() -> bool {
    true
}

impl ModuleSettings {
    /// Sanea y normaliza la lista de prioridades de Auto-Companion:
    /// - Conserva los IDs válidos sin duplicados en el orden recibido.
    /// - Descarta identificadores desconocidos.
    /// - Añade al final cualquier layout de `DEFAULT_AUTO_PRIORITIES` que no estuviera presente.
    pub fn normalize_auto_priorities(list: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(DEFAULT_AUTO_PRIORITIES.len());
        for item in list {
            let s = item.trim();
            if DEFAULT_AUTO_PRIORITIES.contains(&s) && !result.iter().any(|r| r == s) {
                result.push(s.to_string());
            }
        }
        for &default_item in DEFAULT_AUTO_PRIORITIES {
            if !result.iter().any(|r| r == default_item) {
                result.push(default_item.to_string());
            }
        }
        result
    }
}

/// Tiradas por defecto de ingeniería por grado (G1..G5).
pub const DEFAULT_ENGINEERING_ROLLS: [u8; 5] = [1, 1, 1, 1, 4];

pub const DEFAULT_MINING_INTEREST: [&str; 11] = [
    "platinum",
    "painite",
    "lowtemperaturediamond",
    "opal",
    "alexandrite",
    "grandidierite",
    "monazite",
    "musgravite",
    "rhodplumsite",
    "serendibite",
    "benitoite",
];

impl Default for ModuleSettings {
    fn default() -> Self {
        Self {
            alerts_enabled: true,
            online_services: true,
            auto_copy_next_system: true,
            map_value_threshold: 500_000,
            default_neutron_efficiency: 60,
            mining_interest: DEFAULT_MINING_INTEREST
                .iter()
                .map(|k| k.to_string())
                .collect(),
            mining_threshold_cr: 1_000_000,
            sell_search_radius_ly: 100,
            eddn_radius_ly: 200,
            tourism_reference_cr_ly: 100_000,
            engineering_rolls: DEFAULT_ENGINEERING_ROLLS,
            tts: crate::tts::TtsSettings::default(),
            auto_companion_priorities: default_auto_priorities(),
            auto_companion_linger_seconds: default_linger_seconds(),
            auto_companion_voice_cues: false,
            auto_companion_sound_cues: true,
            auto_companion_predictive: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_settings_defaults_include_auto_companion_pro_fields() {
        let s = ModuleSettings::default();
        assert_eq!(s.auto_companion_priorities, default_auto_priorities());
        assert_eq!(s.auto_companion_linger_seconds, 20);
        assert!(!s.auto_companion_voice_cues);
        assert!(s.auto_companion_sound_cues);
        assert!(s.auto_companion_predictive);

        // Deserialización vacía usa defaults
        let parsed: ModuleSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.auto_companion_priorities, default_auto_priorities());
        assert_eq!(parsed.auto_companion_linger_seconds, 20);
        assert!(!parsed.auto_companion_voice_cues);
        assert!(parsed.auto_companion_sound_cues);
        assert!(parsed.auto_companion_predictive);
    }

    #[test]
    fn normalize_auto_priorities_rejects_unknown_and_completes_missing() {
        let custom = vec![
            "trade".to_string(),
            "invalid_role".to_string(),
            "mining".to_string(),
            "trade".to_string(), // duplicado
        ];
        let normalized = ModuleSettings::normalize_auto_priorities(&custom);
        assert_eq!(normalized[0], "trade");
        assert_eq!(normalized[1], "mining");
        assert_eq!(normalized.len(), DEFAULT_AUTO_PRIORITIES.len());
        assert!(!normalized.contains(&"invalid_role".to_string()));
        for expected in DEFAULT_AUTO_PRIORITIES {
            assert!(normalized.contains(&expected.to_string()));
        }
    }

    #[test]
    fn module_settings_f4_defaults() {
        assert_eq!(ModuleSettings::default().engineering_rolls, [1, 1, 1, 1, 4]);
        let s: ModuleSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.engineering_rolls, [1, 1, 1, 1, 4]);
    }

    #[test]
    fn module_settings_defaults() {
        let s = ModuleSettings::default();
        assert_eq!(s.map_value_threshold, 500_000);
        assert_eq!(s.default_neutron_efficiency, 60);
        assert!(s.online_services && s.auto_copy_next_system && s.alerts_enabled);
    }

    #[test]
    fn module_settings_mining_defaults() {
        let s = ModuleSettings::default();
        assert_eq!(s.mining_threshold_cr, 1_000_000);
        assert_eq!(s.sell_search_radius_ly, 100);
        assert_eq!(s.mining_interest.len(), 11);
        assert!(s.mining_interest.contains(&"platinum".to_string()));
        assert_eq!(s.map_value_threshold, 500_000);
        assert_eq!(s.default_neutron_efficiency, 60);
    }

    #[test]
    fn module_settings_f3_defaults() {
        let s = ModuleSettings::default();
        assert_eq!(s.eddn_radius_ly, 200);
        assert_eq!(s.tourism_reference_cr_ly, 100_000);
        assert_eq!(s.sell_search_radius_ly, 100);
    }
}

//! Ajustes compartidos por los módulos.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Sistema estelar determinado como Base de operaciones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_system: Option<String>,
    /// Coordenadas estelares [x, y, z] en años luz de la Base de operaciones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_system_coords: Option<[f64; 3]>,
    /// Preferencias del overlay HUD semitransparente e integrado (F24).
    #[serde(default)]
    pub overlay: OverlaySettings,
}

/// Modos visuales del overlay HUD sobre el juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    Minimal,
    #[default]
    Tactical,
}

/// Posiciones de anclaje rápido o libre del overlay en el monitor activo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayAnchor {
    TopLeft,
    TopCenter,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
    Custom,
}

/// Bloques temáticos activables dentro del overlay táctico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayBlocks {
    #[serde(default = "default_true")]
    pub navigation: bool,
    #[serde(default = "default_true")]
    pub exploration: bool,
    #[serde(default = "default_true")]
    pub exobiology: bool,
    #[serde(default = "default_true")]
    pub combat: bool,
    #[serde(default = "default_true")]
    pub mining: bool,
    #[serde(default = "default_true")]
    pub trade: bool,
    #[serde(default = "default_true")]
    pub colonisation: bool,
}

impl Default for OverlayBlocks {
    fn default() -> Self {
        Self {
            navigation: true,
            exploration: true,
            exobiology: true,
            combat: true,
            mining: true,
            trade: true,
            colonisation: true,
        }
    }
}

pub fn default_overlay_opacity() -> u8 {
    78
}

/// Roles aceptados en `OverlaySettings::pinned_role`.
pub const OVERLAY_VALID_ROLES: &[&str] = &[
    "navigation",
    "exploration",
    "exobiology",
    "combat",
    "mining",
    "trade",
    "colonisation",
    "carrier",
    "engineering",
    "missions",
    "powerplay",
];

/// Configuración persistente del overlay HUD semitransparente (F24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mode: OverlayMode,
    #[serde(default = "default_overlay_opacity")]
    pub opacity: u8,
    #[serde(default = "default_true")]
    pub click_through: bool,
    #[serde(default)]
    pub anchor: OverlayAnchor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_y: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_role: Option<String>,
    #[serde(default)]
    pub blocks: OverlayBlocks,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: OverlayMode::Tactical,
            opacity: default_overlay_opacity(),
            click_through: true,
            anchor: OverlayAnchor::TopRight,
            custom_x: None,
            custom_y: None,
            pinned_role: None,
            blocks: OverlayBlocks::default(),
        }
    }
}

impl OverlaySettings {
    /// Valida opacidad (35..=95) y rol fijado manualmente si existe.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(35..=95).contains(&self.opacity) {
            return Err("invalid-overlay-opacity");
        }
        if let Some(ref role) = self.pinned_role
            && !OVERLAY_VALID_ROLES.contains(&role.trim())
        {
            return Err("invalid-overlay-role");
        }
        Ok(())
    }

    /// Normaliza opacidad y rol fijado tras leer configuración antigua o corrupta.
    pub fn sanitize(&mut self) {
        if !(35..=95).contains(&self.opacity) {
            self.opacity = self.opacity.clamp(35, 95);
        }
        if let Some(ref role) = self.pinned_role {
            let trimmed = role.trim().to_string();
            if OVERLAY_VALID_ROLES.contains(&trimmed.as_str()) {
                self.pinned_role = Some(trimmed);
            } else {
                self.pinned_role = None;
            }
        }
    }
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
            home_system: None,
            home_system_coords: None,
            overlay: OverlaySettings::default(),
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

    #[test]
    fn overlay_settings_defaults_validation_and_sanitize() {
        let def = OverlaySettings::default();
        assert!(!def.enabled);
        assert_eq!(def.mode, OverlayMode::Tactical);
        assert_eq!(def.opacity, 78);
        assert!(def.click_through);
        assert_eq!(def.anchor, OverlayAnchor::TopRight);
        assert!(def.pinned_role.is_none());
        assert!(def.blocks.exobiology && def.blocks.navigation && def.blocks.colonisation);
        assert!(def.validate().is_ok());

        let parsed: ModuleSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.overlay, def);

        let mut bad = OverlaySettings {
            opacity: 10,
            pinned_role: Some("invalid_role".to_owned()),
            ..Default::default()
        };
        assert_eq!(bad.validate(), Err("invalid-overlay-opacity"));
        bad.opacity = 65;
        assert_eq!(bad.validate(), Err("invalid-overlay-role"));

        bad.opacity = 120;
        bad.sanitize();
        assert_eq!(bad.opacity, 95);
        assert!(bad.pinned_role.is_none());
    }
}

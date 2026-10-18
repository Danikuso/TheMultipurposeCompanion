//! Ajustes compartidos por los módulos.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

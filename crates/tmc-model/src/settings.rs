//! Ajustes compartidos por los módulos.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSettings {
    pub alerts_enabled: bool,
    pub online_services: bool,
    pub auto_copy_next_system: bool,
    pub map_value_threshold: u64,
    pub default_neutron_efficiency: u8,
}

impl Default for ModuleSettings {
    fn default() -> Self {
        Self {
            alerts_enabled: true,
            online_services: true,
            auto_copy_next_system: true,
            map_value_threshold: 500_000,
            default_neutron_efficiency: 60,
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
}

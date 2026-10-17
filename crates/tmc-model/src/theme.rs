//! Tema de color del HUD del juego.

use serde::{Serialize, Serializer};

/// Color RGB de 8 bits por canal; se serializa como `"#rrggbb"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn to_css(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_css())
    }
}

/// Colores del HUD resueltos (matriz de GraphicsConfiguration o valores por defecto).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HudTheme {
    pub primary: Rgb,
    pub secondary: Rgb,
    pub background: Rgb,
    pub danger: Rgb,
    pub is_default: bool,
}

impl Default for HudTheme {
    /// Naranja clásico del HUD de Elite Dangerous.
    fn default() -> Self {
        Self {
            primary: Rgb(255, 113, 0),
            secondary: Rgb(255, 180, 60),
            background: Rgb(11, 11, 14),
            danger: Rgb(255, 59, 48),
            is_default: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_serializes_as_css_hex() {
        assert_eq!(Rgb(255, 113, 0).to_css(), "#ff7100");
        assert_eq!(
            serde_json::to_string(&Rgb(1, 2, 255)).unwrap(),
            "\"#0102ff\""
        );
        let t = serde_json::to_value(HudTheme::default()).unwrap();
        assert_eq!(t["primary"], "#ff7100");
        assert_eq!(t["is_default"], true);
    }

    #[test]
    fn default_theme_matches_hud_base_palette() {
        let t = serde_json::to_value(HudTheme::default()).unwrap();
        assert_eq!(t["primary"], "#ff7100");
        assert_eq!(t["secondary"], "#ffb43c");
        assert_eq!(t["background"], "#0b0b0e");
        assert_eq!(t["danger"], "#ff3b30");
        assert_eq!(t["is_default"], true);
    }
}

//! Status.json del juego.

use crate::ParseError;
use bitflags::bitflags;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

bitflags! {
    /// Campo `Flags` de Status.json (bit 31 incluido: se lee sin signo).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags: u32 {
        const DOCKED = 1 << 0;
        const LANDED = 1 << 1;
        const LANDING_GEAR_DOWN = 1 << 2;
        const SHIELDS_UP = 1 << 3;
        const SUPERCRUISE = 1 << 4;
        const FLIGHT_ASSIST_OFF = 1 << 5;
        const HARDPOINTS_DEPLOYED = 1 << 6;
        const IN_WING = 1 << 7;
        const LIGHTS_ON = 1 << 8;
        const CARGO_SCOOP_DEPLOYED = 1 << 9;
        const SILENT_RUNNING = 1 << 10;
        const SCOOPING_FUEL = 1 << 11;
        const SRV_HANDBRAKE = 1 << 12;
        const SRV_TURRET_VIEW = 1 << 13;
        const SRV_TURRET_RETRACTED = 1 << 14;
        const SRV_DRIVE_ASSIST = 1 << 15;
        const FSD_MASS_LOCKED = 1 << 16;
        const FSD_CHARGING = 1 << 17;
        const FSD_COOLDOWN = 1 << 18;
        const LOW_FUEL = 1 << 19;
        const OVERHEATING = 1 << 20;
        const HAS_LAT_LONG = 1 << 21;
        const IS_IN_DANGER = 1 << 22;
        const BEING_INTERDICTED = 1 << 23;
        const IN_MAIN_SHIP = 1 << 24;
        const IN_FIGHTER = 1 << 25;
        const IN_SRV = 1 << 26;
        const HUD_ANALYSIS_MODE = 1 << 27;
        const NIGHT_VISION = 1 << 28;
        const ALTITUDE_FROM_AVERAGE_RADIUS = 1 << 29;
        const FSD_JUMP = 1 << 30;
        const SRV_HIGH_BEAM = 1 << 31;
    }
}

bitflags! {
    /// Campo `Flags2` de Status.json (Odyssey y posteriores).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags2: u32 {
        const ON_FOOT = 1 << 0;
        const IN_TAXI = 1 << 1;
        const IN_MULTICREW = 1 << 2;
        const ON_FOOT_IN_STATION = 1 << 3;
        const ON_FOOT_ON_PLANET = 1 << 4;
        const AIM_DOWN_SIGHT = 1 << 5;
        const LOW_OXYGEN = 1 << 6;
        const LOW_HEALTH = 1 << 7;
        const COLD = 1 << 8;
        const HOT = 1 << 9;
        const VERY_COLD = 1 << 10;
        const VERY_HOT = 1 << 11;
        const GLIDE_MODE = 1 << 12;
        const ON_FOOT_IN_HANGAR = 1 << 13;
        const ON_FOOT_SOCIAL_SPACE = 1 << 14;
        const ON_FOOT_EXTERIOR = 1 << 15;
        const BREATHABLE_ATMOSPHERE = 1 << 16;
        const TELEPRESENCE_MULTICREW = 1 << 17;
        const PHYSICAL_MULTICREW = 1 << 18;
        const FSD_HYPERDRIVE_CHARGING = 1 << 19;
        const SCO_ACTIVE = 1 << 20;
        const SUPERCRUISE_ASSIST = 1 << 21;
        const NPC_CREW_ACTIVE = 1 << 22;
    }
}

/// Serde de bitflags como entero sin signo, conservando bits desconocidos.
macro_rules! bits_serde {
    ($m:ident, $t:ty) => {
        mod $m {
            use super::*;
            pub fn serialize<S: Serializer>(v: &$t, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_u32(v.bits())
            }
            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<$t, D::Error> {
                // u64 acepta cualquier entero sin signo; se descartan bits por encima del 31.
                let raw = u64::deserialize(d)?;
                Ok(<$t>::from_bits_retain(raw as u32))
            }
        }
    };
}
bits_serde!(flags_serde, Flags);
bits_serde!(flags2_serde, Flags2);

/// Panel de la interfaz con foco (`GuiFocus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GuiFocus {
    #[default]
    NoFocus,
    InternalPanel,
    ExternalPanel,
    CommsPanel,
    RolePanel,
    StationServices,
    GalaxyMap,
    SystemMap,
    Orrery,
    Fss,
    Saa,
    Codex,
    Unknown(u8),
}

impl From<u8> for GuiFocus {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::NoFocus,
            1 => Self::InternalPanel,
            2 => Self::ExternalPanel,
            3 => Self::CommsPanel,
            4 => Self::RolePanel,
            5 => Self::StationServices,
            6 => Self::GalaxyMap,
            7 => Self::SystemMap,
            8 => Self::Orrery,
            9 => Self::Fss,
            10 => Self::Saa,
            11 => Self::Codex,
            n => Self::Unknown(n),
        }
    }
}

impl From<GuiFocus> for u8 {
    fn from(g: GuiFocus) -> u8 {
        match g {
            GuiFocus::NoFocus => 0,
            GuiFocus::InternalPanel => 1,
            GuiFocus::ExternalPanel => 2,
            GuiFocus::CommsPanel => 3,
            GuiFocus::RolePanel => 4,
            GuiFocus::StationServices => 5,
            GuiFocus::GalaxyMap => 6,
            GuiFocus::SystemMap => 7,
            GuiFocus::Orrery => 8,
            GuiFocus::Fss => 9,
            GuiFocus::Saa => 10,
            GuiFocus::Codex => 11,
            GuiFocus::Unknown(n) => n,
        }
    }
}

impl Serialize for GuiFocus {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8((*self).into())
    }
}

impl<'de> Deserialize<'de> for GuiFocus {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Valores fuera de u8 se saturan a 255 (desconocido).
        let raw = u64::deserialize(d)?;
        Ok(Self::from(u8::try_from(raw).unwrap_or(u8::MAX)))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fuel {
    #[serde(rename = "FuelMain")]
    pub main: f64,
    #[serde(rename = "FuelReservoir")]
    pub reservoir: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Destination {
    #[serde(rename = "System", default)]
    pub system: u64,
    #[serde(rename = "Body", default)]
    pub body: u32,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Name_Localised", default)]
    pub name_localised: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Status {
    #[serde(rename = "timestamp")]
    pub timestamp: DateTime<Utc>,
    #[serde(with = "flags_serde")]
    pub flags: Flags,
    #[serde(with = "flags2_serde", default = "Flags2::empty")]
    pub flags2: Flags2,
    #[serde(default)]
    pub pips: Option<[u8; 3]>,
    #[serde(default)]
    pub fire_group: Option<u8>,
    #[serde(default)]
    pub gui_focus: GuiFocus,
    #[serde(default)]
    pub fuel: Option<Fuel>,
    #[serde(default)]
    pub cargo: Option<f64>,
    #[serde(default)]
    pub legal_state: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    #[serde(default)]
    pub altitude: Option<f64>,
    #[serde(default)]
    pub heading: Option<f64>,
    #[serde(default)]
    pub body_name: Option<String>,
    #[serde(default)]
    pub planet_radius: Option<f64>,
    #[serde(default)]
    pub balance: Option<i64>,
    #[serde(default)]
    pub destination: Option<Destination>,
    #[serde(default)]
    pub oxygen: Option<f64>,
    #[serde(default)]
    pub health: Option<f64>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub gravity: Option<f64>,
    #[serde(default)]
    pub selected_weapon: Option<String>,
}

impl Status {
    /// Analiza el contenido de Status.json.
    pub fn parse(text: &str) -> Result<Status, ParseError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ParseError::Empty);
        }
        serde_json::from_str(text).map_err(|e| ParseError::Json(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParseError;

    #[test]
    fn parses_docked_flags() {
        let s = Status::parse(
            r#"{"timestamp":"2026-10-16T20:23:04Z","event":"Status","Flags":16842765}"#,
        )
        .unwrap();
        assert!(s.flags.contains(
            Flags::DOCKED
                | Flags::LANDING_GEAR_DOWN
                | Flags::SHIELDS_UP
                | Flags::FSD_MASS_LOCKED
                | Flags::IN_MAIN_SHIP
        ));
        assert!(s.flags2.is_empty());
        assert_eq!(s.gui_focus, GuiFocus::NoFocus);
    }
    #[test]
    fn bit31_does_not_overflow() {
        let s = Status::parse(
            r#"{"timestamp":"2026-10-16T20:23:04Z","event":"Status","Flags":2147483648}"#,
        )
        .unwrap();
        assert!(s.flags.contains(Flags::SRV_HIGH_BEAM));
    }
    #[test]
    fn parses_sco_and_on_foot_flags2() {
        let s = Status::parse(
            r#"{"timestamp":"2026-10-16T20:23:04Z","event":"Status","Flags":0,"Flags2":1048577}"#,
        )
        .unwrap();
        assert!(s.flags2.contains(Flags2::SCO_ACTIVE | Flags2::ON_FOOT));
    }
    #[test]
    fn unknown_gui_focus_is_preserved() {
        let s = Status::parse(
            r#"{"timestamp":"2026-10-16T20:23:04Z","event":"Status","Flags":0,"GuiFocus":42}"#,
        )
        .unwrap();
        assert_eq!(s.gui_focus, GuiFocus::Unknown(42));
    }
    #[test]
    fn empty_text_is_empty_error() {
        assert_eq!(Status::parse("  "), Err(ParseError::Empty));
    }
    #[test]
    fn truncated_json_is_json_error() {
        assert!(matches!(
            Status::parse(r#"{"timestamp":"2026"#),
            Err(ParseError::Json(_))
        ));
    }
    #[test]
    fn parses_fixtures() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/status/");
        for f in ["docked.json", "menu.json", "on_foot.json"] {
            let text = std::fs::read_to_string(format!("{dir}{f}")).unwrap();
            let s = Status::parse(&text).unwrap_or_else(|e| panic!("{f}: {e}"));
            if f == "on_foot.json" {
                assert!(s.oxygen.is_some());
                assert!(s.flags2.contains(Flags2::ON_FOOT));
            }
            if f == "docked.json" {
                assert_eq!(s.gui_focus, GuiFocus::StationServices);
                assert!(s.destination.is_some() && s.fuel.is_some());
            }
        }
    }
}

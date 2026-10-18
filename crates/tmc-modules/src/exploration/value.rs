//! Valor de exploración de cuerpos (FSS / DSS) y estimación de sondas.
//!
//! Las fórmulas y constantes son hechos públicos del juego (foro de Frontier,
//! «Exploration value formulae», vigentes desde la 3.3; ver
//! `docs/research/notes/player_roles_domain.md` §1). Implementación propia.

use serde::Deserialize;
use std::sync::OnceLock;
use tmc_model::events::Scan;

use crate::data::{StarSpecial, star_flags};

/// Coeficiente `q` de la fórmula de planetas.
const Q: f64 = 0.565_918_28;
const FIRST_DISCOVERY: f64 = 2.6;
const MAPPED_KNOWN: f64 = 3.333_333_3;
const FIRST_MAPPED_ONLY: f64 = 8.0956;
const FIRST_MAPPED_AND_DISCOVERED: f64 = 3.699_622_554;
const EFFICIENCY: f64 = 1.25;
const PLANET_FLOOR: f64 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyKind {
    Star { k: f64 },
    Planet { k: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueFlags {
    pub first_discovery: bool,
    pub first_mapped: bool,
    pub efficiency: bool,
    pub odyssey: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyValues {
    pub scanned: u64,
    pub mapped: Option<u64>,
}

fn star_k(class: &str) -> f64 {
    let base = class.split('_').next().unwrap_or("");
    match base {
        "N" | "H" => 22_628.0,
        "SupermassiveBlackHole" => 33.5678,
        _ if star_flags(class).special == Some(StarSpecial::WhiteDwarf) => 14_057.0,
        _ => 1_200.0,
    }
}

/// `k` de un planeta por clase del diario (constantes de la fórmula pública).
fn planet_k(class: &str, terraformable: bool) -> f64 {
    let (base, bonus) = match class {
        "Earthlike body" => (181_126.0, 0.0),
        "Water world" => (64_831.0, 116_295.0),
        "Ammonia world" => (96_932.0, 0.0),
        "Sudarsky class I gas giant" => (1_656.0, 0.0),
        "Sudarsky class II gas giant" | "High metal content body" => (9_654.0, 100_677.0),
        "Metal rich body" => (21_790.0, 65_631.0),
        _ => (300.0, 93_328.0),
    };
    if terraformable { base + bonus } else { base }
}

/// Clasifica un `Scan`; cinturones y anillos (sin clase ni tipo) devuelven `None`.
pub fn body_kind(scan: &Scan) -> Option<BodyKind> {
    if let Some(class) = scan.planet_class.as_deref().filter(|c| !c.is_empty()) {
        // «Terraformed» (ya terraformado) NO cuenta como terraformable.
        let terraformable = is_terraformable(scan);
        return Some(BodyKind::Planet {
            k: planet_k(class, terraformable),
        });
    }
    scan.star_type
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| BodyKind::Star { k: star_k(s) })
}

pub(super) fn is_terraformable(scan: &Scan) -> bool {
    matches!(
        scan.terraform_state.as_deref(),
        Some("Terraformable") | Some("Terraforming")
    )
}

fn round(v: f64) -> u64 {
    if v.is_finite() && v > 0.0 {
        v.round() as u64
    } else {
        0
    }
}

pub fn star_value(k: f64, stellar_mass: f64, first_discovery: bool) -> u64 {
    let v = k + stellar_mass * k / 66.25;
    round(if first_discovery {
        v * FIRST_DISCOVERY
    } else {
        v
    })
}

pub fn planet_base(k: f64, mass_em: f64) -> f64 {
    let m = if mass_em.is_finite() && mass_em > 0.0 {
        mass_em
    } else {
        0.0
    };
    (k + k * Q * m.powf(0.2)).max(PLANET_FLOOR)
}

pub fn planet_scanned_value(base: f64, first_discovery: bool) -> u64 {
    round(if first_discovery {
        base * FIRST_DISCOVERY
    } else {
        base
    })
}

pub fn planet_mapped_value(base: f64, f: &ValueFlags) -> u64 {
    let mut v = match (f.first_discovery, f.first_mapped) {
        (true, true) => base * FIRST_MAPPED_AND_DISCOVERED * FIRST_DISCOVERY,
        (false, true) => base * FIRST_MAPPED_ONLY,
        _ => base * MAPPED_KNOWN,
    };
    if f.efficiency {
        v *= EFFICIENCY;
    }
    if f.odyssey {
        v += (0.3 * v).max(555.0);
    }
    round(v)
}

/// Valores de un `Scan`. `mapped` es el «valor si se mapea»: asume DSS con
/// el bonus de eficiencia (x1,25) conseguido, más los multiplicadores de primer
/// descubrimiento/mapeo según `WasDiscovered`/`WasMapped` y el bonus Odyssey si
/// `odyssey`. Todo planeta o luna (gigantes gaseosos incluidos) es mapeable;
/// estrellas, cinturones y anillos no tienen valor de mapeo (`None`). Para la
/// eficiencia real usar `planet_mapped_value` con `ValueFlags`.
pub fn body_values(scan: &Scan, odyssey: bool) -> BodyValues {
    let first_discovery = scan.was_discovered == Some(false);
    let first_mapped = scan.was_mapped == Some(false);
    match body_kind(scan) {
        Some(BodyKind::Star { k }) => BodyValues {
            scanned: scan
                .stellar_mass
                .map_or(0, |m| star_value(k, m, first_discovery)),
            mapped: None,
        },
        Some(BodyKind::Planet { k }) => {
            let Some(mass) = scan.mass_em.filter(|m| m.is_finite() && *m > 0.0) else {
                return BodyValues {
                    scanned: 0,
                    mapped: None,
                };
            };
            let base = planet_base(k, mass);
            let flags = ValueFlags {
                first_discovery,
                first_mapped,
                efficiency: true,
                odyssey,
            };
            BodyValues {
                scanned: planet_scanned_value(base, first_discovery),
                mapped: Some(planet_mapped_value(base, &flags)),
            }
        }
        None => BodyValues {
            scanned: 0,
            mapped: None,
        },
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ProbeBand {
    max_radius_km: f64,
    probes: u8,
}

#[derive(Debug, Clone, Deserialize)]
struct ProbeTable {
    bands: Vec<ProbeBand>,
}

fn probe_table() -> &'static ProbeTable {
    static TABLE: OnceLock<ProbeTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/probes.json")).unwrap_or_else(|e| {
            tracing::error!(error = %e, "probes.json embebido inválido; tabla de reserva");
            ProbeTable {
                bands: vec![ProbeBand {
                    max_radius_km: f64::MAX,
                    probes: 4,
                }],
            }
        })
    })
}

/// Sondas DSS estimadas para el radio del cuerpo (en metros); mínimo 1.
pub fn estimate_probes(radius_m: f64) -> u8 {
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return 1;
    }
    let km = radius_m / 1000.0;
    let bands = &probe_table().bands;
    bands
        .iter()
        .find(|b| km <= b.max_radius_km)
        .or(bands.last())
        .map_or(1, |b| b.probes)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elw(was_discovered: bool, was_mapped: bool) -> Scan {
        Scan {
            body_name: "Sol 3".into(),
            planet_class: Some("Earthlike body".into()),
            mass_em: Some(1.0),
            was_discovered: Some(was_discovered),
            was_mapped: Some(was_mapped),
            ..Default::default()
        }
    }

    fn within(actual: u64, expected: f64, rel: f64) {
        let err = (actual as f64 - expected).abs() / expected;
        assert!(err <= rel, "{actual} vs {expected}");
    }

    #[test]
    fn earthlike_base_value() {
        assert!((planet_base(181_126.0, 1.0) - 283_628.5).abs() < 1.0);
    }

    #[test]
    fn earthlike_scanned_first_discovery() {
        let v = planet_scanned_value(283_628.5, true);
        assert!((v as f64 - 737_434.0).abs() <= 2.0, "{v}");
        assert_eq!(body_values(&elw(false, true), false).scanned, v);
    }

    #[test]
    fn earthlike_mapped_known_with_efficiency_and_odyssey() {
        let f = ValueFlags {
            first_discovery: false,
            first_mapped: false,
            efficiency: true,
            odyssey: true,
        };
        within(planet_mapped_value(283_628.5, &f), 1_536_321.0, 0.005);
    }

    #[test]
    fn first_mapping_without_first_discovery_uses_8_0956() {
        let v = body_values(&elw(true, false), false);
        within(v.mapped.unwrap(), 2_870_179.0, 0.001);
        assert_eq!(v.scanned, 283_629);
    }

    #[test]
    fn first_discovery_and_first_mapping_use_combined_multiplier() {
        let v = body_values(&elw(false, false), false);
        within(v.mapped.unwrap(), 3_410_285.0, 0.001);
    }

    #[test]
    fn body_values_mapped_assumes_efficiency_and_odyssey() {
        let v = body_values(&elw(true, true), true);
        within(v.mapped.unwrap(), 1_536_321.0, 0.005);
    }

    #[test]
    fn gas_giant_is_mappable() {
        let s = Scan {
            planet_class: Some("Sudarsky class I gas giant".into()),
            mass_em: Some(300.0),
            was_discovered: Some(true),
            was_mapped: Some(true),
            ..Default::default()
        };
        let v = body_values(&s, false);
        within(v.mapped.expect("mapeable"), 19_117.5, 0.002);
        assert_eq!(v.scanned, 4_589);
    }

    #[test]
    fn small_rocky_body_has_floor_of_500() {
        assert_eq!(planet_base(300.0, 0.0001), 500.0);
    }

    #[test]
    fn neutron_star_value() {
        assert!((star_value(22_628.0, 1.4, false) as f64 - 23_106.0).abs() <= 1.0);
    }

    #[test]
    fn g_star_value() {
        assert!((star_value(1_200.0, 1.0, false) as f64 - 1_218.0).abs() <= 1.0);
    }

    #[test]
    fn star_k_by_class() {
        let k = |t: &str| {
            body_kind(&Scan {
                star_type: Some(t.into()),
                ..Default::default()
            })
        };
        assert_eq!(k("N"), Some(BodyKind::Star { k: 22_628.0 }));
        assert_eq!(k("H"), Some(BodyKind::Star { k: 22_628.0 }));
        assert_eq!(k("DAB"), Some(BodyKind::Star { k: 14_057.0 }));
        assert_eq!(k("G"), Some(BodyKind::Star { k: 1_200.0 }));
    }

    #[test]
    fn planet_k_terraformable_variants() {
        let k = |c: &str, t: &str| {
            body_kind(&Scan {
                planet_class: Some(c.into()),
                terraform_state: Some(t.into()),
                ..Default::default()
            })
        };
        assert_eq!(
            k("Water world", "Terraformable"),
            Some(BodyKind::Planet { k: 181_126.0 })
        );
        assert_eq!(k("Water world", ""), Some(BodyKind::Planet { k: 64_831.0 }));
        assert_eq!(
            k("Rocky body", "Terraformable"),
            Some(BodyKind::Planet { k: 93_628.0 })
        );
        assert_eq!(
            k("Metal rich body", ""),
            Some(BodyKind::Planet { k: 21_790.0 })
        );
    }

    fn planet_k_of(class: &str, tf: &str) -> Option<f64> {
        let s = Scan {
            planet_class: Some(class.into()),
            terraform_state: Some(tf.into()),
            ..Default::default()
        };
        match body_kind(&s) {
            Some(BodyKind::Planet { k }) => Some(k),
            _ => None,
        }
    }

    fn fixture_k(file: &str, class: &str) -> f64 {
        let path = format!(
            "{}/../../tests/fixtures/journal/{file}",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(path).unwrap();
        let needle = format!("\"PlanetClass\":\"{class}\"");
        let line = text
            .lines()
            .find(|l| l.contains("\"event\":\"Scan\"") && l.contains(&needle))
            .unwrap();
        let scan: Scan = serde_json::from_str(line).unwrap();
        assert_eq!(scan.planet_class.as_deref(), Some(class));
        match body_kind(&scan) {
            Some(BodyKind::Planet { k }) => k,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn gas_giant_k_from_real_fixtures() {
        assert_eq!(
            fixture_k("real_02.log", "Sudarsky class II gas giant"),
            9_654.0
        );
        assert_eq!(
            fixture_k("exploration_session.log", "Sudarsky class I gas giant"),
            1_656.0
        );
    }

    #[test]
    fn planet_k_other_classes() {
        assert_eq!(planet_k_of("Ammonia world", ""), Some(96_932.0));
        assert_eq!(planet_k_of("High metal content body", ""), Some(9_654.0));
        assert_eq!(
            planet_k_of("High metal content body", "Terraformable"),
            Some(110_331.0)
        );
        for c in [
            "Helium rich gas giant",
            "Water giant",
            "Gas giant with water based life",
        ] {
            assert_eq!(planet_k_of(c, ""), Some(300.0), "{c}");
        }
        // Ya terraformado: no es terraformable.
        assert_eq!(planet_k_of("Water world", "Terraformed"), Some(64_831.0));
    }

    #[test]
    fn belt_cluster_has_no_value() {
        let s = Scan {
            body_name: "HIP 1 A Belt Cluster 1".into(),
            ..Default::default()
        };
        assert_eq!(body_kind(&s), None);
        assert_eq!(
            body_values(&s, false),
            BodyValues {
                scanned: 0,
                mapped: None
            }
        );
    }

    #[test]
    fn missing_mass_gives_zero_values() {
        let mut s = elw(false, false);
        s.mass_em = None;
        assert_eq!(
            body_values(&s, true),
            BodyValues {
                scanned: 0,
                mapped: None
            }
        );
    }

    #[test]
    fn probe_estimate_is_monotonic_in_radius() {
        let n1 = estimate_probes(500_000.0);
        let n2 = estimate_probes(3_000_000.0);
        let n3 = estimate_probes(8_000_000.0);
        assert!(n1 >= 1 && n1 <= n2 && n2 <= n3, "{n1} {n2} {n3}");
        assert_eq!((n1, n2, n3), (3, 5, 7));
        assert_eq!(estimate_probes(0.0), 1);
        assert_eq!(estimate_probes(f64::NAN), 1);
        assert_eq!(estimate_probes(1e15), 8);
    }

    #[test]
    fn probes_json_is_valid_and_monotonic() {
        let t: ProbeTable = serde_json::from_str(include_str!("../../data/probes.json")).unwrap();
        assert_eq!(t.bands.len(), 7);
        for w in t.bands.windows(2) {
            assert!(w[0].max_radius_km < w[1].max_radius_km);
            assert!(w[0].probes <= w[1].probes);
        }
        assert!(t.bands.iter().all(|b| b.probes >= 1));
    }
}

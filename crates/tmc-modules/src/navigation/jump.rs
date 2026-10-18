//! Alcance de salto y combustible por salto a partir del `Loadout`.

use crate::data::{FsdKind, FsdTable, StarSpecial};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use tmc_model::events::Loadout;

/// Tolerancia (al) al validar la fórmula contra `MaxJumpRange`.
const VALIDATION_TOLERANCE_LY: f64 = 0.05;

#[derive(Debug, Clone, PartialEq)]
pub struct FsdSpec {
    pub optimal_mass: f64,
    pub max_fuel_per_jump: f64,
    pub fuel_mul: f64,
    pub fuel_power: f64,
    pub guardian_bonus_ly: f64,
    pub kind: FsdKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipMass {
    pub unladen: f64,
    pub fuel_main: f64,
    pub fuel_reserve: f64,
    pub cargo: f64,
}

impl ShipMass {
    pub fn total(&self) -> f64 {
        self.unladen + self.fuel_main + self.fuel_reserve + self.cargo
    }
}

/// Estrella que supercarga el FSD.
pub type SuperchargeStar = StarSpecial;

/// FSD del `Loadout` con ingeniería y booster Guardian aplicados.
pub fn fsd_from_loadout(lo: &Loadout, table: &FsdTable) -> Option<FsdSpec> {
    let (module, entry) = lo.modules.iter().find_map(|m| {
        let item = m.item.as_deref()?;
        table.find_fsd(item).map(|e| (m, e))
    })?;
    let mut spec = FsdSpec {
        optimal_mass: entry.optimal_mass,
        max_fuel_per_jump: entry.max_fuel_per_jump,
        fuel_mul: entry.fuel_mul,
        fuel_power: entry.fuel_power,
        guardian_bonus_ly: 0.0,
        kind: entry.kind,
    };
    if let Some(eng) = &module.engineering {
        for m in &eng.modifiers {
            match (m.label.as_str(), m.value) {
                ("FSDOptimalMass", Some(v)) if v > 0.0 => spec.optimal_mass = v,
                ("MaxFuelPerJump", Some(v)) if v > 0.0 => spec.max_fuel_per_jump = v,
                _ => {}
            }
        }
    }
    spec.guardian_bonus_ly = lo
        .modules
        .iter()
        .filter_map(|m| m.item.as_deref())
        .find_map(|i| table.find_booster(i))
        .map_or(0.0, |b| b.bonus_ly);
    Some(spec)
}

pub fn jump_range(fsd: &FsdSpec, mass: &ShipMass) -> f64 {
    jump_range_with(fsd, mass, true)
}

fn pos(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// Sin recorte usa todo `fuel_main` (lo usa `eval` para comparar con el juego).
pub fn jump_range_with(fsd: &FsdSpec, mass: &ShipMass, clamp_to_max_fuel: bool) -> f64 {
    let fuel = if clamp_to_max_fuel {
        mass.fuel_main.min(fsd.max_fuel_per_jump)
    } else {
        mass.fuel_main
    };
    if !(pos(fuel)
        && pos(mass.total())
        && pos(fsd.optimal_mass)
        && pos(fsd.fuel_mul)
        && pos(fsd.fuel_power))
    {
        return 0.0;
    }
    (fuel / fsd.fuel_mul).powf(1.0 / fsd.fuel_power) * fsd.optimal_mass / mass.total()
        + fsd.guardian_bonus_ly
}

/// Combustible (t) de un salto; `None` si supera el máximo por salto.
pub fn fuel_for_jump(fsd: &FsdSpec, total_mass: f64, distance_ly: f64) -> Option<f64> {
    if !(distance_ly.is_finite()
        && distance_ly >= 0.0
        && pos(total_mass)
        && pos(fsd.optimal_mass)
        && pos(fsd.fuel_mul)
        && pos(fsd.fuel_power))
    {
        return None;
    }
    let base = (distance_ly - fsd.guardian_bonus_ly).max(0.0);
    let f = fsd.fuel_mul * (base * total_mass / fsd.optimal_mass).powf(fsd.fuel_power);
    (f <= fsd.max_fuel_per_jump + 1e-9).then_some(f)
}

pub fn supercharge_multiplier(kind: FsdKind, star: SuperchargeStar) -> f64 {
    match (kind, star) {
        (FsdKind::ScoMk2, StarSpecial::Neutron) => 6.0,
        (FsdKind::ScoMk2, StarSpecial::WhiteDwarf) => 3.0,
        (_, StarSpecial::Neutron) => 4.0,
        (_, StarSpecial::WhiteDwarf) => 1.5,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RangeSource {
    Formula,
    Scaled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JumpModel {
    pub source: RangeSource,
    pub fsd: Option<FsdSpec>,
    pub game_max_range: Option<f64>,
    pub laden_ref_mass: Option<f64>,
}

fn warn_once(ship_id: Option<u64>, msg: &str) {
    static SEEN: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    let id = ship_id.unwrap_or(u64::MAX);
    let first = SEEN
        .get_or_init(Default::default)
        .lock()
        .map(|mut s| s.insert(id))
        .unwrap_or(false);
    if first {
        tracing::warn!(ship_id = ?ship_id, "{msg}");
    }
}

/// Construye el modelo de alcance validando la fórmula contra `MaxJumpRange`
/// (nave sin carga y con el combustible máximo por salto a bordo).
pub fn build_jump_model(lo: &Loadout, table: &FsdTable) -> JumpModel {
    let fsd = fsd_from_loadout(lo, table);
    let game_max_range = lo.max_jump_range.filter(|r| *r > 0.0);
    let tank = lo
        .fuel_capacity
        .as_ref()
        .and_then(|f| f.main)
        .unwrap_or(0.0);
    // El juego calcula `MaxJumpRange` con solo el combustible máximo por salto
    // a bordo (observado: `MaxJumpRange * (masa_vacío + min(depósito, máx.
    // por salto))` es constante por FSD en el historial real; spec §5.1 corregida).
    let fuel_main = fsd.as_ref().map_or(tank, |f| tank.min(f.max_fuel_per_jump));
    let ref_mass = lo.unladen_mass.map(|u| ShipMass {
        unladen: u,
        fuel_main,
        fuel_reserve: 0.0,
        cargo: 0.0,
    });
    let laden_ref_mass = ref_mass.map(|m| m.total()).filter(|m| pos(*m));
    let ref_mass = ref_mass.filter(|_| laden_ref_mass.is_some());
    let model = |source| JumpModel {
        source,
        fsd: fsd.clone(),
        game_max_range,
        laden_ref_mass,
    };

    if let (Some(f), Some(m)) = (&fsd, &ref_mass) {
        match game_max_range {
            Some(g) if (jump_range(f, m) - g).abs() <= VALIDATION_TOLERANCE_LY => {
                return model(RangeSource::Formula);
            }
            Some(g) => warn_once(
                lo.ship_id,
                &format!("la fórmula de alcance no coincide con MaxJumpRange ({g}); se escala"),
            ),
            // Nada con lo que contradecir la fórmula.
            None => return model(RangeSource::Formula),
        }
    } else if game_max_range.is_some() {
        let msg = if fsd.is_none() {
            "FSD fuera de la tabla; alcance escalado desde MaxJumpRange"
        } else {
            "Loadout sin masa en vacío válida; no se puede validar el alcance"
        };
        warn_once(lo.ship_id, msg);
    }

    // Sin FSD conocido no se sabe cuánto combustible usa el juego como referencia
    // (máx. por salto): el alcance se oculta en vez de escalar con sesgo.
    if game_max_range.is_some() && laden_ref_mass.is_some() && fsd.is_some() {
        model(RangeSource::Scaled)
    } else {
        model(RangeSource::Unknown)
    }
}

impl JumpModel {
    /// Alcance con la masa actual; `None` si no se puede estimar.
    pub fn range_now(&self, mass: &ShipMass) -> Option<f64> {
        match self.source {
            RangeSource::Formula => self.fsd.as_ref().map(|f| jump_range(f, mass)),
            RangeSource::Scaled => {
                let total = mass.total();
                if !pos(total) {
                    return None;
                }
                Some(self.game_max_range? * self.laden_ref_mass? / total)
            }
            RangeSource::Unknown => None,
        }
    }
}

#[cfg(test)]
#[path = "jump_tests.rs"]
mod tests;

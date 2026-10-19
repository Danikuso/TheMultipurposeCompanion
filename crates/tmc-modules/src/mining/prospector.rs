//! Prospector: composición del asteroide, valor estimado y veredicto.
//!
//! Todos los valores son estimaciones: toneladas típicas por nivel de contenido
//! (tabla sin verificar) y mejor precio conocido localmente.

use std::collections::VecDeque;

use serde::{Serialize, Serializer};
use tmc_model::events::{AsteroidContent, ProspectedAsteroid, commodity_key};
use tmc_model::settings::ModuleSettings;

use super::prices::PriceFn;
use crate::data::mining_yield;

const HISTORY_MAX: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialRow {
    pub key: String,
    pub name: Option<String>,
    /// Porcentaje (0-100) tal como lo da el diario.
    pub proportion: f64,
    /// Valor del asteroide completo (fórmula de la spec); `Remaining` no se aplica.
    pub est_value: Option<u64>,
    pub interest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    Worth { est_value: Option<u64> },
    NotWorth,
    NoData,
}

fn ser_content<S: Serializer>(c: &AsteroidContent, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(match c {
        AsteroidContent::Low => "low",
        AsteroidContent::Medium => "medium",
        AsteroidContent::High => "high",
        AsteroidContent::Unknown => "unknown",
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AsteroidView {
    #[serde(serialize_with = "ser_content")]
    pub content: AsteroidContent,
    pub motherlode: Option<String>,
    pub materials: Vec<MaterialRow>,
    pub remaining: Option<f64>,
    pub verdict: Verdict,
    pub est_total: Option<u64>,
}

#[derive(Debug, Default)]
pub struct ProspectorTracker {
    current: Option<AsteroidView>,
    history: VecDeque<AsteroidView>,
    /// Último evento, para recalcular el veredicto sin añadir historial.
    last_event: Option<ProspectedAsteroid>,
}

fn sum_opt(it: impl Iterator<Item = u64>) -> Option<u64> {
    it.reduce(u64::saturating_add)
}

fn build_view(ev: &ProspectedAsteroid, prices: PriceFn, s: &ModuleSettings) -> AsteroidView {
    let content = ev.content_level();
    let tons = mining_yield().tons_for(content);
    let in_interest = |key: &str| s.mining_interest.iter().any(|k| commodity_key(k) == key);

    let materials: Vec<MaterialRow> = ev
        .materials
        .iter()
        .map(|m| {
            let key = commodity_key(&m.name);
            let est_value = match (tons, prices(&key)) {
                (Some(t), Some(p)) => Some(
                    (m.proportion / 100.0 * t * p.price as f64)
                        .round()
                        .clamp(0.0, u64::MAX as f64) as u64,
                ),
                _ => None,
            };
            MaterialRow {
                interest: in_interest(&key),
                key,
                name: m.name_localised.clone(),
                proportion: m.proportion,
                est_value,
            }
        })
        .collect();

    let motherlode_raw = ev.motherlode_material.as_deref().filter(|m| !m.is_empty());
    let motherlode = motherlode_raw.map(|m| {
        ev.motherlode_material_localised
            .clone()
            .unwrap_or_else(|| m.to_string())
    });
    let motherlode_interest = motherlode_raw.is_some_and(|m| in_interest(&commodity_key(m)));

    let est_total = sum_opt(materials.iter().filter_map(|m| m.est_value));
    let interest_sum = sum_opt(
        materials
            .iter()
            .filter(|m| m.interest)
            .filter_map(|m| m.est_value),
    );
    let has_interest = materials.iter().any(|m| m.interest);

    let all_interest_priced = materials
        .iter()
        .filter(|m| m.interest)
        .all(|m| m.est_value.is_some());

    let verdict = if motherlode_interest {
        Verdict::Worth {
            est_value: interest_sum,
        }
    } else if !has_interest {
        Verdict::NotWorth
    } else if interest_sum.is_some_and(|sum| sum >= s.mining_threshold_cr) {
        Verdict::Worth {
            est_value: interest_sum,
        }
    } else if all_interest_priced {
        Verdict::NotWorth
    } else if content == AsteroidContent::High {
        // Precios parciales o ausentes: no se puede descartar, se usa el contenido.
        Verdict::Worth {
            est_value: interest_sum,
        }
    } else {
        Verdict::NoData
    };

    AsteroidView {
        content,
        motherlode,
        materials,
        remaining: ev.remaining,
        verdict,
        est_total,
    }
}

impl ProspectorTracker {
    /// Registra un asteroide prospectado (cada evento cuenta, sin deduplicar);
    /// devuelve siempre `true`.
    pub fn apply(&mut self, ev: &ProspectedAsteroid, prices: PriceFn, s: &ModuleSettings) -> bool {
        let view = build_view(ev, prices, s);
        self.history.push_back(view.clone());
        while self.history.len() > HISTORY_MAX {
            self.history.pop_front();
        }
        self.current = Some(view);
        self.last_event = Some(ev.clone());
        decide!(
            "mining.prospector.applied",
            tmc_model::diagnostics::json!({"history": self.history.len()})
        );
        true
    }

    /// Recalcula la vista actual (y su entrada del historial) con otros precios o
    /// ajustes, sin añadir historial. Devuelve `true` si cambió.
    pub fn recompute(&mut self, prices: PriceFn, s: &ModuleSettings) -> bool {
        let Some(ev) = &self.last_event else {
            return false;
        };
        let view = build_view(ev, prices, s);
        if self.current.as_ref() == Some(&view) {
            return false;
        }
        if let Some(last) = self.history.back_mut() {
            *last = view.clone();
        }
        self.current = Some(view);
        true
    }

    pub fn current(&self) -> Option<&AsteroidView> {
        self.current.as_ref()
    }

    pub fn history(&self) -> &VecDeque<AsteroidView> {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mining::prices::{PriceInfo, PriceSource};
    use tmc_model::events::ProspectedMaterial;

    fn mat(name: &str, p: f64) -> ProspectedMaterial {
        ProspectedMaterial {
            name: name.into(),
            name_localised: None,
            proportion: p,
        }
    }
    fn ev(
        content: &str,
        mats: Vec<ProspectedMaterial>,
        mother: Option<&str>,
    ) -> ProspectedAsteroid {
        ProspectedAsteroid {
            materials: mats,
            content: format!("$AsteroidMaterialContent_{content};"),
            motherlode_material: mother.map(String::from),
            ..Default::default()
        }
    }
    fn priced(key: &str) -> Option<PriceInfo> {
        (key == "platinum").then_some(PriceInfo {
            price: 220_000,
            source: PriceSource::LocalMarket,
            at: chrono::DateTime::UNIX_EPOCH,
        })
    }
    fn none(_: &str) -> Option<PriceInfo> {
        None
    }

    #[test]
    fn recompute_updates_current_without_new_history_entry() {
        let mut t = ProspectorTracker::default();
        let e = ev("High", vec![mat("Platinum", 31.5)], None);
        assert!(!t.recompute(&priced, &ModuleSettings::default()));
        t.apply(&e, &priced, &ModuleSettings::default());
        assert!(!t.recompute(&priced, &ModuleSettings::default()));
        let strict = ModuleSettings {
            mining_threshold_cr: 5_000_000,
            ..ModuleSettings::default()
        };
        assert!(t.recompute(&priced, &strict));
        assert_eq!(t.current().unwrap().verdict, Verdict::NotWorth);
        assert_eq!(t.history().len(), 1);
        assert_eq!(t.history().back().unwrap().verdict, Verdict::NotWorth);
    }

    #[test]
    fn absurd_proportion_does_not_overflow() {
        let mut t = ProspectorTracker::default();
        let e = ev(
            "High",
            vec![mat("Platinum", 1e30), mat("Platinum", 1e30)],
            None,
        );
        t.apply(&e, &priced, &ModuleSettings::default());
        assert_eq!(t.current().unwrap().est_total, Some(u64::MAX));
    }

    #[test]
    fn est_value_formula() {
        let mut t = ProspectorTracker::default();
        let e = ev("High", vec![mat("Platinum", 31.5)], None);
        t.apply(&e, &priced, &ModuleSettings::default());
        let v = t.current().unwrap();
        assert_eq!(v.materials[0].key, "platinum");
        assert_eq!(v.materials[0].est_value, Some(2_079_000));
        assert!(v.materials[0].interest);
        assert_eq!(v.est_total, Some(2_079_000));
    }

    #[test]
    fn verdict_worth_over_threshold() {
        let mut t = ProspectorTracker::default();
        let e = ev("High", vec![mat("Platinum", 31.5)], None);
        t.apply(&e, &priced, &ModuleSettings::default());
        assert_eq!(
            t.current().unwrap().verdict,
            Verdict::Worth {
                est_value: Some(2_079_000)
            }
        );
    }

    #[test]
    fn verdict_worth_for_motherlode_in_interest() {
        let mut t = ProspectorTracker::default();
        let e = ev("Low", vec![mat("Bauxite", 5.0)], Some("Monazite"));
        t.apply(&e, &none, &ModuleSettings::default());
        let v = t.current().unwrap();
        assert_eq!(v.verdict, Verdict::Worth { est_value: None });
        assert_eq!(v.motherlode.as_deref(), Some("Monazite"));
    }

    #[test]
    fn verdict_not_worth_below_threshold() {
        let mut t = ProspectorTracker::default();
        let e = ev("Low", vec![mat("Platinum", 3.0)], None);
        t.apply(&e, &priced, &ModuleSettings::default());
        let v = t.current().unwrap();
        assert_eq!(v.materials[0].est_value, Some(66_000));
        assert_eq!(v.verdict, Verdict::NotWorth);
    }

    #[test]
    fn verdict_without_prices_uses_interest_and_high_content() {
        let s = ModuleSettings::default();
        let mut t = ProspectorTracker::default();
        t.apply(&ev("High", vec![mat("Platinum", 20.0)], None), &none, &s);
        assert_eq!(
            t.current().unwrap().verdict,
            Verdict::Worth { est_value: None }
        );
        t.apply(&ev("Medium", vec![mat("Platinum", 20.0)], None), &none, &s);
        assert_eq!(t.current().unwrap().verdict, Verdict::NoData);
    }

    #[test]
    fn history_keeps_last_8() {
        let s = ModuleSettings::default();
        let mut t = ProspectorTracker::default();
        for i in 1..=10 {
            t.apply(&ev("Low", vec![mat("Bauxite", i as f64)], None), &none, &s);
        }
        assert_eq!(t.history().len(), 8);
        assert_eq!(t.history()[0].materials[0].proportion, 3.0);
        assert_eq!(t.current().unwrap().materials[0].proportion, 10.0);
    }

    fn mixed_price(key: &str) -> Option<PriceInfo> {
        priced(key)
    }

    #[test]
    fn partial_prices_do_not_give_not_worth() {
        let s = ModuleSettings::default();
        let mats = vec![mat("Platinum", 3.0), mat("Painite", 25.0)];
        let mut t = ProspectorTracker::default();
        t.apply(&ev("High", mats.clone(), None), &mixed_price, &s);
        assert_eq!(
            t.current().unwrap().verdict,
            Verdict::Worth {
                est_value: Some(198_000)
            }
        );
        t.apply(&ev("Medium", mats, None), &mixed_price, &s);
        assert_eq!(t.current().unwrap().verdict, Verdict::NoData);
    }

    #[test]
    fn history_does_not_dedup_identical_prospects() {
        let s = ModuleSettings::default();
        let mut t = ProspectorTracker::default();
        let e = ev("Low", vec![mat("Bauxite", 5.0)], None);
        assert!(t.apply(&e, &none, &s));
        assert!(t.apply(&e, &none, &s));
        assert_eq!(t.history().len(), 2);
    }

    #[test]
    fn verdict_worth_at_exact_threshold() {
        let s = ModuleSettings {
            mining_threshold_cr: 66_000,
            ..Default::default()
        };
        let mut t = ProspectorTracker::default();
        t.apply(&ev("Low", vec![mat("Platinum", 3.0)], None), &priced, &s);
        assert_eq!(
            t.current().unwrap().verdict,
            Verdict::Worth {
                est_value: Some(66_000)
            }
        );
    }

    #[test]
    fn motherlode_outside_interest_without_interest_materials_is_not_worth() {
        let mut t = ProspectorTracker::default();
        let e = ev("High", vec![mat("Bauxite", 5.0)], Some("Bauxite"));
        t.apply(&e, &none, &ModuleSettings::default());
        assert_eq!(t.current().unwrap().verdict, Verdict::NotWorth);
    }

    #[test]
    fn unknown_content_has_no_est_value() {
        let mut t = ProspectorTracker::default();
        let e = ev("Whatever", vec![mat("Platinum", 30.0)], None);
        t.apply(&e, &priced, &ModuleSettings::default());
        let v = t.current().unwrap();
        assert_eq!(v.content, AsteroidContent::Unknown);
        assert_eq!(v.materials[0].est_value, None);
        assert_eq!(v.est_total, None);
    }

    #[test]
    fn verdict_serializes_tagged() {
        let w = Verdict::Worth {
            est_value: Some(2_079_000),
        };
        assert_eq!(
            serde_json::to_string(&w).unwrap(),
            r#"{"kind":"worth","est_value":2079000}"#
        );
        assert_eq!(
            serde_json::to_string(&Verdict::NoData).unwrap(),
            r#"{"kind":"no_data"}"#
        );
    }
}

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
}

fn sum_opt(it: impl Iterator<Item = u64>) -> Option<u64> {
    it.reduce(|a, b| a + b)
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
                (Some(t), Some(p)) => {
                    Some((m.proportion / 100.0 * t * p.price as f64).round() as u64)
                }
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

    let verdict = if motherlode_interest {
        Verdict::Worth {
            est_value: interest_sum,
        }
    } else if let Some(sum) = interest_sum {
        if sum >= s.mining_threshold_cr {
            Verdict::Worth {
                est_value: Some(sum),
            }
        } else {
            Verdict::NotWorth
        }
    } else if !has_interest {
        Verdict::NotWorth
    } else if content == AsteroidContent::High {
        Verdict::Worth { est_value: None }
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
    /// Registra un asteroide prospectado; devuelve `true` si el estado cambió.
    pub fn apply(&mut self, ev: &ProspectedAsteroid, prices: PriceFn, s: &ModuleSettings) -> bool {
        let view = build_view(ev, prices, s);
        if self.current.as_ref() == Some(&view) {
            return false;
        }
        self.history.push_back(view.clone());
        while self.history.len() > HISTORY_MAX {
            self.history.pop_front();
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
            source: PriceSource::Market,
        })
    }
    fn none(_: &str) -> Option<PriceInfo> {
        None
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

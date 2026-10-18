//! Sesión de minería: refinados, limpets, carga y tasas por hora (estimaciones).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::events::{EventKind, commodity_key};

use super::prices::PriceFn;

const BINS_MAX: usize = 10;
const MIN_ELAPSED_SECS: f64 = 60.0;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct MiningSession {
    pub started_at: Option<DateTime<Utc>>,
    pub refined: BTreeMap<String, u32>,
    pub limpets_launched: BTreeMap<String, u32>,
    pub limpets_left: Option<u32>,
    pub cargo: Option<u32>,
    pub capacity: Option<u32>,
    pub asteroids_prospected: u32,
    pub cracked: u32,
    /// Tipos refinados, el más reciente al final.
    #[serde(skip)]
    recent: Vec<String>,
}

impl MiningSession {
    /// Reinicia la sesión (conserva capacidad, carga y limpets conocidos).
    pub fn reset(&mut self, now: DateTime<Utc>) {
        self.started_at = Some(now);
        self.refined.clear();
        self.limpets_launched.clear();
        self.asteroids_prospected = 0;
        self.cracked = 0;
        self.recent.clear();
    }

    /// Aplica un evento; devuelve `true` si cambió la sesión.
    ///
    /// `LaunchDrone` no reduce la carga estimada (los limpets sí ocupan bodega,
    /// pero la estimación se corrige con el siguiente `Cargo`).
    pub fn apply_event(&mut self, ev: &EventKind, now: DateTime<Utc>) -> bool {
        match ev {
            EventKind::LoadGame(_) => {
                self.reset(now);
                true
            }
            EventKind::MiningRefined(r) => {
                let key = commodity_key(&r.type_);
                *self.refined.entry(key.clone()).or_default() += 1;
                self.recent.retain(|k| *k != key);
                self.recent.push(key);
                if let Some(c) = self.cargo.as_mut() {
                    *c = c.saturating_add(1);
                }
                true
            }
            EventKind::LaunchDrone(d) => {
                *self.limpets_launched.entry(d.type_.clone()).or_default() += 1;
                if let Some(l) = self.limpets_left.as_mut() {
                    *l = l.saturating_sub(1);
                }
                true
            }
            EventKind::ProspectedAsteroid(_) => {
                self.asteroids_prospected += 1;
                true
            }
            EventKind::AsteroidCracked(_) => {
                self.cracked += 1;
                true
            }
            EventKind::Cargo(c) => {
                // Solo la bodega de la nave (no SRV).
                if c.vessel.as_deref().is_some_and(|v| v != "Ship") {
                    return false;
                }
                let mut changed = false;
                let count = c.count.or_else(|| {
                    c.inventory
                        .as_ref()
                        .map(|inv| inv.iter().map(|i| i.count).sum())
                });
                if let Some(count) = count {
                    changed |= self.cargo.replace(count) != Some(count);
                }
                // Sin `Inventory` (vive en Cargo.json) se conservan los limpets.
                if let Some(inv) = &c.inventory {
                    let drones = inv
                        .iter()
                        .filter(|i| commodity_key(&i.name) == "drones")
                        .map(|i| i.count)
                        .sum();
                    changed |= self.limpets_left.replace(drones) != Some(drones);
                }
                changed
            }
            EventKind::Loadout(l) => match l.cargo_capacity {
                Some(cap) => self.capacity.replace(cap) != Some(cap),
                None => false,
            },
            EventKind::EjectCargo(e) => self.decrement_cargo(e.count),
            EventKind::MarketSell(m) => self.decrement_cargo(m.count),
            _ => false,
        }
    }

    fn decrement_cargo(&mut self, n: u32) -> bool {
        match self.cargo.as_mut() {
            Some(c) if n > 0 => {
                *c = c.saturating_sub(n);
                true
            }
            _ => false,
        }
    }

    fn elapsed_hours(&self, now: DateTime<Utc>) -> Option<f64> {
        let secs = (now - self.started_at?).num_milliseconds() as f64 / 1000.0;
        (secs >= MIN_ELAPSED_SECS).then_some(secs / 3600.0)
    }

    /// Toneladas refinadas por hora; `None` con menos de 60 s de sesión.
    pub fn tons_per_hour(&self, now: DateTime<Utc>) -> Option<f64> {
        let h = self.elapsed_hours(now)?;
        let total: u32 = self.refined.values().sum();
        Some(f64::from(total) / h)
    }

    /// Créditos estimados: toneladas × mejor precio conocido; `None` si ningún
    /// tipo refinado tiene precio (los tipos sin precio aportan 0).
    pub fn est_credits(&self, prices: PriceFn) -> Option<u64> {
        self.refined
            .iter()
            .filter_map(|(k, t)| prices(k).map(|p| p.price.saturating_mul(u64::from(*t))))
            .reduce(u64::saturating_add)
    }

    pub fn credits_per_hour(&self, now: DateTime<Utc>, prices: PriceFn) -> Option<u64> {
        let h = self.elapsed_hours(now)?;
        let total = self.est_credits(prices)?;
        Some((total as f64 / h).round() as u64)
    }

    /// Tipos refinados más recientes primero (máx. 10), con toneladas (estimado).
    pub fn refinery_bins(&self) -> Vec<(String, u32)> {
        self.recent
            .iter()
            .rev()
            .take(BINS_MAX)
            .map(|k| (k.clone(), self.refined.get(k).copied().unwrap_or(0)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mining::prices::{PriceInfo, PriceSource};
    use chrono::{Duration, TimeZone};
    use tmc_model::events::{
        Cargo, CargoItem, EjectCargo, LaunchDrone, LoadGame, Loadout, MiningRefined,
    };

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 10, 20, 0, 0).unwrap()
    }
    fn refine(s: &mut MiningSession, name: &str, n: u32) {
        for _ in 0..n {
            s.apply_event(
                &EventKind::MiningRefined(MiningRefined {
                    type_: format!("${name}_name;"),
                    type_localised: None,
                }),
                t0(),
            );
        }
    }
    fn launch(s: &mut MiningSession, ty: &str, n: u32) {
        for _ in 0..n {
            s.apply_event(
                &EventKind::LaunchDrone(LaunchDrone { type_: ty.into() }),
                t0(),
            );
        }
    }
    fn cargo_ev(count: u32, drones: u32) -> EventKind {
        EventKind::Cargo(Cargo {
            vessel: Some("Ship".into()),
            count: Some(count),
            inventory: Some(vec![CargoItem {
                name: "drones".into(),
                count: drones,
                ..Default::default()
            }]),
        })
    }
    fn prices(key: &str) -> Option<PriceInfo> {
        (key == "platinum").then_some(PriceInfo {
            price: 220_000,
            source: PriceSource::Market,
        })
    }
    fn started() -> MiningSession {
        let mut s = MiningSession::default();
        s.apply_event(&EventKind::LoadGame(LoadGame::default()), t0());
        s
    }

    #[test]
    fn refined_tons_per_mineral() {
        let mut s = started();
        refine(&mut s, "platinum", 12);
        refine(&mut s, "painite", 3);
        assert_eq!(s.refined.get("platinum"), Some(&12));
        assert_eq!(s.refined.get("painite"), Some(&3));
        assert_eq!(s.refined.len(), 2);
    }

    #[test]
    fn tons_per_hour_over_30_min() {
        let mut s = started();
        refine(&mut s, "platinum", 12);
        assert_eq!(s.tons_per_hour(t0() + Duration::minutes(30)), Some(24.0));
        assert_eq!(s.tons_per_hour(t0() + Duration::seconds(30)), None);
        assert_eq!(MiningSession::default().tons_per_hour(t0()), None);
    }

    #[test]
    fn limpets_launched_by_type_and_left_from_cargo() {
        let mut s = started();
        s.apply_event(&cargo_ev(40, 12), t0());
        launch(&mut s, "Prospector", 3);
        launch(&mut s, "Collection", 5);
        assert_eq!(s.limpets_launched.get("Prospector"), Some(&3));
        assert_eq!(s.limpets_launched.get("Collection"), Some(&5));
        // El Cargo se emitió antes de los lanzamientos: 12 - 8.
        assert_eq!(s.limpets_left, Some(4));
        s.apply_event(&cargo_ev(40, 12), t0());
        assert_eq!(s.limpets_left, Some(12));
    }

    #[test]
    fn est_credits_uses_prices() {
        let mut s = started();
        refine(&mut s, "platinum", 12);
        assert_eq!(s.est_credits(&prices), Some(2_640_000));
        assert_eq!(
            s.credits_per_hour(t0() + Duration::hours(1), &prices),
            Some(2_640_000)
        );
        refine(&mut s, "painite", 2);
        assert_eq!(s.est_credits(&prices), Some(2_640_000));
        assert_eq!(s.est_credits(&|_: &str| None), None);
    }

    #[test]
    fn refinery_bins_capped_at_10() {
        let mut s = started();
        for i in 0..12 {
            refine(&mut s, &format!("mineral{i}"), 1);
        }
        refine(&mut s, "mineral5", 1);
        let bins = s.refinery_bins();
        assert_eq!(bins.len(), 10);
        assert_eq!(bins[0], ("mineral5".to_string(), 2));
        assert_eq!(bins[1].0, "mineral11");
    }

    #[test]
    fn session_starts_at_load_game_and_reset() {
        let mut s = MiningSession::default();
        assert_eq!(s.started_at, None);
        s.apply_event(&EventKind::LoadGame(LoadGame::default()), t0());
        assert_eq!(s.started_at, Some(t0()));
        refine(&mut s, "platinum", 1);
        s.reset(t0() + Duration::hours(1));
        assert_eq!(s.started_at, Some(t0() + Duration::hours(1)));
        assert!(s.refined.is_empty());
        assert!(s.refinery_bins().is_empty());
    }

    #[test]
    fn launch_and_refine_after_cargo_adjust_counters() {
        let mut s = started();
        s.apply_event(&cargo_ev(40, 12), t0());
        launch(&mut s, "Prospector", 2);
        refine(&mut s, "platinum", 3);
        assert_eq!(s.limpets_left, Some(10));
        assert_eq!(s.cargo, Some(43));
    }

    #[test]
    fn eject_and_sell_decrement_cargo_and_loadout_sets_capacity() {
        let mut s = started();
        s.apply_event(&cargo_ev(40, 0), t0());
        s.apply_event(
            &EventKind::EjectCargo(EjectCargo {
                count: 5,
                ..Default::default()
            }),
            t0(),
        );
        assert_eq!(s.cargo, Some(35));
        let l = EventKind::Loadout(Loadout {
            cargo_capacity: Some(256),
            ..Default::default()
        });
        assert!(s.apply_event(&l, t0()));
        assert!(!s.apply_event(&l, t0()));
        assert_eq!(s.capacity, Some(256));
    }

    #[test]
    fn cargo_without_inventory_keeps_limpets_and_srv_is_ignored() {
        let mut s = started();
        s.apply_event(&cargo_ev(40, 12), t0());
        let no_inv = EventKind::Cargo(Cargo {
            vessel: Some("Ship".into()),
            count: Some(50),
            inventory: None,
        });
        assert!(s.apply_event(&no_inv, t0()));
        assert_eq!((s.cargo, s.limpets_left), (Some(50), Some(12)));
        let srv = EventKind::Cargo(Cargo {
            vessel: Some("SRV".into()),
            count: Some(1),
            inventory: Some(vec![]),
        });
        assert!(!s.apply_event(&srv, t0()));
        assert_eq!((s.cargo, s.limpets_left), (Some(50), Some(12)));
    }

    #[test]
    fn limpets_saturate_and_sell_decrements_and_reset_keeps_state() {
        let mut s = started();
        s.apply_event(&cargo_ev(40, 1), t0());
        s.apply_event(
            &EventKind::Loadout(Loadout {
                cargo_capacity: Some(256),
                ..Default::default()
            }),
            t0(),
        );
        launch(&mut s, "Prospector", 3);
        assert_eq!(s.limpets_left, Some(0));
        s.apply_event(
            &EventKind::MarketSell(tmc_model::events::MarketSell {
                count: 10,
                ..Default::default()
            }),
            t0(),
        );
        assert_eq!(s.cargo, Some(30));
        s.reset(t0() + Duration::hours(1));
        assert_eq!(
            (s.cargo, s.capacity, s.limpets_left),
            (Some(30), Some(256), Some(0))
        );
    }

    #[test]
    fn credits_per_hour_none_under_60s() {
        let mut s = started();
        refine(&mut s, "platinum", 1);
        assert_eq!(
            s.credits_per_hour(t0() + Duration::seconds(59), &prices),
            None
        );
    }
}

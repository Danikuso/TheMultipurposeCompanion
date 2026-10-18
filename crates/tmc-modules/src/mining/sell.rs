//! «Dónde vender»: ranking de mercados remotos y mejor estación para la carga.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::net::{PadSize, RemoteMarket};

use super::prices::PriceSource;

/// Estaciones mostradas por mineral.
const TOP_PER_COMMODITY: usize = 5;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SellOption {
    pub station_name: String,
    pub system_name: String,
    pub distance_ly: f64,
    pub sell_price: u64,
    pub demand: Option<u64>,
    pub pad: Option<PadSize>,
    /// `None` si la nave o la plataforma de la estación se desconocen.
    pub pad_ok: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}

fn pad_ok(m: &RemoteMarket, ship_pad: Option<PadSize>) -> Option<bool> {
    Some(m.max_pad?.fits(ship_pad?))
}

fn by_price(a: &RemoteMarket, b: &RemoteMarket) -> std::cmp::Ordering {
    b.sell_price
        .cmp(&a.sell_price)
        .then(a.distance_ly.total_cmp(&b.distance_ly))
}

/// Mercados compatibles con la nave (los que no caben se excluyen; con nave o
/// plataforma desconocida no se excluye), por precio descendente y, a igualdad,
/// menor distancia.
fn compatible(markets: &[RemoteMarket], ship_pad: Option<PadSize>) -> Vec<&RemoteMarket> {
    let mut v: Vec<&RemoteMarket> = markets
        .iter()
        .filter(|m| pad_ok(m, ship_pad) != Some(false))
        .collect();
    v.sort_by(|a, b| by_price(a, b));
    v
}

pub fn rank_markets(
    markets: &[RemoteMarket],
    ship_pad: Option<PadSize>,
    top: usize,
) -> Vec<SellOption> {
    compatible(markets, ship_pad)
        .into_iter()
        .take(top)
        .map(|m| SellOption {
            station_name: m.station_name.clone(),
            system_name: m.system_name.clone(),
            distance_ly: m.distance_ly,
            sell_price: m.sell_price,
            demand: m.demand,
            pad: m.max_pad,
            pad_ok: pad_ok(m, ship_pad),
            updated_at: m.updated_at,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SellPlan {
    pub by_commodity: BTreeMap<String, Vec<SellOption>>,
    /// (estación, venta total estimada de la carga).
    pub best_station_total: Option<(String, u64)>,
    pub fetched_at: DateTime<Utc>,
    pub source: PriceSource,
}

impl SellPlan {
    /// `cargo`: toneladas por clave de materia; `results`: mercados remotos por
    /// clave. La mejor estación maximiza la suma toneladas × precio de lo que
    /// compra (empate: la más cercana).
    pub fn build(
        cargo: &BTreeMap<String, u32>,
        results: &BTreeMap<String, Vec<RemoteMarket>>,
        ship_pad: Option<PadSize>,
        fetched_at: DateTime<Utc>,
    ) -> Self {
        let mut by_commodity = BTreeMap::new();
        // (estación, sistema) -> (total, distancia mínima)
        let mut totals: BTreeMap<(String, String), (u64, f64)> = BTreeMap::new();
        for (key, markets) in results {
            by_commodity.insert(
                key.clone(),
                rank_markets(markets, ship_pad, TOP_PER_COMMODITY),
            );
            let tons = u64::from(cargo.get(key).copied().unwrap_or(0));
            if tons == 0 {
                continue;
            }
            // Dedupe por (estación, materia): vale el mejor precio.
            let mut best: BTreeMap<(String, String), (u64, f64)> = BTreeMap::new();
            for m in compatible(markets, ship_pad) {
                best.entry((m.station_name.clone(), m.system_name.clone()))
                    .or_insert((m.sell_price, m.distance_ly));
            }
            for (station, (price, dist)) in best {
                let e = totals.entry(station).or_insert((0, dist));
                e.0 = e.0.saturating_add(tons.saturating_mul(price));
                e.1 = e.1.min(dist);
            }
        }
        let best_station_total = totals
            .into_iter()
            .max_by(|a, b| a.1.0.cmp(&b.1.0).then(b.1.1.total_cmp(&a.1.1)))
            .map(|((station, _), (total, _))| (station, total));
        Self {
            by_commodity,
            best_station_total,
            fetched_at,
            source: PriceSource::Remote,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn market(name: &str, price: u64, pad: Option<PadSize>, dist: f64) -> RemoteMarket {
        RemoteMarket {
            station_name: name.into(),
            system_name: format!("{name} Sys"),
            market_id: None,
            distance_ly: dist,
            distance_to_arrival_ls: None,
            sell_price: price,
            demand: None,
            max_pad: pad,
            station_type: None,
            updated_at: None,
        }
    }

    #[test]
    fn large_pad_market_hidden_for_large_ship_only_when_incompatible() {
        let ms = vec![market("A", 100, Some(PadSize::Medium), 1.0)];
        assert!(rank_markets(&ms, Some(PadSize::Large), 5).is_empty());
        let r = rank_markets(&ms, Some(PadSize::Medium), 5);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].pad_ok, Some(true));
    }

    #[test]
    fn unknown_ship_keeps_markets_with_unknown_pad_ok() {
        let ms = vec![
            market("A", 100, Some(PadSize::Small), 1.0),
            market("B", 90, None, 2.0),
        ];
        let r = rank_markets(&ms, None, 5);
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|o| o.pad_ok.is_none()));
    }

    #[test]
    fn best_station_maximizes_total_sale() {
        let cargo = BTreeMap::from([("platinum".to_string(), 20), ("painite".to_string(), 10)]);
        let results = BTreeMap::from([
            (
                "platinum".to_string(),
                vec![
                    market("Y", 240_000, Some(PadSize::Large), 1.0),
                    market("X", 220_000, Some(PadSize::Large), 5.0),
                ],
            ),
            (
                "painite".to_string(),
                vec![market("X", 500_000, Some(PadSize::Large), 5.0)],
            ),
        ]);
        let plan = SellPlan::build(&cargo, &results, None, DateTime::UNIX_EPOCH);
        assert_eq!(plan.best_station_total, Some(("X".to_string(), 9_400_000)));
        assert_eq!(plan.by_commodity["platinum"][0].station_name, "Y");
    }

    #[test]
    fn duplicate_station_rows_count_once_with_best_price() {
        let cargo = BTreeMap::from([("platinum".to_string(), 10)]);
        let results = BTreeMap::from([(
            "platinum".to_string(),
            vec![market("X", 100, None, 1.0), market("X", 90, None, 1.0)],
        )]);
        let plan = SellPlan::build(&cargo, &results, None, DateTime::UNIX_EPOCH);
        assert_eq!(plan.best_station_total, Some(("X".to_string(), 1000)));
    }

    #[test]
    fn known_ship_with_unknown_market_pad_is_kept() {
        let ms = vec![market("A", 100, None, 1.0)];
        let r = rank_markets(&ms, Some(PadSize::Large), 5);
        assert_eq!((r.len(), r[0].pad_ok), (1, None));
    }

    #[test]
    fn rank_orders_by_price_then_distance_and_truncates() {
        let ms = vec![
            market("Low", 10, None, 1.0),
            market("FarTie", 50, None, 9.0),
            market("NearTie", 50, None, 2.0),
            market("Top", 99, None, 5.0),
        ];
        let r = rank_markets(&ms, None, 3);
        let names: Vec<_> = r.iter().map(|o| o.station_name.as_str()).collect();
        assert_eq!(names, ["Top", "NearTie", "FarTie"]);
    }

    #[test]
    fn ties_prefer_closer_station() {
        let cargo = BTreeMap::from([("platinum".to_string(), 10)]);
        let results = BTreeMap::from([(
            "platinum".to_string(),
            vec![
                market("Far", 200_000, None, 9.0),
                market("Near", 200_000, None, 2.0),
            ],
        )]);
        let plan = SellPlan::build(&cargo, &results, None, DateTime::UNIX_EPOCH);
        assert_eq!(plan.best_station_total.unwrap().0, "Near");
        assert_eq!(plan.by_commodity["platinum"][0].station_name, "Near");
    }
}

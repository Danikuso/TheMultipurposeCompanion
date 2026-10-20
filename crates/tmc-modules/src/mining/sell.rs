//! «Dónde vender»: ranking de mercados remotos y mejor estación para la carga.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_core::StoreTask;
use tmc_model::market::{MarketIdentity, PriceOrigin, PricedMarket, Side};
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
    pub origin: Option<PriceOrigin>,
}

/// Une observaciones de una mercancía. La fecha manda sobre el precio; a igual
/// fecha se prefiere Local, EDDN y Ardent, en ese orden.
pub fn merge_sources(local: &[PricedMarket], remote: &[RemoteMarket]) -> Vec<RemoteMarket> {
    use std::collections::BTreeSet;
    use tmc_model::market::is_fleet_carrier;

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Key {
        Id(u64),
        Name(String, String),
    }
    fn names(m: &RemoteMarket) -> (String, String) {
        (
            m.station_name.trim().to_lowercase(),
            m.system_name.trim().to_lowercase(),
        )
    }
    fn origin_rank(origin: Option<PriceOrigin>) -> u8 {
        match origin {
            Some(PriceOrigin::Local) => 3,
            Some(PriceOrigin::Eddn) => 2,
            Some(PriceOrigin::Ardent) => 1,
            None => 0,
        }
    }
    let observations: Vec<_> = local
        .iter()
        .filter_map(|m| {
            Some(RemoteMarket {
                station_name: m.station_name.clone(),
                system_name: m.system_name.clone(),
                market_id: Some(m.market_id),
                distance_ly: m.distance_ly?,
                distance_to_arrival_ls: None,
                sell_price: m.price,
                demand: Some(m.volume),
                max_pad: m.max_pad,
                station_type: None,
                updated_at: Some(m.updated_at),
                origin: Some(m.origin),
            })
        })
        .chain(remote.iter().cloned())
        .filter(|m| {
            m.distance_ly.is_finite()
                && m.distance_ly >= 0.0
                && !is_fleet_carrier(
                    m.market_id.unwrap_or_default(),
                    m.station_type.as_deref(),
                    Some(&m.station_name),
                )
        })
        .collect();
    let mut ids_by_name: BTreeMap<(String, String), BTreeSet<u64>> = BTreeMap::new();
    for m in &observations {
        if let Some(id) = m.market_id {
            ids_by_name.entry(names(m)).or_default().insert(id);
        }
    }
    let mut merged: BTreeMap<Key, RemoteMarket> = BTreeMap::new();
    for mut m in observations {
        let pair = names(&m);
        if m.market_id.is_none()
            && let Some(ids) = ids_by_name.get(&pair)
            && ids.len() == 1
        {
            m.market_id = ids.first().copied();
        }
        let key = m
            .market_id
            .map(Key::Id)
            .unwrap_or_else(|| Key::Name(pair.0, pair.1));
        match merged.entry(key) {
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(m);
            }
            std::collections::btree_map::Entry::Occupied(mut e) => {
                let old = e.get();
                let order = m
                    .updated_at
                    .cmp(&old.updated_at)
                    .then(origin_rank(m.origin).cmp(&origin_rank(old.origin)))
                    .then(m.sell_price.cmp(&old.sell_price))
                    .then(m.demand.cmp(&old.demand))
                    .then_with(|| old.distance_ly.total_cmp(&m.distance_ly))
                    .then(m.station_name.cmp(&old.station_name))
                    .then(m.system_name.cmp(&old.system_name));
                if order.is_gt() {
                    e.insert(m);
                }
            }
        }
    }
    // Primero se resuelve la frescura: una demanda cero nueva invalida la vieja.
    merged
        .into_values()
        .filter(|m| m.sell_price > 0 && m.demand.is_some_and(|d| d > 0))
        .collect()
}

/// Lectura de la tabla de precios para «Dónde vender». Devuelve `{cmdr, commodities,
/// center, radius_ly, ship_pad, candidates, since, rows: {materia: [PricedMarket]}}`:
/// las mejores ofertas compatibles (plataforma filtrada antes del límite) más las
/// observaciones dirigidas de los candidatos de Ardent, incluidas las cerradas.
#[allow(clippy::too_many_arguments)]
pub fn local_sells_task(
    cmdr: String,
    commodities: Vec<String>,
    center: [f64; 3],
    radius_ly: f64,
    since: DateTime<Utc>,
    candidates: BTreeMap<String, Vec<MarketIdentity>>,
    ship_pad: Option<PadSize>,
    limit: usize,
) -> StoreTask {
    Box::new(move |store| {
        let mut rows = BTreeMap::new();
        for commodity in &commodities {
            let key = tmc_model::events::commodity_key(commodity);
            let mut prices = store.best_prices_for_ship(
                &key,
                Side::Sell,
                center,
                radius_ly,
                since,
                None,
                limit,
                ship_pad,
            )?;
            if let Some(identities) = candidates.get(&key) {
                prices.extend(store.price_observations_for_markets(
                    &key,
                    Side::Sell,
                    identities,
                    center,
                    radius_ly,
                    since,
                    None,
                )?);
            }
            rows.insert(key, prices);
        }
        Ok(serde_json::json!({
            "cmdr": cmdr, "commodities": commodities, "center": center,
            "radius_ly": radius_ly, "ship_pad": ship_pad, "candidates": candidates,
            "since": since, "rows": rows,
        }))
    })
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

/// Mercados cuya plataforma admite la nave (con nave o plataforma desconocida no se
/// excluye). Sin ordenar ni recortar.
pub fn filter_compatible(markets: &[RemoteMarket], ship_pad: Option<PadSize>) -> Vec<RemoteMarket> {
    markets
        .iter()
        .filter(|m| pad_ok(m, ship_pad) != Some(false))
        .cloned()
        .collect()
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
            origin: m.origin,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SellPlan {
    pub by_commodity: BTreeMap<String, Vec<SellOption>>,
    /// (estación, venta total estimada de la carga).
    pub best_station_total: Option<(String, u64)>,
    pub fetched_at: Option<DateTime<Utc>>,
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
        fetched_at: Option<DateTime<Utc>>,
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
            origin: None,
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
        let plan = SellPlan::build(&cargo, &results, None, None);
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
        let plan = SellPlan::build(&cargo, &results, None, None);
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
        let plan = SellPlan::build(&cargo, &results, None, None);
        assert_eq!(plan.best_station_total.unwrap().0, "Near");
        assert_eq!(plan.by_commodity["platinum"][0].station_name, "Near");
    }
}

//! Buscar mercancía: dónde comprarla más barata (locales + Ardent).

use super::market::LOCAL_CANDIDATES;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tmc_core::StoreTask;
use tmc_model::{
    market::{MarketIdentity, PriceOrigin, PricedMarket, Side, is_fleet_carrier},
    net::{PadSize, RemoteExport},
};

/// Ventana de observaciones consideradas, en días.
const WINDOW_DAYS: i64 = 7;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BuyOption {
    pub station_name: String,
    pub system_name: String,
    pub distance_ly: f64,
    pub buy_price: u64,
    pub stock: u64,
    pub pad: Option<PadSize>,
    /// `None` si la nave o la plataforma de la estación se desconocen.
    pub pad_ok: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
    pub origin: PriceOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchResult {
    pub commodity: String,
    pub rows: Vec<BuyOption>,
}

struct Obs {
    market_id: Option<u64>,
    station_name: String,
    system_name: String,
    distance_ly: f64,
    buy_price: u64,
    stock: Option<u64>,
    pad: Option<PadSize>,
    updated_at: Option<DateTime<Utc>>,
    origin: PriceOrigin,
    station_type: Option<String>,
}

fn origin_rank(o: PriceOrigin) -> u8 {
    match o {
        PriceOrigin::Local => 3,
        PriceOrigin::Eddn => 2,
        PriceOrigin::Ardent => 1,
    }
}

/// Une observaciones por mercado: manda la fecha (una oferta cerrada más nueva
/// invalida la antigua) y a igual fecha Local, EDDN y Ardent. Después descarta
/// cerradas, plataformas incompatibles, ordena por precio ascendente y recorta.
pub fn merge_buys(
    local: &[PricedMarket],
    ardent: &[RemoteExport],
    ship_pad: Option<PadSize>,
    top: usize,
) -> Vec<BuyOption> {
    let observations = local
        .iter()
        .filter_map(|m| {
            Some(Obs {
                market_id: Some(m.market_id),
                station_name: m.station_name.clone(),
                system_name: m.system_name.clone(),
                distance_ly: m.distance_ly?,
                buy_price: m.price,
                stock: Some(m.volume),
                pad: m.max_pad,
                updated_at: Some(m.updated_at),
                origin: m.origin,
                station_type: None,
            })
        })
        .chain(ardent.iter().map(|r| Obs {
            market_id: r.market_id,
            station_name: r.station_name.clone(),
            system_name: r.system_name.clone(),
            distance_ly: r.distance_ly,
            buy_price: r.buy_price,
            stock: r.stock,
            pad: r.max_pad,
            updated_at: r.updated_at,
            origin: PriceOrigin::Ardent,
            station_type: r.station_type.clone(),
        }))
        .filter(|o| {
            o.distance_ly.is_finite()
                && o.distance_ly >= 0.0
                && !is_fleet_carrier(
                    o.market_id.unwrap_or_default(),
                    o.station_type.as_deref(),
                    Some(&o.station_name),
                )
        })
        .collect::<Vec<_>>();
    let names = |o: &Obs| {
        (
            o.station_name.trim().to_lowercase(),
            o.system_name.trim().to_lowercase(),
        )
    };
    let mut ids_by_name: BTreeMap<(String, String), BTreeSet<u64>> = BTreeMap::new();
    for o in &observations {
        if let Some(id) = o.market_id {
            ids_by_name.entry(names(o)).or_default().insert(id);
        }
    }
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Key {
        Id(u64),
        Name(String, String),
    }
    let total = observations.len();
    let mut merged: BTreeMap<Key, Obs> = BTreeMap::new();
    for mut o in observations {
        let pair = names(&o);
        if o.market_id.is_none()
            && let Some(ids) = ids_by_name.get(&pair)
            && ids.len() == 1
        {
            o.market_id = ids.first().copied();
        }
        let key = o
            .market_id
            .map(Key::Id)
            .unwrap_or_else(|| Key::Name(pair.0, pair.1));
        match merged.entry(key) {
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(o);
            }
            std::collections::btree_map::Entry::Occupied(mut e) => {
                let old = e.get();
                let order = o
                    .updated_at
                    .cmp(&old.updated_at)
                    .then(origin_rank(o.origin).cmp(&origin_rank(old.origin)))
                    .then(old.buy_price.cmp(&o.buy_price))
                    .then(o.stock.cmp(&old.stock))
                    .then_with(|| old.distance_ly.total_cmp(&o.distance_ly))
                    .then(o.station_name.cmp(&old.station_name))
                    .then(o.system_name.cmp(&old.system_name));
                if order.is_gt() {
                    e.insert(o);
                }
            }
        }
    }
    let mut rows: Vec<BuyOption> = merged
        .into_values()
        // Primero se resuelve la frescura: un stock cero nuevo invalida el viejo.
        .filter(|o| o.buy_price > 0 && o.stock.is_some_and(|s| s > 0))
        .filter_map(|o| {
            let pad_ok = o
                .pad
                .zip(ship_pad)
                .map(|(station, ship)| station.fits(ship));
            (pad_ok != Some(false)).then(|| BuyOption {
                station_name: o.station_name,
                system_name: o.system_name,
                distance_ly: o.distance_ly,
                buy_price: o.buy_price,
                stock: o.stock.unwrap_or_default(),
                pad: o.pad,
                pad_ok,
                updated_at: o.updated_at,
                origin: o.origin,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        a.buy_price
            .cmp(&b.buy_price)
            .then(a.distance_ly.total_cmp(&b.distance_ly))
            .then(a.station_name.cmp(&b.station_name))
    });
    rows.truncate(top);
    decide!("trade.search.merged", {
        "local": local.len(),
        "remote": ardent.len(),
        "observations": total,
        "returned": rows.len(),
        "top": top,
    });
    rows
}

/// Lectura para `trade.search` (el llamador la etiqueta así). Ventana de 7 días
/// hasta `now` (reloj del diario aportado por el llamador).
/// Devuelve `{cmdr, commodity, center, radius_ly, since, candidates, ship_pad,
/// rows: [PricedMarket]}`. Las mejores compras locales compatibles se limitan
/// tras filtrar plataformas; los candidatos remotos se consultan por identidad
/// (incluidos stock o precio cero) para no resucitar exportaciones antiguas.
pub fn search_task(
    cmdr: String,
    commodity: String,
    center: [f64; 3],
    radius_ly: f64,
    candidates: Vec<MarketIdentity>,
    ship_pad: Option<PadSize>,
    now: DateTime<Utc>,
) -> StoreTask {
    let since = now - Duration::days(WINDOW_DAYS);
    Box::new(move |store| {
        let key = tmc_model::events::commodity_key(&commodity);
        let mut rows = store.best_prices_for_ship(
            &key,
            Side::Buy,
            center,
            radius_ly,
            since,
            None,
            LOCAL_CANDIDATES,
            ship_pad,
        )?;
        rows.extend(store.price_observations_for_markets(
            &key,
            Side::Buy,
            &candidates,
            center,
            radius_ly,
            since,
            None,
        )?);
        Ok(
            serde_json::json!({"cmdr":cmdr,"commodity":key,"center":center,"radius_ly":radius_ly,"since":since,"candidates":candidates,"ship_pad":ship_pad,"rows":rows}),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(m: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 19, 18, 0, 0).unwrap() + Duration::minutes(m)
    }
    fn local(id: u64, price: u64, at: DateTime<Utc>) -> PricedMarket {
        PricedMarket {
            market_id: id,
            station_name: format!("Port {id}"),
            system_name: "Other".into(),
            distance_ly: Some(5.0),
            max_pad: Some(PadSize::Large),
            price,
            volume: 100,
            updated_at: at,
            origin: PriceOrigin::Eddn,
        }
    }
    fn export(id: u64, price: u64, at: DateTime<Utc>) -> RemoteExport {
        RemoteExport {
            station_name: format!("Port {id}"),
            system_name: "Other".into(),
            market_id: Some(id),
            distance_ly: 5.0,
            distance_to_arrival_ls: None,
            buy_price: price,
            stock: Some(50),
            max_pad: Some(PadSize::Large),
            station_type: None,
            updated_at: Some(at),
        }
    }
    #[test]
    fn merge_buys_dedups_by_market_newest_wins() {
        let rows = merge_buys(&[local(2, 9500, t(2))], &[export(2, 9000, t(1))], None, 5);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].updated_at, Some(t(2)));
        assert_eq!(rows[0].buy_price, 9500);
        assert_eq!(rows[0].origin, PriceOrigin::Eddn);
    }
    #[test]
    fn merge_buys_sorts_by_price_and_closed_newer_invalidates_old() {
        let mut closed = local(3, 0, t(5));
        closed.volume = 0;
        let rows = merge_buys(
            &[local(1, 9800, t(0)), closed],
            &[export(2, 9100, t(0)), export(3, 8000, t(1))],
            None,
            5,
        );
        let prices: Vec<_> = rows.iter().map(|r| r.buy_price).collect();
        assert_eq!(prices, [9100, 9800]);
        assert_eq!(rows[0].origin, PriceOrigin::Ardent);
        assert_eq!(merge_buys(&[local(1, 9800, t(0))], &[], None, 0).len(), 0);
    }
    #[test]
    fn search_excludes_incompatible_pad() {
        let mut medium = local(1, 9000, t(0));
        medium.max_pad = Some(PadSize::Medium);
        let mut unknown = local(2, 9500, t(0));
        unknown.max_pad = None;
        let ok = local(3, 9900, t(0));
        let rows = merge_buys(&[medium, unknown, ok], &[], Some(PadSize::Large), 5);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].station_name.as_str(), rows[0].pad_ok),
            ("Port 2", None)
        );
        assert_eq!(rows[1].pad_ok, Some(true));
    }
    #[test]
    fn search_task_limits_after_pad_filter_and_keeps_closed_candidate() {
        use tmc_model::market::{CommodityPrice, MarketSnapshot};
        let store = tmc_core::Store::open_in_memory().unwrap();
        let snap = |id: u64, pad, buy, stock| MarketSnapshot {
            market_id: id,
            station_name: format!("Port {id}"),
            system_name: "Sys".into(),
            system_address: None,
            star_pos: Some([0.0; 3]),
            max_pad: pad,
            station_type: None,
            updated_at: t(0),
            origin: PriceOrigin::Eddn,
            prices: vec![CommodityPrice {
                commodity: "gold".into(),
                name_localised: None,
                buy_price: buy,
                sell_price: 0,
                stock,
                demand: 0,
            }],
        };
        let mut all: Vec<_> = (1000..1502)
            .map(|id| snap(id, Some(PadSize::Medium), 1000, 10))
            .collect();
        all.push(snap(1600, Some(PadSize::Large), 9000, 10));
        all.push(snap(1601, Some(PadSize::Large), 0, 0));
        store.replace_markets(&all).unwrap();
        let ident = MarketIdentity {
            market_id: Some(1601),
            station_name: "Port 1601".into(),
            system_name: "Sys".into(),
        };
        let payload = search_task(
            "c".into(),
            "Gold".into(),
            [0.0; 3],
            50.0,
            vec![ident],
            Some(PadSize::Large),
            t(60),
        )(&store)
        .unwrap();
        let rows: Vec<PricedMarket> = serde_json::from_value(payload["rows"].clone()).unwrap();
        assert!(rows.iter().any(|r| r.market_id == 1600));
        assert!(rows.iter().any(|r| r.market_id == 1601 && r.price == 0));
        assert!(rows.iter().all(|r| !(1000..1502).contains(&r.market_id)));
        assert_eq!(payload["commodity"], "gold");
    }
    #[test]
    fn local_rows_without_distance_are_dropped() {
        let mut far = local(1, 9000, t(0));
        far.distance_ly = None;
        assert!(merge_buys(&[far], &[], None, 5).is_empty());
    }
    #[test]
    fn missing_id_resolved_by_name_dedups() {
        let mut idless = export(2, 9000, t(3));
        idless.market_id = None;
        idless.station_name = "PORT 2".into();
        let rows = merge_buys(&[local(2, 9500, t(1))], &[idless], None, 5);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].buy_price, 9000);
        assert_eq!(rows[0].updated_at, Some(t(3)));
    }
}

//! Mercado actual y consultas de ventas conocidas.

use crate::mining::sell::SellOption;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use tmc_core::StoreTask;
use tmc_model::{
    events::Docked,
    market::{MarketIdentity, MarketSnapshot, PricedMarket},
    net::{PadSize, RemoteMarket},
};

pub(super) const LOCAL_CANDIDATES: usize = 500;
const LOCAL_SELL_CANDIDATES: usize = LOCAL_CANDIDATES;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CurrentMarketRow {
    pub commodity: String,
    pub name_localised: Option<String>,
    pub buy_price: u64,
    pub stock: u64,
    pub best_sell: Option<SellOption>,
    pub profit_per_ton: Option<i64>,
    pub rare: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CurrentMarket {
    pub station: String,
    pub system: String,
    pub rows: Vec<CurrentMarketRow>,
    pub as_of: DateTime<Utc>,
}
impl CurrentMarket {
    /// `sells` contiene las opciones ya ordenadas y compatibles por mercancía.
    pub fn build(
        snapshot: &MarketSnapshot,
        sells: &BTreeMap<String, Vec<SellOption>>,
        now: DateTime<Utc>,
    ) -> Self {
        let mut rows: Vec<_> = snapshot
            .prices
            .iter()
            .filter(|p| p.buy_price > 0 && p.stock > 0)
            .map(|p| {
                let best_sell = sells.get(&p.commodity).and_then(|v| v.first()).cloned();
                let profit_per_ton = best_sell.as_ref().map(|s| {
                    (i128::from(s.sell_price) - i128::from(p.buy_price))
                        .clamp(i128::from(i64::MIN), i128::from(i64::MAX))
                        as i64
                });
                CurrentMarketRow {
                    commodity: p.commodity.clone(),
                    name_localised: p.name_localised.clone(),
                    buy_price: p.buy_price,
                    stock: p.stock,
                    best_sell,
                    profit_per_ton,
                    rare: crate::data::rare_goods()
                        .get(&p.commodity)
                        .map(|r| r.sell_distance_ly),
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            b.profit_per_ton
                .cmp(&a.profit_per_ton)
                .then_with(|| b.buy_price.cmp(&a.buy_price))
                .then(a.commodity.cmp(&b.commodity))
        });
        rows.truncate(5);
        decide!("trade.market.built", {
            "offers": snapshot.prices.len(),
            "rows": rows.len(),
            "with_sell": rows.iter().filter(|r| r.best_sell.is_some()).count(),
        });
        Self {
            station: snapshot.station_name.clone(),
            system: snapshot.system_name.clone(),
            rows,
            as_of: now,
        }
    }
}
pub fn snapshot_from_market_json(
    json: &Value,
    docked: &Docked,
    star_pos: [f64; 3],
    now: DateTime<Utc>,
) -> Option<MarketSnapshot> {
    use tmc_model::{
        events::commodity_key,
        market::{CommodityPrice, PriceOrigin, is_fleet_carrier},
    };
    fn text(v: Option<&Value>) -> Option<&str> {
        v?.as_str().filter(|s| !s.trim().is_empty())
    }
    let items = json.get("Items")?.as_array()?;
    let file_id = json
        .get("MarketID")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0);
    if file_id.zip(docked.market_id).is_some_and(|(a, b)| a != b) {
        return None;
    }
    for (field, current) in [
        ("StationName", docked.station_name.as_deref()),
        ("StarSystem", docked.star_system.as_deref()),
    ] {
        if text(json.get(field))
            .zip(current)
            .is_some_and(|(a, b)| !a.eq_ignore_ascii_case(b))
        {
            return None;
        }
    }
    let market_id = file_id.or(docked.market_id).filter(|id| *id > 0)?;
    let station_name = text(json.get("StationName"))
        .or(docked.station_name.as_deref())?
        .to_owned();
    let system_name = text(json.get("StarSystem"))
        .or(docked.star_system.as_deref())?
        .to_owned();
    let station_type = text(json.get("StationType"))
        .or(docked.station_type.as_deref())
        .map(str::to_owned);
    if is_fleet_carrier(market_id, station_type.as_deref(), Some(&station_name))
        || is_fleet_carrier(
            market_id,
            docked.station_type.as_deref(),
            Some(&station_name),
        )
    {
        return None;
    }
    let updated_at = text(json.get("timestamp"))
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or(now);
    let prices = items
        .iter()
        .filter_map(|item| {
            let commodity = commodity_key(text(item.get("Name"))?);
            if commodity.is_empty() {
                return None;
            }
            let num = |key| item.get(key).and_then(Value::as_u64).unwrap_or(0);
            Some(CommodityPrice {
                commodity,
                name_localised: text(item.get("Name_Localised")).map(str::to_owned),
                buy_price: num("BuyPrice"),
                sell_price: num("SellPrice"),
                stock: num("Stock"),
                demand: num("Demand"),
            })
        })
        .collect();
    Some(MarketSnapshot {
        market_id,
        station_name,
        system_name,
        system_address: docked.system_address,
        star_pos: star_pos.iter().all(|v| v.is_finite()).then_some(star_pos),
        max_pad: docked.max_pad(),
        station_type,
        updated_at,
        origin: PriceOrigin::Local,
        prices,
    })
}
pub fn best_sells(
    rows: &[PricedMarket],
    ardent: &[RemoteMarket],
    ship_pad: Option<PadSize>,
    top: usize,
) -> Vec<SellOption> {
    let merged = crate::mining::sell::merge_sources(rows, ardent);
    let ranked = crate::mining::sell::rank_markets(&merged, ship_pad, top);
    decide!("trade.market.best_sells", {
        "local": rows.len(),
        "remote": ardent.len(),
        "merged": merged.len(),
        "returned": ranked.len(),
        "top": top,
    });
    ranked
}
/// Lectura para `trade.best_sells`. Devuelve `{cmdr, commodities, center,
/// radius_ly, since, exclude_market, candidates, ship_pad, rows: {commodity: [PricedMarket]}}`.
/// Los parámetros permiten descartar respuestas de un contexto anterior.
#[allow(clippy::too_many_arguments)]
pub fn best_sells_task(
    cmdr: String,
    commodities: Vec<String>,
    center: [f64; 3],
    radius_ly: f64,
    since: DateTime<Utc>,
    exclude_market: u64,
    candidates: BTreeMap<String, Vec<MarketIdentity>>,
    ship_pad: Option<PadSize>,
) -> StoreTask {
    Box::new(move |store| {
        let mut rows = BTreeMap::new();
        for commodity in &commodities {
            let key = tmc_model::events::commodity_key(commodity);
            // Hasta 500 ventas locales por mercancía; los candidatos remotos se
            // reconcilian por identidad para conservar cierres fuera del top.
            let mut prices = store.best_prices_for_ship(
                &key,
                tmc_model::market::Side::Sell,
                center,
                radius_ly,
                since,
                Some(exclude_market),
                LOCAL_SELL_CANDIDATES,
                ship_pad,
            )?;
            if let Some(identities) = candidates.get(&key) {
                prices.extend(store.price_observations_for_markets(
                    &key,
                    tmc_model::market::Side::Sell,
                    identities,
                    center,
                    radius_ly,
                    since,
                    Some(exclude_market),
                )?);
            }
            rows.insert(key, prices);
        }
        Ok(
            serde_json::json!({"cmdr":cmdr,"commodities":commodities,"center":center,"radius_ly":radius_ly,"since":since,"exclude_market":exclude_market,"candidates":candidates,"ship_pad":ship_pad,"rows":rows}),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use serde_json::json;
    use tmc_model::market::PriceOrigin;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 19, 18, 0, 0).unwrap()
    }
    fn docked() -> Docked {
        serde_json::from_value(json!({"MarketID":128000001,"StationName":"Synthetic Port","StarSystem":"Synthetic System","StationType":"Coriolis"})).unwrap()
    }
    fn snapshot() -> MarketSnapshot {
        let value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/companion/market_trade.json"
        ))
        .unwrap();
        snapshot_from_market_json(&value, &docked(), [0.0; 3], now()).unwrap()
    }
    fn local(id: u64, price: u64) -> PricedMarket {
        PricedMarket {
            market_id: id,
            station_name: format!("Port {id}"),
            system_name: "Other".into(),
            distance_ly: Some(5.0),
            max_pad: Some(PadSize::Large),
            price,
            volume: 100,
            updated_at: now(),
            origin: PriceOrigin::Eddn,
        }
    }
    fn sales() -> BTreeMap<String, Vec<SellOption>> {
        [
            ("gold", 11000),
            ("silver", 5000),
            ("palladium", 13900),
            ("cobalt", 1800),
            ("copper", 2700),
            ("aluminium", 1100),
            ("lavianbrandy", 3500),
        ]
        .into_iter()
        .map(|(key, price)| (key.into(), best_sells(&[local(2, price)], &[], None, 1)))
        .collect()
    }
    #[test]
    fn top_five_buys_by_profit_per_ton() {
        let market = CurrentMarket::build(&snapshot(), &sales(), now());
        assert_eq!(market.rows.len(), 5);
        assert_eq!(market.rows[0].commodity, "gold");
        assert_eq!(market.rows[0].profit_per_ton, Some(1600));
        assert!(
            market.rows[0]
                .best_sell
                .as_ref()
                .unwrap()
                .updated_at
                .is_some()
        );
        assert!(
            market
                .rows
                .windows(2)
                .all(|r| r[0].profit_per_ton >= r[1].profit_per_ton)
        );
    }
    #[test]
    fn merge_sources_dedups_by_market_newest_wins_even_if_cheaper() {
        let mut old = crate::mining::sell::merge_sources(&[local(2, 230000)], &[]).remove(0);
        old.updated_at = Some(now() - Duration::hours(1));
        old.origin = Some(PriceOrigin::Ardent);
        let merged = crate::mining::sell::merge_sources(&[local(2, 225000)], &[old]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].sell_price, 225000);
        assert_eq!(merged[0].origin, Some(PriceOrigin::Eddn));
    }
    #[test]
    fn fewer_than_five_known_sells_filled_with_no_data_rows() {
        let mut sales = sales();
        sales.retain(|k, _| matches!(k.as_str(), "gold" | "silver" | "cobalt"));
        let market = CurrentMarket::build(&snapshot(), &sales, now());
        assert_eq!(market.rows.len(), 5);
        assert!(market.rows[..3].iter().all(|r| r.best_sell.is_some()));
        assert_eq!(market.rows[3].commodity, "palladium");
        assert_eq!(market.rows[4].commodity, "lavianbrandy");
        assert!(market.rows[3..].iter().all(|r| r.best_sell.is_none()));
    }
    #[test]
    fn incompatible_pad_excluded_for_large_ship() {
        let mut better = local(2, 12000);
        better.max_pad = Some(PadSize::Medium);
        let options = best_sells(&[better, local(3, 11000)], &[], Some(PadSize::Large), 1);
        assert_eq!(options[0].station_name, "Port 3");
    }
    #[test]
    fn unknown_pad_included_with_warning() {
        let mut price = local(2, 12000);
        price.max_pad = None;
        assert_eq!(
            best_sells(&[price], &[], Some(PadSize::Large), 1)[0].pad_ok,
            None
        );
    }
    #[test]
    fn no_known_sell_is_no_data() {
        let market = CurrentMarket::build(&snapshot(), &BTreeMap::new(), now());
        assert!(
            market
                .rows
                .iter()
                .all(|r| r.best_sell.is_none() && r.profit_per_ton.is_none())
        );
    }
    #[test]
    fn rare_good_is_marked() {
        let market = CurrentMarket::build(&snapshot(), &BTreeMap::new(), now());
        assert_eq!(
            market
                .rows
                .iter()
                .find(|r| r.commodity == "lavianbrandy")
                .unwrap()
                .rare,
            Some(crate::data::rare_goods()["lavianbrandy"].sell_distance_ly)
        );
    }
    #[test]
    fn market_json_to_snapshot() {
        let snapshot = snapshot();
        assert_eq!(snapshot.prices.len(), 7);
        assert_eq!(snapshot.origin, PriceOrigin::Local);
        let gold = &snapshot.prices[0];
        assert_eq!((gold.buy_price, gold.stock), (9400, 5000));
        assert_eq!(gold.name_localised.as_deref(), Some("Oro"));
    }
    #[test]
    fn corrupt_market_is_tolerant_and_keeps_valid_rows() {
        for bad in [Value::Null, json!([]), json!({"Items":false})] {
            assert!(snapshot_from_market_json(&bad, &docked(), [0.0; 3], now()).is_none());
        }
        let json = json!({"Items":[null,{}, {"Name":false},{"Name":""}, {"Name":"gold","BuyPrice":9400,"Stock":5000,"SellPrice":"bad"}]});
        let s = snapshot_from_market_json(&json, &docked(), [0.0; 3], now()).unwrap();
        assert_eq!(s.prices.len(), 1);
        assert_eq!(s.prices[0].sell_price, 0);
        assert_eq!(s.updated_at, now());
    }
    #[test]
    fn carrier_or_stale_market_file_is_not_saved() {
        let mut value = serde_json::to_value(snapshot().prices).unwrap();
        value = json!({"MarketID":7,"Items":value});
        assert!(snapshot_from_market_json(&value, &docked(), [0.0; 3], now()).is_none());
        let mut d = docked();
        d.station_type = Some("FleetCarrier".into());
        assert!(snapshot_from_market_json(&json!({"Items":[]}), &d, [0.0; 3], now()).is_none());
    }
    #[test]
    fn stale_station_names_and_conflicting_carrier_types_are_rejected() {
        for value in [
            json!({"StationName":"Old Port","Items":[]}),
            json!({"StarSystem":"Old System","Items":[]}),
        ] {
            assert!(snapshot_from_market_json(&value, &docked(), [0.0; 3], now()).is_none());
        }
        let mut d = docked();
        d.station_type = Some("FleetCarrier".into());
        assert!(
            snapshot_from_market_json(
                &json!({"StationType":"Coriolis","Items":[]}),
                &d,
                [0.0; 3],
                now()
            )
            .is_none()
        );
    }
    #[test]
    fn zero_stock_or_zero_buy_price_is_not_a_purchase() {
        let mut s = snapshot();
        s.prices[0].stock = 0;
        s.prices[1].buy_price = 0;
        let m = CurrentMarket::build(&s, &sales(), now());
        assert!(
            m.rows
                .iter()
                .all(|r| r.commodity != "gold" && r.commodity != "silver")
        );
    }
    #[test]
    fn merge_fallback_resolves_missing_id_and_keeps_distinct_known_ids() {
        let local = local(2, 100);
        let mut remote = RemoteMarket {
            station_name: "PORT 2".into(),
            system_name: "other".into(),
            market_id: None,
            distance_ly: 5.0,
            distance_to_arrival_ls: None,
            sell_price: 90,
            demand: Some(1),
            max_pad: None,
            station_type: None,
            updated_at: Some(now() + Duration::seconds(1)),
            origin: Some(PriceOrigin::Ardent),
        };
        let merged = crate::mining::sell::merge_sources(
            std::slice::from_ref(&local),
            std::slice::from_ref(&remote),
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].sell_price, 90);
        assert_eq!(merged[0].market_id, Some(2));
        remote.market_id = Some(3);
        assert_eq!(
            crate::mining::sell::merge_sources(
                std::slice::from_ref(&local),
                std::slice::from_ref(&remote)
            )
            .len(),
            2
        );
        remote.market_id = None;
        let mut other = remote.clone();
        other.sell_price = 80;
        other.updated_at = Some(now());
        assert_eq!(
            crate::mining::sell::merge_sources(&[], &[remote, other]).len(),
            1
        );
    }
    #[test]
    fn merge_skips_missing_distance_zero_demand_and_carriers() {
        let mut missing = local(2, 100);
        missing.distance_ly = None;
        let mut zero = local(3, 100);
        zero.volume = 0;
        let mut remote = RemoteMarket {
            station_name: "Carrier".into(),
            system_name: "Other".into(),
            market_id: Some(4),
            distance_ly: 5.0,
            distance_to_arrival_ls: None,
            sell_price: 100,
            demand: Some(1),
            max_pad: None,
            station_type: Some("FleetCarrier".into()),
            updated_at: Some(now()),
            origin: None,
        };
        assert!(best_sells(&[missing, zero], std::slice::from_ref(&remote), None, 5).is_empty());
        remote.station_type = None;
        remote.demand = Some(0);
        assert!(best_sells(&[], &[remote], None, 5).is_empty());
    }
    #[test]
    fn equal_timestamp_prefers_local_then_eddn_regardless_of_input_order() {
        let eddn = local(2, 100);
        let mut own = eddn.clone();
        own.origin = PriceOrigin::Local;
        own.price = 90;
        let first = crate::mining::sell::merge_sources(&[eddn.clone(), own.clone()], &[]);
        assert_eq!(first, crate::mining::sell::merge_sources(&[own, eddn], &[]));
        assert_eq!(first[0].origin, Some(PriceOrigin::Local));
    }
    #[test]
    fn newest_closed_offer_does_not_resurrect_old_valid_offer() {
        let valid = local(2, 230000);
        for (price, demand) in [(220000, Some(0)), (220000, None), (0, Some(10))] {
            let closed = RemoteMarket {
                station_name: "Port 2".into(),
                system_name: "Other".into(),
                market_id: Some(2),
                distance_ly: 5.0,
                distance_to_arrival_ls: None,
                sell_price: price,
                demand,
                max_pad: None,
                station_type: None,
                updated_at: Some(now() + Duration::seconds(1)),
                origin: Some(PriceOrigin::Ardent),
            };
            assert!(
                best_sells(
                    std::slice::from_ref(&valid),
                    std::slice::from_ref(&closed),
                    None,
                    5
                )
                .is_empty()
            );
            assert!(best_sells(&[], &[closed], None, 5).is_empty());
        }
    }
    #[test]
    fn store_task_closed_observation_blocks_stale_remote_sale() {
        let store = tmc_core::Store::open_in_memory().unwrap();
        let mut current = snapshot();
        current.market_id = 2;
        current.origin = PriceOrigin::Eddn;
        let candidates = BTreeMap::from([(
            "gold".into(),
            vec![MarketIdentity {
                market_id: Some(2),
                station_name: current.station_name.clone(),
                system_name: current.system_name.clone(),
            }],
        )]);
        let distractors: Vec<_> = (1000..1501)
            .map(|id| {
                let mut other = current.clone();
                other.market_id = id;
                other.prices[0].sell_price = 20000;
                other
            })
            .collect();
        store.replace_markets(&distractors).unwrap();
        for (price, demand) in [(11000, 0), (0, 100)] {
            current.prices[0].sell_price = price;
            current.prices[0].demand = demand;
            store
                .replace_markets(std::slice::from_ref(&current))
                .unwrap();
            let mut remote = crate::mining::sell::merge_sources(&[local(2, 90000)], &[]).remove(0);
            remote.updated_at = Some(now() - Duration::hours(1));
            remote.origin = Some(PriceOrigin::Ardent);
            let payload = best_sells_task(
                "cmdr".into(),
                vec!["gold".into()],
                [0.0; 3],
                100.0,
                now() - Duration::hours(48),
                128000001,
                candidates.clone(),
                None,
            )(&store)
            .unwrap();
            let observations: Vec<PricedMarket> =
                serde_json::from_value(payload["rows"]["gold"].clone()).unwrap();
            assert_eq!(
                observations.len(),
                501,
                "500 ventas locales y la observación dirigida cerrada"
            );
            assert!(observations.iter().any(|p| p.market_id == 2));
            assert_eq!(payload["candidates"], json!(candidates));
            let only_top = store
                .best_prices(
                    "gold",
                    tmc_model::market::Side::Sell,
                    [0.0; 3],
                    100.0,
                    now() - Duration::hours(48),
                    None,
                    LOCAL_SELL_CANDIDATES,
                )
                .unwrap();
            assert_eq!(
                best_sells(&only_top, std::slice::from_ref(&remote), None, 5)[0].sell_price,
                90000,
                "sin consulta dirigida se resucitaría la venta vieja"
            );
            assert!(
                best_sells(&observations, &[remote], None, 5)
                    .iter()
                    .all(|s| s.sell_price != 90000),
                "la observación cerrada nueva debe invalidar la oferta vieja"
            );
            current.updated_at += Duration::seconds(1);
        }
    }
    #[test]
    fn store_task_missing_commodity_from_complete_snapshot_blocks_stale_remote_sale() {
        for empty in [false, true] {
            let store = tmc_core::Store::open_in_memory().unwrap();
            let mut current = snapshot();
            current.market_id = 2;
            store
                .replace_markets(std::slice::from_ref(&current))
                .unwrap();
            let mut remote = crate::mining::sell::merge_sources(&[local(2, 90000)], &[]).remove(0);
            remote.updated_at = Some(now() - Duration::hours(1));
            remote.origin = Some(PriceOrigin::Ardent);
            current.updated_at = now() + Duration::seconds(1);
            if empty {
                current.prices.clear();
            } else {
                current.prices.retain(|p| p.commodity != "gold");
            }
            store
                .replace_markets(std::slice::from_ref(&current))
                .unwrap();
            let identity = MarketIdentity {
                market_id: Some(2),
                station_name: current.station_name.clone(),
                system_name: current.system_name.clone(),
            };
            let payload = best_sells_task(
                "cmdr".into(),
                vec!["gold".into()],
                [0.0; 3],
                100.0,
                now() - Duration::hours(48),
                128000001,
                BTreeMap::from([("gold".into(), vec![identity])]),
                None,
            )(&store)
            .unwrap();
            let rows: Vec<PricedMarket> =
                serde_json::from_value(payload["rows"]["gold"].clone()).unwrap();
            assert!(
                best_sells(&rows, &[remote], None, 5).is_empty(),
                "un snapshot completo que omite gold cierra la oferta vieja"
            );
        }
    }
    #[test]
    fn store_task_filters_incompatible_pads_before_limiting_local_sales() {
        let store = tmc_core::Store::open_in_memory().unwrap();
        let snapshots: Vec<_> = (1000..1503)
            .map(|id| {
                let mut s = snapshot();
                s.market_id = id;
                s.station_name = format!("Port {id}");
                s.max_pad = match id {
                    1501 => Some(PadSize::Large),
                    1502 => None,
                    _ => Some(PadSize::Medium),
                };
                s.prices[0].sell_price = match id {
                    1501 => 10000,
                    1502 => 9000,
                    _ => 20000,
                };
                s
            })
            .collect();
        store.replace_markets(&snapshots).unwrap();
        let payload = best_sells_task(
            "cmdr".into(),
            vec!["gold".into()],
            [0.0; 3],
            100.0,
            now() - Duration::hours(48),
            128000001,
            BTreeMap::new(),
            Some(PadSize::Large),
        )(&store)
        .unwrap();
        assert_eq!(payload["ship_pad"], json!(PadSize::Large));
        let rows: Vec<PricedMarket> =
            serde_json::from_value(payload["rows"]["gold"].clone()).unwrap();
        let options = best_sells(&rows, &[], Some(PadSize::Large), 5);
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].station_name, "Port 1501");
        assert_eq!(options[1].station_name, "Port 1502");
        assert_eq!(options[1].pad_ok, None);
    }
    #[test]
    fn store_task_uses_window_radius_and_market_exclusion_and_echoes_query() {
        let store = tmc_core::Store::open_in_memory().unwrap();
        let mut current = snapshot();
        current.market_id = 2;
        current.prices[0].sell_price = 11000;
        store.replace_markets(&[current.clone()]).unwrap();
        current.market_id = 3;
        current.updated_at = now() - Duration::hours(49);
        current.prices[0].sell_price = 99999;
        store.replace_markets(&[current.clone()]).unwrap();
        current.market_id = 4;
        current.updated_at = now();
        current.star_pos = Some([200.0; 3]);
        store.replace_markets(&[current]).unwrap();
        let since = now() - Duration::hours(48);
        let payload = best_sells_task(
            "cmdr".into(),
            vec!["gold".into()],
            [0.0; 3],
            100.0,
            since,
            128000001,
            BTreeMap::new(),
            None,
        )(&store)
        .unwrap();
        assert_eq!(payload["cmdr"], "cmdr");
        assert_eq!(payload["exclude_market"], 128000001u64);
        assert_eq!(payload["center"], json!([0.0, 0.0, 0.0]));
        assert_eq!(payload["radius_ly"], 100.0);
        assert_eq!(payload["since"], json!(since));
        assert_eq!(payload["rows"]["gold"].as_array().unwrap().len(), 1);
        assert_eq!(payload["rows"]["gold"][0]["price"], 11000);
        let excluded = best_sells_task(
            "cmdr".into(),
            vec!["gold".into()],
            [0.0; 3],
            100.0,
            since,
            2,
            BTreeMap::new(),
            None,
        )(&store)
        .unwrap();
        assert!(excluded["rows"]["gold"].as_array().unwrap().is_empty());
    }
}

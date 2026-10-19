//! Libro de precios de materiales: remoto reciente > última venta propia >
//! `Market.json`. Siempre estimaciones.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use tmc_core::StoreTask;
use tmc_model::events::{EventKind, MarketSell, commodity_key, parse_line};
use tmc_model::net::RemoteMarket;

use crate::history;

/// Antigüedad máxima de un precio remoto para preferirlo al local.
const REMOTE_MAX_AGE_MIN: i64 = 15;

/// Origen del precio (siempre una estimación).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// Mejor mercado remoto compatible (Ardent).
    Remote,
    /// Última venta propia registrada (`MarketSell`).
    LocalSell,
    /// `Market.json` del mercado actual.
    LocalMarket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PriceInfo {
    pub price: u64,
    pub source: PriceSource,
    pub at: DateTime<Utc>,
}

/// Resuelve el mejor precio conocido de una clave de materia (`commodity_key`).
pub type PriceFn<'a> = &'a dyn Fn(&str) -> Option<PriceInfo>;

type Entry = (u64, DateTime<Utc>);

#[derive(Debug, Clone, Default)]
pub struct PriceBook {
    remote: BTreeMap<String, Entry>,
    local_sell: BTreeMap<String, Entry>,
    local_market: BTreeMap<String, Entry>,
}

fn put_newer(map: &mut BTreeMap<String, Entry>, key: String, price: u64, at: DateTime<Utc>) {
    match map.get(&key) {
        Some((_, old)) if *old > at => {}
        _ => {
            map.insert(key, (price, at));
        }
    }
}

impl PriceBook {
    /// Registra una venta propia (ignora precios 0).
    pub fn on_market_sell(&mut self, ev: &MarketSell, now: DateTime<Utc>) {
        if ev.sell_price > 0 {
            put_newer(
                &mut self.local_sell,
                commodity_key(&ev.type_),
                ev.sell_price,
                now,
            );
        }
    }

    /// Sustituye los precios de `Market.json` (mercado actual). Tolerante: ignora
    /// lo que no tenga nombre o `SellPrice > 0`.
    pub fn on_market_json(&mut self, json: &Value, now: DateTime<Utc>) {
        let Some(items) = json.get("Items").and_then(Value::as_array) else {
            return;
        };
        self.local_market.clear();
        for it in items {
            let name = it.get("Name").and_then(Value::as_str);
            let price = it.get("SellPrice").and_then(Value::as_u64);
            if let (Some(name), Some(price)) = (name, price)
                && price > 0
            {
                self.local_market.insert(commodity_key(name), (price, now));
            }
        }
    }

    /// Guarda el mejor precio de `markets`.
    ///
    /// Contrato: llamar solo tras una consulta correcta, con mercados ya filtrados
    /// por la plataforma de la nave actual. Si la nave cambia, el módulo llama a
    /// [`Self::clear_remote`] y vuelve a consultar.
    pub fn on_remote(
        &mut self,
        commodity: &str,
        markets: &[RemoteMarket],
        fetched_at: DateTime<Utc>,
    ) {
        let key = commodity_key(commodity);
        match markets.iter().map(|m| m.sell_price).max() {
            Some(p) if p > 0 => {
                self.remote.insert(key, (p, fetched_at));
            }
            _ => {
                self.remote.remove(&key);
            }
        }
    }

    /// Descarta todos los precios remotos (p. ej. al cambiar de nave).
    pub fn clear_remote(&mut self) {
        self.remote.clear();
    }

    /// Carga las últimas ventas reconstruidas por [`price_rebuild_task`]; no pisa
    /// datos más recientes ya conocidos.
    pub fn restore_local_sells(&mut self, stored: &Value) {
        let Some(map) = stored.get("prices").and_then(Value::as_object) else {
            return;
        };
        for (key, v) in map {
            let price = v.get("price").and_then(Value::as_u64);
            let at = v
                .get("at")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<DateTime<Utc>>().ok());
            if let (Some(price), Some(at)) = (price, at) {
                put_newer(&mut self.local_sell, key.clone(), price, at);
            }
        }
    }

    pub fn best_price(&self, commodity: &str, now: DateTime<Utc>) -> Option<PriceInfo> {
        let key = commodity_key(commodity);
        let info = |(price, at): Entry, source| PriceInfo { price, source, at };
        if let Some(&e) = self.remote.get(&key)
            && now - e.1 < Duration::minutes(REMOTE_MAX_AGE_MIN)
        {
            return Some(info(e, PriceSource::Remote));
        }
        if let Some(&e) = self.local_sell.get(&key) {
            return Some(info(e, PriceSource::LocalSell));
        }
        self.local_market
            .get(&key)
            .map(|&e| info(e, PriceSource::LocalMarket))
    }
}

/// Último `MarketSell` por mineral del comandante (resultado `Stored` con tag
/// `mining.prices`: `{cmdr, prices: {clave: {price, at}}}`).
pub fn price_rebuild_task(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json("SELECT COALESCE(MAX(id), 0) FROM event", &[])?;
        let cut = rows[0][0].as_i64().unwrap_or_default();
        let mut prices = serde_json::Map::new();
        history::for_each_before(store, &cmdr, cut, &["MarketSell"], None, |pos| {
            if let Ok(ev) = parse_line(&pos.raw)
                && let EventKind::MarketSell(m) = &ev.kind
                && m.sell_price > 0
            {
                prices
                    .entry(commodity_key(&m.type_))
                    .or_insert_with(|| json!({"price": m.sell_price, "at": ev.timestamp}));
            }
            false
        })?;
        Ok(json!({"cmdr": cmdr, "prices": prices}))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_core::{EventWriter, Store, StoredEvent};
    use tmc_model::net::PadSize;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 18, 12, 0, 0).unwrap()
    }

    fn remote(price: u64) -> RemoteMarket {
        RemoteMarket {
            station_name: "X".into(),
            system_name: "S".into(),
            market_id: None,
            distance_ly: 1.0,
            distance_to_arrival_ls: None,
            sell_price: price,
            demand: None,
            max_pad: Some(PadSize::Large),
            station_type: None,
            updated_at: None,
            origin: None,
        }
    }

    fn sell(ty: &str, price: u64) -> MarketSell {
        MarketSell {
            type_: ty.into(),
            sell_price: price,
            ..Default::default()
        }
    }

    #[test]
    fn price_prefers_recent_remote_over_local() {
        let mut b = PriceBook::default();
        b.on_market_sell(&sell("Platinum", 210_000), now());
        b.on_remote(
            "platinum",
            &[remote(225_000), remote(230_000)],
            now() - Duration::minutes(5),
        );
        let p = b.best_price("platinum", now()).unwrap();
        assert_eq!((p.price, p.source), (230_000, PriceSource::Remote));
    }

    #[test]
    fn remote_exactly_15_min_old_is_stale() {
        let mut b = PriceBook::default();
        b.on_remote(
            "platinum",
            &[remote(230_000)],
            now() - Duration::minutes(15),
        );
        assert!(b.best_price("platinum", now()).is_none());
    }

    #[test]
    fn clear_remote_falls_back_to_local_sell() {
        let mut b = PriceBook::default();
        b.on_market_sell(&sell("Platinum", 210_000), now());
        b.on_remote("platinum", &[remote(230_000)], now());
        b.clear_remote();
        let p = b.best_price("platinum", now()).unwrap();
        assert_eq!((p.price, p.source), (210_000, PriceSource::LocalSell));
    }

    #[test]
    fn restore_does_not_overwrite_newer_local_sell() {
        let mut b = PriceBook::default();
        b.on_market_sell(&sell("Platinum", 300_000), now());
        b.restore_local_sells(&json!({"prices": {"platinum":
            {"price": 1, "at": "2026-10-18T10:00:00Z"}}}));
        assert_eq!(b.best_price("platinum", now()).unwrap().price, 300_000);
    }

    #[test]
    fn rebuild_excludes_other_cmdr_and_zero_price() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        let rows = [
            ("TESTER", "Platinum", 0, "2026-10-18T11:00:00Z"),
            ("TESTER", "Platinum", 100, "2026-10-18T10:00:00Z"),
            ("OTHER", "Painite", 500, "2026-10-18T10:00:00Z"),
        ];
        for (i, (c, ty, price, ts)) in rows.iter().enumerate() {
            let raw = format!(
                r#"{{"timestamp":"{ts}","event":"MarketSell","Type":"{ty}","Count":1,"SellPrice":{price}}}"#
            );
            let ev = parse_line(&raw).unwrap();
            writer.send(StoredEvent {
                file_name: "a.log".into(),
                line: i as u64 + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some((*c).into()),
                json: raw,
            });
        }
        writer.flush_blocking().unwrap();
        let stored = price_rebuild_task("TESTER".into())(&store).unwrap();
        assert_eq!(stored["prices"]["platinum"]["price"], 100);
        assert!(stored["prices"].get("painite").is_none());
    }

    #[test]
    fn price_falls_back_to_last_market_sell() {
        let mut b = PriceBook::default();
        b.on_market_sell(&sell("Platinum", 200_000), now() - Duration::hours(1));
        b.on_market_sell(&sell("Platinum", 210_000), now());
        let p = b.best_price("platinum", now()).unwrap();
        assert_eq!((p.price, p.source), (210_000, PriceSource::LocalSell));
        assert!(b.best_price("painite", now()).is_none());
    }

    #[test]
    fn stale_remote_price_older_than_15_min_falls_back() {
        let mut b = PriceBook::default();
        b.on_market_sell(&sell("Platinum", 210_000), now());
        b.on_remote(
            "platinum",
            &[remote(230_000)],
            now() - Duration::minutes(16),
        );
        let p = b.best_price("platinum", now()).unwrap();
        assert_eq!((p.price, p.source), (210_000, PriceSource::LocalSell));
    }

    #[test]
    fn market_json_is_last_resort_and_lenient() {
        let mut b = PriceBook::default();
        let j = json!({"Items": [
            {"Name": "$platinum_name;", "SellPrice": 205_000},
            {"Name": "$gold_name;", "SellPrice": 0},
            {"SellPrice": 5},
            {"Name": 7}
        ]});
        b.on_market_json(&j, now());
        b.on_market_json(&json!({"oops": 1}), now());
        let p = b.best_price("Platinum", now()).unwrap();
        assert_eq!((p.price, p.source), (205_000, PriceSource::LocalMarket));
        assert!(b.best_price("gold", now()).is_none());
        b.on_market_sell(&sell("Platinum", 210_000), now());
        assert_eq!(
            b.best_price("platinum", now()).unwrap().source,
            PriceSource::LocalSell
        );
    }

    #[test]
    fn prices_rebuilt_from_history_after_restart() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        let rows = [
            ("2026-10-18T10:00:00Z", "Platinum", 210_000),
            ("2026-10-18T10:30:00Z", "Painite", 500_000),
            ("2026-10-18T11:00:00Z", "Platinum", 215_000),
        ];
        for (i, (ts, ty, price)) in rows.iter().enumerate() {
            let raw = format!(
                r#"{{"timestamp":"{ts}","event":"MarketSell","Type":"{ty}","Count":1,"SellPrice":{price},"TotalSale":{price}}}"#
            );
            let ev = parse_line(&raw).unwrap();
            writer.send(StoredEvent {
                file_name: "a.log".into(),
                line: i as u64 + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some("TESTER".into()),
                json: raw,
            });
        }
        writer.flush_blocking().unwrap();
        let stored = price_rebuild_task("TESTER".into())(&store).unwrap();
        assert_eq!(stored["cmdr"], "TESTER");
        let mut b = PriceBook::default();
        b.restore_local_sells(&stored);
        let p = b.best_price("platinum", now()).unwrap();
        assert_eq!((p.price, p.source), (215_000, PriceSource::LocalSell));
        assert_eq!(b.best_price("painite", now()).unwrap().price, 500_000);
    }
}

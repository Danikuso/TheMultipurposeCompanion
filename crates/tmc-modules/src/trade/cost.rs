//! Coste medio de la carga comercial; la carga desconocida conserva su marca parcial.

use crate::mining::sell::SellOption;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tmc_core::StoreTask;
use tmc_model::events::{CargoItem, EventKind, commodity_key, parse_line};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CostBasis {
    entries: HashMap<String, PaidCargo>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct PaidCargo {
    tons: u32,
    // Una retirada reduce proporcionalmente la parte conocida y la desconocida.
    // La media se conserva sin redondear hasta exponer avg_paid.
    known_tons: f64,
    average: f64,
}

impl CostBasis {
    pub fn apply_event(&mut self, ev: &EventKind) {
        match ev {
            EventKind::MarketBuy(m) if m.count > 0 => {
                let e = self.entries.entry(commodity_key(&m.type_)).or_default();
                let added = f64::from(m.count);
                e.average = (e.average * e.known_tons + m.buy_price as f64 * added)
                    / (e.known_tons + added);
                e.known_tons += added;
                e.tons = e.tons.saturating_add(m.count);
            }
            EventKind::MarketSell(m) => self.remove(&m.type_, m.count),
            EventKind::EjectCargo(m) => self.remove(&m.type_, m.count),
            EventKind::Cargo(c) if c.vessel.as_deref().is_none_or(|v| v == "Ship") => {
                if let Some(inventory) = &c.inventory {
                    self.reconcile(inventory);
                }
            }
            _ => {}
        }
    }
    fn remove(&mut self, commodity: &str, count: u32) {
        let key = commodity_key(commodity);
        if let Some(e) = self.entries.get_mut(&key) {
            let remaining = e.tons.saturating_sub(count);
            if remaining == 0 {
                self.entries.remove(&key);
            } else {
                e.known_tons *= f64::from(remaining) / f64::from(e.tons);
                e.tons = remaining;
            }
        }
    }
    pub fn reconcile(&mut self, inventory: &[CargoItem]) {
        let mut totals: HashMap<String, u32> = HashMap::new();
        for item in inventory.iter().filter(|i| !i.is_mission()) {
            let count = item.count.saturating_sub(item.stolen_count());
            if count > 0 {
                let total = totals.entry(commodity_key(&item.name)).or_default();
                *total = total.saturating_add(count);
            }
        }
        self.entries.retain(|key, _| totals.contains_key(key));
        for (key, tons) in totals {
            let e = self.entries.entry(key).or_default();
            if tons < e.tons {
                e.known_tons *= f64::from(tons) / f64::from(e.tons);
            }
            e.tons = tons;
        }
    }
    /// Redondea al crédito más próximo, sin alterar la media interna.
    pub fn avg_paid(&self, commodity: &str) -> Option<u64> {
        self.entries
            .get(&commodity_key(commodity))
            .filter(|e| e.known_tons > 0.0)
            .map(|e| e.average.round() as u64)
    }
    pub fn is_partial(&self, commodity: &str) -> bool {
        self.entries
            .get(&commodity_key(commodity))
            .is_some_and(|e| e.known_tons + 1e-9 < f64::from(e.tons))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CargoKind {
    Trade,
    Mission,
    Stolen,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CargoAdvice {
    SellHere,
    Carry,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CargoLine {
    pub commodity: String,
    pub name_localised: Option<String>,
    pub tons: u32,
    pub paid_avg: Option<u64>,
    pub paid_partial: bool,
    pub price_here: Option<u64>,
    pub best_nearby: Option<SellOption>,
    pub est_profit: Option<i64>,
    pub advice: Option<CargoAdvice>,
    pub kind: CargoKind,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CargoView {
    pub lines: Vec<CargoLine>,
    pub used: u32,
    pub capacity: Option<u32>,
}
impl CargoView {
    pub fn build(
        inventory: &[CargoItem],
        basis: &CostBasis,
        here: &HashMap<String, u64>,
        best: &HashMap<String, SellOption>,
        capacity: Option<u32>,
    ) -> Self {
        let mut view = Self {
            capacity,
            ..Self::default()
        };
        for item in inventory {
            view.used = view.used.saturating_add(item.count);
            let stolen = item.stolen_count().min(item.count);
            let parts = if item.is_mission() {
                vec![(CargoKind::Mission, item.count)]
            } else {
                vec![
                    (CargoKind::Trade, item.count - stolen),
                    (CargoKind::Stolen, stolen),
                ]
            };
            for (kind, tons) in parts.into_iter().filter(|(_, n)| *n > 0) {
                let key = commodity_key(&item.name);
                let trade = kind == CargoKind::Trade;
                let paid_avg = trade.then(|| basis.avg_paid(&key)).flatten();
                let price_here = trade.then(|| here.get(&key).copied()).flatten();
                let best_nearby = trade.then(|| best.get(&key).cloned()).flatten();
                let price = price_here
                    .into_iter()
                    .chain(best_nearby.as_ref().map(|b| b.sell_price))
                    .max();
                let est_profit = paid_avg.zip(price).map(|(paid, price)| {
                    ((i128::from(price) - i128::from(paid)) * i128::from(tons))
                        .clamp(i128::from(i64::MIN), i128::from(i64::MAX))
                        as i64
                });
                let advice = if !trade {
                    None
                } else if let Some(b) = &best_nearby {
                    Some(
                        if price_here
                            .is_some_and(|p| u128::from(p) * 100 >= u128::from(b.sell_price) * 98)
                        {
                            CargoAdvice::SellHere
                        } else {
                            CargoAdvice::Carry
                        },
                    )
                } else {
                    price_here.zip(paid_avg).map(|(here, paid)| {
                        if here > paid {
                            CargoAdvice::SellHere
                        } else {
                            CargoAdvice::Carry
                        }
                    })
                };
                view.lines.push(CargoLine {
                    commodity: key.clone(),
                    name_localised: item.name_localised.clone(),
                    tons,
                    paid_avg,
                    paid_partial: trade && basis.is_partial(&key),
                    price_here,
                    best_nearby,
                    est_profit,
                    advice,
                    kind,
                });
            }
        }
        view
    }
}
/// Lectura para `trade.cost`. El llamador fija `since` a siete días antes del reloj.
/// Se devuelve el corte para poder conciliar eventos que lleguen durante la lectura.
pub fn cost_rebuild_task(cmdr: String, since: DateTime<Utc>) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json("SELECT COALESCE(MAX(id), 0) FROM event", &[])?;
        let cut = rows[0][0].as_i64().unwrap_or_default();
        let mut basis = CostBasis::default();
        let cut_pos = store.query_json(
            "SELECT e.ts, j.name, e.line FROM event e JOIN journal_file j ON j.id = e.file_id WHERE e.cmdr = ?1 AND e.id <= ?2 ORDER BY e.ts DESC, j.name DESC, e.line DESC LIMIT 1",
            &[serde_json::json!(cmdr),serde_json::json!(cut)]
        )?.first().map(|r| serde_json::json!({"ts":r[0],"file":r[1],"line":r[2]})).unwrap_or(serde_json::Value::Null);
        let cursor = crate::history::Position {
            id: 0,
            ts: since.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
            file: String::new(),
            line: 0,
            raw: String::new(),
        };
        crate::history::for_each_after(
            store,
            &cmdr,
            cut,
            &["MarketBuy", "MarketSell", "EjectCargo", "Cargo"],
            Some(cursor),
            |pos| {
                if let Ok(ev) = parse_line(&pos.raw)
                    && ev.timestamp >= since
                {
                    basis.apply_event(&ev.kind);
                }
            },
        )?;
        Ok(serde_json::json!({"cmdr":cmdr,"basis":basis,"cut":cut,"cut_pos":cut_pos}))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_core::{EventWriter, Store, StoredEvent};
    use tmc_model::events::{Cargo, EjectCargo, MarketBuy, MarketSell, parse_line};

    fn buy(b: &mut CostBasis, count: u32, price: u64) {
        b.apply_event(&EventKind::MarketBuy(MarketBuy {
            type_: "Gold".into(),
            count,
            buy_price: price,
            ..Default::default()
        }));
    }
    fn cargo(count: u32) -> CargoItem {
        CargoItem {
            name: "Gold".into(),
            name_localised: Some("Oro".into()),
            count,
            ..Default::default()
        }
    }
    fn basis() -> CostBasis {
        let mut b = CostBasis::default();
        buy(&mut b, 100, 9400);
        buy(&mut b, 50, 9700);
        b
    }
    fn view(b: &CostBasis, inventory: &[CargoItem]) -> CargoView {
        CargoView::build(inventory, b, &HashMap::new(), &HashMap::new(), Some(256))
    }
    fn sell_option(price: u64) -> SellOption {
        SellOption {
            station_name: "Test Port".into(),
            system_name: "Test System".into(),
            distance_ly: 12.0,
            sell_price: price,
            demand: None,
            pad: None,
            pad_ok: None,
            updated_at: None,
        }
    }
    #[test]
    fn weighted_average_over_two_buys() {
        assert_eq!(basis().avg_paid("gold"), Some(9500));
    }
    #[test]
    fn sell_keeps_average_and_reduces_tons() {
        let mut b = basis();
        b.apply_event(&EventKind::MarketSell(MarketSell {
            type_: "Gold".into(),
            count: 60,
            ..Default::default()
        }));
        assert_eq!(b.avg_paid("gold"), Some(9500));
        let before = b.clone();
        b.reconcile(&[cargo(90)]);
        assert_eq!(b, before);
        assert_eq!(view(&b, &[cargo(90)]).lines[0].tons, 90);
    }
    #[test]
    fn mission_and_stolen_cargo_excluded_from_profit() {
        let inventory = [
            CargoItem {
                mission_id: Some(123),
                ..cargo(10)
            },
            CargoItem {
                stolen: Some(3),
                ..cargo(3)
            },
        ];
        let v = CargoView::build(
            &inventory,
            &basis(),
            &HashMap::from([("gold".into(), 10000)]),
            &HashMap::new(),
            None,
        );
        assert_eq!(v.used, 13);
        assert_eq!(v.lines[0].kind, CargoKind::Mission);
        assert_eq!(v.lines[1].kind, CargoKind::Stolen);
        assert!(
            v.lines
                .iter()
                .all(|l| l.est_profit.is_none() && l.advice.is_none())
        );
    }
    #[test]
    fn unknown_origin_cargo_marks_paid_partial() {
        let mut b = CostBasis::default();
        buy(&mut b, 50, 9400);
        b.apply_event(&EventKind::Cargo(Cargo {
            inventory: Some(vec![cargo(80)]),
            ..Default::default()
        }));
        assert!(b.is_partial("gold"));
        assert_eq!(b.avg_paid("gold"), Some(9400));
    }
    #[test]
    fn profit_and_advice() {
        let b = basis();
        let best = HashMap::from([("gold".into(), sell_option(11000))]);
        let v = CargoView::build(
            &[cargo(90)],
            &b,
            &HashMap::from([("gold".into(), 10000)]),
            &best,
            Some(256),
        );
        assert_eq!(v.lines[0].est_profit, Some(135000));
        assert_eq!(v.lines[0].advice, Some(CargoAdvice::Carry));
        let v = CargoView::build(
            &[cargo(90)],
            &b,
            &HashMap::from([("gold".into(), 10900)]),
            &best,
            None,
        );
        assert_eq!(v.lines[0].advice, Some(CargoAdvice::SellHere));
    }
    #[test]
    fn cost_rebuilt_after_restart_matches_live() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        let mut live = CostBasis::default();
        for (i, raw) in include_str!("../../../../tests/fixtures/journal/trade_session.log")
            .lines()
            .enumerate()
        {
            let ev = parse_line(raw).unwrap();
            live.apply_event(&ev.kind);
            writer.send(StoredEvent {
                file_name: "trade.log".into(),
                line: i as u64 + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some("TESTER".into()),
                json: raw.into(),
            });
        }
        writer.flush_blocking().unwrap();
        let r = cost_rebuild_task(
            "TESTER".into(),
            Utc.with_ymd_and_hms(2026, 10, 12, 0, 0, 0).unwrap(),
        )(&store)
        .unwrap();
        let rebuilt: CostBasis = serde_json::from_value(r["basis"].clone()).unwrap();
        assert_eq!(rebuilt, live);
        assert_eq!(rebuilt.avg_paid("gold"), Some(9500));
    }
    #[test]
    fn mixed_stolen_cargo_only_values_legal_tons_and_normalizes_keys() {
        let mut b = basis();
        let inventory = [CargoItem {
            name: "$gold_name;".into(),
            stolen: Some(3),
            ..cargo(93)
        }];
        b.reconcile(&inventory);
        assert!(!b.is_partial("Gold"));
        let v = CargoView::build(
            &inventory,
            &b,
            &HashMap::from([("gold".into(), 10000)]),
            &HashMap::new(),
            None,
        );
        assert_eq!(v.used, 93);
        assert_eq!(v.lines.iter().map(|l| l.tons).sum::<u32>(), 93);
        assert_eq!(v.lines[0].tons, 90);
        assert_eq!(v.lines[0].est_profit, Some(45000));
        assert_eq!(v.lines[1].est_profit, None);
    }
    #[test]
    fn absent_inventory_preserves_basis_and_ejection_scales_cost() {
        let mut b = basis();
        let before = b.clone();
        b.apply_event(&EventKind::Cargo(Cargo::default()));
        assert_eq!(b, before);
        b.apply_event(&EventKind::EjectCargo(EjectCargo {
            type_: "gold".into(),
            count: 60,
            ..Default::default()
        }));
        let before = b.clone();
        b.reconcile(&[cargo(90)]);
        assert_eq!(b, before);
        assert_eq!(b.avg_paid("gold"), Some(9500));
        b.reconcile(&[]);
        assert_eq!(b.avg_paid("gold"), None);
    }
    #[test]
    fn fractional_average_survives_sales_and_rebuy() {
        let mut b = CostBasis::default();
        buy(&mut b, 1, 1);
        buy(&mut b, 1, 2);
        b.apply_event(&EventKind::MarketSell(MarketSell {
            type_: "gold".into(),
            count: 1,
            ..Default::default()
        }));
        buy(&mut b, 1, 3);
        assert_eq!(b.avg_paid("gold"), Some(2));
        let mut b = CostBasis::default();
        buy(&mut b, 2, 1);
        buy(&mut b, 3, 2); // 1,6 cr/t; el valor expuesto se redondea a 2.
        b.apply_event(&EventKind::MarketSell(MarketSell {
            type_: "gold".into(),
            count: 4,
            ..Default::default()
        }));
        buy(&mut b, 1, 1); // 1,3 cr/t; redondear antes habría dado 1,5 → 2.
        assert_eq!(b.avg_paid("gold"), Some(1));
    }
    #[test]
    fn srv_inventory_does_not_replace_ship_basis() {
        let mut b = basis();
        let before = b.clone();
        b.apply_event(&EventKind::Cargo(Cargo {
            vessel: Some("SRV".into()),
            inventory: Some(vec![]),
            ..Default::default()
        }));
        assert_eq!(b, before);
    }
    #[test]
    fn rebuild_pages_and_filters_commander_and_since() {
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        let since = Utc.with_ymd_and_hms(2026, 10, 19, 12, 0, 0).unwrap();
        for i in 0..303 {
            let (cmdr, ts, price) = match i {
                0 => ("TESTER", "2026-10-18T12:00:00Z", 99999),
                1 => ("OTHER", "2026-10-19T12:00:00Z", 99999),
                _ => ("TESTER", "2026-10-19T12:00:00Z", 9400),
            };
            let raw=serde_json::json!({"timestamp":ts,"event":"MarketBuy","Type":"gold","Count":1,"BuyPrice":price}).to_string();
            let ev = parse_line(&raw).unwrap();
            writer.send(StoredEvent {
                file_name: "pages.log".into(),
                line: i + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some(cmdr.into()),
                json: raw,
            });
        }
        writer.flush_blocking().unwrap();
        let result = cost_rebuild_task("TESTER".into(), since)(&store).unwrap();
        let mut rebuilt: CostBasis = serde_json::from_value(result["basis"].clone()).unwrap();
        assert_eq!(rebuilt.avg_paid("gold"), Some(9400));
        let before = rebuilt.clone();
        rebuilt.reconcile(&[cargo(301)]);
        assert_eq!(before, rebuilt);
    }
    #[test]
    fn zero_unknown_and_extreme_prices_remain_safe() {
        let mut b = CostBasis::default();
        buy(&mut b, 0, 100);
        assert_eq!(b.avg_paid("gold"), None);
        b.reconcile(&[cargo(3)]);
        assert!(b.is_partial("gold"));
        assert_eq!(b.avg_paid("gold"), None);
        buy(&mut b, 2, 0);
        assert_eq!(b.avg_paid("gold"), Some(0));
        assert!(b.is_partial("gold"));
        let v = CargoView::build(
            &[cargo(u32::MAX)],
            &b,
            &HashMap::from([("gold".into(), u64::MAX)]),
            &HashMap::new(),
            None,
        );
        assert_eq!(v.lines[0].est_profit, Some(i64::MAX));
        let v = CargoView::build(
            &[CargoItem {
                stolen: Some(99),
                ..cargo(3)
            }],
            &b,
            &HashMap::new(),
            &HashMap::new(),
            None,
        );
        assert_eq!(v.lines.len(), 1);
        assert_eq!(v.lines[0].tons, 3);
        assert_eq!(v.lines[0].kind, CargoKind::Stolen);
    }
    #[test]
    fn advice_threshold_is_inclusive_and_no_best_requires_profit() {
        let b = basis();
        let best = HashMap::from([("gold".into(), sell_option(11000))]);
        for (price, expected) in [(10779, CargoAdvice::Carry), (10780, CargoAdvice::SellHere)] {
            let v = CargoView::build(
                &[cargo(1)],
                &b,
                &HashMap::from([("gold".into(), price)]),
                &best,
                None,
            );
            assert_eq!(v.lines[0].advice, Some(expected));
        }
        for (price, expected, profit) in [
            (9499, CargoAdvice::Carry, -1),
            (9500, CargoAdvice::Carry, 0),
            (9501, CargoAdvice::SellHere, 1),
        ] {
            let v = CargoView::build(
                &[cargo(1)],
                &b,
                &HashMap::from([("gold".into(), price)]),
                &HashMap::new(),
                None,
            );
            assert_eq!(v.lines[0].advice, Some(expected));
            assert_eq!(v.lines[0].est_profit, Some(profit));
        }
        let v = view(&b, &[cargo(1)]);
        assert_eq!(v.lines[0].est_profit, None);
        assert_eq!(v.lines[0].advice, None);
    }
}

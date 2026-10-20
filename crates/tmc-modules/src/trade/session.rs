//! Sesión de comercio: beneficio real frente al coste medio pagado (spec §10.4).
//!
//! Orden obligatorio por evento (lo cumple el cableado del módulo): primero
//! `TradeSession::apply_event` y después `CostBasis::apply_event`. Una venta
//! consume el coste medio de la carga, así que la sesión debe leerlo antes de
//! que `CostBasis` descuente las toneladas vendidas.

use super::CostBasis;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::events::{EventKind, commodity_key};

const MIN_ELAPSED_SECS: f64 = 60.0;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TradeSession {
    pub started_at: Option<DateTime<Utc>>,
    /// Ingresos menos coste medio de lo vendido.
    pub profit: i64,
    pub revenue: u64,
    pub cost_of_sold: u64,
    pub tons_bought: u32,
    pub tons_sold: u32,
    pub operations: u32,
    /// Mercancía y beneficio de la mejor venta individual.
    pub best_trade: Option<(String, i64)>,
    /// Toneladas vendidas sin coste conocido (ventas neutras).
    pub unknown_cost_tons: u32,
    /// Rango y progreso de comercio del último `Rank`/`Progress` (sobreviven al reinicio).
    pub rank: Option<u32>,
    pub progress: Option<u32>,
}

impl TradeSession {
    pub fn reset(&mut self, at: DateTime<Utc>) {
        *self = Self {
            started_at: Some(at),
            rank: self.rank,
            progress: self.progress,
            ..Self::default()
        };
    }

    /// Aplica un evento; devuelve `true` si cambió la sesión. Debe llamarse
    /// ANTES de `CostBasis::apply_event` con el mismo evento.
    ///
    /// Coste de lo vendido: `AvgPricePaid` del evento si existe (lo calcula el
    /// juego); si no, el coste exacto de `CostBasis`. Limitación: la carga robada
    /// de mercado negro se mezcla con la legal en la misma media.
    pub fn apply_event(&mut self, ev: &EventKind, basis: &CostBasis, at: DateTime<Utc>) -> bool {
        match ev {
            EventKind::LoadGame(_) => {
                self.reset(at);
                true
            }
            EventKind::Rank(r) => self.rank.replace(r.trade.unwrap_or_default()) != r.trade,
            EventKind::Progress(r) => self.progress.replace(r.trade.unwrap_or_default()) != r.trade,
            EventKind::MarketBuy(m) if m.count > 0 => {
                self.tons_bought = self.tons_bought.saturating_add(m.count);
                self.operations = self.operations.saturating_add(1);
                true
            }
            EventKind::MarketSell(m) if m.count > 0 => {
                let known = m
                    .avg_price_paid
                    .map(|a| a.saturating_mul(u64::from(m.count)))
                    .or_else(|| basis.sale_cost(&m.type_, m.count));
                // Sin coste conocido la venta es neutra (coste = ingreso) pero se cuenta.
                let cost = known.unwrap_or(m.total_sale);
                let sale_profit = (i128::from(m.total_sale) - i128::from(cost))
                    .clamp(i128::from(i64::MIN), i128::from(i64::MAX))
                    as i64;
                self.revenue = self.revenue.saturating_add(m.total_sale);
                self.cost_of_sold = self.cost_of_sold.saturating_add(cost);
                self.profit = self.profit.saturating_add(sale_profit);
                self.tons_sold = self.tons_sold.saturating_add(m.count);
                self.operations = self.operations.saturating_add(1);
                if known.is_none() {
                    self.unknown_cost_tons = self.unknown_cost_tons.saturating_add(m.count);
                } else if self
                    .best_trade
                    .as_ref()
                    .is_none_or(|(_, p)| sale_profit > *p)
                {
                    self.best_trade = Some((commodity_key(&m.type_), sale_profit));
                }
                decide!("trade.session.sale", {
                    "tons": m.count,
                    "cost_known": known.is_some(),
                    "profit_positive": sale_profit > 0,
                });
                true
            }
            _ => false,
        }
    }

    /// Créditos de beneficio por hora; `None` con menos de 60 s de sesión.
    pub fn credits_per_hour(&self, now: DateTime<Utc>) -> Option<i64> {
        let secs = (now - self.started_at?).num_milliseconds() as f64 / 1000.0;
        (secs >= MIN_ELAPSED_SECS).then(|| (self.profit as f64 / (secs / 3600.0)).round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use tmc_model::events::{LoadGame, MarketBuy, MarketSell};

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 19, 20, 0, 0).unwrap()
    }
    fn buy(count: u32, price: u64) -> EventKind {
        EventKind::MarketBuy(MarketBuy {
            type_: "Gold".into(),
            count,
            buy_price: price,
            total_cost: price * u64::from(count),
            ..Default::default()
        })
    }
    fn sell(count: u32, price: u64, avg: Option<u64>) -> EventKind {
        EventKind::MarketSell(MarketSell {
            type_: "Gold".into(),
            count,
            sell_price: price,
            total_sale: price * u64::from(count),
            avg_price_paid: avg,
            ..Default::default()
        })
    }
    /// Respeta el orden obligatorio: sesión y después coste.
    fn feed(s: &mut TradeSession, b: &mut CostBasis, ev: &EventKind) {
        s.apply_event(ev, b, t0());
        b.apply_event(ev);
    }
    fn started() -> (TradeSession, CostBasis) {
        let mut s = TradeSession::default();
        s.apply_event(
            &EventKind::LoadGame(LoadGame::default()),
            &CostBasis::default(),
            t0(),
        );
        (s, CostBasis::default())
    }

    #[test]
    fn session_profit_equals_sales_minus_avg_cost() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &buy(100, 9400));
        feed(&mut s, &mut b, &sell(100, 11000, None));
        assert_eq!(s.profit, 160_000);
        assert_eq!(s.revenue, 1_100_000);
        assert_eq!(s.cost_of_sold, 940_000);
        assert_eq!((s.operations, s.tons_bought, s.tons_sold), (2, 100, 100));
        assert_eq!(s.best_trade, Some(("gold".into(), 160_000)));
    }
    #[test]
    fn session_reads_basis_before_sale_is_applied() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &buy(10, 9000));
        let sale = sell(10, 11000, None);
        s.apply_event(&sale, &b, t0());
        b.apply_event(&sale);
        assert_eq!(s.profit, 20_000);
        // Orden inverso: el coste ya no existe y la venta no aporta beneficio.
        let (mut s2, mut b2) = started();
        feed(&mut s2, &mut b2, &buy(10, 9000));
        b2.apply_event(&sale);
        s2.apply_event(&sale, &b2, t0());
        assert_eq!(s2.profit, 0);
    }
    #[test]
    fn session_uses_avg_price_paid_when_no_basis() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &sell(10, 11000, Some(9000)));
        assert_eq!(s.profit, 20_000);
    }
    #[test]
    fn sale_without_any_cost_is_neutral() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &sell(10, 11000, None));
        assert_eq!((s.profit, s.tons_sold, s.operations), (0, 10, 1));
        assert_eq!((s.unknown_cost_tons, s.best_trade.clone()), (10, None));
    }
    #[test]
    fn load_game_resets_session() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &buy(10, 9000));
        feed(&mut s, &mut b, &sell(10, 11000, None));
        assert_eq!(s.profit, 20_000);
        let later = t0() + Duration::hours(1);
        s.apply_event(&EventKind::LoadGame(LoadGame::default()), &b, later);
        assert_eq!(s.profit, 0);
        assert_eq!(s.started_at, Some(later));
        assert_eq!((s.operations, s.best_trade), (0, None));
    }
    #[test]
    fn credits_per_hour_after_30_min() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &buy(100, 9400));
        feed(&mut s, &mut b, &sell(100, 11000, None));
        assert_eq!(
            s.credits_per_hour(t0() + Duration::minutes(30)),
            Some(320_000)
        );
        assert_eq!(s.credits_per_hour(t0() + Duration::seconds(30)), None);
        assert_eq!(TradeSession::default().credits_per_hour(t0()), None);
    }
    #[test]
    fn non_integer_average_cost_is_exact() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &buy(1, 100));
        feed(&mut s, &mut b, &buy(1, 101));
        feed(&mut s, &mut b, &sell(2, 150, None));
        assert_eq!(s.cost_of_sold, 201);
        assert_eq!(s.profit, 99);
    }
    #[test]
    fn avg_price_paid_wins_over_partial_basis() {
        let (mut s, mut b) = started();
        feed(&mut s, &mut b, &buy(10, 9000));
        b.reconcile(&[tmc_model::events::CargoItem {
            name: "Gold".into(),
            count: 20,
            ..Default::default()
        }]);
        assert!(b.is_partial("gold"));
        feed(&mut s, &mut b, &sell(10, 11000, Some(8000)));
        assert_eq!(s.cost_of_sold, 80_000);
        assert_eq!(s.unknown_cost_tons, 0);
    }
    #[test]
    fn rank_and_progress_survive_reset() {
        use tmc_model::events::Rank;
        let (mut s, b) = started();
        let r = Rank {
            trade: Some(5),
            ..Default::default()
        };
        s.apply_event(&EventKind::Rank(r.clone()), &b, t0());
        s.apply_event(
            &EventKind::Progress(Rank {
                trade: Some(40),
                ..r
            }),
            &b,
            t0(),
        );
        s.reset(t0());
        assert_eq!((s.rank, s.progress), (Some(5), Some(40)));
    }
}

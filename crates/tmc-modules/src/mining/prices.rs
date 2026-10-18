//! Tipos de precio de materiales; `PriceBook` (Task 21) resolverá `PriceFn`.

use serde::Serialize;

/// Origen del precio (siempre una estimación).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceSource {
    /// `Market.json` del mercado actual.
    Market,
    /// Venta propia registrada (`MarketSell`).
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PriceInfo {
    pub price: u64,
    pub source: PriceSource,
}

/// Resuelve el mejor precio conocido de una clave de materia (`commodity_key`).
pub type PriceFn<'a> = &'a dyn Fn(&str) -> Option<PriceInfo>;

//! Mercados, precios de materias primas y estaciones conocidas.

use crate::net::PadSize;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Identidad de un mercado candidato para reconciliar observaciones locales.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketIdentity {
    pub market_id: Option<u64>,
    pub station_name: String,
    pub system_name: String,
}

/// Procedencia de un precio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceOrigin {
    Local,
    Eddn,
    Ardent,
}

/// Lado de la operación desde el punto de vista del jugador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommodityPrice {
    /// Clave normalizada de la mercancía.
    pub commodity: String,
    pub name_localised: Option<String>,
    pub buy_price: u64,
    pub sell_price: u64,
    pub stock: u64,
    pub demand: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub market_id: u64,
    pub station_name: String,
    pub system_name: String,
    pub system_address: Option<u64>,
    pub star_pos: Option<[f64; 3]>,
    pub max_pad: Option<PadSize>,
    pub station_type: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub origin: PriceOrigin,
    pub prices: Vec<CommodityPrice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StationInfo {
    pub market_id: u64,
    pub station_name: String,
    pub system_name: String,
    pub star_pos: Option<[f64; 3]>,
    pub max_pad: Option<PadSize>,
    pub station_type: Option<String>,
    pub dist_ls: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricedMarket {
    pub market_id: u64,
    pub station_name: String,
    pub system_name: String,
    pub distance_ly: Option<f64>,
    pub max_pad: Option<PadSize>,
    pub price: u64,
    /// Stock (al comprar) o demanda (al vender).
    pub volume: u64,
    pub updated_at: DateTime<Utc>,
    pub origin: PriceOrigin,
}

/// Portaaviones de flota: tipo `FleetCarrier`, o `MarketID` alto con nombre `XXX-XXX`.
pub fn is_fleet_carrier(
    market_id: u64,
    station_type: Option<&str>,
    station_name: Option<&str>,
) -> bool {
    if station_type.is_some_and(|t| t.eq_ignore_ascii_case("FleetCarrier")) {
        return true;
    }
    market_id >= 3_700_000_000 && station_name.is_some_and(is_carrier_callsign)
}

fn is_carrier_callsign(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 7
        && b[3] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 3 || c.is_ascii_uppercase() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_fleet_carrier_rule() {
        assert!(is_fleet_carrier(3_700_000_123, None, Some("K7Q-B2X")));
        assert!(!is_fleet_carrier(
            3_700_000_123,
            None,
            Some("Nueva Colonia")
        ));
        assert!(!is_fleet_carrier(
            128_000_000,
            Some("Coriolis"),
            Some("Test Port")
        ));
        assert!(is_fleet_carrier(1, Some("FleetCarrier"), None));
    }
}

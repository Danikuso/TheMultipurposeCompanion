//! Descompresión y decodificación de mensajes EDDN.

use chrono::{DateTime, Duration, Utc};
use flate2::read::ZlibDecoder;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Read;
use tmc_model::events::commodity_key;
use tmc_model::market::{
    CommodityPrice, MarketSnapshot, PriceOrigin, StationInfo, is_fleet_carrier,
};
use tmc_model::net::PadSize;

pub const COMMODITY_SCHEMA: &str = "https://eddn.edcd.io/schemas/commodity/3";
pub const JOURNAL_SCHEMA: &str = "https://eddn.edcd.io/schemas/journal/1";
pub const MAX_INFLATED_BYTES: usize = 1 << 20;

const MAX_AGE_DAYS: i64 = 7;
const FUTURE_TOLERANCE_MIN: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DropReason {
    Corrupt,
    TooLarge,
    Schema,
    TestSchema,
    Stale,
    Future,
    Carrier,
    Far,
    UnknownCoords,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decoded {
    Market(MarketSnapshot),
    /// Estación de un `Docked` y, si hay `StarPos`, las coordenadas de su sistema.
    Station(StationInfo, Option<(String, Option<u64>, [f64; 3])>),
    Coords(String, Option<u64>, [f64; 3]),
    Ignored(DropReason),
}

/// MarketID de carriers vistos en `Docked`.
#[derive(Debug, Default, Clone)]
pub struct CarrierSet(HashSet<u64>);

impl CarrierSet {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn remember(&mut self, market_id: u64) {
        self.0.insert(market_id);
    }
    pub fn contains(&self, market_id: u64) -> bool {
        self.0.contains(&market_id)
    }
}

#[derive(Deserialize)]
struct SchemaOnly {
    #[serde(rename = "$schemaRef")]
    schema: String,
}

#[derive(Deserialize)]
struct Envelope<M> {
    message: M,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommodityMsg {
    timestamp: DateTime<Utc>,
    system_name: String,
    station_name: String,
    market_id: u64,
    commodities: Vec<RawCommodity>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCommodity {
    name: String,
    buy_price: u64,
    sell_price: u64,
    stock: u64,
    demand: u64,
}

#[derive(Deserialize)]
struct JournalHead {
    timestamp: DateTime<Utc>,
    event: String,
}

#[derive(Deserialize)]
struct JournalBody {
    #[serde(rename = "StarSystem")]
    star_system: Option<String>,
    #[serde(rename = "SystemAddress")]
    system_address: Option<u64>,
    #[serde(rename = "StarPos")]
    star_pos: Option<[f64; 3]>,
    #[serde(rename = "StationName")]
    station_name: Option<String>,
    #[serde(rename = "StationType")]
    station_type: Option<String>,
    #[serde(rename = "MarketID")]
    market_id: Option<u64>,
    #[serde(rename = "DistFromStarLS")]
    dist_ls: Option<f64>,
    #[serde(rename = "LandingPads")]
    landing_pads: Option<Pads>,
}

#[derive(Deserialize, Default)]
struct Pads {
    #[serde(rename = "Small", default)]
    small: u64,
    #[serde(rename = "Medium", default)]
    medium: u64,
    #[serde(rename = "Large", default)]
    large: u64,
}

fn inflate(frame: &[u8]) -> Result<Vec<u8>, DropReason> {
    let mut out = Vec::new();
    ZlibDecoder::new(frame)
        .take(MAX_INFLATED_BYTES as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| DropReason::Corrupt)?;
    if out.len() > MAX_INFLATED_BYTES {
        return Err(DropReason::TooLarge);
    }
    Ok(out)
}

fn check_time(ts: DateTime<Utc>, now: DateTime<Utc>) -> Result<(), DropReason> {
    if ts > now + Duration::minutes(FUTURE_TOLERANCE_MIN) {
        Err(DropReason::Future)
    } else if ts < now - Duration::days(MAX_AGE_DAYS) {
        Err(DropReason::Stale)
    } else {
        Ok(())
    }
}

/// Decodifica un fotograma sin carriers recordados.
pub fn decode(frame: &[u8], now: DateTime<Utc>) -> Decoded {
    decode_with(frame, now, &CarrierSet::default())
}

pub fn decode_with(frame: &[u8], now: DateTime<Utc>, carriers: &CarrierSet) -> Decoded {
    match try_decode(frame, now, carriers) {
        Ok(d) => d,
        Err(r) => Decoded::Ignored(r),
    }
}

fn try_decode(
    frame: &[u8],
    now: DateTime<Utc>,
    carriers: &CarrierSet,
) -> Result<Decoded, DropReason> {
    let bytes = inflate(frame)?;
    // Prefiltro: solo se lee `$schemaRef`; el resto se salta sin construir valores.
    let head: SchemaOnly = serde_json::from_slice(&bytes).map_err(|_| DropReason::Corrupt)?;
    match head.schema.as_str() {
        COMMODITY_SCHEMA => decode_commodity(&bytes, now, carriers),
        JOURNAL_SCHEMA => decode_journal(&bytes, now),
        s if s.ends_with("/test") => Err(DropReason::TestSchema),
        _ => Err(DropReason::Schema),
    }
}

fn decode_commodity(
    bytes: &[u8],
    now: DateTime<Utc>,
    carriers: &CarrierSet,
) -> Result<Decoded, DropReason> {
    let m = serde_json::from_slice::<Envelope<CommodityMsg>>(bytes)
        .map_err(|_| DropReason::Corrupt)?
        .message;
    check_time(m.timestamp, now)?;
    if carriers.contains(m.market_id) || is_fleet_carrier(m.market_id, None, Some(&m.station_name))
    {
        return Err(DropReason::Carrier);
    }
    let prices = m
        .commodities
        .into_iter()
        .map(|c| CommodityPrice {
            commodity: commodity_key(&c.name),
            name_localised: None,
            buy_price: c.buy_price,
            sell_price: c.sell_price,
            stock: c.stock,
            demand: c.demand,
        })
        .collect();
    Ok(Decoded::Market(MarketSnapshot {
        market_id: m.market_id,
        station_name: m.station_name,
        system_name: m.system_name,
        system_address: None,
        star_pos: None,
        max_pad: None,
        station_type: None,
        updated_at: m.timestamp,
        origin: PriceOrigin::Eddn,
        prices,
    }))
}

fn decode_journal(bytes: &[u8], now: DateTime<Utc>) -> Result<Decoded, DropReason> {
    let env: Envelope<serde_json::Value> =
        serde_json::from_slice(bytes).map_err(|_| DropReason::Corrupt)?;
    let head: JournalHead =
        serde_json::from_value(env.message.clone()).map_err(|_| DropReason::Corrupt)?;
    if !matches!(
        head.event.as_str(),
        "Docked" | "FSDJump" | "Location" | "CarrierJump"
    ) {
        return Err(DropReason::Schema);
    }
    check_time(head.timestamp, now)?;
    let b: JournalBody = serde_json::from_value(env.message).map_err(|_| DropReason::Corrupt)?;
    let system = b.star_system.ok_or(DropReason::Corrupt)?;
    let coords = b
        .star_pos
        .map(|p| (system.to_lowercase(), b.system_address, p));
    if head.event != "Docked" {
        return match coords {
            Some((k, id, p)) => Ok(Decoded::Coords(k, id, p)),
            None => Err(DropReason::Corrupt),
        };
    }
    let max_pad = b.landing_pads.and_then(|p| {
        if p.large > 0 {
            Some(PadSize::Large)
        } else if p.medium > 0 {
            Some(PadSize::Medium)
        } else if p.small > 0 {
            Some(PadSize::Small)
        } else {
            None
        }
    });
    let info = StationInfo {
        market_id: b.market_id.ok_or(DropReason::Corrupt)?,
        station_name: b.station_name.ok_or(DropReason::Corrupt)?,
        system_name: system,
        star_pos: b.star_pos,
        max_pad,
        station_type: b.station_type,
        dist_ls: b.dist_ls,
    };
    Ok(Decoded::Station(info, coords))
}

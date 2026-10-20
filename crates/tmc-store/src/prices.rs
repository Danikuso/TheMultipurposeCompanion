//! Precios de mercado, estaciones y coordenadas de sistemas.

use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, Transaction, params};
use std::collections::HashMap;
use tmc_model::market::{
    CommodityPrice, MarketIdentity, MarketSnapshot, PriceOrigin, PricedMarket, Side, StationInfo,
};
use tmc_model::net::PadSize;

fn ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| StoreError::Unsupported(format!("bad timestamp: {e}")))
}

fn origin_str(o: PriceOrigin) -> &'static str {
    match o {
        PriceOrigin::Local => "local",
        PriceOrigin::Eddn => "eddn",
        PriceOrigin::Ardent => "ardent",
    }
}

fn parse_origin(s: &str) -> PriceOrigin {
    match s {
        "eddn" => PriceOrigin::Eddn,
        "ardent" => PriceOrigin::Ardent,
        _ => PriceOrigin::Local,
    }
}

fn pad_num(p: Option<PadSize>) -> Option<i64> {
    p.map(|p| match p {
        PadSize::Small => 1,
        PadSize::Medium => 2,
        PadSize::Large => 3,
    })
}

fn pad_from(n: Option<i64>) -> Option<PadSize> {
    n.and_then(|n| u8::try_from(n).ok())
        .and_then(PadSize::from_ardent)
}

/// Días de retención de precios y estaciones.
pub const PRICE_RETENTION_DAYS: i64 = 7;
/// Días de retención de coordenadas de sistemas.
pub const COORDS_RETENTION_DAYS: i64 = 30;

/// Filas borradas por `purge_prices_older_than`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PurgeCounts {
    pub prices: usize,
    pub stations: usize,
    pub coords: usize,
}

fn unsigned(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

fn id(v: u64) -> Result<i64, StoreError> {
    Ok(i64::try_from(v)?)
}

fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

#[allow(clippy::too_many_arguments)]
fn upsert_station(
    tx: &Transaction<'_>,
    market_id: u64,
    station_name: &str,
    system_name: &str,
    system_address: Option<u64>,
    pos: Option<[f64; 3]>,
    pad: Option<PadSize>,
    station_type: Option<&str>,
    dist_ls: Option<f64>,
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    let [x, y, z] = pos.map_or([None; 3], |p| p.map(Some));
    let address = system_address.map(id).transpose()?;
    tx.execute(
        "INSERT INTO market_station(market_id,station_name,system_name,system_address,x,y,z,max_pad,station_type,dist_ls,updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(market_id) DO UPDATE SET
           station_name=CASE WHEN excluded.updated_at>updated_at THEN excluded.station_name ELSE station_name END,
           system_name=CASE WHEN excluded.updated_at>updated_at THEN excluded.system_name ELSE system_name END,
           system_address=COALESCE(excluded.system_address, system_address),
           x=COALESCE(excluded.x, x), y=COALESCE(excluded.y, y), z=COALESCE(excluded.z, z),
           max_pad=COALESCE(excluded.max_pad, max_pad),
           station_type=COALESCE(excluded.station_type, station_type),
           dist_ls=COALESCE(excluded.dist_ls, dist_ls),
           updated_at=MAX(updated_at, excluded.updated_at)",
        params![
            id(market_id)?,
            station_name,
            system_name,
            address,
            x,
            y,
            z,
            pad_num(pad),
            station_type,
            dist_ls,
            ts(at)
        ],
    )?;
    Ok(())
}

fn upsert_coord(
    tx: &Transaction<'_>,
    name: &str,
    address: Option<u64>,
    p: [f64; 3],
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    tx.execute(
        "INSERT INTO system_coords(name_key,system_address,x,y,z,seen_at) VALUES(?1,?2,?3,?4,?5,?6)
         ON CONFLICT(name_key) DO UPDATE SET system_address=COALESCE(excluded.system_address, system_address),
           x=excluded.x, y=excluded.y, z=excluded.z, seen_at=MAX(seen_at, excluded.seen_at)",
        params![key(name), address.map(id).transpose()?, p[0], p[1], p[2], ts(at)],
    )?;
    Ok(())
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

impl Store {
    /// Sustituye el catálogo de cada mercado si el snapshot es más nuevo que lo guardado.
    /// Devuelve cuántos snapshots se aplicaron.
    pub fn replace_markets(&self, snaps: &[MarketSnapshot]) -> Result<usize, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let mut applied = 0;
        for s in snaps {
            let mid = id(s.market_id)?;
            let stored: Option<String> = tx
                .query_row(
                    "SELECT updated_at FROM market_full_snapshot WHERE market_id=?1",
                    [mid],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(stored) = stored
                && parse_ts(&stored)? >= s.updated_at
            {
                continue;
            }
            // Solo se borran filas no más nuevas que el snapshot: los parciales posteriores sobreviven.
            tx.execute(
                "DELETE FROM market_price WHERE market_id=?1 AND updated_at<=?2",
                params![mid, ts(s.updated_at)],
            )?;
            for p in &s.prices {
                tx.execute(
                    "INSERT INTO market_price(market_id,commodity,name_localised,buy_price,sell_price,stock,demand,updated_at,origin)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
                     ON CONFLICT(market_id,commodity) DO NOTHING",
                    params![
                        mid,
                        key(&p.commodity),
                        p.name_localised,
                        id(p.buy_price)?,
                        id(p.sell_price)?,
                        id(p.stock)?,
                        id(p.demand)?,
                        ts(s.updated_at),
                        origin_str(s.origin)
                    ],
                )?;
            }
            tx.execute(
                "INSERT INTO market_full_snapshot(market_id,updated_at,origin) VALUES(?1,?2,?3)
                 ON CONFLICT(market_id) DO UPDATE SET updated_at=excluded.updated_at,origin=excluded.origin",
                params![mid,ts(s.updated_at),origin_str(s.origin)],
            )?;
            upsert_station(
                &tx,
                s.market_id,
                &s.station_name,
                &s.system_name,
                s.system_address,
                s.star_pos,
                s.max_pad,
                s.station_type.as_deref(),
                None,
                s.updated_at,
            )?;
            if let Some(pos) = s.star_pos {
                upsert_coord(&tx, &s.system_name, s.system_address, pos, s.updated_at)?;
            }
            applied += 1;
        }
        tx.commit()?;
        Ok(applied)
    }

    /// Precios sueltos (Ardent): la observación más nueva gana por (mercado, mercancía).
    pub fn upsert_partial_prices(
        &self,
        rows: &[(StationInfo, CommodityPrice, DateTime<Utc>, PriceOrigin)],
    ) -> Result<usize, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let mut changed = 0;
        for (st, p, at, origin) in rows {
            upsert_station(
                &tx,
                st.market_id,
                &st.station_name,
                &st.system_name,
                None,
                st.star_pos,
                st.max_pad,
                st.station_type.as_deref(),
                st.dist_ls,
                *at,
            )?;
            changed += tx.execute(
                "INSERT INTO market_price(market_id,commodity,name_localised,buy_price,sell_price,stock,demand,updated_at,origin)
                 SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9
                 WHERE NOT EXISTS (
                   SELECT 1 FROM market_full_snapshot f WHERE f.market_id=?1 AND f.updated_at>=?8
                     AND NOT EXISTS (SELECT 1 FROM market_price current WHERE current.market_id=?1 AND current.commodity=?2)
                 )
                 ON CONFLICT(market_id,commodity) DO UPDATE SET
                   name_localised=COALESCE(excluded.name_localised, name_localised),
                   buy_price=excluded.buy_price, sell_price=excluded.sell_price, stock=excluded.stock,
                   demand=excluded.demand, updated_at=excluded.updated_at, origin=excluded.origin
                 WHERE excluded.updated_at > market_price.updated_at",
                params![
                    id(st.market_id)?,
                    key(&p.commodity),
                    p.name_localised,
                    id(p.buy_price)?,
                    id(p.sell_price)?,
                    id(p.stock)?,
                    id(p.demand)?,
                    ts(*at),
                    origin_str(*origin)
                ],
            )?;
        }
        tx.commit()?;
        Ok(changed)
    }

    pub fn upsert_stations(
        &self,
        st: &[StationInfo],
        at: DateTime<Utc>,
    ) -> Result<usize, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        for s in st {
            upsert_station(
                &tx,
                s.market_id,
                &s.station_name,
                &s.system_name,
                None,
                s.star_pos,
                s.max_pad,
                s.station_type.as_deref(),
                s.dist_ls,
                at,
            )?;
        }
        tx.commit()?;
        Ok(st.len())
    }

    pub fn upsert_coords(
        &self,
        c: &[(String, Option<u64>, [f64; 3])],
        at: DateTime<Utc>,
    ) -> Result<usize, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        for (name, address, pos) in c {
            upsert_coord(&tx, name, *address, *pos, at)?;
        }
        tx.commit()?;
        Ok(c.len())
    }

    /// Claves en minúsculas; los nombres de entrada se normalizan igual.
    pub fn coords_by_names(
        &self,
        names: &[String],
    ) -> Result<HashMap<String, [f64; 3]>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare_cached("SELECT x,y,z FROM system_coords WHERE name_key=?1")?;
        let mut out = HashMap::new();
        for name in names {
            let k = key(name);
            if let Some(p) = stmt
                .query_row([&k], |r| Ok([r.get(0)?, r.get(1)?, r.get(2)?]))
                .optional()?
            {
                out.insert(k, p);
            }
        }
        Ok(out)
    }

    pub fn coords_within(
        &self,
        center: [f64; 3],
        radius_ly: f64,
        limit: usize,
    ) -> Result<Vec<(String, [f64; 3])>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare_cached(
            "SELECT name_key,x,y,z FROM system_coords WHERE x BETWEEN ?1 AND ?2 AND y BETWEEN ?3 AND ?4 AND z BETWEEN ?5 AND ?6",
        )?;
        let rows = stmt.query_map(
            params![
                center[0] - radius_ly,
                center[0] + radius_ly,
                center[1] - radius_ly,
                center[1] + radius_ly,
                center[2] - radius_ly,
                center[2] + radius_ly
            ],
            |r| Ok((r.get::<_, String>(0)?, [r.get(1)?, r.get(2)?, r.get(3)?])),
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (name, p) = row?;
            let d = dist(center, p);
            if d <= radius_ly {
                out.push((d, name, p));
            }
        }
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.truncate(limit);
        Ok(out.into_iter().map(|(_, n, p)| (n, p)).collect())
    }

    /// Mejores mercados para una mercancía dentro del radio. `Sell` exige demanda y ordena por
    /// precio descendente; `Buy` exige stock y ordena ascendente. Sin coordenadas, se excluye.
    #[allow(clippy::too_many_arguments)]
    pub fn best_prices(
        &self,
        commodity: &str,
        side: Side,
        center: [f64; 3],
        radius_ly: f64,
        since: DateTime<Utc>,
        exclude_market: Option<u64>,
        limit: usize,
    ) -> Result<Vec<PricedMarket>, StoreError> {
        self.query_prices(
            commodity,
            side,
            center,
            radius_ly,
            since,
            exclude_market,
            limit,
            false,
            None,
            None,
            false,
        )
    }

    /// Mejores precios compatibles con la plataforma de la nave.
    #[allow(clippy::too_many_arguments)]
    pub fn best_prices_for_ship(
        &self,
        commodity: &str,
        side: Side,
        center: [f64; 3],
        radius_ly: f64,
        since: DateTime<Utc>,
        exclude_market: Option<u64>,
        limit: usize,
        ship_pad: Option<PadSize>,
    ) -> Result<Vec<PricedMarket>, StoreError> {
        self.query_prices(
            commodity,
            side,
            center,
            radius_ly,
            since,
            exclude_market,
            limit,
            false,
            None,
            ship_pad,
            true,
        )
    }

    /// Observaciones para reconciliar fuentes, incluidas ofertas sin volumen o precio.
    /// `limit` recorta la respuesta; un resultado recortado no prueba la ausencia
    /// de una observación más reciente para mercados que no aparecen en él.
    #[allow(clippy::too_many_arguments)]
    pub fn price_observations(
        &self,
        commodity: &str,
        side: Side,
        center: [f64; 3],
        radius_ly: f64,
        since: DateTime<Utc>,
        exclude_market: Option<u64>,
        limit: usize,
    ) -> Result<Vec<PricedMarket>, StoreError> {
        self.query_prices(
            commodity,
            side,
            center,
            radius_ly,
            since,
            exclude_market,
            limit,
            true,
            None,
            None,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn price_observations_for_markets(
        &self,
        commodity: &str,
        side: Side,
        identities: &[MarketIdentity],
        center: [f64; 3],
        radius_ly: f64,
        since: DateTime<Utc>,
        exclude_market: Option<u64>,
    ) -> Result<Vec<PricedMarket>, StoreError> {
        if identities.len() > 50 {
            return Err(StoreError::Unsupported(
                "at most 50 candidate markets per commodity".into(),
            ));
        }
        if identities.is_empty() {
            return Ok(Vec::new());
        }
        self.query_prices(
            commodity,
            side,
            center,
            radius_ly,
            since,
            exclude_market,
            identities.len(),
            true,
            Some(identities),
            None,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn query_prices(
        &self,
        commodity: &str,
        side: Side,
        center: [f64; 3],
        radius_ly: f64,
        since: DateTime<Utc>,
        exclude_market: Option<u64>,
        limit: usize,
        include_closed: bool,
        identities: Option<&[MarketIdentity]>,
        ship_pad: Option<PadSize>,
        positive_buy_price: bool,
    ) -> Result<Vec<PricedMarket>, StoreError> {
        let (raw_price_col, raw_vol_col, order) = match side {
            Side::Sell => ("p.sell_price", "p.demand", "DESC"),
            Side::Buy => ("p.buy_price", "p.stock", "ASC"),
        };
        let cond = if include_closed {
            "1".to_owned()
        } else if positive_buy_price && side == Side::Buy {
            // Un precio de compra cero no es una oferta: no debe consumir el límite.
            format!("{raw_vol_col} > 0 AND {raw_price_col} > 0")
        } else {
            format!("{raw_vol_col} > 0")
        };
        let (price_col, vol_col, updated_col, origin_col, from, commodity_condition) =
            if include_closed {
                (
                    format!("COALESCE({raw_price_col},0)"),
                    format!("COALESCE({raw_vol_col},0)"),
                    "COALESCE(p.updated_at,f.updated_at)",
                    "COALESCE(p.origin,f.origin)",
                    "market_station s LEFT JOIN market_price p ON p.market_id=s.market_id AND p.commodity=?1 LEFT JOIN market_full_snapshot f ON f.market_id=s.market_id",
                    "1",
                )
            } else {
                (
                    raw_price_col.to_owned(),
                    raw_vol_col.to_owned(),
                    "p.updated_at",
                    "p.origin",
                    "market_price p JOIN market_station s ON s.market_id=p.market_id",
                    "p.commodity=?1",
                )
            };
        use rusqlite::types::Value as SqlValue;
        let exclude = exclude_market.map(id).transpose()?;
        let mut values: Vec<SqlValue> = vec![
            key(commodity).into(),
            ts(since).into(),
            (center[0] - radius_ly).into(),
            (center[0] + radius_ly).into(),
            (center[1] - radius_ly).into(),
            (center[1] + radius_ly).into(),
            (center[2] - radius_ly).into(),
            (center[2] + radius_ly).into(),
            exclude.map(SqlValue::Integer).unwrap_or(SqlValue::Null),
        ];
        let identity_filter = if let Some(identities) = identities {
            let mut clauses = Vec::new();
            for identity in identities {
                let idx = values.len() + 1;
                if let Some(mid) = identity.market_id {
                    values.push(id(mid)?.into());
                    clauses.push(format!("s.market_id=?{idx}"));
                } else {
                    values.push(identity.station_name.trim().to_owned().into());
                    values.push(identity.system_name.trim().to_owned().into());
                    clauses.push(format!("(s.station_name COLLATE NOCASE=?{idx} AND s.system_name COLLATE NOCASE=?{} AND 1=(SELECT COUNT(*) FROM market_station ident WHERE ident.station_name COLLATE NOCASE=s.station_name AND ident.system_name COLLATE NOCASE=s.system_name))",idx+1));
                }
            }
            format!(" AND ({})", clauses.join(" OR "))
        } else {
            String::new()
        };
        let sql = format!(
            "SELECT s.market_id,s.station_name,s.system_name,s.x,s.y,s.z,s.max_pad,{price_col},{vol_col},{updated_col},{origin_col},s.station_type
             FROM {from}
             WHERE {commodity_condition} AND {updated_col}>=?2 AND {cond}
               AND s.x BETWEEN ?3 AND ?4 AND s.y BETWEEN ?5 AND ?6 AND s.z BETWEEN ?7 AND ?8
               AND (?9 IS NULL OR s.market_id<>?9){identity_filter}
             ORDER BY {price_col} {order}, {updated_col} DESC"
        );
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(values), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                [r.get::<_, f64>(3)?, r.get(4)?, r.get(5)?],
                r.get::<_, Option<i64>>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, Option<String>>(11)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (mid, station_name, system_name, pos, pad, price, volume, at, origin, station_type) =
                row?;
            // Los portaaviones no consumen el límite: nunca son destino de venta.
            if tmc_model::market::is_fleet_carrier(
                unsigned(mid),
                station_type.as_deref(),
                Some(&station_name),
            ) {
                continue;
            }
            let d = dist(center, pos);
            if d > radius_ly {
                continue;
            }
            let max_pad = pad_from(pad);
            if max_pad
                .zip(ship_pad)
                .is_some_and(|(station, ship)| !station.fits(ship))
            {
                continue;
            }
            out.push(PricedMarket {
                market_id: unsigned(mid),
                station_name,
                system_name,
                distance_ly: Some(d),
                max_pad,
                price: unsigned(price),
                volume: unsigned(volume),
                updated_at: parse_ts(&at)?,
                origin: parse_origin(&origin),
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    pub fn market_prices(&self, market_id: u64) -> Result<Vec<CommodityPrice>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT commodity,name_localised,buy_price,sell_price,stock,demand FROM market_price WHERE market_id=?1 ORDER BY commodity",
        )?;
        let rows = stmt.query_map([id(market_id)?], |r| {
            Ok(CommodityPrice {
                commodity: r.get(0)?,
                name_localised: r.get(1)?,
                buy_price: unsigned(r.get::<_, i64>(2)?),
                sell_price: unsigned(r.get::<_, i64>(3)?),
                stock: unsigned(r.get::<_, i64>(4)?),
                demand: unsigned(r.get::<_, i64>(5)?),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Mercancías conocidas con su último nombre localizado no nulo.
    pub fn known_commodities(
        &self,
        limit: usize,
    ) -> Result<Vec<(String, Option<String>)>, StoreError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT commodity,
                    (SELECT name_localised FROM market_price q
                      WHERE q.commodity=p.commodity AND q.name_localised IS NOT NULL
                      ORDER BY q.updated_at DESC LIMIT 1)
             FROM market_price p GROUP BY commodity ORDER BY commodity LIMIT ?1",
        )?;
        let rows = stmt.query_map([i64::try_from(limit)?], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Borra precios y estaciones anteriores a `price_cutoff` y coordenadas anteriores a
    /// `coords_cutoff` (ver `PRICE_RETENTION_DAYS` y `COORDS_RETENTION_DAYS`).
    pub fn purge_prices_older_than(
        &self,
        price_cutoff: DateTime<Utc>,
        coords_cutoff: DateTime<Utc>,
    ) -> Result<PurgeCounts, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let prices = tx.execute(
            "DELETE FROM market_price WHERE updated_at<?1",
            [ts(price_cutoff)],
        )?;
        tx.execute(
            "DELETE FROM market_full_snapshot WHERE updated_at<?1",
            [ts(price_cutoff)],
        )?;
        let stations = tx.execute(
            "DELETE FROM market_station WHERE updated_at<?1",
            [ts(price_cutoff)],
        )?;
        let coords = tx.execute(
            "DELETE FROM system_coords WHERE seen_at<?1",
            [ts(coords_cutoff)],
        )?;
        tx.commit()?;
        Ok(PurgeCounts {
            prices,
            stations,
            coords,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use tmc_model::net::PadSize;

    fn t(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 19, h, 0, 0).unwrap()
    }
    fn price(c: &str, loc: Option<&str>, sell: u64, demand: u64) -> CommodityPrice {
        CommodityPrice {
            commodity: c.into(),
            name_localised: loc.map(str::to_owned),
            buy_price: 0,
            sell_price: sell,
            stock: 0,
            demand,
        }
    }
    fn snap(id: u64, x: f64, at: DateTime<Utc>, prices: Vec<CommodityPrice>) -> MarketSnapshot {
        MarketSnapshot {
            market_id: id,
            station_name: format!("Port {id}"),
            system_name: format!("Sys {id}"),
            system_address: Some(id),
            star_pos: Some([x, 0.0, 0.0]),
            max_pad: Some(PadSize::Large),
            station_type: Some("Coriolis".into()),
            updated_at: at,
            origin: PriceOrigin::Local,
            prices,
        }
    }
    #[test]
    fn observations_keep_closed_sales_and_purchases_for_source_reconciliation() {
        let store = Store::open_in_memory().unwrap();
        let mut p = price("gold", None, 11000, 0);
        p.buy_price = 9400;
        store
            .replace_markets(&[snap(1, 0.0, t(10), vec![p])])
            .unwrap();
        for side in [Side::Sell, Side::Buy] {
            let observed = store
                .price_observations("gold", side, [0.0; 3], 100.0, t(0), None, 5)
                .unwrap();
            assert_eq!(observed.len(), 1);
            assert_eq!(observed[0].volume, 0);
            assert!(near(&store, side, t(0)).is_empty());
        }
        store
            .replace_markets(&[snap(1, 0.0, t(11), vec![price("gold", None, 0, 50)])])
            .unwrap();
        let observed = store
            .price_observations("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, 5)
            .unwrap();
        assert_eq!(observed.len(), 1);
        assert_eq!((observed[0].price, observed[0].volume), (0, 50));
    }

    #[test]
    fn limited_observations_are_partial_and_can_omit_a_closed_market() {
        let store = Store::open_in_memory().unwrap();
        store
            .replace_markets(&[
                snap(1, 0.0, t(10), vec![price("gold", None, 11000, 10)]),
                snap(2, 0.0, t(11), vec![price("gold", None, 0, 0)]),
            ])
            .unwrap();
        let partial = store
            .price_observations("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, 1)
            .unwrap();
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].market_id, 1);
        let complete = store
            .price_observations("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, usize::MAX)
            .unwrap();
        assert_eq!(complete.len(), 2);
        assert_eq!(complete[1].market_id, 2);
        assert_eq!(complete[1].price, 0);
    }

    #[test]
    fn targeted_observations_keep_closed_market_outside_top_limit() {
        let store = Store::open_in_memory().unwrap();
        store
            .replace_markets(&[
                snap(1, 0.0, t(10), vec![price("gold", None, 11000, 10)]),
                snap(2, 0.0, t(11), vec![price("gold", None, 0, 0)]),
            ])
            .unwrap();
        assert_eq!(
            store
                .best_prices("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, 1)
                .unwrap()[0]
                .market_id,
            1
        );
        let candidate = MarketIdentity {
            market_id: Some(2),
            station_name: "ignored if id known".into(),
            system_name: "ignored".into(),
        };
        let observed = store
            .price_observations_for_markets(
                "gold",
                Side::Sell,
                &[candidate],
                [0.0; 3],
                100.0,
                t(0),
                None,
            )
            .unwrap();
        assert_eq!(observed.len(), 1);
        assert_eq!((observed[0].market_id, observed[0].price), (2, 0));
    }

    #[test]
    fn targeted_idless_names_require_unambiguous_station_and_keep_bounds() {
        let store = Store::open_in_memory().unwrap();
        store
            .replace_markets(&[snap(1, 0.0, t(10), vec![price("gold", None, 0, 0)])])
            .unwrap();
        let candidate = MarketIdentity {
            market_id: None,
            station_name: "PORT 1".into(),
            system_name: "SYS 1".into(),
        };
        let query = |exclude| {
            store
                .price_observations_for_markets(
                    "gold",
                    Side::Sell,
                    std::slice::from_ref(&candidate),
                    [0.0; 3],
                    100.0,
                    t(0),
                    exclude,
                )
                .unwrap()
        };
        assert_eq!(query(None).len(), 1);
        assert!(query(Some(1)).is_empty());
        let mut ambiguous = snap(2, 0.0, t(11), vec![price("gold", None, 0, 0)]);
        ambiguous.station_name = "Port 1".into();
        ambiguous.system_name = "Sys 1".into();
        store.replace_markets(&[ambiguous]).unwrap();
        assert!(query(None).is_empty());
        let too_many = vec![candidate; 51];
        assert!(
            store
                .price_observations_for_markets(
                    "gold",
                    Side::Sell,
                    &too_many,
                    [0.0; 3],
                    100.0,
                    t(0),
                    None
                )
                .is_err()
        );
    }

    fn near(store: &Store, side: Side, since: DateTime<Utc>) -> Vec<PricedMarket> {
        store
            .best_prices("gold", side, [0.0; 3], 100.0, since, None, 10)
            .unwrap()
    }

    #[test]
    fn compatible_price_query_filters_before_limit_and_keeps_unknown_pad() {
        let store = Store::open_in_memory().unwrap();
        let mut medium = snap(1, 0.0, t(10), vec![price("gold", None, 20000, 100)]);
        medium.max_pad = Some(PadSize::Medium);
        let mut unknown = snap(2, 0.0, t(10), vec![price("gold", None, 10000, 100)]);
        unknown.max_pad = None;
        store.replace_markets(&[medium, unknown]).unwrap();
        let best = store
            .best_prices_for_ship(
                "gold",
                Side::Sell,
                [0.0; 3],
                100.0,
                t(0),
                None,
                1,
                Some(PadSize::Large),
            )
            .unwrap();
        assert_eq!(best.len(), 1);
        assert_eq!(best[0].market_id, 2);
        assert_eq!(best[0].max_pad, None);
        assert_eq!(
            store
                .best_prices("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, 1)
                .unwrap()[0]
                .market_id,
            1
        );
        assert_eq!(
            store
                .best_prices_for_ship("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, 1, None)
                .unwrap()[0]
                .market_id,
            1
        );
    }
    #[test]
    fn compatible_buy_query_skips_zero_price_rows_before_limit() {
        let store = Store::open_in_memory().unwrap();
        let mut snaps = Vec::new();
        for id in 1..=3u64 {
            let mut p = price("gold", None, 0, 0);
            p.buy_price = 0;
            p.stock = 100;
            snaps.push(snap(id, 0.0, t(10), vec![p]));
        }
        let mut valid = price("gold", None, 0, 0);
        valid.buy_price = 9400;
        valid.stock = 50;
        snaps.push(snap(4, 0.0, t(10), vec![valid]));
        store.replace_markets(&snaps).unwrap();
        let ship = |side| {
            store
                .best_prices_for_ship("gold", side, [0.0; 3], 100.0, t(0), None, 1, None)
                .unwrap()
        };
        let buys = ship(Side::Buy);
        assert_eq!(buys.len(), 1);
        assert_eq!((buys[0].market_id, buys[0].price), (4, 9400));
        // El método legado conserva su comportamiento.
        assert_eq!(near(&store, Side::Buy, t(0)).len(), 4);
    }
    #[test]
    fn empty_complete_snapshot_rejects_older_complete_and_partial_prices() {
        let store = Store::open_in_memory().unwrap();
        store
            .replace_markets(&[snap(1, 0.0, t(12), vec![])])
            .unwrap();
        assert_eq!(
            store
                .replace_markets(&[snap(1, 0.0, t(11), vec![price("gold", None, 10000, 50)])])
                .unwrap(),
            0
        );
        let station = StationInfo {
            market_id: 1,
            station_name: "Port 1".into(),
            system_name: "Sys 1".into(),
            star_pos: Some([0.0; 3]),
            max_pad: None,
            station_type: None,
            dist_ls: None,
        };
        for at in [t(11), t(12)] {
            assert_eq!(
                store
                    .upsert_partial_prices(&[(
                        station.clone(),
                        price("gold", None, 10000, 50),
                        at,
                        PriceOrigin::Ardent
                    )])
                    .unwrap(),
                0
            );
            assert!(store.market_prices(1).unwrap().is_empty());
        }
        assert_eq!(
            store
                .upsert_partial_prices(&[(
                    station,
                    price("gold", None, 11000, 50),
                    t(13),
                    PriceOrigin::Ardent
                )])
                .unwrap(),
            1
        );
        assert_eq!(store.market_prices(1).unwrap()[0].sell_price, 11000);
    }

    #[test]
    fn missing_commodity_from_complete_snapshot_is_a_closed_observation() {
        let store = Store::open_in_memory().unwrap();
        store
            .replace_markets(&[snap(1, 0.0, t(10), vec![price("gold", None, 10000, 50)])])
            .unwrap();
        store
            .replace_markets(&[snap(1, 0.0, t(12), vec![price("silver", None, 100, 50)])])
            .unwrap();
        let identity = MarketIdentity {
            market_id: Some(1),
            station_name: "Port 1".into(),
            system_name: "Sys 1".into(),
        };
        let rows = store
            .price_observations_for_markets(
                "gold",
                Side::Sell,
                &[identity],
                [0.0; 3],
                100.0,
                t(0),
                None,
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (rows[0].price, rows[0].volume, rows[0].updated_at),
            (0, 0, t(12))
        );
    }

    #[test]
    fn station_and_partial_updates_do_not_imply_complete_snapshot() {
        let store = Store::open_in_memory().unwrap();
        let station = StationInfo {
            market_id: 1,
            station_name: "Port 1".into(),
            system_name: "Sys 1".into(),
            star_pos: Some([0.0; 3]),
            max_pad: None,
            station_type: None,
            dist_ls: None,
        };
        store
            .upsert_partial_prices(&[(
                station.clone(),
                price("silver", None, 100, 50),
                t(12),
                PriceOrigin::Ardent,
            )])
            .unwrap();
        store.upsert_stations(&[station], t(13)).unwrap();
        let identity = MarketIdentity {
            market_id: Some(1),
            station_name: "Port 1".into(),
            system_name: "Sys 1".into(),
        };
        assert!(
            store
                .price_observations_for_markets(
                    "gold",
                    Side::Sell,
                    &[identity],
                    [0.0; 3],
                    100.0,
                    t(0),
                    None
                )
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn full_snapshot_marker_expires_with_price_retention() {
        let store = Store::open_in_memory().unwrap();
        store
            .replace_markets(&[snap(1, 0.0, t(10), vec![])])
            .unwrap();
        store.purge_prices_older_than(t(11), t(0)).unwrap();
        assert_eq!(
            store
                .replace_markets(&[snap(1, 0.0, t(9), vec![])])
                .unwrap(),
            1
        );
        let rows = store
            .query_json("SELECT market_id FROM market_full_snapshot", &[])
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn replace_then_best_sell_within_radius() {
        let s = Store::open_in_memory().unwrap();
        s.replace_markets(&[
            snap(1, 10.0, t(12), vec![price("gold", None, 10500, 5)]),
            snap(2, 300.0, t(12), vec![price("gold", None, 11000, 5)]),
        ])
        .unwrap();
        let r = near(&s, Side::Sell, t(0));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].price, 10500);
        assert!((r[0].distance_ly.unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn older_snapshot_does_not_overwrite_newer() {
        let s = Store::open_in_memory().unwrap();
        s.replace_markets(&[snap(1, 0.0, t(12), vec![price("gold", None, 10500, 5)])])
            .unwrap();
        let n = s
            .replace_markets(&[snap(1, 0.0, t(11), vec![price("gold", None, 9000, 5)])])
            .unwrap();
        assert_eq!(n, 0);
        assert_eq!(s.market_prices(1).unwrap()[0].sell_price, 10500);
    }

    #[test]
    fn replace_removes_commodities_missing_from_new_snapshot() {
        let s = Store::open_in_memory().unwrap();
        s.replace_markets(&[snap(
            1,
            0.0,
            t(10),
            vec![price("gold", None, 1, 1), price("silver", None, 1, 1)],
        )])
        .unwrap();
        s.replace_markets(&[snap(1, 0.0, t(11), vec![price("gold", None, 1, 1)])])
            .unwrap();
        assert_eq!(s.market_prices(1).unwrap().len(), 1);
    }

    #[test]
    fn sell_seen_49h_ago_is_excluded() {
        let s = Store::open_in_memory().unwrap();
        let now = t(12);
        s.replace_markets(&[
            snap(
                1,
                1.0,
                now - Duration::hours(49),
                vec![price("gold", None, 12000, 5)],
            ),
            snap(
                2,
                2.0,
                now - Duration::hours(2),
                vec![price("gold", None, 11000, 5)],
            ),
        ])
        .unwrap();
        let r = near(&s, Side::Sell, now - Duration::hours(48));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].price, 11000);
    }

    #[test]
    fn known_commodities_keep_localised_name() {
        let s = Store::open_in_memory().unwrap();
        s.replace_markets(&[
            snap(1, 0.0, t(10), vec![price("gold", Some("Oro"), 1, 1)]),
            snap(2, 0.0, t(11), vec![price("gold", None, 1, 1)]),
        ])
        .unwrap();
        assert_eq!(
            s.known_commodities(10).unwrap(),
            vec![("gold".to_string(), Some("Oro".to_string()))]
        );
    }

    #[test]
    fn sell_requires_demand_and_buy_requires_stock() {
        let s = Store::open_in_memory().unwrap();
        let mut p = price("gold", None, 10000, 0);
        p.buy_price = 9000;
        s.replace_markets(&[snap(1, 0.0, t(10), vec![p])]).unwrap();
        assert!(near(&s, Side::Sell, t(0)).is_empty());
        assert!(near(&s, Side::Buy, t(0)).is_empty());
        let mut p = price("gold", None, 10000, 3);
        p.buy_price = 9000;
        p.stock = 4;
        s.replace_markets(&[snap(1, 0.0, t(11), vec![p])]).unwrap();
        assert_eq!(near(&s, Side::Sell, t(0))[0].volume, 3);
        assert_eq!(near(&s, Side::Buy, t(0))[0].volume, 4);
        assert_eq!(near(&s, Side::Buy, t(0))[0].price, 9000);
    }

    #[test]
    fn purge_deletes_rows_older_than_7_days() {
        let s = Store::open_in_memory().unwrap();
        let now = t(12);
        s.replace_markets(&[
            snap(
                1,
                0.0,
                now - Duration::days(8),
                vec![price("gold", None, 1, 1)],
            ),
            snap(
                2,
                0.0,
                now - Duration::days(1),
                vec![price("gold", None, 1, 1)],
            ),
        ])
        .unwrap();
        s.upsert_coords(
            &[
                ("old".into(), None, [0.0; 3]),
                ("new".into(), None, [1.0; 3]),
            ],
            now - Duration::days(31),
        )
        .unwrap();
        s.upsert_coords(&[("new".into(), None, [1.0; 3])], now)
            .unwrap();
        let c = s
            .purge_prices_older_than(
                now - Duration::days(PRICE_RETENTION_DAYS),
                now - Duration::days(COORDS_RETENTION_DAYS),
            )
            .unwrap();
        assert_eq!(
            c,
            PurgeCounts {
                prices: 1,
                stations: 1,
                coords: 1
            }
        );
        assert!(s.market_prices(1).unwrap().is_empty());
        assert_eq!(s.market_prices(2).unwrap().len(), 1);
    }

    #[test]
    fn coords_within_and_by_names() {
        let s = Store::open_in_memory().unwrap();
        s.upsert_coords(
            &[
                ("Sol".into(), Some(1), [0.0, 0.0, 0.0]),
                ("Alpha".into(), None, [30.0, 0.0, 0.0]),
                ("Far".into(), None, [500.0, 0.0, 0.0]),
            ],
            t(12),
        )
        .unwrap();
        assert_eq!(s.coords_within([0.0; 3], 50.0, 10).unwrap().len(), 2);
        let m = s.coords_by_names(&["sol".to_string()]).unwrap();
        assert_eq!(m["sol"], [0.0, 0.0, 0.0]);
        assert_eq!(s.coords_by_names(&["SOL".to_string()]).unwrap().len(), 1);
    }

    #[test]
    fn partial_prices_newest_wins_and_fill_station() {
        let s = Store::open_in_memory().unwrap();
        let st = StationInfo {
            market_id: 7,
            station_name: "Port".into(),
            system_name: "Sys".into(),
            star_pos: Some([5.0, 0.0, 0.0]),
            max_pad: None,
            station_type: None,
            dist_ls: Some(100.0),
        };
        let rows = |sell, at| {
            vec![(
                st.clone(),
                price("gold", None, sell, 9),
                at,
                PriceOrigin::Ardent,
            )]
        };
        s.upsert_partial_prices(&rows(100, t(12))).unwrap();
        s.upsert_partial_prices(&rows(50, t(11))).unwrap();
        assert_eq!(s.market_prices(7).unwrap()[0].sell_price, 100);
        let r = near(&s, Side::Sell, t(0));
        assert_eq!(r[0].origin, PriceOrigin::Ardent);
    }

    #[test]
    fn same_time_snapshot_is_not_applied() {
        let s = Store::open_in_memory().unwrap();
        let a = snap(1, 0.0, t(12), vec![price("gold", None, 1, 1)]);
        assert_eq!(s.replace_markets(std::slice::from_ref(&a)).unwrap(), 1);
        assert_eq!(s.replace_markets(&[a]).unwrap(), 0);
    }

    #[test]
    fn exclude_market_and_limit_after_distance_filter() {
        let s = Store::open_in_memory().unwrap();
        // El mercado 3 cae en la caja pero fuera de la esfera.
        let mut corner = snap(3, 0.0, t(12), vec![price("gold", None, 99999, 5)]);
        corner.star_pos = Some([90.0, 90.0, 90.0]);
        s.replace_markets(&[
            corner,
            snap(1, 10.0, t(12), vec![price("gold", None, 300, 5)]),
            snap(2, 20.0, t(12), vec![price("gold", None, 200, 5)]),
        ])
        .unwrap();
        let r = s
            .best_prices("gold", Side::Sell, [0.0; 3], 100.0, t(0), None, 2)
            .unwrap();
        assert_eq!(r.iter().map(|m| m.market_id).collect::<Vec<_>>(), [1, 2]);
        let r = s
            .best_prices("gold", Side::Sell, [0.0; 3], 100.0, t(0), Some(1), 5)
            .unwrap();
        assert_eq!(r.iter().map(|m| m.market_id).collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn buy_ascending_and_sell_descending() {
        let s = Store::open_in_memory().unwrap();
        let mk = |id, x, v| {
            let mut p = price("gold", None, v, 5);
            p.buy_price = v;
            p.stock = 5;
            snap(id, x, t(12), vec![p])
        };
        s.replace_markets(&[mk(1, 1.0, 200), mk(2, 2.0, 100), mk(3, 3.0, 300)])
            .unwrap();
        let ids = |side| {
            near(&s, side, t(0))
                .iter()
                .map(|m| m.market_id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Side::Buy), [2, 1, 3]);
        assert_eq!(ids(Side::Sell), [3, 1, 2]);
    }

    #[test]
    fn newer_partial_survives_replace_and_does_not_block_it() {
        let s = Store::open_in_memory().unwrap();
        s.replace_markets(&[snap(
            1,
            0.0,
            t(10),
            vec![price("gold", None, 1, 1), price("silver", None, 1, 1)],
        )])
        .unwrap();
        let st = StationInfo {
            market_id: 1,
            station_name: "Port 1".into(),
            system_name: "Sys 1".into(),
            star_pos: None,
            max_pad: None,
            station_type: None,
            dist_ls: None,
        };
        s.upsert_partial_prices(&[(st, price("gold", None, 777, 9), t(12), PriceOrigin::Ardent)])
            .unwrap();
        // Un snapshot completo intermedio se aplica: el parcial no cuenta para la frescura.
        let n = s
            .replace_markets(&[snap(1, 0.0, t(11), vec![price("copper", None, 5, 5)])])
            .unwrap();
        assert_eq!(n, 1);
        let m = s.market_prices(1).unwrap();
        let keys: Vec<_> = m.iter().map(|p| p.commodity.as_str()).collect();
        assert_eq!(keys, ["copper", "gold"]);
        assert_eq!(m[1].sell_price, 777);
    }

    #[test]
    fn older_station_info_does_not_rename() {
        let s = Store::open_in_memory().unwrap();
        let st = |name: &str| StationInfo {
            market_id: 1,
            station_name: name.into(),
            system_name: "S".into(),
            star_pos: None,
            max_pad: None,
            station_type: None,
            dist_ls: None,
        };
        s.upsert_stations(&[st("New")], t(12)).unwrap();
        s.upsert_stations(&[st("Old")], t(10)).unwrap();
        let name: String = s
            .lock()
            .unwrap()
            .query_row("SELECT station_name FROM market_station", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "New");
    }
}

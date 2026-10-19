//! Precios de mercado, estaciones y coordenadas de sistemas.

use crate::{Store, StoreError};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, Transaction, params};
use std::collections::HashMap;
use tmc_model::market::{
    CommodityPrice, MarketSnapshot, PriceOrigin, PricedMarket, Side, StationInfo,
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
           station_name=excluded.station_name, system_name=excluded.system_name,
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
                    "SELECT MAX(updated_at) FROM market_price WHERE market_id=?1",
                    [mid],
                    |r| r.get(0),
                )
                .optional()?
                .flatten();
            if let Some(stored) = stored
                && parse_ts(&stored)? >= s.updated_at
            {
                continue;
            }
            tx.execute("DELETE FROM market_price WHERE market_id=?1", [mid])?;
            for p in &s.prices {
                tx.execute(
                    "INSERT INTO market_price(market_id,commodity,name_localised,buy_price,sell_price,stock,demand,updated_at,origin)
                     VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
                     ON CONFLICT(market_id,commodity) DO UPDATE SET name_localised=excluded.name_localised,
                       buy_price=excluded.buy_price, sell_price=excluded.sell_price, stock=excluded.stock,
                       demand=excluded.demand, updated_at=excluded.updated_at, origin=excluded.origin",
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
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
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

    pub fn upsert_stations(&self, st: &[StationInfo]) -> Result<usize, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let now = Utc::now();
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
                now,
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
        let (price_col, vol_col, cond, order) = match side {
            Side::Sell => ("p.sell_price", "p.demand", "p.demand > 0", "DESC"),
            Side::Buy => ("p.buy_price", "p.stock", "p.stock > 0", "ASC"),
        };
        let sql = format!(
            "SELECT s.market_id,s.station_name,s.system_name,s.x,s.y,s.z,s.max_pad,{price_col},{vol_col},p.updated_at,p.origin
             FROM market_price p JOIN market_station s ON s.market_id=p.market_id
             WHERE p.commodity=?1 AND p.updated_at>=?2 AND {cond}
               AND s.x BETWEEN ?3 AND ?4 AND s.y BETWEEN ?5 AND ?6 AND s.z BETWEEN ?7 AND ?8
               AND (?9 IS NULL OR s.market_id<>?9)
             ORDER BY {price_col} {order}, p.updated_at DESC"
        );
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let exclude = exclude_market.map(id).transpose()?;
        let rows = stmt.query_map(
            params![
                key(commodity),
                ts(since),
                center[0] - radius_ly,
                center[0] + radius_ly,
                center[1] - radius_ly,
                center[1] + radius_ly,
                center[2] - radius_ly,
                center[2] + radius_ly,
                exclude
            ],
            |r| {
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
                ))
            },
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (mid, station_name, system_name, pos, pad, price, volume, at, origin) = row?;
            let d = dist(center, pos);
            if d > radius_ly {
                continue;
            }
            out.push(PricedMarket {
                market_id: mid as u64,
                station_name,
                system_name,
                distance_ly: Some(d),
                max_pad: pad_from(pad),
                price: price as u64,
                volume: volume as u64,
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
                buy_price: r.get::<_, i64>(2)? as u64,
                sell_price: r.get::<_, i64>(3)? as u64,
                stock: r.get::<_, i64>(4)? as u64,
                demand: r.get::<_, i64>(5)? as u64,
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

    /// Borra precios y estaciones anteriores a `cutoff` y coordenadas con 23 días más de
    /// antigüedad (30 días si `cutoff` es now − 7 d). Devuelve las filas de precio borradas.
    pub fn purge_prices_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize, StoreError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let prices = tx.execute("DELETE FROM market_price WHERE updated_at<?1", [ts(cutoff)])?;
        tx.execute(
            "DELETE FROM market_station WHERE updated_at<?1",
            [ts(cutoff)],
        )?;
        tx.execute(
            "DELETE FROM system_coords WHERE seen_at<?1",
            [ts(cutoff - chrono::Duration::days(23))],
        )?;
        tx.commit()?;
        Ok(prices)
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
    fn near(store: &Store, side: Side, since: DateTime<Utc>) -> Vec<PricedMarket> {
        store
            .best_prices("gold", side, [0.0; 3], 100.0, since, None, 10)
            .unwrap()
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
        assert_eq!(
            s.purge_prices_older_than(now - Duration::days(7)).unwrap(),
            1
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
}

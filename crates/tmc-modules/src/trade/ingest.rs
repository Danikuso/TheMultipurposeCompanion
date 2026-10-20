//! Ingesta de resultados de Ardent en el almacén y lectura de mercancías conocidas.

use chrono::{DateTime, Utc};
use tmc_core::StoreTask;
use tmc_model::{
    events::commodity_key,
    market::{CommodityPrice, PriceOrigin, StationInfo},
    net::{RemoteExport, RemoteMarket},
};

/// Fila lista para `Store::upsert_partial_prices`.
pub type ArdentRow = (StationInfo, CommodityPrice, DateTime<Utc>, PriceOrigin);

/// Convierte las respuestas de Ardent de una mercancía en filas del almacén.
/// `RemoteMarket` aporta `sell_price`/`demand`; `RemoteExport`, `buy_price`/`stock`.
/// Se omiten los mercados sin `market_id` (no se pueden identificar). La fecha es
/// `updated_at` o, si falta, el instante de la consulta. Las coordenadas se
/// rellenan después, dentro de la tarea de escritura.
pub fn ardent_rows(
    markets: &[RemoteMarket],
    exports: &[RemoteExport],
    commodity: &str,
    fetched_at: DateTime<Utc>,
) -> Vec<ArdentRow> {
    let key = commodity_key(commodity);
    let station =
        |market_id: u64, name: &str, system: &str, pad, kind: &Option<String>, ls| StationInfo {
            market_id,
            station_name: name.to_owned(),
            system_name: system.to_owned(),
            star_pos: None,
            max_pad: pad,
            station_type: kind.clone(),
            dist_ls: ls,
        };
    let sells = markets.iter().filter_map(|m| {
        Some((
            station(
                m.market_id.filter(|id| *id > 0)?,
                &m.station_name,
                &m.system_name,
                m.max_pad,
                &m.station_type,
                m.distance_to_arrival_ls,
            ),
            CommodityPrice {
                commodity: key.clone(),
                name_localised: None,
                buy_price: 0,
                sell_price: m.sell_price,
                stock: 0,
                demand: m.demand.unwrap_or(0),
            },
            m.updated_at.unwrap_or(fetched_at),
            PriceOrigin::Ardent,
        ))
    });
    let buys = exports.iter().filter_map(|e| {
        Some((
            station(
                e.market_id.filter(|id| *id > 0)?,
                &e.station_name,
                &e.system_name,
                e.max_pad,
                &e.station_type,
                e.distance_to_arrival_ls,
            ),
            CommodityPrice {
                commodity: key.clone(),
                name_localised: None,
                buy_price: e.buy_price,
                sell_price: 0,
                stock: e.stock.unwrap_or(0),
                demand: 0,
            },
            e.updated_at.unwrap_or(fetched_at),
            PriceOrigin::Ardent,
        ))
    });
    sells.chain(buys).collect()
}

/// Escribe las filas de Ardent cuyo sistema tiene coordenadas conocidas (las demás
/// solo viven en memoria). Etiqueta recomendada: `trade.ardent.write`.
pub fn ardent_write_task(mut rows: Vec<ArdentRow>) -> StoreTask {
    Box::new(move |store| {
        let names: Vec<String> = rows.iter().map(|r| r.0.system_name.clone()).collect();
        let coords = store.coords_by_names(&names)?;
        for row in &mut rows {
            row.0.star_pos = coords
                .get(&row.0.system_name.trim().to_lowercase())
                .copied();
        }
        let total = rows.len();
        rows.retain(|r| r.0.star_pos.is_some());
        let kept = rows.len();
        let changed = store.upsert_partial_prices(&rows)?;
        Ok(serde_json::json!({"rows":total,"written":kept,"changed":changed}))
    })
}

/// Lectura de hasta 400 mercancías conocidas con su nombre localizado
/// (etiqueta `trade.commodities`). Devuelve `{cmdr, items: [[clave, nombre]]}`.
pub fn commodities_task(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let items = store.known_commodities(400)?;
        Ok(serde_json::json!({"cmdr":cmdr,"items":items}))
    })
}

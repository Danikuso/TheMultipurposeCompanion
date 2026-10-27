//! Adaptador puro de Spansh (sin red). Formas y quirks: `docs/research/notes/spike_spansh_f1.md`.

use crate::jsonx;
use crate::transport::url_encode;
use chrono::{DateTime, Utc};
use serde_json::Value;
use tmc_model::market::is_fleet_carrier;
use tmc_model::net::{
    BrokerKind, GalaxyRouteParams, NearbyService, NetFailure, NeutronRouteParams, PadSize,
    PlannedHop, PlannedRoute, RemoteStation, RemoteSystem, SystemSuggestion, TradeCommodity,
    TradeRouteParams, TradeRoutePlan, TradeStop, TraderKind,
};

pub const BASE: &str = "https://spansh.co.uk/api";

/// Estado de un trabajo de ruta.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutePoll {
    Pending,
    Done(PlannedRoute),
    Failed(String),
}

/// Estado de un trabajo de ruta comercial.
#[derive(Debug, Clone, PartialEq)]
pub enum TradePoll {
    Pending,
    Done(TradeRoutePlan),
    Failed(String),
}

/// Ficha por id64 (la respuesta viene envuelta en `record`).
pub fn system_url(id64: u64) -> String {
    format!("{BASE}/system/{id64}")
}

/// Alta de ruta de neutrones (`POST`, formulario).
pub fn neutron_url() -> String {
    format!("{BASE}/route")
}

/// Alta de ruta de galaxia (`POST`, formulario).
pub fn galaxy_url() -> String {
    format!("{BASE}/generic/route")
}

/// Alta de ruta comercial (`POST`, formulario).
pub fn trade_url() -> String {
    format!("{BASE}/trade/route")
}

/// Sondeo de un trabajo de ruta.
pub fn results_url(job: &str) -> String {
    format!("{BASE}/results/{}", url_encode(job))
}

/// Consulta del sistema estelar conocido más cercano a las coordenadas galácticas (en AL).
pub fn nearest_url(x: f64, y: f64, z: f64) -> String {
    format!("{BASE}/nearest?x={x}&y={y}&z={z}")
}

/// Extrae el nombre del sistema más cercano desde la respuesta de `/api/nearest`.
pub fn parse_nearest(body: &str) -> Result<String, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let system = jsonx::req_obj(root, "system")?;
    jsonx::req_str(system, "name")
}

/// Clasifica el mensaje de error al enviar una petición de ruta a Spansh (HTTP 400).
pub fn parse_submit_error(body: &str) -> Option<NetFailure> {
    let v = jsonx::parse(body).ok()?;
    let root = jsonx::as_obj(&v).ok()?;
    let err = jsonx::opt_str(root, "error").ok()??;
    let err_lower = err.to_ascii_lowercase();
    if err_lower.contains("starting system") || err_lower.contains("source system") {
        Some(NetFailure::UnknownOrigin)
    } else if err_lower.contains("finishing system") || err_lower.contains("destination system") {
        Some(NetFailure::UnknownDestination)
    } else {
        None
    }
}

pub fn autocomplete_url(prefix: &str) -> String {
    format!(
        "{BASE}/systems/field_values/system_names?q={}",
        url_encode(prefix)
    )
}

/// `Ok(None)` = Spansh no conoce el sistema (`record` ausente/nulo o cuerpo con `error`).
pub fn parse_system(body: &str) -> Result<Option<RemoteSystem>, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let rec = match root.get("record") {
        Some(Value::Object(r)) => r,
        Some(Value::Null) => return Ok(None),
        Some(_) => return Err(NetFailure::Malformed),
        None if root.contains_key("error") => {
            // Solo «no encontrado» es NoData; cualquier otro error en un 2xx es anómalo.
            let msg = jsonx::opt_str(root, "error")?
                .unwrap_or_default()
                .to_lowercase();
            return if msg.contains("could not find") || msg.contains("not found") {
                Ok(None)
            } else {
                Err(NetFailure::Malformed)
            };
        }
        None => return Err(NetFailure::Malformed),
    };

    let body_count = jsonx::opt_u32(rec, "body_count")?;
    let mut primary_star = None;
    match rec.get("bodies") {
        None | Some(Value::Null) => {}
        Some(Value::Array(bodies)) => {
            for b in bodies {
                let b = jsonx::as_obj(b)?;
                if jsonx::opt_bool(b, "is_main_star")? == Some(true) && primary_star.is_none() {
                    primary_star = jsonx::opt_str(b, "subtype")?;
                }
            }
        }
        Some(_) => return Err(NetFailure::Malformed),
    }

    // Solo la potencia que controla el sistema (y su estado). `power` lista las
    // potencias que compiten y nunca se vuelca a `powers`.
    let controlling_power = jsonx::opt_str(rec, "controlling_power")?;
    let power_state = jsonx::opt_str(rec, "power_state")?;
    let (powers, powerplay_state) = match controlling_power {
        Some(p) => (vec![p], power_state),
        None => (Vec::new(), None),
    };

    Ok(Some(RemoteSystem {
        id64: jsonx::req_u64(rec, "id64")?,
        name: jsonx::req_str(rec, "name")?,
        // Que Spansh tenga cuerpos solo prueba que alguien los envió.
        discovered: body_count.filter(|n| *n > 0).map(|_| true),
        body_count,
        primary_star,
        allegiance: jsonx::opt_str(rec, "allegiance")?,
        government: jsonx::opt_str(rec, "government")?,
        controlling_faction: jsonx::opt_str(rec, "controlling_minor_faction")?,
        faction_state: jsonx::opt_str(rec, "controlling_minor_faction_state")?,
        economy: jsonx::opt_str(rec, "primary_economy")?,
        second_economy: jsonx::opt_str(rec, "secondary_economy")?,
        security: jsonx::opt_str(rec, "security")?,
        population: jsonx::opt_u64(rec, "population")?,
        powers,
        powerplay_state,
    }))
}

fn num(x: f64) -> String {
    format!("{x}")
}

fn flag(b: bool) -> String {
    if b { "1" } else { "0" }.to_string()
}

fn pair(k: &str, v: String) -> (String, String) {
    (k.to_string(), v)
}

pub fn neutron_form(p: &NeutronRouteParams) -> Vec<(String, String)> {
    vec![
        pair("from", p.from.clone()),
        pair("to", p.to.clone()),
        pair("range", num(p.range_ly)),
        pair("efficiency", p.efficiency.to_string()),
        pair("supercharge_multiplier", num(p.supercharge_multiplier)),
    ]
}

/// `from`/`to` pueden ser id64 en decimal o nombres directos de sistemas estelares
/// (Spansh `generic/route` / Exact Plotter admite ambos).
pub fn galaxy_form(p: &GalaxyRouteParams) -> Vec<(String, String)> {
    let s = &p.ship;
    vec![
        pair("source", p.from.clone()),
        pair("destination", p.to.clone()),
        pair("optimal_mass", num(s.optimal_mass)),
        pair("base_mass", num(s.unladen_mass)),
        pair("tank_size", num(s.fuel_main_capacity)),
        pair("internal_tank_size", num(s.fuel_reserve)),
        pair("max_fuel_per_jump", num(s.max_fuel_per_jump)),
        pair("fuel_power", num(s.fuel_power)),
        pair("fuel_multiplier", num(s.fuel_mul)),
        pair("cargo", num(s.cargo)),
        pair("range_boost", num(s.guardian_bonus_ly)),
        pair("supercharge_multiplier", num(s.supercharge_multiplier)),
        pair("use_injections", flag(p.use_injections)),
        pair("refuel_every_scoopable", flag(p.refuel_scoopable)),
        pair(
            "algorithm",
            p.algorithm.as_deref().unwrap_or("optimistic").to_string(),
        ),
    ]
}

/// Formulario de la ruta comercial.
///
/// Limitaciones conocidas: no se envía `max_system_distance` (`TradeRouteParams`
/// no lo tiene; Spansh usa su valor por defecto, 1000 en las pruebas) y Spansh
/// solo distingue plataforma grande: con `Medium`/`Small` se envía
/// `requires_large_pad=0` y la respuesta no trae el tamaño, así que no hay filtro fino.
/// Spansh solo distingue plataforma grande
/// (`requires_large_pad`); `max_price_age` va en segundos; los portaaviones
/// se excluyen siempre (`allow_player_owned=0`).
pub fn trade_form(p: &TradeRouteParams) -> Vec<(String, String)> {
    vec![
        pair("system", p.system.clone()),
        pair("station", p.station.clone()),
        pair("starting_capital", p.starting_capital.to_string()),
        pair("max_cargo", p.max_cargo.to_string()),
        pair("max_hops", p.max_hops.to_string()),
        pair("max_hop_distance", num(p.max_hop_distance_ly)),
        pair(
            "max_price_age",
            (u64::from(p.max_price_age_days) * 86_400).to_string(),
        ),
        pair("requires_large_pad", flag(p.min_pad == PadSize::Large)),
        pair("allow_planetary", flag(p.allow_planetary)),
        pair("allow_player_owned", flag(false)),
        pair("permit", flag(false)),
        pair("unique", flag(true)),
    ]
}

pub fn parse_job(body: &str) -> Result<String, NetFailure> {
    let v = jsonx::parse(body)?;
    jsonx::req_str(jsonx::as_obj(&v)?, "job")
}

/// Interpreta la respuesta de sondeo. Sin `result` y sin error = `Pending`.
pub fn parse_route_result(body: &str) -> Result<RoutePoll, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let error = jsonx::opt_str(root, "error")?;
    let status = jsonx::opt_str(root, "status")?;
    if status.as_deref() == Some("failed") || error.is_some() {
        return Ok(RoutePoll::Failed(
            error.unwrap_or_else(|| "failed".to_string()),
        ));
    }
    let result = match root.get("result") {
        None | Some(Value::Null) => return Ok(RoutePoll::Pending),
        Some(Value::Object(r)) => r,
        Some(_) => return Err(NetFailure::Malformed),
    };
    if result.contains_key("system_jumps") {
        parse_neutron(result).map(RoutePoll::Done)
    } else if result.contains_key("jumps") {
        parse_galaxy(result).map(RoutePoll::Done)
    } else {
        Err(NetFailure::Malformed)
    }
}

fn endpoint_is_carrier(e: &jsonx::Obj) -> bool {
    let name = e.get("station").and_then(Value::as_str);
    let id = e.get("market_id").and_then(Value::as_u64);
    // Sin `market_id` no se excluye por el nombre (se conservaría una estación real).
    id.is_some() && is_fleet_carrier(id.unwrap_or(0), None, name)
}

fn hop_is_carrier(h: &Value) -> bool {
    ["source", "destination"].iter().any(|k| {
        h.get(k)
            .and_then(Value::as_object)
            .is_some_and(endpoint_is_carrier)
    })
}

/// Un salto; `Ok(None)` si toca un portaaviones. Las mercancías corruptas se
/// saltan y un salto sin ninguna válida se descarta (`Err`).
fn parse_hop(h: &Value) -> Result<Option<TradeStop>, NetFailure> {
    let h = jsonx::as_obj(h)?;
    let (src, dst) = (
        jsonx::as_obj(h.get("source").ok_or(NetFailure::Malformed)?)?,
        jsonx::as_obj(h.get("destination").ok_or(NetFailure::Malformed)?)?,
    );
    if endpoint_is_carrier(src) || endpoint_is_carrier(dst) {
        return Ok(None);
    }
    let from_station = jsonx::req_str(src, "station")?;
    let to_station = jsonx::req_str(dst, "station")?;
    let raw = jsonx::req_array(h, "commodities")?;
    let commodities: Vec<TradeCommodity> =
        raw.iter().filter_map(|c| parse_commodity(c).ok()).collect();
    if !raw.is_empty() && commodities.is_empty() {
        return Err(NetFailure::Malformed);
    }
    let updated = |o: &jsonx::Obj| -> Option<DateTime<Utc>> {
        o.get("market_updated_at")
            .and_then(Value::as_i64)
            .and_then(|t| DateTime::from_timestamp(t, 0))
    };
    let data_updated_at = match (updated(src), updated(dst)) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    Ok(Some(TradeStop {
        from_system: jsonx::req_str(src, "system")?,
        from_station,
        to_system: jsonx::req_str(dst, "system")?,
        to_station,
        from_market_id: jsonx::opt_u64(src, "market_id")?,
        to_market_id: jsonx::opt_u64(dst, "market_id")?,
        commodities,
        stop_profit: h
            .get("total_profit")
            .and_then(Value::as_i64)
            .ok_or(NetFailure::Malformed)?,
        distance_ly: jsonx::req_f64(h, "distance")?,
        data_updated_at,
    }))
}

fn parse_commodity(c: &Value) -> Result<TradeCommodity, NetFailure> {
    let c = jsonx::as_obj(c)?;
    let section = |k: &str| -> Result<&jsonx::Obj, NetFailure> {
        jsonx::as_obj(c.get(k).ok_or(NetFailure::Malformed)?)
    };
    let buy_price = jsonx::req_u64(section("source_commodity")?, "buy_price")?;
    let sell_price = jsonx::req_u64(section("destination_commodity")?, "sell_price")?;
    Ok(TradeCommodity {
        commodity: jsonx::req_str(c, "name")?,
        name_localised: None,
        amount: jsonx::opt_u32(c, "amount")?.ok_or(NetFailure::Malformed)?,
        buy_price,
        sell_price,
        profit_per_ton: i64::try_from(sell_price).map_err(|_| NetFailure::Malformed)?
            - i64::try_from(buy_price).map_err(|_| NetFailure::Malformed)?,
    })
}

/// Interpreta el sondeo de una ruta comercial: `result` es un array de saltos.
/// Se descartan los saltos con origen o destino en un portaaviones (por el
/// nombre `XXX-NNN`; Spansh no da el tipo de estación).
pub fn parse_trade_result(body: &str) -> Result<TradePoll, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let error = jsonx::opt_str(root, "error")?;
    let status = jsonx::opt_str(root, "status")?;
    if status.as_deref() == Some("failed") || error.is_some() {
        return Ok(TradePoll::Failed(
            error.unwrap_or_else(|| "failed".to_string()),
        ));
    }
    let hops = match root.get("result") {
        None | Some(Value::Null) => return Ok(TradePoll::Pending),
        Some(Value::Array(a)) => a,
        Some(_) => return Err(NetFailure::Malformed),
    };
    let mut stops = Vec::new();
    for h in hops {
        if let Ok(Some(stop)) = parse_hop(h) {
            stops.push(stop);
        }
    }
    if !hops.is_empty() && stops.is_empty() && !hops.iter().any(hop_is_carrier) {
        return Err(NetFailure::Malformed);
    }
    let total_profit = stops.iter().map(|s| s.stop_profit).sum();
    Ok(TradePoll::Done(TradeRoutePlan {
        stops,
        total_profit,
    }))
}

fn position(o: &jsonx::Obj) -> Result<Option<[f64; 3]>, NetFailure> {
    Ok(Some([
        jsonx::req_f64(o, "x")?,
        jsonx::req_f64(o, "y")?,
        jsonx::req_f64(o, "z")?,
    ]))
}

/// Puntos de paso: `jumps` saltos y `distance_jumped` del tramo; el origen (`jumps == 0`) se descarta.
fn parse_neutron(result: &jsonx::Obj) -> Result<PlannedRoute, NetFailure> {
    let mut hops = Vec::new();
    for e in jsonx::req_array(result, "system_jumps")? {
        let e = jsonx::as_obj(e)?;
        let jumps = jsonx::req_u64(e, "jumps")?;
        let hop = PlannedHop {
            name: jsonx::req_str(e, "system")?,
            id64: Some(jsonx::req_u64(e, "id64")?),
            star_pos: position(e)?,
            distance_ly: jsonx::req_f64(e, "distance_jumped")?,
            jumps: u32::try_from(jumps).map_err(|_| NetFailure::Malformed)?,
            neutron: jsonx::opt_bool(e, "neutron_star")?.unwrap_or(false),
            refuel: None,
            star_class: None,
        };
        if jumps != 0 {
            hops.push(hop);
        }
    }
    Ok(PlannedRoute {
        hops,
        total_ly: jsonx::req_f64(result, "distance")?,
    })
}

/// Cada elemento es un salto; el primero es el origen y se descarta.
fn parse_galaxy(result: &jsonx::Obj) -> Result<PlannedRoute, NetFailure> {
    let jumps = jsonx::req_array(result, "jumps")?;
    let first = jumps.first().ok_or(NetFailure::Malformed)?;
    let total_ly = jsonx::req_f64(jsonx::as_obj(first)?, "distance_to_destination")?;
    let mut hops = Vec::new();
    for e in jumps {
        let e = jsonx::as_obj(e)?;
        hops.push(PlannedHop {
            name: jsonx::req_str(e, "name")?,
            id64: jsonx::opt_u64(e, "id64")?,
            star_pos: position(e)?,
            distance_ly: jsonx::req_f64(e, "distance")?,
            jumps: 1,
            neutron: jsonx::opt_bool(e, "has_neutron")?.unwrap_or(false),
            refuel: Some(jsonx::opt_bool(e, "must_refuel")?.unwrap_or(false)),
            star_class: None,
        });
    }
    hops.remove(0);
    Ok(PlannedRoute { hops, total_ly })
}

/// Sugerencias de `min_max` (nombre + id64). Si `min_max` no existe se usan los `values` sin id64.
pub fn parse_autocomplete(body: &str) -> Result<Vec<SystemSuggestion>, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let values = jsonx::req_array(root, "values")?;
    let mut out = Vec::new();
    match root.get("min_max") {
        Some(Value::Array(items)) if !items.is_empty() => {
            for it in items {
                let it = jsonx::as_obj(it)?;
                out.push(SystemSuggestion {
                    name: jsonx::req_str(it, "name")?,
                    id64: jsonx::opt_u64(it, "id64")?,
                });
            }
        }
        _ => {
            for x in values {
                let name = x.as_str().ok_or(NetFailure::Malformed)?;
                out.push(SystemSuggestion {
                    name: name.to_string(),
                    id64: None,
                });
            }
        }
    }
    Ok(out)
}

/// Máximo de estaciones que se piden y se devuelven.
const MAX_STATIONS: usize = 5;

/// Búsqueda de estaciones (`POST`, cuerpo JSON).
pub fn station_search_url() -> String {
    format!("{BASE}/stations/search")
}

/// Cuerpo de `POST /stations/search`: las estaciones más cercanas a
/// `reference_system` con el servicio y el tipo pedidos. Sin tipo concreto
/// (otros servicios) no se añade ningún filtro de servicio.
pub fn station_search_body(reference_system: &str, service: NearbyService) -> Value {
    let filters = match service {
        NearbyService::MaterialTrader(Some(k)) => {
            let kind = match k {
                TraderKind::Raw => "Raw",
                TraderKind::Manufactured => "Manufactured",
                TraderKind::Encoded => "Encoded",
            };
            serde_json::json!({"material_trader": {"value": [kind]}})
        }
        NearbyService::TechnologyBroker(Some(b)) => {
            let kind = match b {
                BrokerKind::Guardian => "Guardian",
                BrokerKind::Human => "Human",
            };
            serde_json::json!({"technology_broker": {"value": [kind]}})
        }
        _ => serde_json::json!({}),
    };
    serde_json::json!({
        "filters": filters,
        "sort": [{"distance": {"direction": "asc"}}],
        "size": MAX_STATIONS,
        "page": 0,
        "reference_system": reference_system,
    })
}

fn opt_count(o: &jsonx::Obj, k: &str) -> Result<u64, NetFailure> {
    Ok(jsonx::opt_u64(o, k)?.unwrap_or(0))
}

/// Estaciones de `/stations/search` (máx. 5, por distancia asc). Spansh solo da
/// el número de plataformas de cada tamaño: la máxima se deriva de ellas. Se
/// descartan carriers y obras, y lo que no admite `min_pad` (re-comprobado en local).
pub fn parse_station_search(
    body: &str,
    min_pad: PadSize,
) -> Result<Vec<RemoteStation>, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let items = jsonx::req_array(root, "results")?;
    let (total, mut bad) = (items.len(), 0);
    let mut out = Vec::new();
    for item in items {
        let parsed = (|| -> Result<Option<RemoteStation>, NetFailure> {
            let o = jsonx::as_obj(item)?;
            let name = jsonx::req_str(o, "name")?;
            let station_type = jsonx::opt_str(o, "type")?;
            let market_id = jsonx::opt_u64(o, "market_id")?;
            let max_pad = if opt_count(o, "large_pads")? > 0 {
                Some(PadSize::Large)
            } else if opt_count(o, "medium_pads")? > 0 {
                Some(PadSize::Medium)
            } else if opt_count(o, "small_pads")? > 0 {
                Some(PadSize::Small)
            } else {
                None
            };
            let updated = jsonx::opt_str(o, "market_updated_at")?
                .or(jsonx::opt_str(o, "updated_at")?)
                .and_then(|s| s.parse().ok());
            let st = RemoteStation {
                system_name: jsonx::req_str(o, "system_name")?,
                distance_ly: jsonx::req_f64(o, "distance")?,
                distance_to_arrival_ls: match o.get("distance_to_arrival") {
                    None | Some(Value::Null) => None,
                    Some(d) => Some(d.as_f64().ok_or(NetFailure::Malformed)?),
                },
                max_pad,
                updated_at: updated,
                station_name: name,
            };
            let skip =
                crate::ardent::excluded(&st.station_name, station_type.as_deref(), market_id);
            let keep = !skip && st.max_pad.is_some_and(|p| p.fits(min_pad));
            Ok(keep.then_some(st))
        })();
        match parsed {
            Ok(Some(st)) => out.push(st),
            Ok(None) => {}
            Err(_) => bad += 1,
        }
    }
    if total > 0 && bad == total {
        return Err(NetFailure::Malformed);
    }
    out.sort_by(|a, b| a.distance_ly.total_cmp(&b.distance_ly));
    out.truncate(MAX_STATIONS);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::net::ShipParams;

    const SYSTEM: &str = include_str!("../../../tests/fixtures/net/spansh_system_ok.json");
    const SYSTEM_404: &str = include_str!("../../../tests/fixtures/net/spansh_system_404.json");
    const JOB: &str = include_str!("../../../tests/fixtures/net/spansh_route_job.json");
    const NEUTRON: &str =
        include_str!("../../../tests/fixtures/net/spansh_route_neutron_result.json");
    const GALAXY: &str =
        include_str!("../../../tests/fixtures/net/spansh_route_galaxy_result.json");
    const AUTOCOMPLETE: &str = include_str!("../../../tests/fixtures/net/spansh_autocomplete.json");

    fn form_get<'a>(f: &'a [(String, String)], k: &str) -> Option<&'a str> {
        f.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
    }

    #[test]
    fn urls_match_the_spike() {
        assert_eq!(
            system_url(10477373803),
            "https://spansh.co.uk/api/system/10477373803"
        );
        assert_eq!(neutron_url(), "https://spansh.co.uk/api/route");
        assert_eq!(galaxy_url(), "https://spansh.co.uk/api/generic/route");
        assert_eq!(
            results_url("1F2B690E-CA6C-11F1-B0E5-AD2C2EDDEC53"),
            "https://spansh.co.uk/api/results/1F2B690E-CA6C-11F1-B0E5-AD2C2EDDEC53"
        );
        assert_eq!(
            autocomplete_url("Col 359"),
            "https://spansh.co.uk/api/systems/field_values/system_names?q=Col%20359"
        );
        assert_eq!(
            nearest_url(-926.96875, 145.71875, 6116.375),
            "https://spansh.co.uk/api/nearest?x=-926.96875&y=145.71875&z=6116.375"
        );
    }

    #[test]
    fn parses_nearest_system_and_errors() {
        let json = r#"{"system":{"distance":10.36,"id64":22648839483193,"name":"Byua Euq FU-K b36-10","x":-916.6,"y":146.2,"z":6116.8}}"#;
        assert_eq!(parse_nearest(json).unwrap(), "Byua Euq FU-K b36-10");

        assert_eq!(
            parse_submit_error(r#"{"error":"Could not find starting system"}"#),
            Some(NetFailure::UnknownOrigin)
        );
        assert_eq!(
            parse_submit_error(r#"{"error":"Could not find destination system"}"#),
            Some(NetFailure::UnknownDestination)
        );
        assert_eq!(
            parse_submit_error(r#"{"error":"ship details are invalid"}"#),
            None
        );
    }

    #[test]
    fn parses_recorded_spansh_system() {
        let s = parse_system(SYSTEM).unwrap().unwrap();
        assert_eq!(s.id64, 10477373803);
        assert_eq!(s.name, "Sol");
        assert_eq!(s.allegiance.as_deref(), Some("Federation"));
        assert_eq!(s.government.as_deref(), Some("Democracy"));
        assert_eq!(s.controlling_faction.as_deref(), Some("Mother Gaia"));
        assert_eq!(s.faction_state.as_deref(), Some("Expansion"));
        assert_eq!(s.economy.as_deref(), Some("Refinery"));
        assert_eq!(s.second_economy.as_deref(), Some("Service"));
        assert_eq!(s.security.as_deref(), Some("High"));
        assert_eq!(s.population, Some(18_320_926_115));
        assert_eq!(s.body_count, Some(40));
        assert_eq!(s.discovered, Some(true));
        assert_eq!(s.primary_star.as_deref(), Some("G (White-Yellow) Star"));
        // Solo la potencia que controla, nunca las 6 que compiten.
        assert_eq!(s.powers, vec!["Jerome Archer".to_string()]);
        assert_eq!(s.powerplay_state.as_deref(), Some("Stronghold"));
    }

    #[test]
    fn contested_powers_are_never_listed_without_controlling_power() {
        let body = r#"{"record":{"id64":5,"name":"X","power":["A","B"],"power_state":"Contested","body_count":0}}"#;
        let s = parse_system(body).unwrap().unwrap();
        assert!(s.powers.is_empty());
        assert_eq!(s.powerplay_state, None);
        assert_eq!(s.discovered, None);
        assert_eq!(s.primary_star, None);
        assert_eq!(s.body_count, Some(0));
    }

    #[test]
    fn error_field_in_2xx_is_no_data_only_when_not_found() {
        assert_eq!(
            parse_system(r#"{"error":"Could not find record with id64 9"}"#),
            Ok(None)
        );
        assert_eq!(
            parse_system(r#"{"error":"internal failure"}"#),
            Err(NetFailure::Malformed)
        );
    }

    #[test]
    fn recorded_404_body_is_no_data() {
        assert_eq!(parse_system(SYSTEM_404), Ok(None));
    }

    #[test]
    fn malformed_or_truncated_bodies_fail_cleanly() {
        for bad in [
            "",
            "{",
            "null",
            "[]",
            "42",
            r#"{"record":{"id64":"10477373803","name":"Sol"}}"#,
            r#"{"record":{"id64":1,"name":5}}"#,
            r#"{"record":{"id64":1,"name":"S","bodies":"x"}}"#,
            r#"{"record":{"id64":1,"name":"S","population":"many"}}"#,
            r#"{"record":{"id64":1,"name":"S","body_count":99999999999}}"#,
            r#"{"other":1}"#,
        ] {
            assert_eq!(parse_system(bad), Err(NetFailure::Malformed), "{bad}");
        }
        for bad in ["", "{", "null", r#"{"job":7}"#, "{}"] {
            assert_eq!(parse_job(bad), Err(NetFailure::Malformed), "{bad}");
        }
        for bad in ["", "{", "null", r#"{"values":"x"}"#, r#"{"values":[1]}"#] {
            assert_eq!(parse_autocomplete(bad), Err(NetFailure::Malformed), "{bad}");
        }
        for bad in [
            "",
            "{",
            "null",
            r#"{"result":{"x":1}}"#,
            r#"{"result":{"system_jumps":[{"system":1}]}}"#,
        ] {
            assert_eq!(parse_route_result(bad), Err(NetFailure::Malformed), "{bad}");
        }
    }

    #[test]
    fn parses_job_and_autocomplete() {
        assert_eq!(
            parse_job(JOB).unwrap(),
            "1F2B690E-CA6C-11F1-B0E5-AD2C2EDDEC53"
        );
        let got = parse_autocomplete(AUTOCOMPLETE).unwrap();
        let pairs: Vec<(&str, Option<u64>)> =
            got.iter().map(|s| (s.name.as_str(), s.id64)).collect();
        assert_eq!(
            pairs,
            [
                ("Sol", Some(10477373803)),
                ("Solitude", Some(5059379007779)),
                ("Sollaro", Some(11538024121505)),
                ("Solibamba", Some(5267550898539)),
                ("Solati", Some(1458376315610)),
            ]
        );
        assert_eq!(
            parse_autocomplete(r#"{"values":["A","B"]}"#).unwrap()[1],
            SystemSuggestion {
                name: "B".into(),
                id64: None
            }
        );
        assert_eq!(
            parse_autocomplete(r#"{"min_max":[],"values":[]}"#),
            Ok(vec![])
        );
    }

    #[test]
    fn parses_recorded_neutron_route() {
        let RoutePoll::Done(r) = parse_route_result(NEUTRON).unwrap() else {
            panic!("debía estar terminada");
        };
        // El origen (jumps 0) se descarta: quedan 4 puntos de paso.
        assert_eq!(r.hops.len(), 4);
        assert!((r.total_ly - 22000.4740453411).abs() < 1e-9);
        let h = &r.hops[0];
        assert_eq!(h.name, "PSR J1752-2806");
        assert_eq!(h.id64, Some(22712681061));
        assert_eq!(h.star_pos, Some([-10.96875, -6.84375, 407.28125]));
        assert!((h.distance_ly - 407.486400990496).abs() < 1e-9);
        assert_eq!(h.jumps, 10);
        assert!(h.neutron);
        assert_eq!(h.refuel, None);
        assert_eq!(h.star_class, None);
        let last = r.hops.last().unwrap();
        assert_eq!(last.name, "Colonia");
        assert_eq!(last.jumps, 1);
        assert!(!last.neutron);
        // 10 + 6 + 2 + 1 saltos agrupados en los tramos.
        assert_eq!(r.hops.iter().map(|h| h.jumps).sum::<u32>(), 19);
    }

    #[test]
    fn parses_recorded_galaxy_route() {
        let RoutePoll::Done(r) = parse_route_result(GALAXY).unwrap() else {
            panic!("debía estar terminada");
        };
        assert_eq!(r.hops.len(), 3);
        assert!((r.total_ly - 22000.4740453411).abs() < 1e-9);
        assert_eq!(r.hops[0].name, "Core Sys Sector QT-R b4-5");
        assert_eq!(r.hops[0].id64, Some(11665802339769));
        assert!((r.hops[0].distance_ly - 37.8882255554199).abs() < 1e-9);
        assert_eq!(r.hops[0].refuel, Some(true));
        assert_eq!(r.hops[0].jumps, 1);
        assert!(!r.hops[0].neutron);
        assert_eq!(r.hops[2].name, "Colonia");
        assert_eq!(r.hops[2].refuel, Some(false));
    }

    #[test]
    fn pending_and_failed_jobs() {
        let pending = r#"{"job":"J","status":"queued","state":"started"}"#;
        assert_eq!(parse_route_result(pending), Ok(RoutePoll::Pending));
        assert_eq!(
            parse_route_result(r#"{"status":"failed","error":"boom"}"#),
            Ok(RoutePoll::Failed("boom".into()))
        );
        assert_eq!(
            parse_route_result(r#"{"status":"failed"}"#),
            Ok(RoutePoll::Failed("failed".into()))
        );
        assert_eq!(
            parse_route_result(r#"{"error":"Unable to find route"}"#),
            Ok(RoutePoll::Failed("Unable to find route".into()))
        );
    }

    #[test]
    fn neutron_form_has_spike_fields() {
        let f = neutron_form(&NeutronRouteParams {
            from: "Sol".into(),
            to: "Colonia".into(),
            range_ly: 50.0,
            efficiency: 60,
            supercharge_multiplier: 4.0,
            from_coords: None,
        });
        assert_eq!(form_get(&f, "from"), Some("Sol"));
        assert_eq!(form_get(&f, "to"), Some("Colonia"));
        assert_eq!(form_get(&f, "range"), Some("50"));
        assert_eq!(form_get(&f, "efficiency"), Some("60"));
        assert_eq!(form_get(&f, "supercharge_multiplier"), Some("4"));
        assert_eq!(f.len(), 5);
    }

    #[test]
    fn galaxy_form_maps_ship_and_uses_ones_and_zeros() {
        let f = galaxy_form(&GalaxyRouteParams {
            from: "10477373803".into(),
            to: "3238296097059".into(),
            ship: ShipParams {
                unladen_mass: 300.0,
                fuel_main_capacity: 32.0,
                fuel_reserve: 2.0,
                cargo: 0.0,
                optimal_mass: 1052.0,
                max_fuel_per_jump: 5.2,
                fuel_mul: 0.012,
                fuel_power: 2.45,
                guardian_bonus_ly: 10.5,
                supercharge_multiplier: 4.0,
            },
            refuel_scoopable: true,
            use_injections: false,
            algorithm: None,
            from_coords: None,
        });
        assert_eq!(form_get(&f, "source"), Some("10477373803"));
        assert_eq!(form_get(&f, "destination"), Some("3238296097059"));
        assert_eq!(form_get(&f, "optimal_mass"), Some("1052"));
        assert_eq!(form_get(&f, "base_mass"), Some("300"));
        assert_eq!(form_get(&f, "tank_size"), Some("32"));
        assert_eq!(form_get(&f, "internal_tank_size"), Some("2"));
        assert_eq!(form_get(&f, "max_fuel_per_jump"), Some("5.2"));
        assert_eq!(form_get(&f, "fuel_power"), Some("2.45"));
        assert_eq!(form_get(&f, "fuel_multiplier"), Some("0.012"));
        assert_eq!(form_get(&f, "cargo"), Some("0"));
        assert_eq!(form_get(&f, "range_boost"), Some("10.5"));
        assert_eq!(form_get(&f, "use_injections"), Some("0"));
        assert_eq!(form_get(&f, "refuel_every_scoopable"), Some("1"));
        assert_eq!(form_get(&f, "supercharge_multiplier"), Some("4"));
        assert_eq!(form_get(&f, "algorithm"), Some("optimistic"));
    }

    #[test]
    fn galaxy_form_sends_system_names_and_optimistic_algorithm() {
        let f = galaxy_form(&GalaxyRouteParams {
            from: "Sol".into(),
            to: "Colonia".into(),
            ship: ShipParams {
                unladen_mass: 250.0,
                fuel_main_capacity: 32.0,
                fuel_reserve: 0.63,
                cargo: 0.0,
                optimal_mass: 1400.0,
                max_fuel_per_jump: 5.5,
                fuel_mul: 0.012,
                fuel_power: 2.75,
                guardian_bonus_ly: 10.5,
                supercharge_multiplier: 4.0,
            },
            refuel_scoopable: true,
            use_injections: true,
            algorithm: Some("fuel".into()),
            from_coords: Some([-926.0, 145.0, 6116.0]),
        });
        assert_eq!(form_get(&f, "source"), Some("Sol"));
        assert_eq!(form_get(&f, "destination"), Some("Colonia"));
        assert_eq!(form_get(&f, "algorithm"), Some("fuel"));
        assert_eq!(form_get(&f, "use_injections"), Some("1"));
        assert_eq!(form_get(&f, "refuel_every_scoopable"), Some("1"));
    }

    const TRADE: &str = include_str!("../../../tests/fixtures/net/spansh_trade_result.json");

    fn trade_params(pad: PadSize) -> TradeRouteParams {
        TradeRouteParams {
            system: "Sol".into(),
            station: "Abraham Lincoln".into(),
            starting_capital: 5_000_000,
            max_cargo: 256,
            max_hops: 3,
            max_hop_distance_ly: 30.5,
            min_pad: pad,
            max_price_age_days: 2,
            allow_planetary: false,
        }
    }

    #[test]
    fn trade_form_has_real_params() {
        let large = trade_form(&trade_params(PadSize::Large));
        let get = |f: &[(String, String)], k: &str| {
            f.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone())
        };
        assert_eq!(get(&large, "max_cargo").as_deref(), Some("256"));
        assert_eq!(get(&large, "starting_capital").as_deref(), Some("5000000"));
        assert_eq!(get(&large, "requires_large_pad").as_deref(), Some("1"));
        assert_eq!(get(&large, "max_price_age").as_deref(), Some("172800"));
        assert_eq!(get(&large, "max_hop_distance").as_deref(), Some("30.5"));
        assert_eq!(get(&large, "allow_player_owned").as_deref(), Some("0"));
        let medium = trade_form(&trade_params(PadSize::Medium));
        assert_eq!(get(&medium, "requires_large_pad").as_deref(), Some("0"));
        assert_eq!(trade_url(), "https://spansh.co.uk/api/trade/route");
    }

    #[test]
    fn parses_trade_result_fixture() {
        let TradePoll::Done(plan) = parse_trade_result(TRADE).unwrap() else {
            panic!("se esperaba Done");
        };
        assert_eq!(plan.stops.len(), 2);
        let c = &plan.stops[0].commodities[0];
        assert_eq!(c.commodity, "Biowaste");
        assert_eq!((c.buy_price, c.sell_price, c.profit_per_ton), (54, 89, 35));
        assert_eq!(plan.stops[0].from_station, "Fictional Port");
        assert_eq!(plan.stops[0].from_market_id, Some(900000001));
        assert_eq!(plan.stops[0].stop_profit, 15500);
        assert_eq!(plan.total_profit, 15500 + 1_459_500);
        assert_eq!(
            plan.stops[0].data_updated_at.map(|t| t.timestamp()),
            Some(1792240800)
        );
    }

    #[test]
    fn trade_result_pending_is_pending() {
        assert_eq!(
            parse_trade_result(r#"{"status":"queued"}"#),
            Ok(TradePoll::Pending)
        );
        assert!(matches!(
            parse_trade_result(r#"{"status":"failed","error":"x"}"#),
            Ok(TradePoll::Failed(_))
        ));
        assert_eq!(parse_trade_result("[]"), Err(NetFailure::Malformed));
    }

    #[test]
    fn trade_result_drops_carrier_stops() {
        let body = TRADE
            .replace("Fictional Dock", "ABC-123")
            .replace("900000002", "3700000002");
        let TradePoll::Done(plan) = parse_trade_result(&body).unwrap() else {
            panic!("se esperaba Done");
        };
        assert!(plan.stops.is_empty());
        assert_eq!(plan.total_profit, 0);
    }

    #[test]
    fn trade_result_carrier_rule_and_partial_rows() {
        // Sin market_id alto el nombre tipo callsign no excluye.
        let body = TRADE.replace("Fictional Dock", "ABC-123");
        let TradePoll::Done(plan) = parse_trade_result(&body).unwrap() else {
            panic!();
        };
        assert_eq!(plan.stops.len(), 2);
        // Salto corrupto: se salta, el otro se conserva.
        let mut v: serde_json::Value = serde_json::from_str(TRADE).unwrap();
        v["result"][0]["commodities"][0] = serde_json::json!({"junk": 1});
        let TradePoll::Done(plan) = parse_trade_result(&v.to_string()).unwrap() else {
            panic!();
        };
        assert_eq!(plan.stops[0].commodities.len(), 1);
        v["result"][0] = serde_json::json!({"junk": 1});
        let TradePoll::Done(plan) = parse_trade_result(&v.to_string()).unwrap() else {
            panic!();
        };
        assert_eq!(plan.stops.len(), 1);
        v["result"][1] = serde_json::json!(5);
        assert_eq!(
            parse_trade_result(&v.to_string()),
            Err(NetFailure::Malformed)
        );
        assert_eq!(
            parse_trade_result(r#"{"status":"ok","result":[]}"#),
            Ok(TradePoll::Done(TradeRoutePlan {
                stops: vec![],
                total_profit: 0
            }))
        );
    }

    const STATIONS: &str =
        include_str!("../../../tests/fixtures/net/spansh_stations_material_trader.json");

    #[test]
    fn typed_trader_request_is_spansh_station_search() {
        assert_eq!(
            station_search_url(),
            "https://spansh.co.uk/api/stations/search"
        );
        let b = station_search_body("Sol", NearbyService::MaterialTrader(Some(TraderKind::Raw)));
        assert_eq!(
            b["filters"]["material_trader"]["value"],
            serde_json::json!(["Raw"])
        );
        assert_eq!(b["size"], 5);
        assert_eq!(b["page"], 0);
        assert_eq!(b["sort"][0]["distance"]["direction"], "asc");
        assert_eq!(b["reference_system"], "Sol");
        assert!(b["filters"].get("technology_broker").is_none());
    }

    #[test]
    fn typed_broker_request_is_spansh_station_search() {
        let b = station_search_body(
            "Sol",
            NearbyService::TechnologyBroker(Some(BrokerKind::Guardian)),
        );
        assert_eq!(
            b["filters"]["technology_broker"]["value"],
            serde_json::json!(["Guardian"])
        );
        assert!(b["filters"].get("material_trader").is_none());
    }

    #[test]
    fn parses_spansh_station_search_fixture() {
        let s = parse_station_search(STATIONS, PadSize::Small).unwrap();
        assert!(s.len() <= 5);
        assert_eq!(s[0].station_name, "Mercader Ficticio");
        assert_eq!(s[0].system_name, "Testia Alfa");
        assert_eq!(s[0].max_pad, Some(PadSize::Large));
        assert_eq!(s[1].max_pad, Some(PadSize::Medium));
        assert!(s.windows(2).all(|w| w[0].distance_ly <= w[1].distance_ly));
        assert!(s[0].updated_at.is_some());
        // El re-filtro local de plataforma descarta el puesto sin plataformas grandes.
        let l = parse_station_search(STATIONS, PadSize::Large).unwrap();
        assert!(l.iter().all(|x| x.max_pad == Some(PadSize::Large)));
        assert!(l.len() < s.len());
    }

    #[test]
    fn station_search_drops_carriers_and_rejects_garbage() {
        let body = r#"{"results":[
            {"name":"K7Q-B2X","market_id":3700000123,"type":"Drake-Class Carrier","system_name":"S","distance":1.0,"large_pads":1},
            {"name":"Real","market_id":123,"type":"Outpost","system_name":"S","distance":2.0,"medium_pads":1,"small_pads":2}]}"#;
        let s = parse_station_search(body, PadSize::Small).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].max_pad, Some(PadSize::Medium));
        assert_eq!(
            parse_station_search("not json", PadSize::Small),
            Err(NetFailure::Malformed)
        );
        assert_eq!(
            parse_station_search(r#"{"foo":1}"#, PadSize::Small),
            Err(NetFailure::Malformed)
        );
    }
}

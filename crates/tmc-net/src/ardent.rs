//! Adaptador puro de Ardent (sin red). Rutas, filtros y quirks:
//! `docs/research/notes/spike_ardent_f2.md`.
//!
//! Decisiones:
//! - `nearby/imports` ignora `minLandingPadSize`: el filtro de plataforma de
//!   imports es local. `nearest/*` lo filtra en servidor y se re-comprueba.
//! - `maxDaysAgo` puede no respetarse en rutas nearby: se re-filtra por
//!   `updatedAt` con un «ahora» inyectado.
//! - Se excluyen obras de construcción/colonización, carriers y asentamientos
//!   a pie (no admiten naves normales).
//! - `Type` del diario -> nombre de Ardent: minúsculas sin espacios y una
//!   tabla de excepciones verificada contra la API (ver `ardent_commodity`).

use crate::jsonx;
use crate::transport::url_encode;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use tmc_model::net::{NearbyService, NetFailure, PadSize, RemoteMarket, RemoteStation};

pub const BASE: &str = "https://api.ardent-insight.com/v2";
/// Volumen mínimo de demanda que se pide al servidor.
pub const MIN_VOLUME: u32 = 100;
/// Antigüedad máxima (días) de los datos de mercado.
pub const MAX_DAYS_AGO: i64 = 3;
pub const MAX_MARKETS: usize = 50;
pub const MAX_STATIONS: usize = 5;

/// Nombre de materia de Ardent a partir del `Type` del diario: minúsculas,
/// sin espacios y con una tabla de excepciones. Verificado contra la API
/// (2026-10-18): `opal` devuelve mercados; `void opal`/`voidopal` devuelven `[]`
/// (el `Type` del diario para las Void Opals ya es `opal`). `lowtemperaturediamond`
/// funciona; la forma en plural devuelve `[]`.
pub fn ardent_commodity(journal_type: &str) -> String {
    let key: String = journal_type
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    match key.as_str() {
        "voidopal" | "voidopals" | "opals" => "opal".to_string(),
        "lowtemperaturediamonds" => "lowtemperaturediamond".to_string(),
        _ => key,
    }
}

fn pad_param(p: PadSize) -> u8 {
    match p {
        PadSize::Small => 1,
        PadSize::Medium => 2,
        PadSize::Large => 3,
    }
}

/// Compradores cercanos de una materia. `min_pad` no se envía: Ardent lo
/// ignora en imports y el filtro es local (`parse_markets`).
pub fn imports_nearby_url(
    system_address: u64,
    commodity: &str,
    max_distance_ly: u32,
    _min_pad: PadSize,
) -> String {
    format!(
        "{BASE}/system/address/{system_address}/commodity/name/{}/nearby/imports?maxDistance={max_distance_ly}&fleetCarriers=false&minVolume={MIN_VOLUME}&maxDaysAgo={MAX_DAYS_AGO}",
        url_encode(commodity)
    )
}

/// Estaciones más cercanas con un servicio; el servidor filtra por plataforma.
pub fn nearest_url(system_address: u64, service: NearbyService, min_pad: PadSize) -> String {
    let svc = match service {
        NearbyService::InterstellarFactors => "interstellar-factors",
    };
    format!(
        "{BASE}/system/address/{system_address}/nearest/{svc}?minLandingPadSize={}",
        pad_param(min_pad)
    )
}

fn is_carrier_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 7
        && b[3] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 3 || c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Obras de construcción/colonización, carriers y asentamientos a pie.
fn excluded(name: &str, station_type: Option<&str>) -> bool {
    let lower = name.to_lowercase();
    lower.contains("construction site")
        || lower.contains("colonisationship")
        || lower.contains("colonisation ship")
        || matches!(station_type, Some("FleetCarrier" | "OnFootSettlement"))
        || is_carrier_name(name)
}

fn opt_f64(o: &jsonx::Obj, k: &str) -> Result<Option<f64>, NetFailure> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(Some).ok_or(NetFailure::Malformed),
    }
}

fn opt_pad(o: &jsonx::Obj) -> Result<Option<PadSize>, NetFailure> {
    Ok(jsonx::opt_u64(o, "maxLandingPadSize")?
        .and_then(|n| u8::try_from(n).ok())
        .and_then(PadSize::from_ardent))
}

fn opt_time(o: &jsonx::Obj) -> Result<Option<DateTime<Utc>>, NetFailure> {
    Ok(jsonx::opt_str(o, "updatedAt")?.and_then(|s| s.parse().ok()))
}

fn rows(body: &str) -> Result<Vec<jsonx::Obj>, NetFailure> {
    let v = jsonx::parse(body)?;
    let Value::Array(items) = v else {
        return Err(NetFailure::Malformed);
    };
    items
        .iter()
        .map(|i| jsonx::as_obj(i).cloned())
        .collect::<Result<_, _>>()
}

/// Compradores ordenados por precio desc (máx. 50). Descarta lo excluido, lo
/// anterior a `now - 3 d` (o sin fecha) y lo que no admite `min_pad`.
/// `demandBracket` (número o texto) no se usa: el filtro es `demand`/`minVolume`.
pub fn parse_markets(
    body: &str,
    now: DateTime<Utc>,
    min_pad: PadSize,
) -> Result<Vec<RemoteMarket>, NetFailure> {
    let cutoff = now - Duration::days(MAX_DAYS_AGO);
    let mut out = Vec::new();
    for o in rows(body)? {
        let station_type = jsonx::opt_str(&o, "stationType")?;
        let name = jsonx::req_str(&o, "stationName")?;
        let market = RemoteMarket {
            system_name: jsonx::req_str(&o, "systemName")?,
            market_id: jsonx::opt_u64(&o, "marketId")?,
            distance_ly: jsonx::req_f64(&o, "distance")?,
            distance_to_arrival_ls: opt_f64(&o, "distanceToArrival")?,
            sell_price: jsonx::req_u64(&o, "sellPrice")?,
            demand: jsonx::opt_u64(&o, "demand")?,
            max_pad: opt_pad(&o)?,
            updated_at: opt_time(&o)?,
            station_name: name,
            station_type,
        };
        let fresh = market.updated_at.is_some_and(|t| t >= cutoff);
        let fits = market.max_pad.is_some_and(|p| p.fits(min_pad));
        if !excluded(&market.station_name, market.station_type.as_deref()) && fresh && fits {
            out.push(market);
        }
    }
    out.sort_by(|a, b| {
        b.sell_price
            .cmp(&a.sell_price)
            .then(a.distance_ly.total_cmp(&b.distance_ly))
    });
    out.truncate(MAX_MARKETS);
    Ok(out)
}

/// Estaciones más cercanas (máx. 5) que admiten `min_pad` (re-comprobado en local).
pub fn parse_stations(body: &str, min_pad: PadSize) -> Result<Vec<RemoteStation>, NetFailure> {
    let mut out = Vec::new();
    for o in rows(body)? {
        let station_type = jsonx::opt_str(&o, "stationType")?;
        let st = RemoteStation {
            station_name: jsonx::req_str(&o, "stationName")?,
            system_name: jsonx::req_str(&o, "systemName")?,
            distance_ly: jsonx::req_f64(&o, "distance")?,
            distance_to_arrival_ls: opt_f64(&o, "distanceToArrival")?,
            max_pad: opt_pad(&o)?,
            updated_at: opt_time(&o)?,
        };
        if !excluded(&st.station_name, station_type.as_deref())
            && st.max_pad.is_some_and(|p| p.fits(min_pad))
        {
            out.push(st);
        }
    }
    out.sort_by(|a, b| a.distance_ly.total_cmp(&b.distance_ly));
    out.truncate(MAX_STATIONS);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLATINUM: &str =
        include_str!("../../../tests/fixtures/net/ardent_nearby_imports_platinum.json");
    const EMPTY: &str =
        include_str!("../../../tests/fixtures/net/ardent_nearby_imports_empty.json");
    const NEAREST: &str = include_str!("../../../tests/fixtures/net/ardent_nearest_if.json");
    const NOT_FOUND: &str = include_str!("../../../tests/fixtures/net/ardent_system_404.json");

    fn now() -> DateTime<Utc> {
        "2026-10-18T12:00:00Z".parse().unwrap()
    }

    fn row(name: &str, stype: &str, price: u64, bracket: &str, updated: &str) -> String {
        format!(
            r#"{{"marketId":1,"stationName":"{name}","stationType":{stype},"distanceToArrival":10.0,"maxLandingPadSize":2,"systemName":"S","sellPrice":{price},"demand":200,"demandBracket":{bracket},"updatedAt":{updated},"distance":7}}"#
        )
    }

    fn ts(s: &str) -> String {
        format!("\"{s}\"")
    }

    #[test]
    fn imports_url_has_filters() {
        let u = imports_nearby_url(10477373803, "platinum", 100, PadSize::Large);
        assert_eq!(
            u,
            "https://api.ardent-insight.com/v2/system/address/10477373803/commodity/name/platinum/nearby/imports?maxDistance=100&fleetCarriers=false&minVolume=100&maxDaysAgo=3"
        );
        // Ardent ignora minLandingPadSize en imports: no se envía.
        assert!(!u.contains("minLandingPadSize"));
        assert!(imports_nearby_url(1, "low temp", 5, PadSize::Small).contains("/name/low%20temp/"));
    }

    #[test]
    fn nearest_url_sends_pad_to_server() {
        assert_eq!(
            nearest_url(
                10477373803,
                NearbyService::InterstellarFactors,
                PadSize::Medium
            ),
            "https://api.ardent-insight.com/v2/system/address/10477373803/nearest/interstellar-factors?minLandingPadSize=2"
        );
        assert!(
            nearest_url(1, NearbyService::InterstellarFactors, PadSize::Large)
                .ends_with("minLandingPadSize=3")
        );
    }

    #[test]
    fn journal_type_maps_to_ardent_name() {
        assert_eq!(ardent_commodity("Platinum"), "platinum");
        assert_eq!(
            ardent_commodity("LowTemperatureDiamond"),
            "lowtemperaturediamond"
        );
        assert_eq!(
            ardent_commodity("Low Temperature Diamonds"),
            "lowtemperaturediamond"
        );
        assert_eq!(ardent_commodity("opal"), "opal");
        assert_eq!(ardent_commodity("Void Opal"), "opal");
        assert_eq!(ardent_commodity(" Painite "), "painite");
    }

    #[test]
    fn parses_markets_fixture_sorted_by_price() {
        let m = parse_markets(PLATINUM, now(), PadSize::Small).unwrap();
        // Quedan el hub y el outpost: obra, carrier y asentamiento a pie fuera.
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].sell_price, 299000);
        assert_eq!(m[0].station_name, "Ficticia Hub");
        assert_eq!(m[0].system_name, "Testia Alfa");
        assert_eq!(m[0].max_pad, Some(PadSize::Large));
        assert_eq!(m[0].market_id, Some(9100000001));
        assert_eq!(m[0].distance_ly, 12.0);
        assert_eq!(m[0].distance_to_arrival_ls, Some(120.5));
        assert_eq!(m[0].demand, Some(500));
        assert_eq!(m[0].station_type.as_deref(), Some("Coriolis"));
        assert_eq!(
            m[0].updated_at,
            Some("2026-10-18T10:00:00Z".parse().unwrap())
        );
        assert_eq!(m[1].sell_price, 288000);
        assert_eq!(m[1].max_pad, Some(PadSize::Medium));
    }

    #[test]
    fn pad_filter_is_local() {
        let m = parse_markets(PLATINUM, now(), PadSize::Large).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].station_name, "Ficticia Hub");
    }

    #[test]
    fn construction_sites_and_carriers_are_excluded() {
        let names: Vec<String> = parse_markets(PLATINUM, now(), PadSize::Small)
            .unwrap()
            .into_iter()
            .map(|m| m.station_name)
            .collect();
        assert_eq!(names, ["Ficticia Hub", "Prueba Outpost"]);
        let t = ts("2026-10-18T11:00:00Z");
        let body = format!(
            "[{},{},{},{},{}]",
            row("Ship ColonisationShip Alpha", "null", 500, "1", &t),
            row("AB1-9Z9", "null", 400, "1", &t),
            row("Real Port", "\"Coriolis\"", 300, "1", &t),
            row("Dock", "\"FleetCarrier\"", 600, "1", &t),
            row("Nombre-Largo", "\"Outpost\"", 100, "1", &t),
        );
        let m = parse_markets(&body, now(), PadSize::Small).unwrap();
        let names: Vec<&str> = m.iter().map(|m| m.station_name.as_str()).collect();
        assert_eq!(names, ["Real Port", "Nombre-Largo"]);
    }

    #[test]
    fn stale_rows_are_filtered_locally() {
        let body = format!(
            "[{},{},{}]",
            row(
                "Fresca",
                "\"Coriolis\"",
                100,
                "1",
                &ts("2026-10-15T12:00:01Z")
            ),
            row(
                "Caducada",
                "\"Coriolis\"",
                900,
                "1",
                &ts("2026-10-15T11:59:59Z")
            ),
            row("Sin fecha", "\"Coriolis\"", 50, "1", "null"),
        );
        let m = parse_markets(&body, now(), PadSize::Small).unwrap();
        let names: Vec<&str> = m.iter().map(|m| m.station_name.as_str()).collect();
        // Límite de 3 días: 2026-10-15T12:00:00; la fila sin fecha se descarta.
        assert_eq!(names, ["Fresca"]);
    }

    #[test]
    fn demand_bracket_accepts_string_and_number() {
        let t = ts("2026-10-18T11:00:00Z");
        let body = format!(
            "[{},{},{}]",
            row("Num", "\"Coriolis\"", 3, "3", &t),
            row("Txt", "\"Coriolis\"", 2, "\"High\"", &t),
            row("Nulo", "\"Coriolis\"", 1, "null", &t),
        );
        let m = parse_markets(&body, now(), PadSize::Small).unwrap();
        assert_eq!(
            m.iter().map(|m| m.sell_price).collect::<Vec<_>>(),
            [3, 2, 1]
        );
    }

    #[test]
    fn caps_at_fifty_and_sorts_desc() {
        let t = ts("2026-10-18T11:00:00Z");
        let rows: Vec<String> = (0..60)
            .map(|i| row(&format!("E{i}"), "\"Coriolis\"", 1000 + i, "1", &t))
            .collect();
        let m = parse_markets(&format!("[{}]", rows.join(",")), now(), PadSize::Small).unwrap();
        assert_eq!(m.len(), 50);
        assert_eq!(m[0].sell_price, 1059);
        assert_eq!(m[49].sell_price, 1010);
    }

    #[test]
    fn empty_imports_is_empty_markets() {
        assert_eq!(parse_markets(EMPTY, now(), PadSize::Small), Ok(vec![]));
        assert_eq!(parse_stations(EMPTY, PadSize::Small), Ok(vec![]));
    }

    #[test]
    fn malformed_bodies_fail_cleanly() {
        for bad in [
            "",
            "{",
            "null",
            "42",
            NOT_FOUND,
            r#"[1]"#,
            r#"[{"stationName":"X"}]"#,
            r#"[{"stationName":"X","systemName":"S","sellPrice":"x","distance":1}]"#,
            r#"[{"stationName":5,"systemName":"S","sellPrice":1,"distance":1}]"#,
        ] {
            assert_eq!(
                parse_markets(bad, now(), PadSize::Small),
                Err(NetFailure::Malformed),
                "{bad}"
            );
        }
        for bad in ["", "{", "null", NOT_FOUND, r#"[{"stationName":"X"}]"#] {
            assert_eq!(
                parse_stations(bad, PadSize::Small),
                Err(NetFailure::Malformed),
                "{bad}"
            );
        }
    }

    #[test]
    fn parses_nearest_interstellar_factors() {
        let s = parse_stations(NEAREST, PadSize::Small).unwrap();
        // Por distancia: 5, 9 y 14 ly.
        let got: Vec<(&str, f64)> = s
            .iter()
            .map(|s| (s.station_name.as_str(), s.distance_ly))
            .collect();
        assert_eq!(
            got,
            [
                ("Factores Ficticios", 5.0),
                ("Puesto Solo M", 9.0),
                ("Asentamiento Sin Tipo", 14.0)
            ]
        );
        assert_eq!(s[0].system_name, "Testia Alfa");
        assert_eq!(s[0].distance_to_arrival_ls, Some(50.0));
        assert_eq!(s[0].max_pad, Some(PadSize::Large));
        assert_eq!(
            s[0].updated_at,
            Some("2026-10-18T12:00:00Z".parse().unwrap())
        );
    }

    #[test]
    fn nearest_rechecks_pad_locally_and_caps_at_five() {
        let m = parse_stations(NEAREST, PadSize::Medium).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[1].station_name, "Puesto Solo M");
        let l = parse_stations(NEAREST, PadSize::Large).unwrap();
        assert_eq!(l.len(), 1);
        let one = r#"{"stationName":"S","stationType":"Coriolis","systemName":"Y","maxLandingPadSize":3,"distance":"#;
        let rows: Vec<String> = (0..8).map(|i| format!("{one}{}}}", 20 - i)).collect();
        let s = parse_stations(&format!("[{}]", rows.join(",")), PadSize::Small).unwrap();
        assert_eq!(s.len(), 5);
        assert_eq!(s[0].distance_ly, 13.0);
        assert_eq!(s[4].distance_ly, 17.0);
    }
}

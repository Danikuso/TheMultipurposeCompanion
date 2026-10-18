//! Adaptador puro de Spansh (sin red). Formas y quirks: `docs/research/notes/spike_spansh_f1.md`.

use crate::jsonx;
use crate::transport::url_encode;
use serde_json::Value;
use tmc_model::net::{
    GalaxyRouteParams, NetFailure, NeutronRouteParams, PlannedHop, PlannedRoute, RemoteSystem,
    SystemSuggestion,
};

pub const BASE: &str = "https://spansh.co.uk/api";

/// Estado de un trabajo de ruta.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutePoll {
    Pending,
    Done(PlannedRoute),
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

/// Sondeo de un trabajo de ruta.
pub fn results_url(job: &str) -> String {
    format!("{BASE}/results/{}", url_encode(job))
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

/// `from`/`to` deben ser id64 en decimal (Spansh pide `source`/`destination` por id64).
///
/// Suposición NO verificada (spike §4): `internal_tank_size` se rellena con
/// `ShipParams::fuel_reserve`. Spansh ignora `reserve_size` y la semántica de
/// `internal_tank_size` está sin confirmar con una ruta real; no se inventan valores.
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
            id64: Some(jsonx::req_u64(e, "id64")?),
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
    }
}

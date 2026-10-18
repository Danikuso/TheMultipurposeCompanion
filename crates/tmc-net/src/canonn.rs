//! Adaptador puro de Canonn (`getSystemPoi`). En F1 la capa 3 está DESACTIVADA
//! (decisión del propietario: falta el permiso formal de Canonn); estas
//! funciones existen y están probadas, pero `NetClient` no las invoca.

use crate::jsonx;
use crate::transport::url_encode;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use tmc_model::net::{NetFailure, RemoteBodyBio, RemoteSpeciesHint};

pub const BASE: &str = "https://us-central1-canonn-api-236217.cloudfunctions.net/query";

/// Consulta por sistema (el servicio no acepta id64 ni filtra por cuerpo).
/// No se envía `cmdr`: privacidad.
pub fn body_url(system_name: &str) -> String {
    format!(
        "{BASE}/getSystemPoi?system={}&odyssey=Y",
        url_encode(system_name)
    )
}

/// Nombre del cuerpo sin el prefijo `<sistema> ` (sin distinguir mayúsculas).
/// La estrella principal (`body == sistema`) normaliza a `""`.
pub fn normalize_body_name(star_system: &str, body_name: &str) -> String {
    let sys = star_system.trim();
    let body = body_name.trim();
    if body.eq_ignore_ascii_case(sys) {
        return String::new();
    }
    if body.len() > sys.len()
        && body.is_char_boundary(sys.len())
        && body[..sys.len()].eq_ignore_ascii_case(sys)
        && body[sys.len()..].starts_with(' ')
    {
        return body[sys.len() + 1..].trim().to_string();
    }
    body.to_string()
}

/// Filtra las observaciones de Biología del cuerpo (igualdad exacta del nombre
/// normalizado, sin distinguir mayúsculas; `body: null` = estrella principal),
/// deduplica por `(entryid, latitude, longitude)` y calcula pesos.
///
/// `resolve` traduce `entryid` a `(género, especie)` desde `codex/ref` cacheado
/// en tiempo de ejecución (nunca una tabla en el repo: datos GPL-3.0). Las
/// entradas sin resolver se omiten tanto de los hints como del denominador, de
/// modo que los pesos suman 1,0. Sin filas -> `hints` vacío (el llamador lo
/// trata como `NoData`).
pub fn parse_body(
    body: &str,
    star_system: &str,
    body_name: &str,
    resolve: &dyn Fn(u64) -> Option<(String, String)>,
) -> Result<RemoteBodyBio, NetFailure> {
    let v = jsonx::parse(body)?;
    let root = jsonx::as_obj(&v)?;
    let rows: &[Value] = match root.get("codex") {
        None | Some(Value::Null) => &[],
        Some(Value::Array(a)) => a,
        Some(_) => return Err(NetFailure::Malformed),
    };
    let target = normalize_body_name(star_system, body_name).to_lowercase();

    let mut seen: HashSet<(u64, String, String)> = HashSet::new();
    let mut per_entry: BTreeMap<u64, u32> = BTreeMap::new();
    for row in rows {
        let row = jsonx::as_obj(row)?;
        if jsonx::opt_str(row, "hud_category")?.as_deref() != Some("Biology") {
            continue;
        }
        let entry = jsonx::req_u64(row, "entryid")?;
        let row_body = jsonx::opt_str(row, "body")?.unwrap_or_default();
        if row_body.trim().to_lowercase() != target {
            continue;
        }
        let coord = |k: &str| row.get(k).map(Value::to_string).unwrap_or_default();
        if seen.insert((entry, coord("latitude"), coord("longitude"))) {
            *per_entry.entry(entry).or_insert(0) += 1;
        }
    }

    let mut per_species: BTreeMap<(String, String), u32> = BTreeMap::new();
    for (entry, n) in per_entry {
        if let Some(key) = resolve(entry) {
            *per_species.entry(key).or_insert(0) += n;
        }
    }
    let total: u32 = per_species.values().sum();
    let mut hints: Vec<RemoteSpeciesHint> = per_species
        .into_iter()
        .map(|((genus, species), n)| RemoteSpeciesHint {
            genus,
            species,
            weight: f64::from(n) / f64::from(total),
        })
        .collect();
    hints.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    Ok(RemoteBodyBio { hints })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = include_str!("../../../tests/fixtures/net/canonn_body_ok.json");
    const EMPTY: &str = include_str!("../../../tests/fixtures/net/canonn_body_empty.json");
    const SYS: &str = "Sistema Sintetico de Prueba";

    fn resolve(id: u64) -> Option<(String, String)> {
        match id {
            9900001 => Some(("GeneroA".into(), "EspecieAlfa".into())),
            9900002 => Some(("GeneroB".into(), "EspecieBeta".into())),
            9900004 => Some(("GeneroC".into(), "EspecieGamma".into())),
            _ => None,
        }
    }

    #[test]
    fn url_uses_system_name_and_odyssey() {
        assert_eq!(
            body_url("HIP 1727"),
            "https://us-central1-canonn-api-236217.cloudfunctions.net/query/getSystemPoi?system=HIP%201727&odyssey=Y"
        );
    }

    #[test]
    fn normalizes_body_names() {
        assert_eq!(
            normalize_body_name("HIP 1727", "HIP 1727 AB 1 h a"),
            "AB 1 h a"
        );
        assert_eq!(normalize_body_name("HIP 1727", "hip 1727 A 1"), "A 1");
        assert_eq!(normalize_body_name("HIP 1727", "HIP 1727"), "");
        assert_eq!(normalize_body_name("HIP 1727", "  HIP 1727 A 1 "), "A 1");
    }

    #[test]
    fn exact_match_case_insensitive_and_deduplicated() {
        let bio = parse_body(OK, SYS, &format!("{SYS} A 1 a"), &resolve).unwrap();
        // "A 1 a" (3 filas) + "a 1 A" (1 fila); "A 1" y el cuerpo nulo no cuentan.
        // Alfa: 3 observaciones distintas, Beta: 1.
        assert_eq!(bio.hints.len(), 2);
        let alfa = bio
            .hints
            .iter()
            .find(|h| h.species == "EspecieAlfa")
            .unwrap();
        let beta = bio
            .hints
            .iter()
            .find(|h| h.species == "EspecieBeta")
            .unwrap();
        assert_eq!(alfa.genus, "GeneroA");
        assert!((alfa.weight - 0.75).abs() < 1e-12);
        assert!((beta.weight - 0.25).abs() < 1e-12);
    }

    #[test]
    fn a1_never_matches_a1a() {
        let bio = parse_body(OK, SYS, &format!("{SYS} A 1"), &resolve).unwrap();
        assert_eq!(bio.hints.len(), 1);
        assert_eq!(bio.hints[0].species, "EspecieGamma");
        assert!((bio.hints[0].weight - 1.0).abs() < 1e-12);
    }

    #[test]
    fn main_star_matches_null_body_rows_only() {
        let bio = parse_body(OK, SYS, SYS, &resolve).unwrap();
        assert_eq!(bio.hints.len(), 1);
        assert_eq!(bio.hints[0].species, "EspecieBeta");
    }

    #[test]
    fn duplicates_count_once_and_geology_is_ignored() {
        let body = r#"{"codex":[
          {"body":"B","entryid":9900001,"hud_category":"Biology","latitude":"1","longitude":"2"},
          {"body":"B","entryid":9900001,"hud_category":"Biology","latitude":"1","longitude":"2"},
          {"body":"B","entryid":9900002,"hud_category":"Biology","latitude":"1","longitude":"2"},
          {"body":"B","entryid":9900002,"hud_category":"Geology","latitude":"5","longitude":"5"}
        ]}"#;
        let bio = parse_body(body, "S", "S B", &resolve).unwrap();
        assert_eq!(bio.hints.len(), 2);
        let sum: f64 = bio.hints.iter().map(|h| h.weight).sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!(bio.hints.iter().all(|h| (h.weight - 0.5).abs() < 1e-12));
    }

    #[test]
    fn unresolved_entries_are_skipped_and_empty_system_has_no_hints() {
        let none = |_: u64| None;
        assert!(
            parse_body(OK, SYS, &format!("{SYS} A 1 a"), &none)
                .unwrap()
                .hints
                .is_empty()
        );
        assert!(
            parse_body(EMPTY, SYS, &format!("{SYS} A 1"), &resolve)
                .unwrap()
                .hints
                .is_empty()
        );
    }

    #[test]
    fn malformed_bodies_fail_cleanly() {
        for bad in [
            "",
            "{",
            "null",
            r#"{"codex":"x"}"#,
            r#"{"codex":[{"entryid":"x","hud_category":"Biology"}]}"#,
        ] {
            assert_eq!(
                parse_body(bad, "S", "S A", &resolve),
                Err(NetFailure::Malformed),
                "{bad}"
            );
        }
    }
}

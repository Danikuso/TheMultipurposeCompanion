//! Adaptador puro de EDSM (respaldo de la ficha de sistema).

use crate::jsonx;
use crate::transport::url_encode;
use serde_json::Value;
use tmc_model::net::{NetFailure, RemoteSystem};

pub const BASE: &str = "https://www.edsm.net/api-v1";

pub fn system_url(name: &str) -> String {
    format!(
        "{BASE}/system?systemName={}&showId=1&showInformation=1&showPrimaryStar=1",
        url_encode(name)
    )
}

/// `{}` y `[]` (sistema desconocido) -> `Ok(None)`. EDSM no da `discovered`,
/// `body_count` ni Powerplay: quedan en `None`/vacío.
pub fn parse_system(body: &str) -> Result<Option<RemoteSystem>, NetFailure> {
    let v = jsonx::parse(body)?;
    let o = match &v {
        Value::Array(a) if a.is_empty() => return Ok(None),
        Value::Object(o) if o.is_empty() => return Ok(None),
        Value::Object(o) => o,
        _ => return Err(NetFailure::Malformed),
    };
    let empty = jsonx::Obj::new();
    let info = match o.get("information") {
        None | Some(Value::Null) => &empty,
        Some(Value::Array(a)) if a.is_empty() => &empty,
        Some(Value::Object(i)) => i,
        Some(_) => return Err(NetFailure::Malformed),
    };
    let primary_star = match o.get("primaryStar") {
        None | Some(Value::Null) => None,
        Some(Value::Object(s)) => jsonx::opt_str(s, "type")?,
        Some(_) => return Err(NetFailure::Malformed),
    };
    Ok(Some(RemoteSystem {
        id64: jsonx::req_u64(o, "id64")?,
        name: jsonx::req_str(o, "name")?,
        discovered: None,
        body_count: None,
        primary_star,
        allegiance: jsonx::opt_str(info, "allegiance")?,
        government: jsonx::opt_str(info, "government")?,
        controlling_faction: jsonx::opt_str(info, "faction")?,
        faction_state: jsonx::opt_str(info, "factionState")?,
        economy: jsonx::opt_str(info, "economy")?,
        second_economy: jsonx::opt_str(info, "secondEconomy")?,
        security: jsonx::opt_str(info, "security")?,
        population: jsonx::opt_u64(info, "population")?,
        powers: Vec::new(),
        powerplay_state: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOL: &str = r#"{"name":"Sol","id":27,"id64":10477373803,"information":{"allegiance":"Federation","government":"Democracy","faction":"Mother Gaia","factionState":"Expansion","population":22780919531,"security":"High","economy":"Refinery","secondEconomy":"Service","reserve":"Common"},"primaryStar":{"type":"G (White-Yellow) Star","name":"Sol","isScoopable":true}}"#;

    #[test]
    fn url_encodes_name() {
        assert_eq!(
            system_url("Col 359 Sector"),
            "https://www.edsm.net/api-v1/system?systemName=Col%20359%20Sector&showId=1&showInformation=1&showPrimaryStar=1"
        );
    }

    #[test]
    fn parses_edsm_system() {
        let s = parse_system(SOL).unwrap().unwrap();
        assert_eq!(s.id64, 10477373803);
        assert_eq!(s.name, "Sol");
        assert_eq!(s.allegiance.as_deref(), Some("Federation"));
        assert_eq!(s.government.as_deref(), Some("Democracy"));
        assert_eq!(s.controlling_faction.as_deref(), Some("Mother Gaia"));
        assert_eq!(s.faction_state.as_deref(), Some("Expansion"));
        assert_eq!(s.economy.as_deref(), Some("Refinery"));
        assert_eq!(s.second_economy.as_deref(), Some("Service"));
        assert_eq!(s.security.as_deref(), Some("High"));
        assert_eq!(s.population, Some(22_780_919_531));
        assert_eq!(s.primary_star.as_deref(), Some("G (White-Yellow) Star"));
        assert_eq!(s.body_count, None);
        assert_eq!(s.discovered, None);
        assert!(s.powers.is_empty());
        assert_eq!(s.powerplay_state, None);
    }

    #[test]
    fn system_without_information_still_parses() {
        let s = parse_system(r#"{"name":"Zz","id64":9,"information":[]}"#)
            .unwrap()
            .unwrap();
        assert_eq!(s.id64, 9);
        assert_eq!(s.government, None);
        assert_eq!(s.primary_star, None);
    }

    #[test]
    fn edsm_empty_object_or_array_is_no_data() {
        assert_eq!(parse_system("{}"), Ok(None));
        assert_eq!(parse_system("[]"), Ok(None));
        assert_eq!(parse_system(""), Err(NetFailure::Malformed));
    }

    #[test]
    fn malformed_or_truncated_bodies_fail_cleanly() {
        for bad in [
            "{",
            "null",
            "42",
            r#"[1]"#,
            r#"{"name":"Sol"}"#,
            r#"{"name":5,"id64":1}"#,
            r#"{"name":"Sol","id64":"1"}"#,
            r#"{"name":"Sol","id64":1,"information":{"population":"x"}}"#,
            r#"{"name":"Sol","id64":1,"primaryStar":{"type":3}}"#,
            &SOL[..60],
        ] {
            assert_eq!(parse_system(bad), Err(NetFailure::Malformed), "{bad}");
        }
    }
}

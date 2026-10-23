//! Adaptador para la API de ticks del BGS de EDCD (`https://tick.edcd.io/api/ticks`).

use chrono::{DateTime, Utc};
use serde_json::Value;
use tmc_model::net::{BgsTickInfo, NetFailure};

pub const BGS_TICK_URL: &str = "https://tick.edcd.io/api/ticks";

/// Parsea el listado de ticks de EDCD. Devuelve el tick más reciente.
pub fn parse_ticks(
    body: &str,
    fallback_updated_at: DateTime<Utc>,
) -> Result<Option<BgsTickInfo>, NetFailure> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(NetFailure::Malformed);
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|_| NetFailure::Malformed)?;

    match value {
        Value::Array(items) => {
            for item in items.iter().rev() {
                if let Some(info) = parse_entry(item, fallback_updated_at)? {
                    return Ok(Some(info));
                }
            }
            Ok(None)
        }
        Value::Object(_) => parse_entry(&value, fallback_updated_at),
        _ => Err(NetFailure::Malformed),
    }
}

fn parse_entry(
    value: &Value,
    fallback_updated_at: DateTime<Utc>,
) -> Result<Option<BgsTickInfo>, NetFailure> {
    let Some(obj) = value.as_object() else {
        return Err(NetFailure::Malformed);
    };

    let time_str = obj
        .get("time")
        .or_else(|| obj.get("TIME"))
        .and_then(|v| v.as_str());

    let Some(time_str) = time_str else {
        return Ok(None);
    };

    let latest_tick = time_str
        .parse::<DateTime<Utc>>()
        .map_err(|_| NetFailure::Malformed)?;

    let updated_at = obj
        .get("updated_at")
        .or_else(|| obj.get("UPDATED_AT"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or(fallback_updated_at);

    Ok(Some(BgsTickInfo {
        latest_tick,
        updated_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn bgs_tick_url_is_correct() {
        assert_eq!(BGS_TICK_URL, "https://tick.edcd.io/api/ticks");
    }

    #[test]
    fn parse_ticks_from_edcd_array_returns_latest_tick() {
        let json = r#"[
            {
                "_id": "5bdde1209b55231c5c568f6a",
                "time": "2018-11-04T18:03:00.000Z",
                "updated_at": "2018-11-04T18:03:12.784Z",
                "__v": 0
            },
            {
                "_id": "67113a303e0ddb850d99042b",
                "time": "2026-10-17T16:25:36.000Z",
                "updated_at": "2026-10-17T16:30:00.000Z",
                "__v": 0
            }
        ]"#;

        let now = Utc::now();
        let result = parse_ticks(json, now).expect("parse ok");
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(
            info.latest_tick,
            Utc.with_ymd_and_hms(2026, 10, 17, 16, 25, 36).unwrap()
        );
        assert_eq!(
            info.updated_at,
            Utc.with_ymd_and_hms(2026, 10, 17, 16, 30, 0).unwrap()
        );
    }

    #[test]
    fn parse_ticks_with_uppercase_keys() {
        let json = r#"[
            {"TIME": "2026-10-23T04:15:00Z"}
        ]"#;
        let now = Utc::now();
        let info = parse_ticks(json, now)
            .expect("parse ok")
            .expect("some tick");
        assert_eq!(
            info.latest_tick,
            Utc.with_ymd_and_hms(2026, 10, 23, 4, 15, 0).unwrap()
        );
        assert_eq!(info.updated_at, now);
    }

    #[test]
    fn parse_ticks_single_object() {
        let json = r#"{"time": "2026-10-23T06:00:00Z", "updated_at": "2026-10-23T06:05:00Z"}"#;
        let now = Utc::now();
        let info = parse_ticks(json, now)
            .expect("parse ok")
            .expect("some tick");
        assert_eq!(
            info.latest_tick,
            Utc.with_ymd_and_hms(2026, 10, 23, 6, 0, 0).unwrap()
        );
    }

    #[test]
    fn parse_ticks_empty_array_returns_none() {
        let now = Utc::now();
        let result = parse_ticks("[]", now).expect("parse ok");
        assert!(result.is_none());
    }

    #[test]
    fn parse_ticks_malformed_json_returns_error() {
        let now = Utc::now();
        assert_eq!(parse_ticks("{ invalid", now), Err(NetFailure::Malformed));
        assert_eq!(parse_ticks("", now), Err(NetFailure::Malformed));
    }
}

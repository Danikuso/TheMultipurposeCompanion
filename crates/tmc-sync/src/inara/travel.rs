//! Mapeo Inara de ubicación y viajes.

use serde_json::{Map, Value, json};
use tmc_model::events::{Docked, EventKind, LocationInfo, SurfaceEvent};

use super::commander::push_minor_factions;
use super::{inara_row, raw_value, ts_iso};
use crate::router::{MappedInput, MappedRow};

fn opt_str(obj: &mut Map<String, Value>, key: &str, v: Option<&str>) {
    if let Some(s) = v.map(str::trim).filter(|s| !s.is_empty()) {
        obj.insert(key.into(), json!(s));
    }
}

fn opt_coords(obj: &mut Map<String, Value>, coords: Option<[f64; 3]>) {
    if let Some([x, y, z]) = coords {
        obj.insert("starsystemCoords".into(), json!([x, y, z]));
    }
}

fn attach_ship(obj: &mut Map<String, Value>, input: &MappedInput<'_>) {
    opt_str(obj, "shipType", input.snap.ship_type.as_deref());
    if let Some(id) = input.snap.ship_id {
        obj.insert("shipGameID".into(), json!(id));
    }
}

fn attach_taxi_flags(obj: &mut Map<String, Value>, raw: Option<&Value>, dropship: bool) {
    if raw.and_then(|v| v.get("Taxi")).and_then(Value::as_bool) == Some(true) {
        obj.insert("isTaxiShuttle".into(), json!(true));
    }
    if dropship {
        obj.insert("isTaxiDropship".into(), json!(true));
    }
}

pub(crate) fn map_travel(fid: &str, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) -> bool {
    let ts_owned = ts_iso(input);
    let ts = ts_owned.as_str();
    let raw = raw_value(input);

    match &input.ev.kind {
        EventKind::FsdJump(loc) => {
            push_fsd_jump(fid, ts, loc, raw.as_ref(), input, out);
            true
        }
        EventKind::CarrierJump(loc) => {
            push_carrier_jump(fid, ts, loc, input, out);
            true
        }
        EventKind::Docked(docked) => {
            push_dock(fid, ts, docked, raw.as_ref(), input, out);
            true
        }
        EventKind::Touchdown(surf) => {
            push_touchdown(fid, ts, surf, raw.as_ref(), input, out);
            true
        }
        EventKind::Location(loc) => {
            push_location(fid, ts, loc, raw.as_ref(), out);
            true
        }
        EventKind::Other if input.ev.name == "DropShipDeploy" => {
            push_dropship_land(fid, ts, raw.as_ref(), input, out);
            true
        }
        _ => false,
    }
}

fn push_fsd_jump(
    fid: &str,
    ts: &str,
    loc: &LocationInfo,
    raw: Option<&Value>,
    input: &MappedInput<'_>,
    out: &mut Vec<MappedRow>,
) {
    let mut data = Map::new();
    data.insert("starsystemName".into(), json!(loc.star_system));
    data.insert("starsystemCoords".into(), json!(loc.star_pos));
    if let Some(dist) = loc.jump_dist {
        data.insert("jumpDistance".into(), json!(dist));
    }
    attach_ship(&mut data, input);
    attach_taxi_flags(&mut data, raw, false);

    out.push(inara_row(
        fid,
        "addCommanderTravelFSDJump",
        0,
        ts,
        Value::Object(data),
        None,
    ));

    if let Some(raw_v) = raw {
        push_minor_factions(fid, ts, raw_v, 1, out);
    }
}

fn push_carrier_jump(
    fid: &str,
    ts: &str,
    loc: &LocationInfo,
    input: &MappedInput<'_>,
    out: &mut Vec<MappedRow>,
) {
    let mut data = Map::new();
    data.insert("starsystemName".into(), json!(loc.star_system));
    data.insert("starsystemCoords".into(), json!(loc.star_pos));
    opt_str(&mut data, "stationName", loc.station_name.as_deref());
    if let Some(mid) = loc.market_id {
        data.insert("marketID".into(), json!(mid));
    }
    attach_ship(&mut data, input);

    out.push(inara_row(
        fid,
        "addCommanderTravelCarrierJump",
        0,
        ts,
        Value::Object(data),
        None,
    ));
}

fn push_dock(
    fid: &str,
    ts: &str,
    docked: &Docked,
    raw: Option<&Value>,
    input: &MappedInput<'_>,
    out: &mut Vec<MappedRow>,
) {
    let Some(station) = docked
        .station_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };
    let Some(system) = docked
        .star_system
        .as_deref()
        .or(input.snap.system_name.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };

    let mut data = Map::new();
    data.insert("starsystemName".into(), json!(system));
    opt_coords(&mut data, input.snap.star_pos);
    data.insert("stationName".into(), json!(station));
    if let Some(mid) = docked.market_id {
        data.insert("marketID".into(), json!(mid));
    }
    attach_ship(&mut data, input);
    attach_taxi_flags(&mut data, raw, false);

    out.push(inara_row(
        fid,
        "addCommanderTravelDock",
        0,
        ts,
        Value::Object(data),
        None,
    ));
}

fn push_touchdown(
    fid: &str,
    ts: &str,
    surf: &SurfaceEvent,
    raw: Option<&Value>,
    input: &MappedInput<'_>,
    out: &mut Vec<MappedRow>,
) {
    if surf.player_controlled == Some(false) {
        return;
    }
    let system = raw
        .and_then(|v| v.get("StarSystem"))
        .and_then(Value::as_str)
        .or(input.snap.system_name.as_deref());
    let body = surf.body.as_deref().or(input.snap.body_name.as_deref());
    let (Some(system), Some(body)) = (system, body) else {
        return;
    };

    let mut data = Map::new();
    data.insert("starsystemName".into(), json!(system));
    opt_coords(&mut data, input.snap.star_pos);
    data.insert("starsystemBodyName".into(), json!(body));
    if let (Some(lat), Some(lon)) = (surf.latitude, surf.longitude) {
        data.insert("starsystemBodyCoords".into(), json!([lat, lon]));
    }
    attach_ship(&mut data, input);
    attach_taxi_flags(&mut data, raw, false);

    out.push(inara_row(
        fid,
        "addCommanderTravelLand",
        0,
        ts,
        Value::Object(data),
        None,
    ));
}

fn push_dropship_land(
    fid: &str,
    ts: &str,
    raw: Option<&Value>,
    input: &MappedInput<'_>,
    out: &mut Vec<MappedRow>,
) {
    let system = raw
        .and_then(|v| v.get("StarSystem"))
        .and_then(Value::as_str)
        .or(input.snap.system_name.as_deref());
    let body = raw
        .and_then(|v| v.get("Body"))
        .and_then(Value::as_str)
        .or(input.snap.body_name.as_deref());
    let (Some(system), Some(body)) = (system, body) else {
        return;
    };

    let mut data = Map::new();
    data.insert("starsystemName".into(), json!(system));
    opt_coords(&mut data, input.snap.star_pos);
    data.insert("starsystemBodyName".into(), json!(body));
    attach_ship(&mut data, input);
    attach_taxi_flags(&mut data, raw, true);

    out.push(inara_row(
        fid,
        "addCommanderTravelLand",
        0,
        ts,
        Value::Object(data),
        None,
    ));
}

fn push_location(
    fid: &str,
    ts: &str,
    loc: &LocationInfo,
    raw: Option<&Value>,
    out: &mut Vec<MappedRow>,
) {
    let mut data = Map::new();
    data.insert("starsystemName".into(), json!(loc.star_system));
    data.insert("starsystemCoords".into(), json!(loc.star_pos));
    if loc.docked {
        opt_str(&mut data, "stationName", loc.station_name.as_deref());
        if let Some(mid) = loc.market_id {
            data.insert("marketID".into(), json!(mid));
        }
    }
    opt_str(&mut data, "starsystemBodyName", loc.body.as_deref());
    if let (Some(lat), Some(lon)) = (
        raw.and_then(|v| v.get("Latitude")).and_then(Value::as_f64),
        raw.and_then(|v| v.get("Longitude")).and_then(Value::as_f64),
    ) {
        data.insert("starsystemBodyCoords".into(), json!([lat, lon]));
    }

    out.push(inara_row(
        fid,
        "setCommanderTravelLocation",
        0,
        ts,
        Value::Object(data),
        Some("current"),
    ));

    if let Some(raw_v) = raw {
        push_minor_factions(fid, ts, raw_v, 1, out);
    }
}

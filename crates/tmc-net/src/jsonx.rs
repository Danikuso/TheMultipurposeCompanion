//! Utilidades de lectura estricta de JSON para los adaptadores: un tipo
//! inesperado es `Malformed`, nunca un pánico.

use serde_json::{Map, Value};
use tmc_model::net::NetFailure;

pub(crate) type Obj = Map<String, Value>;

pub(crate) fn parse(body: &str) -> Result<Value, NetFailure> {
    serde_json::from_str(body).map_err(|_| NetFailure::Malformed)
}

pub(crate) fn as_obj(v: &Value) -> Result<&Obj, NetFailure> {
    v.as_object().ok_or(NetFailure::Malformed)
}

/// Ausente o `null` -> `None`; presente con otro tipo -> `Malformed`.
pub(crate) fn opt_str(o: &Obj, k: &str) -> Result<Option<String>, NetFailure> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(NetFailure::Malformed),
    }
}

pub(crate) fn req_str(o: &Obj, k: &str) -> Result<String, NetFailure> {
    opt_str(o, k)?.ok_or(NetFailure::Malformed)
}

pub(crate) fn opt_u64(o: &Obj, k: &str) -> Result<Option<u64>, NetFailure> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(Some).ok_or(NetFailure::Malformed),
    }
}

pub(crate) fn req_u64(o: &Obj, k: &str) -> Result<u64, NetFailure> {
    opt_u64(o, k)?.ok_or(NetFailure::Malformed)
}

pub(crate) fn opt_u32(o: &Obj, k: &str) -> Result<Option<u32>, NetFailure> {
    match opt_u64(o, k)? {
        None => Ok(None),
        Some(n) => u32::try_from(n)
            .map(Some)
            .map_err(|_| NetFailure::Malformed),
    }
}

pub(crate) fn req_f64(o: &Obj, k: &str) -> Result<f64, NetFailure> {
    match o.get(k) {
        Some(v) => v.as_f64().ok_or(NetFailure::Malformed),
        None => Err(NetFailure::Malformed),
    }
}

pub(crate) fn opt_bool(o: &Obj, k: &str) -> Result<Option<bool>, NetFailure> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(NetFailure::Malformed),
    }
}

pub(crate) fn req_array<'a>(o: &'a Obj, k: &str) -> Result<&'a Vec<Value>, NetFailure> {
    match o.get(k) {
        Some(Value::Array(a)) => Ok(a),
        _ => Err(NetFailure::Malformed),
    }
}

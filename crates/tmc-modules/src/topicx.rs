//! Utilidades comunes de los módulos para publicar topics solo cuando cambian.

use serde_json::Value;
use std::collections::HashMap;
use tmc_core::TopicSet;

/// Copia del valor sin `as_of`: ese campo avanza con cada evento y no cuenta como cambio.
pub(crate) fn strip_as_of(v: Option<&Value>) -> Option<Value> {
    v.map(|v| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("as_of");
        }
        v
    })
}

/// Guarda `v` en `topic` y lo marca sucio solo si cambió (ignorando `as_of`).
pub(crate) fn set_topic(
    values: &mut HashMap<&'static str, Value>,
    topic: &'static str,
    v: Value,
    dirty: &mut TopicSet,
) {
    if strip_as_of(values.get(topic)) != strip_as_of(Some(&v)) {
        values.insert(topic, v);
        dirty.insert(topic.to_owned());
    }
}

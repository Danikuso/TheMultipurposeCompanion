//! Coordenadas de los destinos de misión.
//!
//! Orden de resolución: lo visto en vivo (`FSDJump`/`Location`), después el almacén
//! (`system_coords` + historial de saltos; lectura `missions.coords`) y, por último,
//! la red (`SystemCoordsByName`) para los nombres que sigan faltando.

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use tmc_core::StoreTask;
use tmc_model::net::SystemCoords;

pub const TAG_COORDS: &str = "missions.coords";
/// Consultas de coordenadas en vuelo como máximo.
pub const MAX_COORDS_INFLIGHT: usize = 10;

pub fn coord_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn finite(p: &[f64; 3]) -> bool {
    p.iter().all(|v| v.is_finite())
}

#[derive(Debug, Default)]
pub struct CoordResolver {
    known: HashMap<String, [f64; 3]>,
    /// Ya preguntados al almacén.
    asked_local: HashSet<String>,
    /// El almacén no los tenía: candidatos a la red.
    missing_local: HashMap<String, String>,
    /// La red respondió sin datos o falló: no se repiten.
    unresolvable: HashSet<String>,
}

impl CoordResolver {
    /// Nombres aún no preguntados al almacén (y los marca como preguntados).
    pub fn need(&mut self, names: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for n in names {
            let k = coord_key(n);
            if k.is_empty() || self.known.contains_key(&k) || !self.asked_local.insert(k) {
                continue;
            }
            out.push(n.clone());
        }
        out
    }

    /// Resultado del almacén para los nombres pedidos (`asked`); claves en minúsculas.
    pub fn on_local_for(&mut self, asked: &[String], map: HashMap<String, [f64; 3]>) {
        for n in asked {
            let k = coord_key(n);
            if !map.contains_key(&k) && !self.known.contains_key(&k) {
                self.missing_local.insert(k, n.clone());
            }
        }
        self.on_local(map);
    }

    pub fn on_local(&mut self, map: HashMap<String, [f64; 3]>) {
        for (k, p) in map {
            self.learn(&k, p);
        }
    }

    pub fn on_remote(&mut self, c: &SystemCoords) {
        self.learn(&c.name, c.star_pos);
    }

    /// Coordenadas conocidas por cualquier vía (p. ej. un salto en vivo).
    pub fn learn(&mut self, name: &str, pos: [f64; 3]) {
        let k = coord_key(name);
        if k.is_empty() || !finite(&pos) {
            return;
        }
        self.missing_local.remove(&k);
        self.unresolvable.remove(&k);
        self.known.insert(k, pos);
    }

    pub fn mark_unresolvable(&mut self, key: &str) {
        let k = coord_key(key);
        self.missing_local.remove(&k);
        self.unresolvable.insert(k);
    }

    pub fn is_unresolvable(&self, name: &str) -> bool {
        self.unresolvable.contains(&coord_key(name))
    }

    /// Nombres que faltan en local y pueden pedirse a la red, en orden estable.
    pub fn remote_candidates(&self) -> Vec<String> {
        let mut v: Vec<String> = self.missing_local.values().cloned().collect();
        v.sort();
        v
    }

    pub fn get(&self, name: &str) -> Option<[f64; 3]> {
        self.known.get(&coord_key(name)).copied()
    }

    /// Olvida lo preguntado (cambio de comandante); las coordenadas siguen valiendo.
    pub fn reset_requests(&mut self) {
        self.asked_local.clear();
        self.missing_local.clear();
        self.unresolvable.clear();
    }
}

/// Lectura (`missions.coords`): tabla `system_coords` y, para lo que falte, el último
/// `FSDJump`/`Location`/`CarrierJump` del comandante en ese sistema.
pub fn coords_task(cmdr: String, names: Vec<String>) -> StoreTask {
    Box::new(move |store| {
        let mut found = store.coords_by_names(&names)?;
        let missing: Vec<String> = names
            .iter()
            .map(|n| coord_key(n))
            .filter(|k| !found.contains_key(k))
            .collect();
        if !missing.is_empty() {
            let rows = store.query_json(
                "SELECT lower(json_extract(e.json, '$.StarSystem')), json_extract(e.json, '$.StarPos') \
                 FROM event e WHERE e.cmdr = ?1 AND e.name IN ('FSDJump', 'Location', 'CarrierJump') \
                 AND lower(json_extract(e.json, '$.StarSystem')) IN (SELECT value FROM json_each(?2)) \
                 ORDER BY e.ts DESC, e.id DESC",
                &[json!(cmdr), json!(json!(missing).to_string())],
            )?;
            for r in rows {
                let (Some(k), Some(pos)) = (r[0].as_str(), r[1].as_str()) else {
                    continue;
                };
                let Ok(p) = serde_json::from_str::<[f64; 3]>(pos) else {
                    continue;
                };
                found.entry(k.to_owned()).or_insert(p);
            }
        }
        Ok(json!({"cmdr": cmdr, "names": names, "coords": found}))
    })
}

/// Interpreta el resultado de [`coords_task`]: nombres pedidos y coordenadas halladas.
pub fn coords_from_result(v: &Value) -> (Vec<String>, HashMap<String, [f64; 3]>) {
    let names = serde_json::from_value(v["names"].clone()).unwrap_or_default();
    let coords = serde_json::from_value(v["coords"].clone()).unwrap_or_default();
    (names, coords)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_asks_each_name_once_and_learns() {
        let mut r = CoordResolver::default();
        let names = vec!["Lave".to_owned(), "Diso".to_owned()];
        assert_eq!(r.need(&names), names);
        assert!(r.need(&names).is_empty());
        r.on_local_for(
            &names,
            HashMap::from([("lave".to_owned(), [1.0, 2.0, 3.0])]),
        );
        assert_eq!(r.get("LAVE"), Some([1.0, 2.0, 3.0]));
        assert_eq!(r.remote_candidates(), vec!["Diso".to_owned()]);
        r.on_remote(&SystemCoords {
            name: "Diso".into(),
            id64: None,
            star_pos: [4.0, 5.0, 6.0],
        });
        assert!(r.remote_candidates().is_empty());
        assert_eq!(r.get("diso"), Some([4.0, 5.0, 6.0]));
        r.learn("X", [f64::NAN, 0.0, 0.0]);
        assert_eq!(r.get("X"), None);
    }
}

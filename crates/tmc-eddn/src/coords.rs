//! Índice acotado de coordenadas de sistemas.

use std::collections::HashMap;

pub const COORD_INDEX_CAP: usize = 30_000;

struct Entry {
    id64: Option<u64>,
    pos: [f64; 3],
    seq: u64,
}

#[derive(Default)]
pub struct CoordIndex {
    map: HashMap<String, Entry>,
    seq: u64,
}

pub(crate) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

impl CoordIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserta o actualiza. Si está lleno descarta la entrada más lejana a `center`
    /// (o la más antigua si no hay centro).
    pub fn insert(
        &mut self,
        name_key: String,
        id64: Option<u64>,
        pos: [f64; 3],
        center: Option<[f64; 3]>,
    ) {
        self.seq += 1;
        let seq = self.seq;
        if !self.map.contains_key(&name_key) && self.map.len() >= COORD_INDEX_CAP {
            // Con centro, una entrada más lejana que la más lejana actual se rechaza.
            if let Some(c) = center {
                let farthest = self
                    .map
                    .values()
                    .map(|e| dist(e.pos, c))
                    .fold(f64::NEG_INFINITY, f64::max);
                if dist(pos, c) > farthest {
                    return;
                }
            }
            let victim = match center {
                Some(c) => self
                    .map
                    .iter()
                    .max_by(|a, b| dist(a.1.pos, c).total_cmp(&dist(b.1.pos, c)))
                    .map(|(k, _)| k.clone()),
                None => self
                    .map
                    .iter()
                    .min_by_key(|(_, e)| e.seq)
                    .map(|(k, _)| k.clone()),
            };
            if let Some(k) = victim {
                self.map.remove(&k);
            }
        }
        self.map.insert(name_key, Entry { id64, pos, seq });
    }

    pub fn get(&self, name_key: &str) -> Option<[f64; 3]> {
        self.map.get(name_key).map(|e| e.pos)
    }

    pub fn get_id64(&self, name_key: &str) -> Option<u64> {
        self.map.get(name_key).and_then(|e| e.id64)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Descarta las entradas a más de `keep_radius_ly` de `center`.
    pub fn recenter(&mut self, center: [f64; 3], keep_radius_ly: f64) {
        self.map
            .retain(|_, e| dist(e.pos, center) <= keep_radius_ly);
    }
}

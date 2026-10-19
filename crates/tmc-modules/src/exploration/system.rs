//! Estado de los cuerpos del sistema actual y su proyección al tópico
//! `exploration.system` (progreso del FSS, valores, señales y consejo).

use super::value::planet_mapped_value;
use super::value::{
    BodyKind, ValueFlags, body_kind, body_values, estimate_probes, is_terraformable, planet_base,
};
use crate::data::{StarSpecial, star_flags};
use serde::{Serialize, Serializer, ser::SerializeMap};
use std::collections::{BTreeMap, HashMap};
use tmc_model::events::{Scan, Signal};

/// Sondas DSS: `estimated` es `true` mientras sea una aproximación por radio;
/// si se omite en el tópico, equivale a `false` (objetivo conocido).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProbeHint {
    pub n: u8,
    #[serde(skip_serializing_if = "is_false")]
    pub estimated: bool,
}

/// Consejo por cuerpo. Se serializa en `snake_case` (`map`, `land`, `not_worth`); la UI lo traduce con `exploration-advice-<valor>` (con `_` como `-`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Advice {
    Map,
    Land,
    NotWorth,
}

fn is_default_advice(advice: &[Advice]) -> bool {
    advice == [Advice::NotWorth]
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Fila del panel. Las seis marcas conservan sus campos Rust y se serializan
/// en `marks`: 1 terraformable, 2 landable, 4 scoopable, 8 first_discovery,
/// 16 already_mapped, 32 mapped_by_you. `marks` ausente equivale a 0.
/// Las señales a 0 y los `Option` a `None` se omiten.
/// `advice` ausente equivale a `[NotWorth]`; Map y Land siempre son explícitos.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyRow {
    pub body_id: u32,
    pub short_name: String,
    /// Tipo de cuerpo; etiqueta en la clave Fluent `exploration-kind-<kind_key>`.
    pub kind_key: String,
    /// Clase espectral de las estrellas (para mostrar «Estrella K»).
    pub star_class: Option<String>,
    pub distance_ls: Option<f64>,
    pub scanned_value: u64,
    pub mapped_value: Option<u64>,
    pub bio: u32,
    pub geo: u32,
    pub terraformable: bool,
    pub landable: bool,
    pub scoopable: bool,
    pub first_discovery: bool,
    pub already_mapped: bool,
    pub mapped_by_you: bool,
    pub probes: Option<ProbeHint>,
    pub advice: Vec<Advice>,
}

impl Serialize for BodyRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_map(None)?;
        row.serialize_entry("body_id", &self.body_id)?;
        row.serialize_entry("short_name", &self.short_name)?;
        row.serialize_entry("kind_key", &self.kind_key)?;
        if let Some(class) = &self.star_class {
            row.serialize_entry("star_class", class)?;
        }
        if let Some(distance) = self.distance_ls {
            row.serialize_entry("distance_ls", &distance)?;
        }
        row.serialize_entry("scanned_value", &self.scanned_value)?;
        if let Some(mapped) = self.mapped_value {
            row.serialize_entry("mapped_value", &mapped)?;
        }
        if self.bio != 0 {
            row.serialize_entry("bio", &self.bio)?;
        }
        if self.geo != 0 {
            row.serialize_entry("geo", &self.geo)?;
        }
        let marks = u8::from(self.terraformable)
            | (u8::from(self.landable) << 1)
            | (u8::from(self.scoopable) << 2)
            | (u8::from(self.first_discovery) << 3)
            | (u8::from(self.already_mapped) << 4)
            | (u8::from(self.mapped_by_you) << 5);
        if marks != 0 {
            row.serialize_entry("marks", &marks)?;
        }
        if let Some(probes) = &self.probes {
            row.serialize_entry("probes", probes)?;
        }
        if !is_default_advice(&self.advice) {
            row.serialize_entry("advice", &self.advice)?;
        }
        row.end()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FssProgress {
    /// Fracción 0..1 de cuerpos escaneados sobre el total del sistema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_count: Option<u32>,
    pub scanned: u32,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SystemTopic {
    pub system: String,
    pub system_address: u64,
    /// `WasDiscovered` de la estrella de llegada; `None` hasta escanearla.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovered: Option<bool>,
    pub fss: FssProgress,
    /// Lo que el comandante ha ganado de verdad en este sistema hasta ahora:
    /// valor escaneado de cada cuerpo o, si lo mapeó él, su valor de mapeo.
    pub value_now: u64,
    /// `value_now` más lo que añadiría mapear las filas con consejo `Map`
    /// (valor de mapeo menos valor escaneado de cada una).
    pub value_if_mapped: u64,
    /// Cuerpos del FSS aún sin escanear (`body_count` − escaneados).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unidentified: Option<u32>,
    /// Filas omitidas por el tope de tamaño del tópico (las de menor valor);
    /// ausente si es 0. Los totales y el FSS cuentan siempre todos los cuerpos.
    #[serde(skip_serializing_if = "is_zero")]
    pub truncated: u32,
    pub star_classes: Vec<String>,
    pub rows: Vec<BodyRow>,
}

/// Presupuesto de bytes JSON para las filas (el tópico completo no debe pasar de 64 KiB).
pub const ROWS_BUDGET_BYTES: usize = 56 * 1024;

fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[derive(Debug, Clone, Copy, Default)]
struct Signals {
    bio: u32,
    geo: u32,
    from_saa: bool,
}

#[derive(Debug, Clone, Copy)]
struct Mapping {
    probes_used: Option<u32>,
    target: Option<u32>,
}

#[derive(Debug, Clone)]
struct Body {
    scan: Scan,
    mapping: Option<Mapping>,
}

/// Cuerpos conocidos del sistema actual.
#[derive(Debug, Default)]
pub struct SystemBodies {
    known: bool,
    name: String,
    address: u64,
    discovered: Option<bool>,
    body_count: Option<u32>,
    complete: bool,
    bodies: BTreeMap<u32, Body>,
    signals: HashMap<u32, Signals>,
    /// `SAAScanComplete` recibidos antes del `Scan` de su cuerpo.
    pending_mapping: HashMap<u32, Mapping>,
}

fn count(signals: &[Signal], pick: fn(&Signal) -> bool) -> u32 {
    signals
        .iter()
        .filter(|s| pick(s))
        .fold(0u32, |a, s| a.saturating_add(s.count))
}

impl SystemBodies {
    pub fn has_data(&self) -> bool {
        self.known || !self.bodies.is_empty()
    }

    pub fn address(&self) -> u64 {
        self.address
    }

    pub fn scanned(&self) -> u32 {
        u32::try_from(self.bodies.len()).unwrap_or(u32::MAX)
    }

    pub fn body_count(&self) -> Option<u32> {
        self.body_count
    }

    /// Entra en un sistema. Devuelve `true` si cambió algo (otro sistema o primera vez).
    pub fn enter(&mut self, name: &str, address: u64) -> bool {
        if !self.known {
            // Primer sistema conocido: adopta lo ya recibido (p. ej. escaneos
            // anteriores al primer Location/jump) en vez de borrarlo.
            self.known = true;
            self.name = name.to_owned();
            self.address = address;
            return true;
        }
        let same = self.known
            && if address != 0 {
                self.address == address
            } else {
                self.name == name
            };
        if same {
            return false;
        }
        *self = Self {
            known: true,
            name: name.to_owned(),
            address,
            ..Self::default()
        };
        true
    }

    pub fn on_scan(&mut self, scan: &Scan) -> bool {
        let Some(id) = scan.body_id else {
            return false;
        };
        let Some(kind) = body_kind(scan) else {
            return false; // anillos y cinturones: sin valor, no son filas
        };
        if matches!(kind, BodyKind::Star { .. })
            && (scan.distance_from_arrival_ls == Some(0.0) || id == 0)
            && scan.was_discovered.is_some()
        {
            self.discovered = scan.was_discovered;
        }
        let mapping = self
            .bodies
            .get(&id)
            .and_then(|b| b.mapping)
            .or_else(|| self.pending_mapping.remove(&id));
        self.bodies.insert(
            id,
            Body {
                scan: scan.clone(),
                mapping,
            },
        );
        true
    }

    fn other_system(&self, address: Option<u64>) -> bool {
        self.known && address.is_some_and(|a| a != 0 && a != self.address)
    }

    pub fn on_fss_discovery(&mut self, body_count: Option<u32>, address: Option<u64>) -> bool {
        if self.other_system(address) || body_count.is_none() || self.body_count == body_count {
            return false;
        }
        self.body_count = body_count;
        true
    }

    pub fn on_all_found(&mut self, count: Option<u32>, address: Option<u64>) -> bool {
        if self.other_system(address) {
            return false;
        }
        let before = (self.complete, self.body_count);
        self.complete = true;
        if self.body_count.is_none() {
            self.body_count = count;
        }
        before != (self.complete, self.body_count)
    }

    pub fn on_fss_signals(&mut self, body_id: Option<u32>, signals: &[Signal]) -> bool {
        let Some(id) = body_id else { return false };
        let entry = self.signals.entry(id).or_default();
        if entry.from_saa {
            return false; // el DSS manda sobre el FSS
        }
        let (bio, geo) = (
            count(signals, Signal::is_biological),
            count(signals, Signal::is_geological),
        );
        let changed = (entry.bio, entry.geo) != (bio, geo);
        entry.bio = bio;
        entry.geo = geo;
        changed
    }

    pub fn on_saa_signals(&mut self, body_id: Option<u32>, signals: &[Signal]) -> bool {
        let Some(id) = body_id else { return false };
        self.signals.insert(
            id,
            Signals {
                bio: count(signals, Signal::is_biological),
                geo: count(signals, Signal::is_geological),
                from_saa: true,
            },
        );
        true
    }

    pub fn on_saa_complete(
        &mut self,
        body_id: Option<u32>,
        probes_used: Option<u32>,
        target: Option<u32>,
    ) -> bool {
        let Some(id) = body_id else { return false };
        let mapping = Mapping {
            probes_used,
            target,
        };
        match self.bodies.get_mut(&id) {
            Some(body) => {
                body.mapping = Some(mapping);
                true
            }
            None => {
                // El `Scan` aún no ha llegado: se aplica cuando llegue.
                self.pending_mapping.insert(id, mapping);
                false
            }
        }
    }

    fn short_name(&self, full: &str) -> String {
        let rest = full
            .strip_prefix(self.name.as_str())
            .map(str::trim)
            .filter(|r| !r.is_empty());
        rest.unwrap_or(full).to_owned()
    }

    fn row(&self, id: u32, body: &Body, threshold: u64, odyssey: bool) -> BodyRow {
        let scan = &body.scan;
        let first_discovery = scan.was_discovered == Some(false);
        let values = body_values(scan, odyssey);
        let efficiency = body
            .mapping
            .is_none_or(|m| match (m.probes_used, m.target) {
                (Some(used), Some(target)) => used <= target,
                _ => true,
            });
        let mapped_value = mapped_with(scan, odyssey, efficiency).or(values.mapped);
        let probes = mapped_value.and_then(|_| match body.mapping.and_then(|m| m.target) {
            Some(t) => Some(ProbeHint {
                n: u8::try_from(t).unwrap_or(u8::MAX),
                estimated: false,
            }),
            None => scan.radius.map(|r| ProbeHint {
                n: estimate_probes(r),
                estimated: true,
            }),
        });
        let sig = self.signals.get(&id).copied().unwrap_or_default();
        let landable = scan.landable == Some(true);
        let mapped_by_you = body.mapping.is_some();
        let mut advice = Vec::new();
        if !mapped_by_you && mapped_value.is_some_and(|v| v >= threshold) {
            advice.push(Advice::Map);
        }
        if sig.bio > 0 && landable {
            advice.push(Advice::Land);
        }
        if advice.is_empty() {
            advice.push(Advice::NotWorth);
        }
        let star_class = scan
            .star_type
            .clone()
            .filter(|_| scan.planet_class.as_deref().is_none_or(str::is_empty));
        BodyRow {
            body_id: id,
            short_name: self.short_name(&scan.body_name),
            kind_key: kind_key(scan),
            scoopable: star_class
                .as_deref()
                .is_some_and(|c| star_flags(c).scoopable),
            star_class,
            distance_ls: scan
                .distance_from_arrival_ls
                .filter(|d| d.is_finite())
                .map(|d| (d * 10.0).round() / 10.0),
            scanned_value: values.scanned,
            mapped_value,
            bio: sig.bio,
            geo: sig.geo,
            terraformable: is_terraformable(scan),
            landable,
            first_discovery,
            already_mapped: scan.was_mapped == Some(true),
            mapped_by_you,
            probes,
            advice,
        }
    }

    pub fn render(&self, threshold: u64, odyssey: bool) -> SystemTopic {
        let mut rows: Vec<BodyRow> = self
            .bodies
            .iter()
            .map(|(id, b)| self.row(*id, b, threshold, odyssey))
            .collect();
        // Totales y FSS sobre TODOS los cuerpos; el tope solo recorta filas.
        rows.sort_by(|a, b| {
            let key = |r: &BodyRow| (r.mapped_value.unwrap_or(0), r.scanned_value);
            key(b).cmp(&key(a)).then(a.body_id.cmp(&b.body_id))
        });
        let mut value_now = 0u64;
        let mut extra = 0u64;
        for r in &rows {
            value_now = value_now.saturating_add(match (r.mapped_by_you, r.mapped_value) {
                (true, Some(m)) => m,
                _ => r.scanned_value,
            });
            if r.advice.contains(&Advice::Map) {
                extra = extra
                    .saturating_add(r.mapped_value.unwrap_or(0).saturating_sub(r.scanned_value));
            }
        }
        let scanned = self.scanned();
        let progress = if self.complete {
            Some(1.0)
        } else {
            self.body_count
                .filter(|n| *n > 0)
                .map(|n| (f64::from(scanned.min(n)) / f64::from(n) * 1000.0).round() / 1000.0)
        };
        let mut used = 0usize;
        let keep = rows
            .iter()
            .take_while(|r| {
                used += serde_json::to_vec(r).map_or(0, |v| v.len()) + 1;
                used <= ROWS_BUDGET_BYTES
            })
            .count();
        let truncated = u32::try_from(rows.len() - keep).unwrap_or(u32::MAX);
        rows.truncate(keep);
        let mut stars: Vec<(u32, f64, String)> = self
            .bodies
            .iter()
            .filter_map(|(id, b)| {
                let is_star =
                    body_kind(&b.scan).is_some_and(|k| matches!(k, BodyKind::Star { .. }));
                if is_star {
                    b.scan.star_type.as_ref().map(|st| {
                        let dist = b.scan.distance_from_arrival_ls.unwrap_or(f64::MAX);
                        let base = st.split('_').next().unwrap_or(st).to_string();
                        (*id, dist, base)
                    })
                } else {
                    None
                }
            })
            .collect();
        stars.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let mut star_classes = Vec::new();
        for (_, _, base) in stars {
            if !star_classes.contains(&base) {
                star_classes.push(base);
            }
        }
        SystemTopic {
            truncated,
            system: self.name.clone(),
            system_address: self.address,
            discovered: self.discovered,
            fss: FssProgress {
                progress,
                body_count: self.body_count,
                scanned,
                complete: self.complete,
            },
            value_now,
            value_if_mapped: value_now.saturating_add(extra),
            unidentified: self.body_count.map(|n| n.saturating_sub(scanned)),
            star_classes,
            rows,
        }
    }
}

/// Valor de mapeo con la eficiencia indicada (planetas y lunas); `None` si no aplica.
fn mapped_with(scan: &Scan, odyssey: bool, efficiency: bool) -> Option<u64> {
    let BodyKind::Planet { k } = body_kind(scan)? else {
        return None;
    };
    let mass = scan.mass_em.filter(|m| m.is_finite() && *m > 0.0)?;
    let flags = ValueFlags {
        first_discovery: scan.was_discovered == Some(false),
        first_mapped: scan.was_mapped == Some(false),
        efficiency,
        odyssey,
    };
    Some(planet_mapped_value(planet_base(k, mass), &flags))
}

/// Identificador del tipo de cuerpo; la UI lo traduce con la clave Fluent `exploration-kind-<id>`.
pub(crate) fn kind_key(scan: &Scan) -> String {
    let slug = match scan.planet_class.as_deref().filter(|c| !c.is_empty()) {
        Some(class) => match class {
            "Earthlike body" => "earthlike",
            "Water world" => "water-world",
            "Ammonia world" => "ammonia-world",
            "Sudarsky class I gas giant" => "gas-giant-1",
            "Sudarsky class II gas giant" => "gas-giant-2",
            "Sudarsky class III gas giant" => "gas-giant-3",
            "Sudarsky class IV gas giant" => "gas-giant-4",
            "Sudarsky class V gas giant" => "gas-giant-5",
            "Helium rich gas giant" => "helium-gas-giant",
            "Water giant" => "water-giant",
            "Gas giant with water based life" | "Gas giant with ammonia based life" => {
                "gas-giant-life"
            }
            "Rocky body" => "rocky",
            "Rocky ice body" => "rocky-ice",
            "Icy body" => "icy",
            "High metal content body" => "high-metal",
            "Metal rich body" => "metal-rich",
            _ => "other-planet",
        },
        None => {
            let class = scan.star_type.as_deref().unwrap_or("");
            let base = class.split('_').next().unwrap_or("");
            match base {
                "O" | "B" | "A" | "F" | "G" | "K" | "M" => "star-main",
                "N" => "star-neutron",
                "H" | "SupermassiveBlackHole" => "star-black-hole",
                "L" | "T" | "Y" => "star-brown-dwarf",
                "W" | "WN" | "WNC" | "WC" | "WO" => "star-wolf-rayet",
                "C" | "CN" | "CJ" | "CH" | "CHd" => "star-carbon",
                "MS" | "S" => "star-s",
                "TTS" | "AeBe" => "star-protostar",
                _ if star_flags(class).special == Some(StarSpecial::WhiteDwarf) => {
                    "star-white-dwarf"
                }
                _ => "star-other",
            }
        }
    };
    slug.to_owned()
}

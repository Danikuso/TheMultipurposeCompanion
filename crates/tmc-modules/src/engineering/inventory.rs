//! Inventario de materiales: instantánea `Materials` más deltas, y cambios de la sesión.
//!
//! Única fuente del inventario (en vivo, en `eval` y en tests).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::events::{EventKind, MaterialCategory, MaterialUse, material_key, reward_category};

use crate::data::{grade_cap, materials};

const RECENT_MAX: usize = 50;

/// Origen de un cambio de inventario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum ChangeOrigin {
    Mission,
    Scan,
    Mining,
    Collected,
    Trade,
    Synthesis,
    Engineering,
    Discarded,
}

/// Un cambio de la cantidad de un material.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialChange {
    pub key: String,
    pub delta: i32,
    pub origin: ChangeOrigin,
    /// Unidades que no cupieron por el máximo del grado.
    pub lost: u32,
    pub at: DateTime<Utc>,
}

/// Línea del inventario lista para mostrar.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialLine {
    pub key: String,
    pub name_localised: Option<String>,
    /// Respaldo de `materials.json` si el diario no trae nombre localizado.
    pub name_en: Option<String>,
    /// 0 si el material no está catalogado.
    pub grade: u8,
    pub count: u32,
    /// 0 si no está catalogado (sin máximo conocido).
    pub max: u32,
    pub at_max: bool,
    pub known: bool,
}

impl MaterialLine {
    /// Nombre mostrado: localizado, luego inglés del catálogo, luego la clave del diario.
    pub fn display_name(&self) -> &str {
        self.name_localised
            .as_deref()
            .or(self.name_en.as_deref())
            .unwrap_or(&self.key)
    }
}

#[derive(Debug, Clone)]
struct Entry {
    category: MaterialCategory,
    name_localised: Option<String>,
    count: u32,
}

/// Inventario de materiales del comandante.
#[derive(Debug, Clone, Default)]
pub struct MaterialInventory {
    entries: HashMap<String, Entry>,
    snapshot: bool,
}

fn origin_for(category: MaterialCategory) -> ChangeOrigin {
    match category {
        MaterialCategory::Encoded => ChangeOrigin::Scan,
        MaterialCategory::Raw => ChangeOrigin::Mining,
        MaterialCategory::Manufactured => ChangeOrigin::Collected,
    }
}

fn cap_of(key: &str) -> Option<u32> {
    materials().get(key).map(|d| grade_cap(d.grade))
}

impl MaterialInventory {
    pub fn has_snapshot(&self) -> bool {
        self.snapshot
    }

    pub fn count(&self, key: &str) -> u32 {
        self.entries.get(&material_key(key)).map_or(0, |e| e.count)
    }

    /// Claves (ordenadas) de los materiales catalogados que están en su máximo.
    pub fn at_max(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .entries
            .iter()
            .filter(|(k, e)| e.count > 0 && cap_of(k).is_some_and(|c| c > 0 && e.count >= c))
            .map(|(k, _)| k.clone())
            .collect();
        v.sort();
        v
    }

    fn line(key: &str, e: &Entry) -> MaterialLine {
        let def = materials().get(key);
        let max = def.map_or(0, |d| grade_cap(d.grade));
        MaterialLine {
            key: key.to_owned(),
            name_localised: e.name_localised.clone(),
            name_en: def.map(|d| d.name_en.clone()),
            grade: def.map_or(0, |d| d.grade),
            count: e.count,
            max,
            at_max: max > 0 && e.count >= max,
            known: def.is_some(),
        }
    }

    /// Líneas con existencias, por categoría y grado (0 = no catalogado), ordenadas por clave.
    pub fn by_category(&self) -> BTreeMap<MaterialCategory, BTreeMap<u8, Vec<MaterialLine>>> {
        let mut out: BTreeMap<MaterialCategory, BTreeMap<u8, Vec<MaterialLine>>> = BTreeMap::new();
        for (key, e) in self.entries.iter().filter(|(_, e)| e.count > 0) {
            let line = Self::line(key, e);
            out.entry(e.category)
                .or_default()
                .entry(line.grade)
                .or_default()
                .push(line);
        }
        for grades in out.values_mut() {
            for lines in grades.values_mut() {
                lines.sort_by(|a, b| a.key.cmp(&b.key));
            }
        }
        out
    }

    fn category_of(&self, key: &str, hint: Option<MaterialCategory>) -> Option<MaterialCategory> {
        self.entries
            .get(key)
            .map(|e| e.category)
            .or_else(|| materials().get(key).map(|d| d.category))
            .or(hint)
    }

    fn add(
        &mut self,
        name: &str,
        loc: Option<&str>,
        category: Option<MaterialCategory>,
        qty: u32,
        origin: ChangeOrigin,
        at: DateTime<Utc>,
    ) -> Option<MaterialChange> {
        let key = material_key(name);
        if key.is_empty() || qty == 0 {
            return None;
        }
        let Some(category) = self.category_of(&key, category) else {
            decide!("engineering.inventory.skip_no_category", { "key": key });
            return None;
        };
        let cap = cap_of(&key);
        let entry = self.entries.entry(key.clone()).or_insert(Entry {
            category,
            name_localised: None,
            count: 0,
        });
        if let Some(l) = loc.filter(|l| !l.is_empty()) {
            entry.name_localised = Some(l.to_owned());
        }
        let room = cap.map_or(u32::MAX, |c| c.saturating_sub(entry.count));
        let added = qty.min(room);
        entry.count += added;
        let lost = qty - added;
        Some(MaterialChange {
            key,
            delta: i32::try_from(added).unwrap_or(i32::MAX),
            origin,
            lost,
            at,
        })
    }

    fn sub(
        &mut self,
        name: &str,
        qty: u32,
        origin: ChangeOrigin,
        at: DateTime<Utc>,
    ) -> Option<MaterialChange> {
        let key = material_key(name);
        let e = self.entries.get_mut(&key)?;
        let removed = qty.min(e.count);
        e.count -= removed;
        (removed > 0).then(|| MaterialChange {
            key,
            delta: -i32::try_from(removed).unwrap_or(i32::MAX),
            origin,
            lost: 0,
            at,
        })
    }

    fn sub_all(
        &mut self,
        uses: &[MaterialUse],
        origin: ChangeOrigin,
        at: DateTime<Utc>,
    ) -> Vec<MaterialChange> {
        uses.iter()
            .filter_map(|u| self.sub(&u.name, u.count, origin, at))
            .collect()
    }

    /// Aplica un evento y devuelve los cambios de la sesión que produce.
    /// `Materials` resincroniza todo y no cuenta como cambio.
    pub fn apply_event(&mut self, ev: &EventKind, at: DateTime<Utc>) -> Vec<MaterialChange> {
        match ev {
            EventKind::Materials(m) => {
                self.entries.clear();
                self.snapshot = true;
                for (cat, list) in [
                    (MaterialCategory::Raw, &m.raw),
                    (MaterialCategory::Manufactured, &m.manufactured),
                    (MaterialCategory::Encoded, &m.encoded),
                ] {
                    for mc in list {
                        let key = material_key(&mc.name);
                        if key.is_empty() {
                            continue;
                        }
                        let e = self.entries.entry(key).or_insert(Entry {
                            category: cat,
                            name_localised: None,
                            count: 0,
                        });
                        e.count = e.count.saturating_add(mc.count);
                        if let Some(l) = mc.name_localised.as_deref().filter(|l| !l.is_empty()) {
                            e.name_localised = Some(l.to_owned());
                        }
                    }
                }
                decide!("engineering.inventory.snapshot", { "entries": self.entries.len() });
                Vec::new()
            }
            EventKind::MaterialCollected(c) => {
                let key = material_key(&c.name);
                let hint = MaterialCategory::from_journal(&c.category);
                let Some(cat) = self.category_of(&key, hint) else {
                    return Vec::new();
                };
                self.add(
                    &c.name,
                    c.name_localised.as_deref(),
                    Some(cat),
                    c.count,
                    origin_for(cat),
                    at,
                )
                .into_iter()
                .collect()
            }
            EventKind::MaterialDiscarded(c) => self
                .sub(&c.name, c.count, ChangeOrigin::Discarded, at)
                .into_iter()
                .collect(),
            EventKind::MaterialTrade(t) => {
                let mut out: Vec<MaterialChange> = self
                    .sub(&t.paid.material, t.paid.quantity, ChangeOrigin::Trade, at)
                    .into_iter()
                    .collect();
                out.extend(self.add(
                    &t.received.material,
                    t.received.material_localised.as_deref(),
                    MaterialCategory::from_journal(&t.received.category),
                    t.received.quantity,
                    ChangeOrigin::Trade,
                    at,
                ));
                out
            }
            EventKind::Synthesis(s) => self.sub_all(&s.materials, ChangeOrigin::Synthesis, at),
            EventKind::EngineerCraft(c) => {
                self.sub_all(&c.ingredients, ChangeOrigin::Engineering, at)
            }
            EventKind::EngineerContribution(c) if c.type_.eq_ignore_ascii_case("material") => c
                .material
                .as_deref()
                .and_then(|m| self.sub(m, c.quantity, ChangeOrigin::Engineering, at))
                .into_iter()
                .collect(),
            EventKind::TechnologyBroker(b) => {
                self.sub_all(&b.materials, ChangeOrigin::Engineering, at)
            }
            EventKind::MissionCompleted(m) => m
                .materials_reward
                .iter()
                .filter_map(|r| {
                    self.add(
                        &r.name,
                        r.name_localised.as_deref(),
                        reward_category(r),
                        r.count,
                        ChangeOrigin::Mission,
                        at,
                    )
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Cambios de la sesión: totales por origen y los últimos 50.
#[derive(Debug, Clone, Default)]
pub struct SessionChanges {
    since: Option<DateTime<Utc>>,
    totals: BTreeMap<ChangeOrigin, (u32, u32)>,
    recent: Vec<MaterialChange>,
}

impl SessionChanges {
    pub fn push(&mut self, changes: &[MaterialChange]) {
        for c in changes {
            let t = self.totals.entry(c.origin).or_default();
            if c.delta >= 0 {
                t.0 = t.0.saturating_add(c.delta.unsigned_abs());
            } else {
                t.1 = t.1.saturating_add(c.delta.unsigned_abs());
            }
            self.recent.push(c.clone());
        }
        if self.recent.len() > RECENT_MAX {
            let excess = self.recent.len() - RECENT_MAX;
            self.recent.drain(..excess);
        }
    }

    /// Por origen: (entra, sale).
    pub fn totals(&self) -> BTreeMap<ChangeOrigin, (u32, u32)> {
        self.totals.clone()
    }

    /// Los últimos 50 cambios, el más reciente al final.
    pub fn recent(&self) -> &[MaterialChange] {
        &self.recent
    }

    pub fn since(&self) -> Option<DateTime<Utc>> {
        self.since
    }

    pub fn reset(&mut self, at: DateTime<Utc>) {
        self.totals.clear();
        self.recent.clear();
        self.since = Some(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::events::parse_line;

    fn ev(json: &str) -> EventKind {
        parse_line(json).unwrap().kind
    }

    fn at() -> DateTime<Utc> {
        "2026-10-21T12:00:00Z".parse().unwrap()
    }

    fn materials_ev(raw: &str) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"Materials",{raw}}}"#
        ))
    }

    fn collected(cat: &str, name: &str, count: u32) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"MaterialCollected","Category":"{cat}","Name":"{name}","Count":{count}}}"#
        ))
    }

    fn trade(paid: (&str, &str, u32), got: (&str, &str, u32)) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"MaterialTrade","MarketID":1,"TraderType":"raw","Paid":{{"Material":"{}","Category":"{}","Quantity":{}}},"Received":{{"Material":"{}","Category":"{}","Quantity":{}}}}}"#,
            paid.0, paid.1, paid.2, got.0, got.1, got.2
        ))
    }

    #[test]
    fn snapshot_then_deltas() {
        let mut inv = MaterialInventory::default();
        inv.apply_event(
            &materials_ev(r#""Raw":[{"Name":"iron","Count":120}]"#),
            at(),
        );
        inv.apply_event(&collected("Raw", "iron", 3), at());
        inv.apply_event(&trade(("iron", "Raw", 6), ("carbon", "Raw", 1)), at());
        assert_eq!(inv.count("iron"), 117);
        assert_eq!(inv.count("carbon"), 1);
        assert!(inv.has_snapshot());
    }

    #[test]
    fn snapshot_resyncs_after_drift() {
        let mut inv = MaterialInventory::default();
        inv.apply_event(
            &materials_ev(r#""Raw":[{"Name":"iron","Count":120}]"#),
            at(),
        );
        inv.apply_event(&collected("Raw", "iron", 3), at());
        inv.apply_event(&trade(("iron", "Raw", 6), ("carbon", "Raw", 1)), at());
        inv.apply_event(&materials_ev(r#""Raw":[{"Name":"iron","Count":50}]"#), at());
        assert_eq!(inv.count("iron"), 50);
        assert_eq!(inv.count("carbon"), 0);
    }

    #[test]
    fn collected_beyond_cap_is_clamped_and_flagged() {
        let mut inv = MaterialInventory::default();
        inv.apply_event(
            &materials_ev(r#""Raw":[{"Name":"iron","Count":299}]"#),
            at(),
        );
        let ch = inv.apply_event(&collected("Raw", "iron", 3), at());
        assert_eq!(inv.count("iron"), 300);
        assert_eq!(ch.len(), 1);
        assert_eq!((ch[0].delta, ch[0].lost), (1, 2));
        assert_eq!(inv.at_max(), vec!["iron".to_owned()]);
    }

    #[test]
    fn mission_reward_category_from_catalog() {
        let mut inv = MaterialInventory::default();
        let e = ev(
            r#"{"timestamp":"2026-10-21T12:00:00Z","event":"MissionCompleted","MissionID":1,"Name":"x","MaterialsReward":[{"Name":"shieldpatternanalysis","Count":2}]}"#,
        );
        let ch = inv.apply_event(&e, at());
        assert_eq!(inv.count("shieldpatternanalysis"), 2);
        assert_eq!((ch[0].delta, ch[0].origin), (2, ChangeOrigin::Mission));
        assert!(inv.by_category().contains_key(&MaterialCategory::Encoded));
        // Con categoría explícita en el formato del diario (material no catalogado).
        let e = ev(
            r#"{"timestamp":"2026-10-21T12:00:00Z","event":"MissionCompleted","MissionID":2,"Name":"x","MaterialsReward":[{"Name":"foo_new","Count":1,"Category":"$MICRORESOURCE_CATEGORY_Manufactured;"}]}"#,
        );
        inv.apply_event(&e, at());
        assert_eq!(inv.count("foo_new"), 1);
        assert!(
            inv.by_category()
                .contains_key(&MaterialCategory::Manufactured)
        );
    }

    #[test]
    fn engineer_craft_consumes_ingredients_not_below_zero() {
        let mut inv = MaterialInventory::default();
        inv.apply_event(
            &materials_ev(r#""Manufactured":[{"Name":"heatvanes","Count":1}]"#),
            at(),
        );
        let e = ev(
            r#"{"timestamp":"2026-10-21T12:00:00Z","event":"EngineerCraft","Engineer":"Felicity Farseer","EngineerID":300100,"BlueprintName":"FSD_LongRange","Level":4,"Ingredients":[{"Name":"heatvanes","Count":3}]}"#,
        );
        let ch = inv.apply_event(&e, at());
        assert_eq!(inv.count("heatvanes"), 0);
        assert_eq!((ch[0].delta, ch[0].origin), (-1, ChangeOrigin::Engineering));
    }

    #[test]
    fn unknown_material_kept_with_journal_name() {
        let mut inv = MaterialInventory::default();
        inv.apply_event(
            &materials_ev(r#""Encoded":[{"Name":"foo_new","Count":4},{"Count":9}]"#),
            at(),
        );
        assert_eq!(inv.count("foo_new"), 4);
        let by = inv.by_category();
        let line = &by[&MaterialCategory::Encoded][&0][0];
        assert_eq!(
            (line.known, line.grade, line.count, line.max, line.at_max),
            (false, 0, 4, 0, false)
        );
        assert_eq!(line.display_name(), "foo_new");
        // La entrada sin nombre se descarta.
        assert_eq!(by[&MaterialCategory::Encoded][&0].len(), 1);
        assert!(inv.at_max().is_empty());
    }

    #[test]
    fn name_en_fallback_when_no_localised() {
        let mut inv = MaterialInventory::default();
        inv.apply_event(&materials_ev(r#""Raw":[{"Name":"iron","Count":5}]"#), at());
        let by = inv.by_category();
        let line = &by[&MaterialCategory::Raw][&1][0];
        assert_eq!(line.name_localised, None);
        assert_eq!(line.name_en.as_deref(), Some("Iron"));
        assert_eq!(line.display_name(), "Iron");
    }

    #[test]
    fn session_totals_by_origin() {
        let mut inv = MaterialInventory::default();
        let mut s = SessionChanges::default();
        inv.apply_event(
            &materials_ev(
                r#""Raw":[{"Name":"iron","Count":20}],"Manufactured":[{"Name":"heatvanes","Count":3}]"#,
            ),
            at(),
        );
        s.push(&inv.apply_event(&collected("Encoded", "shieldpatternanalysis", 2), at()));
        s.push(&inv.apply_event(&trade(("iron", "Raw", 6), ("carbon", "Raw", 1)), at()));
        s.push(&inv.apply_event(
            &ev(
                r#"{"timestamp":"2026-10-21T12:00:00Z","event":"Synthesis","Name":"FSD","Materials":[{"Name":"heatvanes","Count":1}]}"#,
            ),
            at(),
        ));
        let t = s.totals();
        assert_eq!(t[&ChangeOrigin::Scan], (2, 0));
        assert_eq!(t[&ChangeOrigin::Trade], (1, 6));
        assert_eq!(t[&ChangeOrigin::Synthesis], (0, 1));
        assert_eq!(s.recent().len(), 4);
    }

    #[test]
    fn recent_keeps_last_fifty_and_reset_clears() {
        let mut s = SessionChanges::default();
        let c = MaterialChange {
            key: "iron".into(),
            delta: 1,
            origin: ChangeOrigin::Mining,
            lost: 0,
            at: at(),
        };
        s.push(&vec![c; 60]);
        assert_eq!(s.recent().len(), 50);
        assert_eq!(s.totals()[&ChangeOrigin::Mining], (60, 0));
        s.reset(at());
        assert!(s.recent().is_empty() && s.totals().is_empty());
    }

    #[test]
    fn fixture_replay_reaches_expected_inventory() {
        let log = include_str!("../../../../tests/fixtures/journal/engineering_session.log");
        let mut inv = MaterialInventory::default();
        let mut s = SessionChanges::default();
        for line in log.lines() {
            let e = parse_line(line).unwrap();
            s.push(&inv.apply_event(&e.kind, e.timestamp));
        }
        let got: Vec<(&str, u32)> = [
            "iron",
            "polonium",
            "carbon",
            "conductiveceramics",
            "pharmaceuticalisolators",
            "heatvanes",
            "shieldpatternanalysis",
            "foo_new",
        ]
        .into_iter()
        .map(|k| (k, inv.count(k)))
        .collect();
        assert_eq!(
            got,
            vec![
                ("iron", 117),
                ("polonium", 100),
                ("carbon", 1),
                ("conductiveceramics", 26),
                ("pharmaceuticalisolators", 8),
                ("heatvanes", 3),
                ("shieldpatternanalysis", 14),
                ("foo_new", 4),
            ]
        );
        assert_eq!(inv.at_max(), vec!["polonium".to_owned()]);
        assert_eq!(s.totals()[&ChangeOrigin::Mission], (2, 0));
        assert_eq!(s.recent().iter().map(|c| c.lost).sum::<u32>(), 2);
    }
}

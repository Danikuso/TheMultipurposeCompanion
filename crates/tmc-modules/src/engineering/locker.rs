//! Taquilla de a pie: instantánea `ShipLocker` / `ShipLocker.json` más `BackpackChange`.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use tmc_model::events::{EventKind, LockerChange, LockerItem, ShipLockerContents};

/// Línea de la taquilla lista para mostrar.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LockerLine {
    pub key: String,
    pub name_localised: Option<String>,
    pub count: u32,
}

/// Taquilla por secciones, ordenada por nombre.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LockerView {
    pub items: Vec<LockerLine>,
    pub components: Vec<LockerLine>,
    pub consumables: Vec<LockerLine>,
    pub data: Vec<LockerLine>,
}

type Section = BTreeMap<String, (Option<String>, u32)>;

/// Inventario de a pie. Sin E/S: el archivo llega ya leído a `apply_file`.
#[derive(Debug, Clone, Default)]
pub struct LockerInventory {
    /// 0 Items, 1 Components, 2 Consumables, 3 Data.
    sections: [Section; 4],
}

fn section_of(type_: &str) -> Option<usize> {
    match type_.to_ascii_lowercase().as_str() {
        "item" => Some(0),
        "component" => Some(1),
        "consumable" => Some(2),
        "data" => Some(3),
        _ => None,
    }
}

fn build(list: &[LockerItem]) -> Section {
    let mut s = Section::new();
    for i in list.iter().filter(|i| i.count > 0 && !i.name.is_empty()) {
        let e = s.entry(i.name.to_lowercase()).or_default();
        e.1 += i.count;
        if e.0.is_none() {
            e.0.clone_from(&i.name_localised);
        }
    }
    s
}

impl LockerInventory {
    fn replace(&mut self, c: &ShipLockerContents) {
        if c.is_stub() {
            return;
        }
        let lists = [&c.items, &c.components, &c.consumables, &c.data];
        for (i, list) in lists.iter().enumerate() {
            // Una lista ausente no es una lista vacía: se conserva la sección.
            if c.has_list(i) {
                self.sections[i] = build(list);
            }
        }
    }

    pub fn apply_event(&mut self, ev: &EventKind) {
        match ev {
            EventKind::ShipLocker(c) => self.replace(c),
            EventKind::BackpackChange(b) => {
                for ch in &b.added {
                    self.adjust(ch, true);
                }
                for ch in &b.removed {
                    self.adjust(ch, false);
                }
            }
            _ => {}
        }
    }

    fn adjust(&mut self, ch: &LockerChange, add: bool) {
        let Some(i) = section_of(&ch.type_) else {
            return;
        };
        if ch.name.is_empty() {
            return;
        }
        let key = ch.name.to_lowercase();
        let sec = &mut self.sections[i];
        let e = sec.entry(key.clone()).or_default();
        if e.0.is_none() {
            e.0.clone_from(&ch.name_localised);
        }
        e.1 = if add {
            e.1.saturating_add(ch.count)
        } else {
            e.1.saturating_sub(ch.count)
        };
        if e.1 == 0 {
            sec.remove(&key);
        }
    }

    /// Contenido de `ShipLocker.json` (mismo formato que el evento).
    pub fn apply_file(&mut self, v: &Value) {
        if let Some(c) = ShipLockerContents::from_value(v) {
            self.replace(&c);
        }
    }

    pub fn view(&self) -> LockerView {
        let lines = |s: &Section| {
            let mut v: Vec<LockerLine> = s
                .iter()
                .map(|(k, (n, c))| LockerLine {
                    key: k.clone(),
                    name_localised: n.clone(),
                    count: *c,
                })
                .collect();
            v.sort_by_cached_key(|l| l.name_localised.as_deref().unwrap_or(&l.key).to_lowercase());
            v
        };
        LockerView {
            items: lines(&self.sections[0]),
            components: lines(&self.sections[1]),
            consumables: lines(&self.sections[2]),
            data: lines(&self.sections[3]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::events::parse_line;

    fn ev(json: &str) -> EventKind {
        parse_line(json).unwrap().kind
    }

    fn locker(body: &str) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"ShipLocker"{body}}}"#
        ))
    }

    fn change(kind: &str, ty: &str, count: u32) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"BackpackChange","{kind}":[{{"Name":"graphene","Type":"{ty}","Count":{count}}}]}}"#
        ))
    }

    const G5: &str = r#","Items":[],"Components":[{"Name":"graphene","Name_Localised":"Graphene","OwnerID":0,"Count":5}],"Consumables":[],"Data":[]"#;

    fn graphene(l: &LockerInventory) -> u32 {
        l.view()
            .components
            .iter()
            .find(|x| x.key == "graphene")
            .map_or(0, |x| x.count)
    }

    #[test]
    fn locker_snapshot_from_event_or_file() {
        let mut l = LockerInventory::default();
        l.apply_event(&locker(G5));
        assert_eq!(graphene(&l), 5);
        l.apply_event(&locker(""));
        l.apply_file(&serde_json::json!({"timestamp":"t","event":"ShipLocker",
            "Components":[{"Name":"graphene","Count":7}]}));
        assert_eq!(graphene(&l), 7);
    }

    #[test]
    fn backpack_change_adjusts_section() {
        let mut l = LockerInventory::default();
        l.apply_event(&locker(G5));
        l.apply_event(&change("Added", "Component", 3));
        assert_eq!(graphene(&l), 8);
        l.apply_event(&change("Removed", "Component", 10));
        assert_eq!(graphene(&l), 0);
    }

    #[test]
    fn stub_without_file_keeps_previous() {
        let mut l = LockerInventory::default();
        l.apply_event(&locker(G5));
        let before = l.view();
        l.apply_event(&locker(""));
        assert_eq!(l.view(), before);
    }

    #[test]
    fn partial_lists_replace_only_present_categories() {
        let mut l = LockerInventory::default();
        l.apply_event(&locker(G5));
        l.apply_event(&locker(r#","Data":[{"Name":"x","Count":1}]"#));
        assert_eq!(graphene(&l), 5);
        assert_eq!(l.view().data.len(), 1);
    }

    #[test]
    fn view_sorted_by_name() {
        let mut l = LockerInventory::default();
        l.apply_event(&locker(
            r#","Components":[{"Name":"b","Name_Localised":"Alfa","Count":1},{"Name":"a","Name_Localised":"Zeta","Count":1}]"#,
        ));
        let names: Vec<_> = l.view().components.into_iter().map(|x| x.key).collect();
        assert_eq!(names, ["b", "a"]);
    }
}

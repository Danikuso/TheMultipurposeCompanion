//! Libro de datos de exploración sin vender; misma aplicación en vivo e historial.

use super::value::{
    BodyKind, ValueFlags, body_kind, body_values, planet_base, planet_mapped_value,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tmc_model::events::EventKind;

mod history;
pub use history::rebuild_from_history;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnsoldLedger {
    #[serde(with = "body_map")]
    by_body: BTreeMap<(String, u32), u64>,
    #[serde(with = "body_map")]
    mapped_pending: BTreeMap<(String, u32), u64>,
    #[serde(with = "body_map")]
    inefficient_pending: BTreeMap<(String, u32), u64>,
    #[serde(with = "body_map")]
    mapped_efficiency: BTreeMap<(String, u32), bool>,
}

impl UnsoldLedger {
    pub fn set(&mut self, system: &str, body_id: u32, value: u64) {
        if system.is_empty() {
            return;
        }
        let key = (system.to_owned(), body_id);
        if value == 0 {
            self.by_body.remove(&key);
        } else {
            self.by_body.insert(key, value);
        }
    }
    pub fn apply_event(&mut self, current_system: &str, ev: &EventKind, odyssey: bool) {
        match ev {
            EventKind::Scan(scan) => {
                let Some(id) = scan.body_id else {
                    return;
                };
                let system = scan
                    .star_system
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(current_system);
                if system.is_empty() {
                    return;
                }
                let key = (system.to_owned(), id);
                let values = body_values(scan, odyssey);
                if let Some(mapped) = values.mapped {
                    self.mapped_pending.insert(key.clone(), mapped);
                    if let Some(BodyKind::Planet { k }) = body_kind(scan)
                        && let Some(mass) = scan.mass_em.filter(|m| m.is_finite() && *m > 0.0)
                    {
                        let flags = ValueFlags {
                            first_discovery: scan.was_discovered == Some(false),
                            first_mapped: scan.was_mapped == Some(false),
                            efficiency: false,
                            odyssey,
                        };
                        self.inefficient_pending.insert(
                            key.clone(),
                            planet_mapped_value(planet_base(k, mass), &flags),
                        );
                    }
                }
                let earned = if let Some(efficient) = self.mapped_efficiency.get(&key) {
                    // Un Scan repetido nunca sustituye un DSS ya cobrado por el valor FSS.
                    self.by_body
                        .get(&key)
                        .copied()
                        .or_else(|| {
                            if *efficient {
                                self.mapped_pending.get(&key).copied()
                            } else {
                                self.inefficient_pending.get(&key).copied()
                            }
                        })
                        .unwrap_or(values.scanned)
                } else {
                    values.scanned
                };
                self.set(system, id, earned);
            }
            EventKind::SaaScanComplete(mapping) => {
                let Some(id) = mapping.body_id else {
                    return;
                };
                if current_system.is_empty() {
                    return;
                }
                let key = (current_system.to_owned(), id);
                let efficient = match (mapping.probes_used, mapping.efficiency_target) {
                    (Some(used), Some(target)) => used <= target,
                    _ => true,
                };
                self.mapped_efficiency.insert(key.clone(), efficient);
                let mapped = if efficient {
                    self.mapped_pending.get(&key)
                } else {
                    self.inefficient_pending.get(&key)
                }
                .copied();
                if let Some(value) = mapped {
                    self.set(current_system, id, value);
                }
            }
            EventKind::SellExplorationData(sale) => self.sell(&sale.systems),
            EventKind::MultiSellExplorationData(sale) => {
                if sale.discovered.is_empty() {
                    self.sell_all();
                } else {
                    self.sell(
                        &sale
                            .discovered
                            .iter()
                            .map(|s| s.system_name.clone())
                            .collect::<Vec<_>>(),
                    );
                }
            }
            EventKind::Died => self.died(),
            _ => {}
        }
    }
    pub fn sell(&mut self, systems: &[String]) {
        self.by_body
            .retain(|(system, _), _| !systems.contains(system));
        self.mapped_pending
            .retain(|(system, _), _| !systems.contains(system));
        self.inefficient_pending
            .retain(|(system, _), _| !systems.contains(system));
        self.mapped_efficiency
            .retain(|(system, _), _| !systems.contains(system));
    }
    pub fn sell_all(&mut self) {
        *self = Self::default();
    }
    pub fn died(&mut self) {
        self.sell_all();
    }
    /// Valor sin vender de los sistemas indicados (lo que cobraría su venta).
    pub fn total_for(&self, systems: &[String]) -> u64 {
        self.by_body
            .iter()
            .filter(|((system, _), _)| systems.contains(system))
            .fold(0, |sum, (_, value)| sum.saturating_add(*value))
    }
    pub fn total(&self) -> u64 {
        self.by_body
            .values()
            .fold(0, |sum, value| sum.saturating_add(*value))
    }
    pub fn system_count(&self) -> u32 {
        let mut previous = None;
        let mut count = 0u32;
        for (system, _) in self.by_body.keys() {
            if previous != Some(system) {
                count = count.saturating_add(1);
                previous = Some(system);
            }
        }
        count
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LedgerContext {
    pub system: String,
    pub odyssey: bool,
}
impl Default for LedgerContext {
    fn default() -> Self {
        Self {
            system: String::new(),
            odyssey: true,
        }
    }
}
impl LedgerContext {
    pub fn apply(&mut self, ledger: &mut UnsoldLedger, event: &EventKind) {
        match event {
            EventKind::LoadGame(load) => {
                self.odyssey = load.odyssey.unwrap_or(self.odyssey);
            }
            EventKind::FsdJump(location)
            | EventKind::CarrierJump(location)
            | EventKind::Location(location) => {
                self.system = location.star_system.clone();
            }
            _ => {}
        }
        ledger.apply_event(&self.system, event, self.odyssey);
    }
}

// JSON no admite claves tupla: codifica cada mapa como filas [sistema,body_id,valor].
mod body_map {
    use super::*;
    pub fn serialize<S: serde::Serializer, T: Serialize>(
        map: &BTreeMap<(String, u32), T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter()
            .map(|((system, id), value)| (system, id, value))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<(String, u32), T>, D::Error> {
        Ok(Vec::<(String, u32, T)>::deserialize(deserializer)?
            .into_iter()
            .map(|(system, id, value)| ((system, id), value))
            .collect())
    }
}

/// Concilia el snapshot de SQLite con la cola aún no confirmada de la sesión.
#[derive(Default)]
pub(super) struct UnsoldState {
    ledger: UnsoldLedger,
    context: LedgerContext,
    cmdr: Option<String>,
    generation: u64,
    pending: Option<String>,
    cut: i64,
    seen: BTreeMap<String, u64>,
    unconfirmed: Vec<LedgerRecord>,
    now: Option<DateTime<Utc>>,
    requested_at: Option<DateTime<Utc>>,
    since_checkpoint: usize,
    refresh_after_pending: bool,
}

#[derive(Clone)]
struct LedgerRecord {
    file: String,
    line: u64,
    kind: EventKind,
}

impl UnsoldState {
    pub fn topic(&self) -> serde_json::Value {
        let total = self.ledger.total();
        serde_json::json!({"total":total,"systems":self.ledger.system_count(),"at_risk":total>0})
    }

    fn request(&mut self, ctx: &mut tmc_core::ModuleCtx) {
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        self.generation = self.generation.wrapping_add(1);
        let tag = format!("unsold:{}", self.generation);
        self.pending = Some(tag.clone());
        self.requested_at = self.now;
        self.since_checkpoint = 0;
        self.refresh_after_pending = false;
        ctx.store_task(tag, rebuild_from_history(cmdr));
    }

    pub fn on_event(
        &mut self,
        event: &tmc_model::source::SourceEvent,
        state: &tmc_model::state::GameState,
        ctx: &mut tmc_core::ModuleCtx,
    ) -> bool {
        use tmc_model::source::SourceEvent;
        let at = match event {
            SourceEvent::Journal { ev, .. } => Some(ev.timestamp),
            SourceEvent::Status(status) => Some(status.timestamp),
            _ => None,
        };
        if let Some(at) = at {
            self.now = Some(self.now.map_or(at, |previous| previous.max(at)));
        }
        let changed = match event {
            SourceEvent::Journal { ev, file, line, .. } => {
                let mut changed = false;
                let name = state
                    .cmdr
                    .as_ref()
                    .map(|c| c.name.as_str())
                    .filter(|n| !n.is_empty());
                if let Some(name) = name
                    && self.cmdr.as_deref() != Some(name)
                {
                    let generation = self.generation;
                    *self = Self {
                        cmdr: Some(name.to_owned()),
                        generation,
                        now: self.now,
                        ..Default::default()
                    };
                    self.request(ctx);
                    changed = true;
                }
                if self.seen.get(file).is_some_and(|seen| line <= seen) {
                    return changed;
                }
                self.seen.insert(file.clone(), *line);
                // Otra sesión del mismo comandante refresca el historial sin perder la cola.
                if matches!(ev.kind, EventKind::LoadGame(_)) && self.pending.is_none() {
                    self.request(ctx);
                }
                if let Some(kind) = compact(&ev.kind) {
                    self.context.apply(&mut self.ledger, &kind);
                    self.push(LedgerRecord {
                        file: file.clone(),
                        line: *line,
                        kind,
                    });
                    self.since_checkpoint = self.since_checkpoint.saturating_add(1);
                    changed = true;
                }
                changed
            }
            SourceEvent::Stored { module, tag, value }
                if module == "exploration" && self.pending.as_deref() == Some(tag.as_str()) =>
            {
                if value.get("cmdr").and_then(serde_json::Value::as_str) != self.cmdr.as_deref() {
                    return false;
                }
                let Some(cut) = value
                    .get("cut")
                    .and_then(serde_json::Value::as_i64)
                    .filter(|cut| *cut >= self.cut)
                else {
                    return false;
                };
                let Ok(mut ledger) =
                    serde_json::from_value::<UnsoldLedger>(value["ledger"].clone())
                else {
                    return false;
                };
                let Ok(mut context) =
                    serde_json::from_value::<LedgerContext>(value["context"].clone())
                else {
                    return false;
                };
                let Ok(files) =
                    serde_json::from_value::<BTreeMap<String, u64>>(value["files"].clone())
                else {
                    return false;
                };
                self.unconfirmed.retain(|record| {
                    files
                        .get(&record.file)
                        .is_none_or(|line| record.line > *line)
                });
                for record in &self.unconfirmed {
                    context.apply(&mut ledger, &record.kind);
                }
                for (file, line) in files {
                    self.seen
                        .entry(file)
                        .and_modify(|seen| *seen = (*seen).max(line))
                        .or_insert(line);
                }
                self.ledger = ledger;
                self.context = context;
                self.cut = cut;
                self.pending = None;
                self.since_checkpoint = self.unconfirmed.len();
                if self.refresh_after_pending {
                    self.request(ctx);
                }
                true
            }
            SourceEvent::ImportProgress { done, total } if done == total && self.cmdr.is_some() => {
                if self.pending.is_some() {
                    self.refresh_after_pending = true;
                } else {
                    self.request(ctx);
                }
                false
            }
            _ => false,
        };
        let overdue = self
            .now
            .zip(self.requested_at)
            .is_some_and(|(now, at)| now.signed_duration_since(at).num_seconds() >= 60);
        if self.cmdr.is_some()
            && (overdue || (self.pending.is_none() && self.since_checkpoint >= 256))
        {
            self.request(ctx);
        }
        changed
    }

    fn push(&mut self, record: LedgerRecord) {
        if matches!(&record.kind, EventKind::Died)
            | matches!(&record.kind,EventKind::MultiSellExplorationData(sale) if sale.discovered.is_empty())
        {
            // El reset hace irrelevantes los cuerpos anteriores, pero conserva el contexto.
            let load = self
                .unconfirmed
                .iter()
                .rev()
                .find(|r| matches!(r.kind, EventKind::LoadGame(_)))
                .cloned();
            let location = self
                .unconfirmed
                .iter()
                .rev()
                .find(|r| {
                    matches!(
                        r.kind,
                        EventKind::Location(_) | EventKind::FsdJump(_) | EventKind::CarrierJump(_)
                    )
                })
                .cloned();
            self.unconfirmed.clear();
            self.unconfirmed.extend(load);
            self.unconfirmed.extend(location);
        }
        let replace =
            self.unconfirmed
                .last()
                .is_some_and(|previous| match (&previous.kind, &record.kind) {
                    (EventKind::Scan(a), EventKind::Scan(b)) => a == b,
                    (
                        EventKind::Location(_) | EventKind::FsdJump(_) | EventKind::CarrierJump(_),
                        EventKind::Location(_) | EventKind::FsdJump(_) | EventKind::CarrierJump(_),
                    ) => true,
                    (EventKind::LoadGame(_), EventKind::LoadGame(_)) => true,
                    _ => false,
                });
        if replace {
            self.unconfirmed.pop();
        }
        self.unconfirmed.push(record);
    }
}

/// La cola conserva únicamente los campos financieros y el contexto, nunca el JSON completo.
fn compact(event: &EventKind) -> Option<EventKind> {
    Some(match event {
        EventKind::Scan(scan) => EventKind::Scan(Box::new(tmc_model::events::Scan {
            body_id: scan.body_id,
            star_system: scan.star_system.clone(),
            star_type: scan.star_type.clone(),
            stellar_mass: scan.stellar_mass,
            mass_em: scan.mass_em,
            planet_class: scan.planet_class.clone(),
            terraform_state: scan.terraform_state.clone(),
            was_discovered: scan.was_discovered,
            was_mapped: scan.was_mapped,
            ..Default::default()
        })),
        EventKind::LoadGame(load) if load.odyssey.is_none() => return None,
        EventKind::LoadGame(_)
        | EventKind::FsdJump(_)
        | EventKind::CarrierJump(_)
        | EventKind::Location(_)
        | EventKind::SaaScanComplete(_)
        | EventKind::SellExplorationData(_)
        | EventKind::MultiSellExplorationData(_)
        | EventKind::Died => event.clone(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::events::parse_line;

    fn event(name: &str, extra: &str) -> EventKind {
        parse_line(&format!(
            r#"{{"timestamp":"2026-10-18T10:00:00Z","event":"{name}"{extra}}}"#
        ))
        .unwrap()
        .kind
    }
    fn scan(id: u32) -> EventKind {
        event(
            "Scan",
            &format!(
                r#", "BodyID":{id},"BodyName":"A {id}","PlanetClass":"Earthlike body","MassEM":1.0,"WasDiscovered":true,"WasMapped":true"#
            ),
        )
    }
    fn pending() -> UnsoldLedger {
        let mut ledger = UnsoldLedger::default();
        ledger.set("A", 1, 100);
        ledger.set("A", 2, 200);
        ledger.set("B", 3, 300);
        ledger
    }

    #[test]
    fn partial_sale_removes_only_listed_systems() {
        let mut ledger = pending();
        ledger.sell(&["A".into()]);
        assert_eq!(ledger.total(), 300);
        assert_eq!(ledger.system_count(), 1);
    }
    #[test]
    fn multi_sell_removes_listed_systems() {
        let mut ledger = pending();
        ledger.apply_event(
            "Station",
            &event(
                "MultiSellExplorationData",
                r#", "Discovered":[{"SystemName":"A","NumBodies":2}]"#,
            ),
            true,
        );
        assert_eq!(ledger.total(), 300);
        ledger.apply_event(
            "Station",
            &event("MultiSellExplorationData", r#", "Discovered":[]"#),
            true,
        );
        assert_eq!(ledger.total(), 0);
    }
    #[test]
    fn death_zeroes_unsold() {
        let mut ledger = pending();
        ledger.apply_event("Station", &event("Died", ""), true);
        assert_eq!((ledger.total(), ledger.system_count()), (0, 0));
    }
    #[test]
    fn repeated_scan_does_not_double_count() {
        let mut ledger = UnsoldLedger::default();
        let scan = scan(7);
        ledger.apply_event("A", &scan, true);
        let once = ledger.total();
        ledger.apply_event("A", &scan, true);
        assert_eq!(ledger.total(), once);
        assert!(once > 0);
        ledger.set("A", 7, 1000);
        ledger.set("A", 7, 1000);
        assert_eq!(ledger.total(), 1000);
    }
    #[test]
    fn mapping_upgrades_value_without_double_counting() {
        let mut ledger = UnsoldLedger::default();
        let scan = scan(7);
        ledger.apply_event("A", &scan, true);
        let scanned = ledger.total();
        ledger.apply_event(
            "A",
            &event(
                "SAAScanComplete",
                r#", "BodyID":7,"ProbesUsed":6,"EfficiencyTarget":6"#,
            ),
            true,
        );
        let mapped = ledger.total();
        assert!(mapped > scanned);
        let EventKind::Scan(scan) = scan else {
            panic!()
        };
        assert_eq!(
            mapped,
            super::super::value::body_values(&scan, true)
                .mapped
                .unwrap()
        );
    }
    #[test]
    fn duplicate_scan_after_mapping_does_not_lose_mapped_value() {
        let mut ledger = UnsoldLedger::default();
        let scan = scan(7);
        ledger.apply_event("A", &scan, true);
        ledger.apply_event(
            "A",
            &event(
                "SAAScanComplete",
                r#", "BodyID":7,"ProbesUsed":9,"EfficiencyTarget":6"#,
            ),
            true,
        );
        let mapped = ledger.total();
        ledger.apply_event("A", &scan, true);
        assert_eq!(ledger.total(), mapped);
        ledger.apply_event(
            "A",
            &event("SellExplorationData", r#", "Systems":["A"]"#),
            true,
        );
        assert_eq!(ledger.total(), 0);
    }
    #[test]
    fn scan_explicit_system_and_zero_value_bodies_are_handled_without_phantom_systems() {
        let mut ledger = UnsoldLedger::default();
        ledger.apply_event(
            "Other",
            &event(
                "Scan",
                r#", "BodyID":1,"StarSystem":"A","PlanetClass":"Earthlike body","MassEM":1.0"#,
            ),
            true,
        );
        ledger.set("Empty", 9, 0);
        assert_eq!(ledger.system_count(), 1);
        ledger.sell(&["A".into()]);
        assert_eq!(ledger.total(), 0);
    }
    #[test]
    fn serializable_ledger_preserves_pending_mapping_for_store_results() {
        let mut ledger = UnsoldLedger::default();
        ledger.apply_event("A", &scan(7), true);
        let value = serde_json::to_value(&ledger).unwrap();
        let mut restored: UnsoldLedger = serde_json::from_value(value).unwrap();
        restored.apply_event(
            "A",
            &event(
                "SAAScanComplete",
                r#", "BodyID":7,"ProbesUsed":6,"EfficiencyTarget":6"#,
            ),
            true,
        );
        assert!(restored.total() > ledger.total());
    }

    #[test]
    fn rebuild_from_history_matches_live_accumulation() {
        let store = tmc_core::Store::open_in_memory().unwrap();
        let writer = tmc_core::EventWriter::spawn(&store);
        let mut state = tmc_model::state::GameState::default();
        let mut ledger = UnsoldLedger::default();
        let mut context = LedgerContext::default();
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/journal/exploration_session.log"
        ));
        for (line, json) in fixture.lines().enumerate() {
            let ev = parse_line(json).unwrap();
            state.apply_event(&ev);
            context.apply(&mut ledger, &ev.kind);
            writer.send(tmc_core::StoredEvent {
                file_name: "exploration.log".into(),
                line: line as u64 + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: state.cmdr.as_ref().map(|c| c.name.clone()),
                json: json.into(),
            });
        }
        writer.flush_blocking().unwrap();
        let result = rebuild_from_history("CMDR TESTER".into())(&store).unwrap();
        let restored: UnsoldLedger = serde_json::from_value(result["ledger"].clone()).unwrap();
        assert_eq!(restored.total(), ledger.total());
        assert_eq!(restored.system_count(), ledger.system_count());
    }

    fn source(
        file: &str,
        line: u64,
        name: &str,
        extra: &str,
        replaying: bool,
    ) -> tmc_model::source::SourceEvent {
        tmc_model::source::SourceEvent::Journal {
            ev: parse_line(&format!(
                r#"{{"timestamp":"2026-10-18T10:00:00Z","event":"{name}"{extra}}}"#
            ))
            .unwrap(),
            file: file.into(),
            line,
            replaying,
        }
    }
    fn feed(
        module: &mut super::super::Exploration,
        state: &mut tmc_model::state::GameState,
        event: tmc_model::source::SourceEvent,
    ) -> Vec<(String, tmc_core::StoreTask)> {
        use tmc_core::Module;
        if let tmc_model::source::SourceEvent::Journal { ev, .. } = &event {
            state.apply_event(ev);
        }
        let mut ctx = tmc_core::ModuleCtx::new("exploration", false, 1);
        module.on_event(&event, state, &mut ctx);
        let (_, tasks, _) = ctx.take_outputs();
        tasks
    }
    fn topic(module: &super::super::Exploration) -> serde_json::Value {
        use tmc_core::Module;
        module.topic_value("exploration.unsold").unwrap()
    }
    fn store_event(store: &tmc_core::Store, event: &tmc_model::source::SourceEvent, cmdr: &str) {
        let tmc_model::source::SourceEvent::Journal { ev, file, line, .. } = event else {
            panic!()
        };
        let writer = tmc_core::EventWriter::spawn(store);
        writer.send(tmc_core::StoredEvent {
            file_name: file.clone(),
            line: *line,
            ts: ev.timestamp,
            name: ev.name.clone(),
            cmdr: Some(cmdr.into()),
            json: ev.raw.get().into(),
        });
        writer.flush_blocking().unwrap();
    }

    #[test]
    fn rebuilt_history_replays_only_uncovered_tail_and_rejects_other_commander_results() {
        use tmc_model::source::SourceEvent;
        let store = tmc_core::Store::open_in_memory().unwrap();
        let mut module = super::super::Exploration::new();
        let mut state = tmc_model::state::GameState::default();
        let load = source(
            "live.log",
            1,
            "LoadGame",
            r#", "Commander":"A","Odyssey":false"#,
            true,
        );
        let tasks = feed(&mut module, &mut state, load.clone());
        assert_eq!(tasks.len(), 1);
        let (tag, task) = tasks.into_iter().next().unwrap();
        let jump = source(
            "live.log",
            2,
            "FSDJump",
            r#", "StarSystem":"Sys","SystemAddress":1"#,
            true,
        );
        feed(&mut module, &mut state, jump.clone());
        let scan = source(
            "live.log",
            3,
            "Scan",
            r#", "BodyID":7,"PlanetClass":"Earthlike body","MassEM":1.0,"WasDiscovered":true,"WasMapped":true"#,
            true,
        );
        feed(&mut module, &mut state, scan.clone());
        for event in [&load, &jump, &scan] {
            store_event(&store, event, "A");
        }
        let response = task(&store).unwrap();
        let mapped = source(
            "live.log",
            4,
            "SAAScanComplete",
            r#", "BodyID":7,"ProbesUsed":6,"EfficiencyTarget":6"#,
            false,
        );
        feed(&mut module, &mut state, mapped.clone());
        let expected = topic(&module)["total"].clone();
        assert!(expected.as_u64().unwrap() > 0);
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag: tag.clone(),
                value: response.clone(),
            },
        );
        assert_eq!(topic(&module)["total"], expected);
        assert_eq!(topic(&module)["systems"], 1);
        assert_eq!(topic(&module)["at_risk"], true);
        // Ya aplicado en el snapshot o en la cola: ni Scan repetido ni DSS repetido suman de nuevo.
        feed(&mut module, &mut state, scan);
        feed(&mut module, &mut state, mapped);
        assert_eq!(topic(&module)["total"], expected);
        let next = feed(
            &mut module,
            &mut state,
            source("b.log", 1, "LoadGame", r#", "Commander":"B""#, false),
        );
        assert_eq!(next.len(), 1);
        assert_eq!(topic(&module)["total"], 0);
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag,
                value: response,
            },
        );
        assert_eq!(topic(&module)["total"], 0);
    }

    #[test]
    fn missing_store_result_retries_after_60_seconds_without_losing_live_balance() {
        use tmc_model::source::SourceEvent;
        let mut module = super::super::Exploration::new();
        let mut state = tmc_model::state::GameState::default();
        let first = feed(
            &mut module,
            &mut state,
            source("live", 1, "LoadGame", r#", "Commander":"A""#, false),
        );
        let (old_tag, old_task) = first.into_iter().next().unwrap();
        let broken = tmc_core::Store::open_in_memory().unwrap();
        broken.exec_json("DROP TABLE event", &[]).unwrap();
        assert!(old_task(&broken).is_err()); // El actor solo registra este error: no llega Stored.
        feed(
            &mut module,
            &mut state,
            source(
                "live",
                2,
                "Scan",
                r#", "BodyID":1,"StarSystem":"A","PlanetClass":"Earthlike body","MassEM":1.0"#,
                false,
            ),
        );
        let balance = topic(&module)["total"].clone();
        let status =
            tmc_model::status::Status::parse(r#"{"timestamp":"2026-10-18T10:01:01Z","Flags":0}"#)
                .unwrap();
        let retry = feed(&mut module, &mut state, SourceEvent::Status(status));
        assert_eq!(retry.len(), 1);
        let (new_tag, new_task) = retry.into_iter().next().unwrap();
        assert_ne!(old_tag, new_tag);
        let store = tmc_core::Store::open_in_memory().unwrap();
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag: old_tag,
                value: rebuild_from_history("A".into())(&store).unwrap(),
            },
        );
        assert_eq!(topic(&module)["total"], balance);
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag: new_tag,
                value: new_task(&store).unwrap(),
            },
        );
        assert_eq!(topic(&module)["total"], balance);
    }

    #[test]
    fn confirmed_checkpoints_bound_long_session_overlay() {
        use tmc_model::source::SourceEvent;
        let store = tmc_core::Store::open_in_memory().unwrap();
        let writer = tmc_core::EventWriter::spawn(&store);
        let mut module = super::super::Exploration::new();
        let mut state = tmc_model::state::GameState::default();
        let load = source("long", 1, "LoadGame", r#", "Commander":"A""#, false);
        store_event(&store, &load, "A");
        let (tag, task) = feed(&mut module, &mut state, load)
            .into_iter()
            .next()
            .unwrap();
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag,
                value: task(&store).unwrap(),
            },
        );
        let mut checkpoints = 0;
        for id in 1..=600 {
            let event = source(
                "long",
                id + 1,
                "Scan",
                &format!(
                    r#", "BodyID":{id},"StarSystem":"A","PlanetClass":"Earthlike body","MassEM":1.0"#
                ),
                false,
            );
            let SourceEvent::Journal { ev, file, line, .. } = &event else {
                unreachable!()
            };
            writer.send(tmc_core::StoredEvent {
                file_name: file.clone(),
                line: *line,
                ts: ev.timestamp,
                name: ev.name.clone(),
                cmdr: Some("A".into()),
                json: ev.raw.get().into(),
            });
            let tasks = feed(&mut module, &mut state, event);
            for (tag, task) in tasks {
                checkpoints += 1;
                writer.flush_blocking().unwrap();
                feed(
                    &mut module,
                    &mut state,
                    SourceEvent::Stored {
                        module: "exploration".into(),
                        tag,
                        value: task(&store).unwrap(),
                    },
                );
            }
        }
        assert!(checkpoints >= 2, "no periodic checkpoints");
        assert!(
            module.unsold.unconfirmed.len() < 256,
            "overlay grew for the whole session"
        );
        assert_eq!(topic(&module)["systems"], 1);
    }

    #[test]
    fn completed_import_refreshes_history_and_keeps_uncommitted_live_tail() {
        use tmc_model::source::SourceEvent;
        let store = tmc_core::Store::open_in_memory().unwrap();
        let mut module = super::super::Exploration::new();
        let mut state = tmc_model::state::GameState::default();
        let (tag, task) = feed(
            &mut module,
            &mut state,
            source("live", 1, "LoadGame", r#", "Commander":"A""#, false),
        )
        .into_iter()
        .next()
        .unwrap();
        let first_snapshot = task(&store).unwrap();
        feed(
            &mut module,
            &mut state,
            source(
                "live",
                2,
                "Scan",
                r#", "BodyID":7,"StarSystem":"Live","PlanetClass":"Earthlike body","MassEM":1.0"#,
                false,
            ),
        );
        let live = topic(&module)["total"].as_u64().unwrap();
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag: tag.clone(),
                value: first_snapshot.clone(),
            },
        );
        let old=SourceEvent::Journal {ev:parse_line(r#"{"timestamp":"2026-01-01T10:00:00Z","event":"Scan","BodyID":7,"StarSystem":"Old","PlanetClass":"Earthlike body","MassEM":1.0}"#).unwrap(),file:"old".into(),line:1,replaying:true};
        store_event(&store, &old, "A");
        let (new_tag, new_task) = feed(
            &mut module,
            &mut state,
            SourceEvent::ImportProgress { done: 1, total: 1 },
        )
        .into_iter()
        .next()
        .unwrap();
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag: new_tag,
                value: new_task(&store).unwrap(),
            },
        );
        assert_eq!(topic(&module)["total"].as_u64().unwrap(), 2 * live);
        assert_eq!(topic(&module)["systems"], 2);
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag,
                value: first_snapshot,
            },
        );
        assert_eq!(topic(&module)["total"].as_u64().unwrap(), 2 * live);
    }

    #[test]
    fn death_compacts_overlay_but_preserves_system_and_odyssey_for_reconstruction() {
        use tmc_model::source::SourceEvent;
        let mut module = super::super::Exploration::new();
        let mut state = tmc_model::state::GameState::default();
        let (tag, task) = feed(
            &mut module,
            &mut state,
            source(
                "live",
                1,
                "LoadGame",
                r#", "Commander":"A","Odyssey":false"#,
                false,
            ),
        )
        .into_iter()
        .next()
        .unwrap();
        feed(
            &mut module,
            &mut state,
            source("live", 2, "LoadGame", r#", "Commander":"A""#, false),
        );
        feed(
            &mut module,
            &mut state,
            source("live", 3, "FSDJump", r#", "StarSystem":"A""#, false),
        );
        for id in 1..=20 {
            feed(
                &mut module,
                &mut state,
                source(
                    "live",
                    id + 3,
                    "Scan",
                    &format!(r#", "BodyID":{id},"PlanetClass":"Earthlike body","MassEM":1.0"#),
                    false,
                ),
            );
        }
        feed(
            &mut module,
            &mut state,
            source("live", 24, "Died", "", false),
        );
        assert!(module.unsold.unconfirmed.len() <= 3);
        feed(
            &mut module,
            &mut state,
            source(
                "live",
                25,
                "Scan",
                r#", "BodyID":99,"PlanetClass":"Earthlike body","MassEM":1.0,"WasMapped":true,"WasDiscovered":true"#,
                false,
            ),
        );
        feed(
            &mut module,
            &mut state,
            source(
                "live",
                26,
                "SAAScanComplete",
                r#", "BodyID":99,"ProbesUsed":6,"EfficiencyTarget":6"#,
                false,
            ),
        );
        let expected = topic(&module);
        let store = tmc_core::Store::open_in_memory().unwrap();
        feed(
            &mut module,
            &mut state,
            SourceEvent::Stored {
                module: "exploration".into(),
                tag,
                value: task(&store).unwrap(),
            },
        );
        assert_eq!(topic(&module), expected);
        assert_eq!(topic(&module)["systems"], 1);
    }
}

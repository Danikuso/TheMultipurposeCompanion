//! Dónde conseguirlo: comerciante/bróker más cercano por tipo necesario y pistas de
//! adquisición (spec §3.4.6-7), e ingeniero recomendado para un blueprint y grado.
//!
//! Estado puro: la emisión de las peticiones de red la hace el módulo (Task 20).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use tmc_model::events::EngineerStatus;
use tmc_model::net::{BrokerKind, NearbyService, NetQuery, PadSize, RemoteStation, TraderKind};

use super::engineers::{EngineerBook, EngineerRow, NextStep, engineer_rows, next_step_for};
use super::trades::TradeProposal;
use crate::data::{Hint, HintKind, engineer, materials};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SourceStatus {
    Loading,
    Ok(RemoteStation),
    NoData,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourcesView {
    pub traders: BTreeMap<TraderKind, SourceStatus>,
    pub brokers: BTreeMap<BrokerKind, SourceStatus>,
    pub hints: BTreeMap<String, Vec<Hint>>,
}

#[derive(Debug, Clone, Default)]
pub struct SourcesState {
    current_system: Option<u64>,
    disabled: bool,
    traders: BTreeSet<TraderKind>,
    brokers: BTreeSet<BrokerKind>,
    hints: BTreeMap<String, Vec<Hint>>,
    /// `None` = el servicio respondió sin estación (`NoData`).
    trader_results: BTreeMap<TraderKind, Option<RemoteStation>>,
    broker_results: BTreeMap<BrokerKind, Option<RemoteStation>>,
}

fn broker_kind_of(key: &str) -> BrokerKind {
    match materials().get(key) {
        Some(m) if m.family.starts_with("guardian") => BrokerKind::Guardian,
        _ => BrokerKind::Human,
    }
}

fn status_of(disabled: bool, r: Option<&Option<RemoteStation>>) -> SourceStatus {
    if disabled {
        return SourceStatus::Disabled;
    }
    match r {
        None => SourceStatus::Loading,
        Some(None) => SourceStatus::NoData,
        Some(Some(s)) => SourceStatus::Ok(s.clone()),
    }
}

impl SourcesState {
    pub fn current_system(&self) -> Option<u64> {
        self.current_system
    }

    /// Un sistema distinto descarta los resultados (son relativos al sistema).
    pub fn set_current_system(&mut self, addr: Option<u64>) {
        if self.current_system != addr {
            self.current_system = addr;
            self.trader_results.clear();
            self.broker_results.clear();
        }
    }

    /// Red desactivada (modo Legacy, repetición o sin conexión): nada se pide.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Tipos de comerciante necesarios: los de los materiales que faltan y los de
    /// los intercambios propuestos.
    pub fn needed_traders(
        missing: &BTreeMap<String, u32>,
        proposals: &[TradeProposal],
    ) -> BTreeSet<TraderKind> {
        let mut set: BTreeSet<TraderKind> = missing
            .iter()
            .filter(|(_, n)| **n > 0)
            .filter_map(|(k, _)| materials().get(k))
            .map(|m| TraderKind::from(m.category))
            .collect();
        set.extend(proposals.iter().map(|p| p.trader));
        set
    }

    /// Brókers necesarios: solo si algún faltante tiene la pista `Broker`.
    pub fn needed_brokers(missing: &BTreeMap<String, u32>) -> BTreeSet<BrokerKind> {
        missing
            .iter()
            .filter(|(_, n)| **n > 0)
            .filter_map(|(k, _)| materials().get(k))
            .filter(|m| m.hints.iter().any(|h| h.kind == HintKind::Broker))
            .map(|m| broker_kind_of(&m.key))
            .collect()
    }

    /// Actualiza lo necesario; descarta resultados de tipos que ya no hacen falta.
    pub fn set_needs(&mut self, missing: &BTreeMap<String, u32>, proposals: &[TradeProposal]) {
        self.traders = Self::needed_traders(missing, proposals);
        self.brokers = Self::needed_brokers(missing);
        self.hints = missing
            .iter()
            .filter(|(_, n)| **n > 0)
            .filter_map(|(k, _)| {
                let h = &materials().get(k)?.hints;
                (!h.is_empty()).then(|| (k.clone(), h.clone()))
            })
            .collect();
        self.trader_results.retain(|k, _| self.traders.contains(k));
        self.broker_results.retain(|k, _| self.brokers.contains(k));
    }

    /// Peticiones por enviar (tipos necesarios aún sin resultado). Sin sistema actual
    /// o con nombre vacío no se genera nada: una petición tipada con nombre vacío
    /// cachearía `NoData` durante una hora.
    pub fn requests(&self, system_name: &str, min_pad: PadSize) -> Vec<NetQuery> {
        let name = system_name.trim();
        let Some(system_address) = self.current_system else {
            return Vec::new();
        };
        if name.is_empty() || self.disabled {
            return Vec::new();
        }
        let query = |service| NetQuery::NearestService {
            system_address,
            system_name: name.to_string(),
            service,
            min_pad,
        };
        let traders = self
            .traders
            .iter()
            .filter(|k| !self.trader_results.contains_key(k))
            .map(|k| query(NearbyService::MaterialTrader(Some(*k))));
        let brokers = self
            .brokers
            .iter()
            .filter(|k| !self.broker_results.contains_key(k))
            .map(|k| query(NearbyService::TechnologyBroker(Some(*k))));
        traders.chain(brokers).collect()
    }

    /// Guarda el resultado de una búsqueda. `false` y sin cambios si es de otro
    /// sistema o el servicio no es tipado.
    pub fn on_stations(
        &mut self,
        system_address: u64,
        service: NearbyService,
        rows: Vec<RemoteStation>,
    ) -> bool {
        if self.current_system != Some(system_address) {
            return false;
        }
        let best = rows
            .into_iter()
            .filter(|r| r.distance_ly.is_finite())
            .min_by(|a, b| a.distance_ly.total_cmp(&b.distance_ly));
        match service {
            NearbyService::MaterialTrader(Some(k)) => {
                self.trader_results.insert(k, best);
                true
            }
            NearbyService::TechnologyBroker(Some(k)) => {
                self.broker_results.insert(k, best);
                true
            }
            _ => false,
        }
    }

    pub fn view(&self) -> SourcesView {
        SourcesView {
            traders: self
                .traders
                .iter()
                .map(|k| (*k, status_of(self.disabled, self.trader_results.get(k))))
                .collect(),
            brokers: self
                .brokers
                .iter()
                .map(|k| (*k, status_of(self.disabled, self.broker_results.get(k))))
                .collect(),
            hints: self.hints.clone(),
        }
    }
}

/// Ingenieros recomendados para un blueprint hasta un grado.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EngineerPick {
    pub nearest_unlocked: Option<EngineerRow>,
    pub nearest_any: Option<EngineerRow>,
    /// Paso del más cercano si no está desbloqueado (`have` = 0: no conoce el inventario).
    pub next_step: Option<NextStep>,
}

/// Ingenieros que aplican `blueprint` hasta `grade` (como mínimo), ordenados como
/// `engineer_rows`; los vetados (`Barred`) no se recomiendan.
pub fn pick_engineer(
    blueprint: &str,
    grade: u8,
    book: &EngineerBook,
    pos: Option<[f64; 3]>,
) -> EngineerPick {
    let capable: Vec<EngineerRow> = engineer_rows(book, pos)
        .into_iter()
        .filter(|r| r.status != EngineerStatus::Barred)
        .filter(|r| {
            engineer(r.id).is_some_and(|d| {
                d.blueprints
                    .iter()
                    .any(|b| b.blueprint == blueprint && b.max_grade >= grade)
            })
        })
        .collect();
    let nearest_unlocked = capable
        .iter()
        .find(|r| r.status == EngineerStatus::Unlocked)
        .cloned();
    let nearest_any = capable.first().cloned();
    let next_step = nearest_any
        .as_ref()
        .filter(|r| r.status != EngineerStatus::Unlocked)
        .and_then(|r| next_step_for(book, r.id, &|_| 0, &|_| 0));
    EngineerPick {
        nearest_unlocked,
        nearest_any,
        next_step,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::events::{EventKind, MaterialCategory, parse_line};

    fn key_of_category(cat: MaterialCategory) -> String {
        let mut keys: Vec<&String> = materials()
            .values()
            .filter(|m| m.category == cat && m.hints.iter().all(|h| h.kind != HintKind::Broker))
            .map(|m| &m.key)
            .collect();
        keys.sort();
        keys[0].clone()
    }

    fn station(name: &str, d: f64) -> RemoteStation {
        RemoteStation {
            station_name: name.into(),
            system_name: "Sol".into(),
            distance_ly: d,
            distance_to_arrival_ls: None,
            max_pad: None,
            updated_at: None,
        }
    }

    #[test]
    fn traders_requested_only_for_needed_kinds() {
        let missing = BTreeMap::from([(key_of_category(MaterialCategory::Encoded), 2)]);
        assert_eq!(
            SourcesState::needed_traders(&missing, &[]),
            BTreeSet::from([TraderKind::Encoded])
        );
        let mut s = SourcesState::default();
        s.set_current_system(Some(1));
        s.set_needs(&missing, &[]);
        assert_eq!(
            s.requests("Sol", PadSize::Large),
            vec![NetQuery::NearestService {
                system_address: 1,
                system_name: "Sol".into(),
                service: NearbyService::MaterialTrader(Some(TraderKind::Encoded)),
                min_pad: PadSize::Large,
            }]
        );
        assert!(s.view().brokers.is_empty());
    }

    #[test]
    fn broker_only_when_missing_material_has_broker_hint() {
        let key = materials()
            .values()
            .find(|m| m.hints.iter().any(|h| h.kind == HintKind::Broker))
            .unwrap()
            .key
            .clone();
        let mut s = SourcesState::default();
        s.set_current_system(Some(1));
        s.set_needs(&BTreeMap::from([(key.clone(), 1)]), &[]);
        assert!(s.requests("Sol", PadSize::Small).iter().any(|q| matches!(
            q,
            NetQuery::NearestService {
                service: NearbyService::TechnologyBroker(Some(BrokerKind::Human)),
                ..
            }
        )));
        assert!(s.view().hints.contains_key(&key));
    }

    #[test]
    fn no_typed_request_without_system_name_or_address() {
        let missing = BTreeMap::from([(key_of_category(MaterialCategory::Raw), 1)]);
        let mut s = SourcesState::default();
        s.set_needs(&missing, &[]);
        assert!(s.requests("Sol", PadSize::Large).is_empty());
        s.set_current_system(Some(1));
        assert!(s.requests("", PadSize::Large).is_empty());
        assert!(s.requests("   ", PadSize::Large).is_empty());
        assert_eq!(s.requests("Sol", PadSize::Large).len(), 1);
        s.set_disabled(true);
        assert!(s.requests("Sol", PadSize::Large).is_empty());
    }

    #[test]
    fn late_station_result_for_previous_system_rejected() {
        let mut s = SourcesState::default();
        s.set_current_system(Some(2));
        s.set_needs(
            &BTreeMap::from([(key_of_category(MaterialCategory::Raw), 1)]),
            &[],
        );
        let svc = NearbyService::MaterialTrader(Some(TraderKind::Raw));
        let before = s.view();
        assert!(!s.on_stations(1, svc, vec![station("Old", 5.0)]));
        assert_eq!(s.view(), before);
        assert_eq!(before.traders[&TraderKind::Raw], SourceStatus::Loading);
        assert!(s.on_stations(2, svc, vec![station("Far", 9.0), station("Near", 3.0)]));
        assert_eq!(
            s.view().traders[&TraderKind::Raw],
            SourceStatus::Ok(station("Near", 3.0))
        );
        s.set_current_system(Some(3));
        assert_eq!(s.view().traders[&TraderKind::Raw], SourceStatus::Loading);
        s.on_stations(3, svc, vec![]);
        assert_eq!(s.view().traders[&TraderKind::Raw], SourceStatus::NoData);
    }

    pub(crate) fn book_with(entries: &[(u64, &str, &str)]) -> EngineerBook {
        let list: Vec<String> = entries
            .iter()
            .map(|(id, name, st)| {
                format!(r#"{{"Engineer":"{name}","EngineerID":{id},"Progress":"{st}"}}"#)
            })
            .collect();
        let line = format!(
            r#"{{"timestamp":"2026-10-21T12:00:00Z","event":"EngineerProgress","Engineers":[{}]}}"#,
            list.join(",")
        );
        let mut book = EngineerBook::default();
        let kind: EventKind = parse_line(&line).unwrap().kind;
        book.apply_event(&kind);
        book
    }

    #[test]
    fn pick_prefers_unlocked_and_reports_next_step_for_nearest_locked() {
        // Felicity (300100) desbloqueada y lejos; Elvira (300160) bloqueada y cerca.
        let book = book_with(&[
            (300100, "Felicity Farseer", "Unlocked"),
            (300160, "Elvira Martuuk", "Known"),
        ]);
        let pos = Some(engineer(300160).unwrap().star_pos);
        let pick = pick_engineer("fsd_longrange", 5, &book, pos);
        assert_eq!(pick.nearest_unlocked.unwrap().id, 300100);
        assert_eq!(pick.nearest_any.unwrap().id, 300160);
        assert!(pick.next_step.is_some());
    }

    #[test]
    fn pick_respects_max_grade() {
        let book = book_with(&[
            (300100, "Felicity Farseer", "Unlocked"),
            (300220, "Professor Palin", "Known"),
        ]);
        // Felicity solo llega a G3 en engine_dirty.
        let pick = pick_engineer("engine_dirty", 5, &book, None);
        assert_eq!(pick.nearest_unlocked, None);
        assert_eq!(pick.nearest_any.unwrap().id, 300220);
        let pick3 = pick_engineer("engine_dirty", 3, &book, None);
        assert_eq!(pick3.nearest_unlocked.unwrap().id, 300100);
        assert_eq!(pick_engineer("nope", 1, &book, None).nearest_any, None);
    }
}

//! Módulo `exobiology`: candidatos por cuerpo, muestreo en curso y valor sin
//! vender (con el bonus ×5 de primer registro estimado).
//!
//! Orden de arranque (ver `sampling.rs`): al conocerse el comandante se
//! encargan `load_open_samples`, `load_history` y `load_unsold`; cada resultado
//! se aplica al llegar como `Stored` (con comprobación de comandante) y es
//! idempotente respecto al replay del diario.

pub mod geo;
pub mod persist;
pub mod predict;
pub mod sampling;
pub mod species;

use crate::data::{bio_rules, species_table};
use crate::exploration::system::kind_key;
use chrono::{DateTime, Utc};
use geo::RelDir;
use predict::{Candidate, HistoryStats, Likelihood, body_profile, is_primary_star};
use sampling::{ExoSampleRow, SamplingChange, SamplingTracker};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::events::{EventKind, GenusRef, JournalEvent, Scan, ScanOrganic, SellOrganicData};
use tmc_model::net::{NetOutcome, NetQuery, NetResponse, RemoteBodyBio};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{FlightMode, Galaxy, GameState};

const TOPIC_BODY: &str = "exobiology.body";
const TOPIC_SAMPLING: &str = "exobiology.sampling";
const TOPIC_UNSOLD: &str = "exobiology.unsold";
const TAG_OPEN: &str = "load_open_samples";
const TAG_HISTORY: &str = "load_history";
const TAG_UNSOLD: &str = "load_unsold";
/// Distancia mínima entre muestras cuando el género no está en la tabla (el
/// CCR más frecuente del juego es 100 m); documentado como suposición.
pub const DEFAULT_CCR_M: u32 = 100;
const STANDARD_GRAVITY: f64 = 9.806_65;
/// Un `bio:` sin respuesta pasado este tiempo se da por perdido.
const INFLIGHT_TTL_S: i64 = 60;
/// Un `Failed` se reintenta pasado este tiempo.
const FAILED_COOLDOWN_S: i64 = 300;

type BodyKey = (u64, u32);

/// Estimación del bonus de primer registro (spec §5.4): `WasFootfalled ==
/// false` -> `true`; sin ese campo, un sistema no descubierto; sin datos,
/// `None`. Se confirma con `SellOrganicData.Bonus > 0`.
pub fn first_logged_estimate(scan: Option<&Scan>, system_discovered: Option<bool>) -> Option<bool> {
    match scan.and_then(|s| s.was_footfalled) {
        Some(footfalled) => Some(!footfalled),
        None => system_discovered.map(|discovered| !discovered),
    }
}

fn bonus_multiplier(first_logged: Option<bool>) -> u64 {
    if first_logged == Some(true) { 5 } else { 1 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CompletedInfo {
    value: u64,
    first_logged: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NetState {
    Done,
    Disabled,
    Failed(DateTime<Utc>),
}

#[derive(Debug, Serialize)]
struct CandidateView {
    species: String,
    species_name: Option<String>,
    genus: String,
    genus_name: Option<String>,
    value: u64,
    value_x5: u64,
    min_m: u32,
    likelihood: Likelihood,
    sources: Vec<predict::ProbSource>,
    /// `none` | `in-progress` | `done`.
    status: &'static str,
    /// Muestras tomadas (1..=3) cuando `status == "in-progress"`.
    progress: Option<u8>,
}

#[derive(Debug, Serialize)]
struct GenusView {
    genus: String,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct DirectionView {
    rel: RelDir,
    compass: &'static str,
}

fn key_string(key: BodyKey) -> String {
    format!("bio:{}:{}", key.0, key.1)
}

fn parse_key(text: &str) -> Option<BodyKey> {
    let mut parts = text.strip_prefix("bio:")?.split(':');
    let key = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    parts.next().is_none().then_some(key)
}

fn landed_like(state: &GameState) -> bool {
    matches!(
        state.location.mode,
        FlightMode::Landed | FlightMode::OnFoot | FlightMode::InSrv
    )
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

pub struct Exobiology {
    settings: ModuleSettings,
    cmdr: Option<String>,
    tracker: SamplingTracker,
    history: HistoryStats,
    infos: HashMap<(u64, u32, String), CompletedInfo>,
    system: Option<u64>,
    system_discovered: Option<bool>,
    star_class: Option<String>,
    scans: HashMap<BodyKey, Scan>,
    bio_counts: HashMap<BodyKey, u32>,
    genera: HashMap<BodyKey, Vec<GenusRef>>,
    organic_bodies: HashSet<BodyKey>,
    /// Cuerpo en el que está el jugador (aproximación, aterrizaje, desembarco).
    current: Option<BodyKey>,
    current_name: Option<String>,
    /// Último cuerpo con señales biológicas.
    signals_body: Option<BodyKey>,
    /// Nombres `_Localised` vistos en el diario por símbolo de códice.
    names: HashMap<String, String>,
    canonn: HashMap<BodyKey, RemoteBodyBio>,
    net: HashMap<BodyKey, NetState>,
    inflight: HashMap<String, (u64, DateTime<Utc>)>,
    now: Option<DateTime<Utc>>,
    discarded: Option<String>,
    /// Instante de la última venta o muerte (descarta filas `load_unsold` viejas).
    last_reset: Option<DateTime<Utc>>,
    /// (aciertos, total) de la estimación ×5 frente a `SellOrganicData.Bonus`.
    accuracy: (u32, u32),
    body_value: Value,
    sampling_value: Value,
    unsold_value: Value,
}

impl Exobiology {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            cmdr: None,
            tracker: SamplingTracker::default(),
            history: HistoryStats::default(),
            infos: HashMap::new(),
            system: None,
            system_discovered: None,
            star_class: None,
            scans: HashMap::new(),
            bio_counts: HashMap::new(),
            genera: HashMap::new(),
            organic_bodies: HashSet::new(),
            current: None,
            current_name: None,
            signals_body: None,
            names: HashMap::new(),
            canonn: HashMap::new(),
            net: HashMap::new(),
            inflight: HashMap::new(),
            now: None,
            discarded: None,
            last_reset: None,
            accuracy: (0, 0),
            body_value: Value::Null,
            sampling_value: Value::Null,
            unsold_value: json!({"total": 0, "species": 0}),
        }
    }

    /// (aciertos, total) de la estimación del bonus frente a las ventas vistas.
    pub fn estimate_accuracy(&self) -> (u32, u32) {
        self.accuracy
    }

    fn tick(&mut self, at: DateTime<Utc>) {
        if self.now.is_none_or(|n| at > n) {
            self.now = Some(at);
        }
    }

    fn focus(&self) -> Option<BodyKey> {
        self.current.or(self.signals_body)
    }

    fn targets(&self) -> Vec<BodyKey> {
        let mut out: Vec<BodyKey> = self.current.into_iter().collect();
        if let Some(key) = self.signals_body
            && !out.contains(&key)
        {
            out.push(key);
        }
        out
    }

    fn enter_system(&mut self, address: u64) {
        if address == 0 || self.system == Some(address) {
            return;
        }
        self.system = Some(address);
        self.system_discovered = None;
        self.star_class = None;
        self.scans.clear();
        self.bio_counts.clear();
        self.genera.clear();
        self.canonn.clear();
        self.net.clear();
        self.inflight.clear();
        self.current = None;
        self.current_name = None;
        self.signals_body = None;
    }

    /// Un cambio de comandante descarta lo personal y vuelve a pedir su
    /// historial al almacén.
    fn check_cmdr(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let Some(name) = state
            .cmdr
            .as_ref()
            .map(|c| c.name.clone())
            .filter(|n| !n.is_empty())
        else {
            return;
        };
        if self.cmdr.as_deref() == Some(name.as_str()) {
            return;
        }
        self.cmdr = Some(name.clone());
        self.tracker = SamplingTracker::default();
        self.history = HistoryStats::default();
        self.infos.clear();
        self.organic_bodies.clear();
        self.discarded = None;
        self.last_reset = None;
        self.accuracy = (0, 0);
        ctx.store_task(TAG_OPEN, persist::load_open_samples(name.clone()));
        ctx.store_task(TAG_HISTORY, predict::load_history(name.clone()));
        ctx.store_task(TAG_UNSOLD, persist::load_unsold(name));
    }

    fn set_current(
        &mut self,
        address: Option<u64>,
        body_id: Option<u32>,
        name: Option<&String>,
        state: &GameState,
    ) {
        let Some(body_id) = body_id else { return };
        let Some(address) = address
            .filter(|a| *a != 0)
            .or(Some(state.location.system_address).filter(|a| *a != 0))
        else {
            return;
        };
        self.enter_system(address);
        self.current = Some((address, body_id));
        if let Some(name) = name {
            self.current_name = Some(name.clone());
        }
    }

    fn on_scan(&mut self, scan: &Scan, state: &GameState) {
        let Some(address) = scan
            .system_address
            .or(Some(state.location.system_address))
            .filter(|a| *a != 0)
        else {
            return;
        };
        self.enter_system(address);
        if is_primary_star(scan) {
            self.star_class.clone_from(&scan.star_type);
            self.system_discovered = scan.was_discovered;
        }
        if let Some(body_id) = scan.body_id {
            self.scans.insert((address, body_id), scan.clone());
        }
    }

    fn on_signals(
        &mut self,
        address: Option<u64>,
        body_id: Option<u32>,
        signals: &[tmc_model::events::Signal],
        genera: Option<&[GenusRef]>,
        state: &GameState,
    ) {
        let (Some(body_id), Some(address)) = (
            body_id,
            address
                .filter(|a| *a != 0)
                .or(Some(state.location.system_address).filter(|a| *a != 0)),
        ) else {
            return;
        };
        self.enter_system(address);
        let count: u32 = signals
            .iter()
            .filter(|s| s.is_biological())
            .map(|s| s.count)
            .sum();
        let key = (address, body_id);
        self.bio_counts.insert(key, count);
        if let Some(genera) = genera.filter(|g| !g.is_empty()) {
            for genus in genera {
                if let Some(name) = &genus.genus_localised {
                    self.names.insert(genus.genus.clone(), name.clone());
                }
            }
            self.genera.insert(key, genera.to_vec());
        }
        if count > 0 {
            self.signals_body = Some(key);
        }
    }

    fn on_organic(
        &mut self,
        ev: &ScanOrganic,
        at: DateTime<Utc>,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) {
        if let (Some(symbol), Some(name)) = (&ev.species, &ev.species_localised) {
            self.names.insert(symbol.clone(), name.clone());
        }
        if let (Some(symbol), Some(name)) = (&ev.genus, &ev.genus_localised) {
            self.names.insert(symbol.clone(), name.clone());
        }
        let (Some(address), Some(body), Some(species)) =
            (ev.system_address, ev.body, ev.species.as_deref())
        else {
            return;
        };
        let key = (address, body);
        self.organic_bodies.insert(key);
        if self.current.is_none() {
            self.set_current(Some(address), Some(body), None, state);
        }
        let pos = state
            .status
            .as_ref()
            .and_then(|s| Some((s.latitude?, s.longitude?)));
        let cmdr = self.cmdr.clone().unwrap_or_default();
        match self.tracker.handle_scan(ev, &cmdr, at, pos, ctx) {
            SamplingChange::Discarded { previous_species } => {
                self.discarded = Some(previous_species);
            }
            SamplingChange::Started => self.discarded = None,
            SamplingChange::Completed => {
                self.discarded = None;
                let info = CompletedInfo {
                    value: species_table().species(species).map_or(0, |s| s.value),
                    first_logged: first_logged_estimate(
                        self.scans.get(&key),
                        self.system_discovered,
                    ),
                };
                self.infos.insert((address, body, species.to_owned()), info);
            }
            SamplingChange::Progressed(_) | SamplingChange::Ignored => {}
        }
    }

    fn on_sold(&mut self, sale: &SellOrganicData, at: DateTime<Utc>, ctx: &mut ModuleCtx) {
        for item in &sale.bio_data {
            let estimate = item.species.as_ref().and_then(|species| {
                self.infos
                    .iter()
                    .find(|((_, _, s), _)| s == species)
                    .and_then(|(_, info)| info.first_logged)
            });
            if let Some(estimate) = estimate {
                self.accuracy.1 += 1;
                self.accuracy.0 += u32::from(estimate == (item.bonus > 0));
            }
        }
        if self.accuracy.1 > 0 {
            tracing::info!(
                hits = self.accuracy.0,
                total = self.accuracy.1,
                "first-logged estimate accuracy"
            );
        }
        self.infos.clear();
        self.last_reset = Some(at);
        let cmdr = self.cmdr.clone().unwrap_or_default();
        self.tracker.handle_sold(&cmdr, ctx);
    }

    fn on_died(&mut self, at: DateTime<Utc>, ctx: &mut ModuleCtx) {
        self.infos.clear();
        self.discarded = None;
        self.last_reset = Some(at);
        let cmdr = self.cmdr.clone().unwrap_or_default();
        self.tracker.handle_died(&cmdr, ctx);
    }

    fn on_journal(&mut self, je: &JournalEvent, state: &GameState, ctx: &mut ModuleCtx) {
        self.check_cmdr(state, ctx);
        match &je.kind {
            EventKind::FsdJump(l) | EventKind::CarrierJump(l) | EventKind::Location(l) => {
                self.enter_system(l.system_address);
            }
            EventKind::Scan(s) => self.on_scan(s, state),
            EventKind::FssBodySignals(f) => {
                self.on_signals(f.system_address, f.body_id, &f.signals, None, state);
            }
            EventKind::SaaSignalsFound(f) => self.on_signals(
                f.system_address,
                f.body_id,
                &f.signals,
                Some(&f.genuses),
                state,
            ),
            EventKind::ApproachBody(b) => {
                self.set_current(b.system_address, b.body_id, b.body.as_ref(), state);
            }
            EventKind::Touchdown(t) | EventKind::Liftoff(t) => {
                self.set_current(t.system_address, t.body_id, t.body.as_ref(), state);
            }
            EventKind::Disembark(e) | EventKind::Embark(e) if e.on_planet => {
                self.set_current(e.system_address, e.body_id, e.body.as_ref(), state);
            }
            EventKind::LeaveBody(_) => {
                if self.current == self.signals_body {
                    self.signals_body = None;
                }
                self.current = None;
                self.current_name = None;
            }
            EventKind::ScanOrganic(ev) => self.on_organic(ev, je.timestamp, state, ctx),
            EventKind::SellOrganicData(sale) => self.on_sold(sale, je.timestamp, ctx),
            EventKind::Died => self.on_died(je.timestamp, ctx),
            _ => {}
        }
    }

    fn on_stored(&mut self, tag: &str, value: &Value) {
        if value.get("cmdr").and_then(Value::as_str) != self.cmdr.as_deref() {
            return;
        }
        match tag {
            TAG_OPEN => {
                let rows: Vec<ExoSampleRow> =
                    serde_json::from_value(value["rows"].clone()).unwrap_or_default();
                self.tracker.restore(rows);
                if let Some(s) = self.tracker.sampling().filter(|s| !s.complete) {
                    let key = (s.system_address, s.body_id);
                    self.organic_bodies.insert(key);
                    if self.system.is_none() {
                        self.system = Some(key.0);
                    }
                    if self.current.is_none() && self.system == Some(key.0) {
                        self.current = Some(key);
                    }
                }
            }
            TAG_HISTORY => self.history = HistoryStats::from_task_value(value),
            TAG_UNSOLD => self.apply_unsold_rows(value),
            _ => {}
        }
    }

    fn apply_unsold_rows(&mut self, value: &Value) {
        for row in value["rows"].as_array().into_iter().flatten() {
            let (Some(address), Some(body), Some(species)) = (
                row.get(0).and_then(Value::as_u64),
                row.get(1)
                    .and_then(Value::as_u64)
                    .and_then(|b| u32::try_from(b).ok()),
                row.get(2).and_then(Value::as_str),
            ) else {
                continue;
            };
            let at = row
                .get(3)
                .and_then(Value::as_str)
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|t| t.to_utc());
            // Una venta o muerte ya vista en esta sesión invalida filas más viejas.
            if self
                .last_reset
                .is_some_and(|reset| at.is_none_or(|t| t <= reset))
            {
                continue;
            }
            self.tracker.mark_completed(address, body, species);
            let scan: Option<Scan> = row
                .get(4)
                .and_then(Value::as_str)
                .and_then(|json| serde_json::from_str(json).ok());
            self.infos
                .entry((address, body, species.to_owned()))
                .or_insert_with(|| CompletedInfo {
                    value: species_table().species(species).map_or(0, |s| s.value),
                    first_logged: first_logged_estimate(scan.as_ref(), None),
                });
        }
    }

    fn apply_response(&mut self, r: &NetResponse) {
        if r.module != "exobiology" || !r.key.starts_with("bio:") {
            return;
        }
        self.inflight.remove(&r.key);
        let Some(key) = parse_key(&r.key).filter(|k| self.targets().contains(k)) else {
            return;
        };
        match &r.outcome {
            NetOutcome::BodyBio(bio) => {
                self.canonn.insert(key, bio.clone());
                self.net.insert(key, NetState::Done);
            }
            NetOutcome::NoData => {
                self.net.insert(key, NetState::Done);
            }
            NetOutcome::Disabled => {
                self.net.insert(key, NetState::Disabled);
            }
            NetOutcome::Failed(_) => {
                self.net.insert(key, NetState::Failed(r.fetched_at));
            }
            _ => {}
        }
    }

    fn online(&self, state: &GameState) -> bool {
        self.settings.online_services && state.galaxy != Galaxy::Legacy
    }

    /// Pide a Canonn los cuerpos con señales biológicas (o aquel en el que se
    /// ha aterrizado) que aún no tienen respuesta. Las peticiones del replay
    /// se descartan y se repiten con el primer evento en vivo.
    fn request_canonn(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let targets = self.targets();
        self.canonn.retain(|k, _| targets.contains(k));
        self.net.retain(|k, _| targets.contains(k));
        if let Some(now) = self.now {
            self.net.retain(|_, s| match s {
                NetState::Failed(at) => (now - *at).num_seconds() < FAILED_COOLDOWN_S,
                _ => true,
            });
            self.inflight
                .retain(|_, (_, sent)| (now - *sent).num_seconds() <= INFLIGHT_TTL_S);
        }
        if !self.online(state) {
            return;
        }
        for key in targets {
            let wanted = self.bio_counts.get(&key).is_some_and(|n| *n > 0)
                || (self.current == Some(key) && landed_like(state));
            let text = key_string(key);
            if !wanted || self.net.contains_key(&key) || self.inflight.contains_key(&text) {
                continue;
            }
            let Some(scan) = self.scans.get(&key) else {
                continue;
            };
            let query = NetQuery::BodyBio {
                system_address: key.0,
                body_id: key.1,
                body_name: scan.body_name.clone(),
                profile: body_profile(scan, self.star_class.as_deref()),
            };
            if let Some(id) = ctx.request(text.clone(), query) {
                self.inflight
                    .insert(text, (id, self.now.unwrap_or(DateTime::UNIX_EPOCH)));
            }
        }
    }

    fn genus_name(&self, symbol: &str) -> Option<String> {
        self.names
            .get(symbol)
            .cloned()
            .or_else(|| species_table().genus(symbol).map(|g| g.name.clone()))
    }

    fn species_name(&self, symbol: &str) -> Option<String> {
        if symbol.is_empty() {
            return None;
        }
        self.names
            .get(symbol)
            .cloned()
            .or_else(|| species_table().species(symbol).map(|s| s.name.clone()))
    }

    fn genus_of(&self, species: &str) -> String {
        species_table()
            .species(species)
            .map(|s| s.genus.clone())
            .unwrap_or_default()
    }

    /// Especies exactas ya observadas en el cuerpo (completadas o en curso).
    fn exact_species(&self, key: BodyKey) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .tracker
            .completed()
            .iter()
            .filter(|(a, b, _)| (*a, *b) == key)
            .map(|(_, _, sp)| (self.genus_of(sp), sp.clone()))
            .collect();
        if let Some(s) = self
            .tracker
            .sampling()
            .filter(|s| (s.system_address, s.body_id) == key)
            && !out.iter().any(|(_, sp)| *sp == s.species)
        {
            let genus = Some(self.genus_of(&s.species))
                .filter(|g| !g.is_empty())
                .unwrap_or_else(|| s.genus.clone());
            out.push((genus, s.species.clone()));
        }
        out
    }

    fn candidates(&self, key: BodyKey) -> Vec<Candidate> {
        let Some(scan) = self.scans.get(&key) else {
            return Vec::new();
        };
        let profile = body_profile(scan, self.star_class.as_deref());
        let confirmed: Option<Vec<String>> = self
            .genera
            .get(&key)
            .map(|g| g.iter().map(|r| r.genus.clone()).collect());
        let table = species_table();
        let mut candidates = predict::predict(
            &profile,
            confirmed.as_deref(),
            &self.history,
            self.canonn.get(&key),
            bio_rules(),
            table,
        );
        for (genus, species) in self.exact_species(key) {
            candidates.retain(|c| c.genus != genus || c.species == species);
            if !candidates.iter().any(|c| c.species == species) {
                candidates.push(Candidate {
                    value: table.species(&species).map_or(0, |s| s.value),
                    species,
                    genus,
                    likelihood: Likelihood::High,
                    sources: Vec::new(),
                });
            }
        }
        candidates.sort_by_key(|c| std::cmp::Reverse(c.value));
        candidates
    }

    fn render_candidate(&self, c: Candidate, key: BodyKey) -> CandidateView {
        let done = self
            .tracker
            .completed()
            .iter()
            .any(|(a, b, sp)| (*a, *b) == key && *sp == c.species);
        let progress = self
            .tracker
            .sampling()
            .filter(|s| {
                !s.complete
                    && (s.system_address, s.body_id) == key
                    && !c.species.is_empty()
                    && s.species == c.species
            })
            .map(|s| s.scans.min(3));
        CandidateView {
            species_name: self.species_name(&c.species),
            genus_name: self.genus_name(&c.genus),
            value_x5: c.value.saturating_mul(5),
            min_m: species_table()
                .genus(&c.genus)
                .map_or(DEFAULT_CCR_M, |g| g.ccr_m),
            status: if done {
                "done"
            } else if progress.is_some() {
                "in-progress"
            } else {
                "none"
            },
            progress,
            species: c.species,
            genus: c.genus,
            value: c.value,
            likelihood: c.likelihood,
            sources: c.sources,
        }
    }

    fn render_body(&self) -> Value {
        let Some(key) = self.focus() else {
            return Value::Null;
        };
        let scan = self.scans.get(&key);
        let candidates: Vec<CandidateView> = self
            .candidates(key)
            .into_iter()
            .map(|c| self.render_candidate(c, key))
            .collect();
        let genera = self.genera.get(&key);
        let confirmed: Vec<GenusView> = genera
            .into_iter()
            .flatten()
            .map(|g| GenusView {
                genus: g.genus.clone(),
                name: self.genus_name(&g.genus),
            })
            .collect();
        let completed = self
            .tracker
            .completed()
            .iter()
            .filter(|(a, b, _)| (*a, *b) == key)
            .count();
        let name = scan
            .map(|s| s.body_name.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| self.current_name.clone());
        let atmosphere = scan.and_then(|s| {
            [s.atmosphere_type.as_ref(), s.atmosphere.as_ref()]
                .into_iter()
                .flatten()
                .find(|a| !a.trim().is_empty())
                .cloned()
        });
        json!({
            "body": name,
            "body_id": key.1,
            "system_address": key.0,
            "kind": scan.and_then(|s| s.planet_class.clone()),
            "kind_key": scan.filter(|s| s.planet_class.as_deref().is_some_and(|c| !c.is_empty())).map(kind_key),
            "atmosphere_key": scan.map(|s| atmosphere_key(s.atmosphere_type.as_deref())),
            "gravity_g": scan.and_then(|s| s.surface_gravity).map(|g| (g / STANDARD_GRAVITY * 100.0).round() / 100.0),
            "temperature_k": scan.and_then(|s| s.surface_temperature).map(round1),
            "atmosphere": atmosphere,
            "bio_signals": self.bio_counts.get(&key).copied().unwrap_or(0),
            "first_logged": first_logged_estimate(scan, self.system_discovered),
            "dss": genera.is_some(),
            "candidates": candidates,
            "confirmed_genera": confirmed,
            "completed": completed,
        })
    }

    fn completed_value(&self, key: &(u64, u32, String)) -> u64 {
        let info = self.infos.get(key);
        let value = info.map_or_else(
            || species_table().species(&key.2).map_or(0, |s| s.value),
            |i| i.value,
        );
        value.saturating_mul(bonus_multiplier(info.and_then(|i| i.first_logged)))
    }

    fn unsold_total(&self) -> u64 {
        self.tracker
            .completed()
            .iter()
            .map(|k| self.completed_value(k))
            .fold(0, u64::saturating_add)
    }

    fn render_unsold(&self) -> Value {
        json!({"total": self.unsold_total(), "species": self.tracker.completed().len()})
    }

    fn render_sampling(&self, state: &GameState) -> Value {
        let Some(s) = self.tracker.sampling() else {
            return Value::Null;
        };
        let genus = Some(self.genus_of(&s.species))
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| s.genus.clone());
        let min_m = species_table()
            .genus(&genus)
            .map_or(DEFAULT_CCR_M, |g| g.ccr_m);
        let pos = state
            .status
            .as_ref()
            .and_then(|st| Some((st.latitude?, st.longitude?, st.planet_radius?, st.heading)));
        let (mut distance, mut direction) = (None, None);
        if !s.complete
            && let Some((lat, lon, radius, heading)) = pos
        {
            distance = self
                .tracker
                .distance_to_previous((lat, lon), radius)
                .map(f64::round);
            direction = heading
                .and_then(|h| self.tracker.direction_to_previous((lat, lon), h))
                .map(|(rel, compass)| DirectionView { rel, compass });
        }
        let remaining = distance.map(|d| (f64::from(min_m) - d).max(0.0));
        let key = (s.system_address, s.body_id);
        let pending = if s.complete {
            0
        } else {
            species_table()
                .species(&s.species)
                .map_or(0, |sp| sp.value)
                .saturating_mul(bonus_multiplier(first_logged_estimate(
                    self.scans.get(&key),
                    self.system_discovered,
                )))
        };
        json!({
            "species": s.species,
            "species_name": self.species_name(&s.species),
            "genus": genus,
            "genus_name": self.genus_name(&genus),
            "body_id": s.body_id,
            "points": s.scans.min(3),
            "complete": s.complete,
            "distance_m": distance,
            "min_m": min_m,
            "remaining_m": remaining,
            "direction": direction,
            "unsold_after": self.unsold_total().saturating_add(pending),
            "discarded_species": self.discarded,
            "discarded_name": self.discarded.as_deref().and_then(|d| self.species_name(d)),
        })
    }
}

impl Default for Exobiology {
    fn default() -> Self {
        Self::new()
    }
}

fn update(slot: &mut Value, new: Value, name: &str, dirty: &mut TopicSet) {
    if *slot != new {
        *slot = new;
        dirty.insert(name.to_owned());
    }
}

impl Module for Exobiology {
    fn id(&self) -> &'static str {
        "exobiology"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut dirty = TopicSet::new();
        match ev {
            SourceEvent::Status(st) => {
                self.tick(st.timestamp);
                let sampling = self.render_sampling(state);
                update(
                    &mut self.sampling_value,
                    sampling,
                    TOPIC_SAMPLING,
                    &mut dirty,
                );
                return dirty;
            }
            SourceEvent::Settings(s) => {
                self.settings = s.clone();
                self.net.retain(|_, st| *st != NetState::Disabled);
            }
            SourceEvent::Journal { ev, .. } => {
                self.tick(ev.timestamp);
                self.on_journal(ev, state, ctx);
            }
            SourceEvent::Stored { module, tag, value } if module == "exobiology" => {
                self.on_stored(tag, value);
            }
            SourceEvent::Net(response) => self.apply_response(response),
            _ => return dirty,
        }
        self.request_canonn(state, ctx);
        let (body, sampling, unsold) = (
            self.render_body(),
            self.render_sampling(state),
            self.render_unsold(),
        );
        update(&mut self.body_value, body, TOPIC_BODY, &mut dirty);
        update(
            &mut self.sampling_value,
            sampling,
            TOPIC_SAMPLING,
            &mut dirty,
        );
        update(&mut self.unsold_value, unsold, TOPIC_UNSOLD, &mut dirty);
        dirty
    }

    fn migrations(&self) -> &'static [tmc_core::Migration] {
        persist::MIGRATIONS
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        let v = match topic {
            TOPIC_BODY => &self.body_value,
            TOPIC_SAMPLING => &self.sampling_value,
            TOPIC_UNSOLD => &self.unsold_value,
            _ => return None,
        };
        (!v.is_null()).then(|| v.clone())
    }

    fn topics(&self) -> Vec<String> {
        vec![
            TOPIC_BODY.into(),
            TOPIC_SAMPLING.into(),
            TOPIC_UNSOLD.into(),
        ]
    }

    /// `Active` al estar aterrizado, a pie o en SRV en un cuerpo con señales
    /// biológicas (o con un `ScanOrganic` previo), o con un muestreo en curso
    /// en el cuerpo actual.
    fn relevance(&self, state: &GameState) -> Relevance {
        let Some(key) = self.focus() else {
            return Relevance::Normal;
        };
        let sampling_here = self
            .tracker
            .sampling()
            .is_some_and(|s| !s.complete && (s.system_address, s.body_id) == key);
        let bio_here =
            self.bio_counts.get(&key).is_some_and(|n| *n > 0) || self.organic_bodies.contains(&key);
        if sampling_here || (bio_here && landed_like(state)) {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }
}

/// Kebab-case id of a journal `AtmosphereType` (`CarbonDioxide` -> `carbon-dioxide`);
/// missing or empty is `"none"`. The UI translates it with `exobiology-atmosphere-<id>`.
fn atmosphere_key(atmosphere_type: Option<&str>) -> String {
    let raw = atmosphere_type.map(str::trim).unwrap_or("");
    if raw.is_empty() {
        return "none".to_owned();
    }
    let mut key = String::new();
    for (index, ch) in raw.chars().enumerate() {
        if ch.is_whitespace() || ch == '_' {
            if !key.ends_with('-') {
                key.push('-');
            }
        } else if ch.is_uppercase() {
            if index > 0 && !key.ends_with('-') {
                key.push('-');
            }
            key.extend(ch.to_lowercase());
        } else {
            key.push(ch);
        }
    }
    key
}

#[cfg(test)]
mod tests;

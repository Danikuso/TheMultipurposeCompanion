//! Evaluación local de alcance de salto, valores de exploración y predicción
//! de especies sobre un historial de diarios (spec F1 §10.1). Solo produce
//! métricas agregadas; reutiliza las funciones públicas de los módulos.

use crate::combat::vouchers::{VoucherKind, VoucherLedger, declared_amount, redeem_kind};
use crate::data::{FsdTable, bio_rules, fsd_table, species_table};
use crate::exobiology::first_logged_estimate;
use crate::exobiology::predict::{
    BioRules, BodyProfileKey, HistoryStats, Likelihood, body_profile, is_primary_star, predict,
};
use crate::exobiology::species::SpeciesTable;
use crate::exploration::unsold::UnsoldLedger;
use crate::exploration::value::{body_kind, estimate_probes};
use crate::mining::session::MiningSession;
use crate::missions::{MassacreStack, MissionBook, MissionKind};
use crate::navigation::jump::{RangeSource, ShipMass, build_jump_model, jump_range_with};
use crate::trade::CostBasis;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use tmc_model::events::{
    AsteroidContent, EventKind, JournalEvent, Loadout, OrganicScanType, SaaScanComplete, Scan,
    ScanOrganic, SellOrganicData,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RangeReport {
    pub cases: u32,
    pub within_tolerance: u32,
    pub engineered_cases: u32,
    pub guardian_cases: u32,
    pub max_abs_error: f64,
    pub mean_abs_error: f64,
    pub full_tank_hits: u32,
    pub max_fuel_hits: u32,
    /// Aciertos con masa = vacío + solo el combustible máximo por salto.
    pub max_fuel_mass_hits: u32,
    /// Por FSD visto: cuántos loadouts y masas de referencia distintas lo respaldan.
    pub fsd_stats: Vec<FsdStat>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FsdStat {
    pub item: String,
    pub loadouts: u32,
    pub distinct_masses: u32,
    /// `optimal_mass` ajustado con este mismo historial (comprobación circular).
    pub fitted: bool,
    pub verified: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueReport {
    pub sales: u32,
    pub computed: u64,
    pub paid: u64,
    pub rel_error: f64,
    pub uncovered_sales: u32,
    /// Suma de `BaseValue + Bonus` de las mismas ventas (bruto antes de descuentos).
    pub gross: u64,
    /// Suma del valor bruto del libro (sin factor de portaaviones) de las ventas cubiertas.
    pub computed_raw: u64,
    /// Ventas en portaaviones a las que se aplicó el factor observado.
    pub carrier_sales: u32,
    pub gross_rel_error: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PredictionReport {
    pub bodies: u32,
    pub hit_high_or_medium: u32,
    pub ratio: f64,
    pub first_logged_accuracy: f64,
    pub first_logged_cases: u32,
    pub likelihoods: Vec<Option<Likelihood>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProbeReport {
    pub cases: u32,
    pub mean_abs_error: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpeciesValueReport {
    pub entries: u32,
    pub matches: u32,
    pub mismatches: u32,
    pub unknown: u32,
    pub mismatch_symbols: Vec<String>,
    pub unknown_symbols: Vec<String>,
}

/// Conciliación de vales de un tipo (importes en créditos).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VoucherKindReport {
    pub earned: u64,
    /// Importe declarado en los `RedeemVoucher` del tipo.
    pub redeemed: u64,
    /// Vales perdidos al morir.
    pub lost: u64,
    /// Saldo del libro al final del historial.
    pub pending: u64,
    /// `ganado - perdido - canjeado - pendiente`; negativo si se canjeó más de
    /// lo visto en el historial (p. ej. vales anteriores a los diarios).
    pub difference: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VoucherReport {
    pub redeem_events: u32,
    pub bounty: VoucherKindReport,
    pub combat_bond: VoucherKindReport,
}

/// Toneladas refinadas atribuidas a los asteroides de un nivel de contenido.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct YieldLevel {
    pub asteroids: u32,
    pub tons: u32,
    pub mean: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MiningYieldReport {
    pub low: YieldLevel,
    pub medium: YieldLevel,
    pub high: YieldLevel,
    pub unknown: YieldLevel,
    /// Refinados anteriores a cualquier asteroide prospectado de la sesión.
    pub unattributed_tons: u32,
}

/// `CostBasis` frente a `AvgPricePaid` de las ventas del juego.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CostReport {
    /// Ventas comparables (coste conocido, completo y `AvgPricePaid` > 0).
    pub sales: u32,
    /// Ventas con diferencia exactamente 0.
    pub exact: u32,
    pub mean_abs_diff: f64,
    pub max_abs_diff: u64,
    /// Ventas no comparables (sin coste conocido, parcial o sin dato del juego).
    pub skipped: u32,
}

/// Misiones de masacre redirigidas y su recuento local en ese momento.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MassacreReport {
    pub redirected: u32,
    /// Recuento local igual al requerido.
    pub matched: u32,
    /// Recuento local menor que el requerido.
    pub short: u32,
    /// Recuento local mayor que el requerido.
    pub over: u32,
    /// Redirigidas que no figuraban como masacre en el libro (historial previo).
    pub unknown: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvalReport {
    pub cost: CostReport,
    pub massacre: MassacreReport,
    pub vouchers: VoucherReport,
    pub mining: MiningYieldReport,
    pub range: RangeReport,
    pub value: ValueReport,
    pub prediction: PredictionReport,
    pub probes: ProbeReport,
    pub species_values: SpeciesValueReport,
    pub missing_fsd: Vec<String>,
}

pub fn eval_events(events: impl Iterator<Item = JournalEvent>) -> EvalReport {
    eval_with(events, fsd_table(), bio_rules(), species_table())
}

/// Tolerancia (al) de la spec §10.3.
const RANGE_TOLERANCE_LY: f64 = 0.05;
/// Factor observado (no documentado por Frontier) en `TotalEarnings` /
/// (`BaseValue` + `Bonus`) de las ventas de cartografía en portaaviones.
const CARRIER_PAYOUT_FACTOR: f64 = 0.88;

pub(crate) fn eval_with(
    events: impl Iterator<Item = JournalEvent>,
    fsd: &FsdTable,
    rules: &BioRules,
    species: &SpeciesTable,
) -> EvalReport {
    let mut run = Run::new(fsd, rules, species);
    for event in events {
        run.apply(&event.kind);
        run.track_economy(&event);
    }
    run.finish()
}

type BodyKey = (u64, u32);

struct Run<'a> {
    fsd: &'a FsdTable,
    rules: &'a BioRules,
    species: &'a SpeciesTable,
    system: String,
    odyssey: bool,
    ledger: UnsoldLedger,
    loadouts: HashSet<String>,
    scans: HashMap<BodyKey, Scan>,
    /// Clase y `WasDiscovered` de la estrella principal por sistema.
    primary: HashMap<u64, (String, Option<bool>)>,
    genera: HashMap<BodyKey, Vec<String>>,
    evaluated: HashSet<(BodyKey, String)>,
    history: HistoryStats,
    history_seen: HashSet<(BodyKey, String)>,
    completed: Vec<(String, Option<String>, BodyKey)>,
    range: RangeReport,
    range_measured: u32,
    range_error_sum: f64,
    value: ValueReport,
    prediction: PredictionReport,
    first_logged_hits: u32,
    probe_error_sum: f64,
    probes: ProbeReport,
    species_values: SpeciesValueReport,
    missing_fsd: BTreeSet<String>,
    /// Por FSD: loadouts, masas de referencia (x10), ajustado, verificado.
    fsd_stats: BTreeMap<String, (u32, BTreeSet<i64>, bool, bool)>,
    /// Cuerpos escaneados (estrellas/planetas) y `BodyCount` del FSS por sistema.
    coverage: HashMap<String, (Option<u32>, HashSet<u32>)>,
    at_carrier: bool,
    voucher_ledger: VoucherLedger,
    vouchers: VoucherReport,
    mining_session: MiningSession,
    /// Índice (`level_index`) del último asteroide prospectado de la sesión.
    last_asteroid: Option<usize>,
    mining_levels: [YieldLevel; 4],
    unattributed_tons: u32,
    cost_basis: CostBasis,
    cost: CostReport,
    cost_diff_sum: u64,
    mission_book: MissionBook,
    massacre_stack: MassacreStack,
    massacre: MassacreReport,
}

fn level_index(c: AsteroidContent) -> usize {
    match c {
        AsteroidContent::Low => 0,
        AsteroidContent::Medium => 1,
        AsteroidContent::High => 2,
        AsteroidContent::Unknown => 3,
    }
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn is_fsd_item(item: &str) -> bool {
    item.to_ascii_lowercase().starts_with("int_hyperdrive")
}

fn ratio(part: u32, whole: u32) -> f64 {
    if whole == 0 {
        0.0
    } else {
        f64::from(part) / f64::from(whole)
    }
}

impl<'a> Run<'a> {
    fn new(fsd: &'a FsdTable, rules: &'a BioRules, species: &'a SpeciesTable) -> Self {
        Self {
            fsd,
            rules,
            species,
            system: String::new(),
            odyssey: true,
            ledger: UnsoldLedger::default(),
            loadouts: HashSet::new(),
            scans: HashMap::new(),
            primary: HashMap::new(),
            genera: HashMap::new(),
            evaluated: HashSet::new(),
            history: HistoryStats::default(),
            history_seen: HashSet::new(),
            completed: Vec::new(),
            range: RangeReport::default(),
            range_measured: 0,
            range_error_sum: 0.0,
            value: ValueReport::default(),
            prediction: PredictionReport::default(),
            first_logged_hits: 0,
            probe_error_sum: 0.0,
            probes: ProbeReport::default(),
            species_values: SpeciesValueReport::default(),
            missing_fsd: BTreeSet::new(),
            fsd_stats: BTreeMap::new(),
            coverage: HashMap::new(),
            at_carrier: false,
            voucher_ledger: VoucherLedger::default(),
            vouchers: VoucherReport::default(),
            mining_session: MiningSession::default(),
            last_asteroid: None,
            mining_levels: [YieldLevel::default(); 4],
            unattributed_tons: 0,
            cost_basis: CostBasis::default(),
            cost: CostReport::default(),
            cost_diff_sum: 0,
            mission_book: MissionBook::default(),
            massacre_stack: MassacreStack::default(),
            massacre: MassacreReport::default(),
        }
    }

    fn apply(&mut self, kind: &EventKind) {
        match kind {
            EventKind::LoadGame(load) => self.odyssey = load.odyssey.unwrap_or(self.odyssey),
            EventKind::FsdJump(l) | EventKind::CarrierJump(l) | EventKind::Location(l) => {
                self.system = l.star_system.clone();
            }
            EventKind::Docked(docked) => {
                self.at_carrier = docked.station_type.as_deref() == Some("FleetCarrier");
            }
            EventKind::Undocked(_) => self.at_carrier = false,
            EventKind::FssDiscoveryScan(fss) => {
                if let (Some(name), Some(count)) = (fss.system_name.as_ref(), fss.body_count) {
                    self.coverage.entry(name.clone()).or_default().0 = Some(count);
                }
            }
            EventKind::Loadout(lo) => self.loadout(lo),
            EventKind::Scan(scan) => self.scan(scan),
            EventKind::SaaScanComplete(done) => self.probe(done),
            EventKind::SaaSignalsFound(found) => {
                if let (Some(address), Some(id)) = (found.system_address, found.body_id) {
                    let genera: Vec<String> =
                        found.genuses.iter().map(|g| g.genus.clone()).collect();
                    if !genera.is_empty() {
                        self.genera.insert((address, id), genera);
                    }
                }
            }
            EventKind::ScanOrganic(organic) => self.organic(organic),
            EventKind::SellOrganicData(sale) => self.organic_sale(sale),
            EventKind::SellExplorationData(sale) => {
                self.exploration_sale(
                    &sale.systems,
                    sale.total_earnings,
                    sale.base_value.unwrap_or(0) + sale.bonus.unwrap_or(0),
                );
            }
            EventKind::MultiSellExplorationData(sale) => {
                let systems: Vec<String> = sale
                    .discovered
                    .iter()
                    .map(|s| s.system_name.clone())
                    .collect();
                self.exploration_sale(
                    &systems,
                    sale.total_earnings,
                    sale.base_value.unwrap_or(0) + sale.bonus.unwrap_or(0),
                );
            }
            _ => {}
        }
        // Mismo libro que el módulo: aplica Scan, ventas y muerte.
        self.ledger.apply_event(&self.system, kind, self.odyssey);
    }

    /// Vales y rendimiento de minería con los mismos libros que los módulos
    /// (`VoucherLedger` y `MiningSession`); aquí solo se miden sus diferencias.
    fn track_economy(&mut self, ev: &JournalEvent) {
        self.track_trade_and_missions(ev);
        let kinds = [VoucherKind::Bounty, VoucherKind::CombatBond];
        let before = kinds.map(|k| self.voucher_ledger.total(k));
        self.voucher_ledger
            .apply_event(&ev.kind, &self.system, ev.timestamp);
        let after = kinds.map(|k| self.voucher_ledger.total(k));
        let redeem = match &ev.kind {
            EventKind::RedeemVoucher(r) => {
                let kind = redeem_kind(r).map(|k| usize::from(k == VoucherKind::CombatBond));
                if kind.is_some() {
                    self.vouchers.redeem_events += 1;
                }
                kind.map(|i| (i, declared_amount(r)))
            }
            _ => None,
        };
        let died = matches!(ev.kind, EventKind::Died);
        for (i, report) in [&mut self.vouchers.bounty, &mut self.vouchers.combat_bond]
            .into_iter()
            .enumerate()
        {
            if died {
                report.lost += before[i];
            } else if redeem.is_none() && after[i] > before[i] {
                report.earned += after[i] - before[i];
            }
            if let Some((kind, amount)) = redeem
                && kind == i
            {
                report.redeemed += amount;
            }
        }

        let refined_before: u32 = self.mining_session.refined.values().sum();
        self.mining_session.apply_event(&ev.kind, ev.timestamp);
        let refined_after: u32 = self.mining_session.refined.values().sum();
        match &ev.kind {
            EventKind::LoadGame(_) => self.last_asteroid = None,
            EventKind::ProspectedAsteroid(p) => {
                let index = level_index(p.content_level());
                self.last_asteroid = Some(index);
                self.mining_levels[index].asteroids += 1;
            }
            EventKind::MiningRefined(_) if refined_after > refined_before => {
                let tons = refined_after - refined_before;
                match self.last_asteroid {
                    Some(index) => self.mining_levels[index].tons += tons,
                    None => self.unattributed_tons += tons,
                }
            }
            _ => {}
        }
    }

    /// Coste pagado y pilas de masacre con los mismos libros que los módulos.
    /// Cada comprobación se hace con el estado previo al evento.
    fn track_trade_and_missions(&mut self, ev: &JournalEvent) {
        match &ev.kind {
            EventKind::MarketSell(sale) => {
                let ours = self
                    .cost_basis
                    .avg_paid(&sale.type_)
                    .filter(|_| !self.cost_basis.is_partial(&sale.type_));
                match (ours, sale.avg_price_paid.filter(|p| *p > 0)) {
                    (Some(ours), Some(game)) => {
                        let diff = ours.abs_diff(game);
                        self.cost.sales += 1;
                        self.cost.exact += u32::from(diff == 0);
                        self.cost.max_abs_diff = self.cost.max_abs_diff.max(diff);
                        self.cost_diff_sum += diff;
                    }
                    _ => self.cost.skipped += 1,
                }
            }
            EventKind::MissionRedirected(r) => {
                self.massacre.redirected += 1;
                match self
                    .mission_book
                    .get(r.mission_id)
                    .filter(|m| m.kind == MissionKind::Massacre)
                {
                    Some(m) => {
                        let local = self.massacre_stack.kills_for(r.mission_id);
                        let needed = m.progress.kills_needed.unwrap_or(0);
                        match local.cmp(&needed) {
                            std::cmp::Ordering::Equal => self.massacre.matched += 1,
                            std::cmp::Ordering::Less => self.massacre.short += 1,
                            std::cmp::Ordering::Greater => self.massacre.over += 1,
                        }
                    }
                    None => self.massacre.unknown += 1,
                }
            }
            _ => {}
        }
        self.cost_basis.apply_event(&ev.kind);
        // La pila se aplica con el libro tal como estaba antes del evento.
        self.massacre_stack
            .apply_event(&ev.kind, ev.timestamp, &self.mission_book);
        self.mission_book.apply_event(&ev.kind, ev.timestamp);
    }

    fn loadout(&mut self, lo: &Loadout) {
        let Some(game) = lo.max_jump_range.filter(|r| *r > 0.0) else {
            return;
        };
        let Some(unladen) = lo.unladen_mass else {
            return;
        };
        let Some(fsd_module) = lo
            .modules
            .iter()
            .find(|m| m.item.as_deref().is_some_and(is_fsd_item))
        else {
            return;
        };
        let Ok(key) = serde_json::to_string(lo) else {
            return;
        };
        if !self.loadouts.insert(key) {
            return;
        }
        self.range.cases += 1;
        if fsd_module.engineering.is_some() {
            self.range.engineered_cases += 1;
        }
        if lo.modules.iter().any(|m| {
            m.item
                .as_deref()
                .is_some_and(|i| i.to_ascii_lowercase().contains("guardianfsdbooster"))
        }) {
            self.range.guardian_cases += 1;
        }
        let item = fsd_module
            .item
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let entry = self.fsd.find_fsd(&item);
        if entry.is_none() {
            self.missing_fsd.insert(item.clone());
        }
        // Usa el modelo de producción: la referencia de masa sale de él.
        let model = build_jump_model(lo, self.fsd);
        let stat = self.fsd_stats.entry(item).or_default();
        stat.0 += 1;
        stat.2 = entry.is_some_and(|e| e.provenance.starts_with("ajustado"));
        stat.3 = entry.is_some_and(|e| e.verified);
        if let Some(reference) = model.laden_ref_mass {
            stat.1.insert((reference * 10.0).round() as i64);
        }
        let (Some(spec), Some(reference)) = (&model.fsd, model.laden_ref_mass) else {
            return;
        };
        let at_reference = ShipMass {
            unladen,
            fuel_main: reference - unladen,
            fuel_reserve: 0.0,
            cargo: 0.0,
        };
        let Some(modelled) = model.range_now(&at_reference) else {
            return;
        };
        let error = (modelled - game).abs();
        if model.source == RangeSource::Formula && error <= RANGE_TOLERANCE_LY {
            self.range.within_tolerance += 1;
            self.range.max_fuel_mass_hits += 1;
        }
        // Columnas heredadas (masa con depósito lleno), solo para comparar.
        let mass = ShipMass {
            fuel_main: lo
                .fuel_capacity
                .as_ref()
                .and_then(|f| f.main)
                .unwrap_or(0.0),
            ..at_reference
        };
        if (jump_range_with(spec, &mass, true) - game).abs() <= RANGE_TOLERANCE_LY {
            self.range.max_fuel_hits += 1;
        }
        if (jump_range_with(spec, &mass, false) - game).abs() <= RANGE_TOLERANCE_LY {
            self.range.full_tank_hits += 1;
        }
        self.range_measured += 1;
        self.range_error_sum += error;
        self.range.max_abs_error = self.range.max_abs_error.max(error);
    }

    fn scan(&mut self, scan: &Scan) {
        if let (Some(system), Some(id)) = (scan.star_system.as_ref(), scan.body_id)
            && body_kind(scan).is_some()
        {
            self.coverage
                .entry(system.clone())
                .or_default()
                .1
                .insert(id);
        }
        let Some(address) = scan.system_address else {
            return;
        };
        if is_primary_star(scan) {
            self.primary.entry(address).or_insert_with(|| {
                (
                    scan.star_type.clone().unwrap_or_default(),
                    scan.was_discovered,
                )
            });
        }
        if let Some(id) = scan.body_id {
            self.scans.insert((address, id), scan.clone());
        }
    }

    fn probe(&mut self, done: &SaaScanComplete) {
        let (Some(address), Some(id), Some(target)) =
            (done.system_address, done.body_id, done.efficiency_target)
        else {
            return;
        };
        let Some(radius) = self.scans.get(&(address, id)).and_then(|s| s.radius) else {
            return;
        };
        self.probes.cases += 1;
        self.probe_error_sum += (f64::from(estimate_probes(radius)) - f64::from(target)).abs();
    }

    fn exploration_sale(&mut self, systems: &[String], total: Option<u64>, gross: u64) {
        let paid = total.unwrap_or(0);
        if paid == 0 {
            return;
        }
        let covered = !systems.is_empty()
            && systems.iter().all(|name| {
                self.coverage.get(name).is_some_and(|(count, ids)| {
                    count.is_some_and(|c| c > 0 && ids.len() as u32 >= c)
                })
            });
        let raw = if systems.is_empty() {
            self.ledger.total()
        } else {
            self.ledger.total_for(systems)
        };
        if !covered || raw == 0 {
            self.value.uncovered_sales += 1;
            return;
        }
        let computed = if self.at_carrier {
            self.value.carrier_sales += 1;
            (raw as f64 * CARRIER_PAYOUT_FACTOR).round() as u64
        } else {
            raw
        };
        self.value.sales += 1;
        self.value.computed += computed;
        self.value.computed_raw += raw;
        self.value.paid += paid;
        self.value.gross += gross;
    }

    fn organic(&mut self, organic: &ScanOrganic) {
        if organic.scan_type != OrganicScanType::Analyse {
            return;
        }
        let (Some(species), Some(address), Some(body)) = (
            organic.species.as_deref(),
            organic.system_address,
            organic.body,
        ) else {
            return;
        };
        let key = (address, body);
        self.completed
            .push((species.to_owned(), organic.variant.clone(), key));
        let Some(scan) = self.scans.get(&key) else {
            return;
        };
        let star = self.primary.get(&address).map(|(class, _)| class.as_str());
        let profile = body_profile(scan, star);
        if self.evaluated.insert((key, species.to_ascii_lowercase())) {
            let genera = self.genera.get(&key).map(Vec::as_slice);
            let level = predict(
                &profile,
                genera,
                &self.history,
                None,
                self.rules,
                self.species,
            )
            .iter()
            .find(|c| same(&c.species, species))
            .map(|c| c.likelihood);
            self.prediction.bodies += 1;
            if matches!(level, Some(Likelihood::High | Likelihood::Medium)) {
                self.prediction.hit_high_or_medium += 1;
            }
            self.prediction.likelihoods.push(level);
        }
        if self
            .history_seen
            .insert((key, species.to_ascii_lowercase()))
        {
            self.history
                .add(BodyProfileKey::from(&profile), species.to_owned());
        }
    }

    fn organic_sale(&mut self, sale: &SellOrganicData) {
        for entry in &sale.bio_data {
            let Some(symbol) = entry.species.as_deref() else {
                continue;
            };
            self.species_value(symbol, entry.value);
            let found = self.completed.iter().position(|(species, variant, _)| {
                same(species, symbol)
                    && match (variant, &entry.variant) {
                        (Some(a), Some(b)) => same(a, b),
                        _ => true,
                    }
            });
            let Some(index) = found else {
                continue;
            };
            let (_, _, key) = self.completed.remove(index);
            let discovered = self.primary.get(&key.0).and_then(|(_, d)| *d);
            let estimate = first_logged_estimate(self.scans.get(&key), discovered);
            self.prediction.first_logged_cases += 1;
            if estimate == Some(entry.bonus > 0) {
                self.first_logged_hits += 1;
            }
        }
    }

    fn species_value(&mut self, symbol: &str, value: u64) {
        if value == 0 {
            return;
        }
        let report = &mut self.species_values;
        report.entries += 1;
        let known = self
            .species
            .genera
            .iter()
            .flat_map(|g| &g.species)
            .find(|s| same(&s.symbol, symbol));
        let (list, counter) = match known {
            None => (&mut report.unknown_symbols, &mut report.unknown),
            Some(s) if s.value == value => {
                report.matches += 1;
                return;
            }
            Some(_) => (&mut report.mismatch_symbols, &mut report.mismatches),
        };
        *counter += 1;
        if !list.iter().any(|s| same(s, symbol)) {
            list.push(symbol.to_owned());
        }
    }

    fn finish(mut self) -> EvalReport {
        self.range.fsd_stats = std::mem::take(&mut self.fsd_stats)
            .into_iter()
            .map(|(item, (loadouts, masses, fitted, verified))| FsdStat {
                item,
                loadouts,
                distinct_masses: masses.len() as u32,
                fitted,
                verified,
            })
            .collect();
        if self.range_measured > 0 {
            self.range.mean_abs_error = self.range_error_sum / f64::from(self.range_measured);
        }
        if self.value.paid > 0 {
            self.value.rel_error = (self.value.computed as f64 - self.value.paid as f64).abs()
                / self.value.paid as f64;
        }
        self.prediction.ratio = ratio(self.prediction.hit_high_or_medium, self.prediction.bodies);
        self.prediction.first_logged_accuracy =
            ratio(self.first_logged_hits, self.prediction.first_logged_cases);
        if self.value.gross > 0 {
            self.value.gross_rel_error = (self.value.computed_raw as f64 - self.value.gross as f64)
                .abs()
                / self.value.gross as f64;
        }
        if self.probes.cases > 0 {
            self.probes.mean_abs_error = self.probe_error_sum / f64::from(self.probes.cases);
        }
        if self.cost.sales > 0 {
            self.cost.mean_abs_diff = self.cost_diff_sum as f64 / f64::from(self.cost.sales);
        }
        let mut vouchers = self.vouchers;
        for (report, kind) in [
            (&mut vouchers.bounty, VoucherKind::Bounty),
            (&mut vouchers.combat_bond, VoucherKind::CombatBond),
        ] {
            report.pending = self.voucher_ledger.total(kind);
            report.difference = i64::try_from(report.earned).unwrap_or(i64::MAX)
                - i64::try_from(report.lost + report.redeemed + report.pending).unwrap_or(i64::MAX);
        }
        let [mut low, mut medium, mut high, mut unknown] = self.mining_levels;
        for level in [&mut low, &mut medium, &mut high, &mut unknown] {
            if level.asteroids > 0 {
                level.mean = f64::from(level.tons) / f64::from(level.asteroids);
            }
        }
        EvalReport {
            cost: self.cost,
            massacre: self.massacre,
            vouchers,
            mining: MiningYieldReport {
                low,
                medium,
                high,
                unknown,
                unattributed_tons: self.unattributed_tons,
            },
            range: self.range,
            value: self.value,
            prediction: self.prediction,
            probes: self.probes,
            species_values: self.species_values,
            missing_fsd: self.missing_fsd.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exploration::value::body_values;
    use crate::navigation::jump::fsd_from_loadout;
    use tmc_model::events::parse_line;

    fn events(lines: &[String]) -> Vec<JournalEvent> {
        lines.iter().map(|l| parse_line(l).unwrap()).collect()
    }

    fn loadout_line(
        ship_id: u32,
        fsd_item: &str,
        eng: &str,
        booster: &str,
        max_range: f64,
    ) -> String {
        format!(
            r#"{{"timestamp":"2026-04-10T20:00:00Z","event":"Loadout","Ship":"python","ShipID":{ship_id},"UnladenMass":400.0,"MaxJumpRange":{max_range},"FuelCapacity":{{"Main":32.0,"Reserve":0.63}},"Modules":[{{"Slot":"FrameShiftDrive","Item":"{fsd_item}","Health":1.0{eng}}}{booster}]}}"#
        )
    }

    /// Range of a loadout line parsed through the real model.
    fn expected_range(line: &str, clamp: bool) -> f64 {
        let EventKind::Loadout(lo) = parse_line(line).unwrap().kind else {
            panic!("Loadout")
        };
        let fsd = fsd_from_loadout(&lo, fsd_table()).unwrap();
        let mass = ShipMass {
            unladen: lo.unladen_mass.unwrap(),
            fuel_main: 32.0,
            fuel_reserve: 0.0,
            cargo: 0.0,
        };
        jump_range_with(&fsd, &mass, clamp)
    }

    const FSD: &str = "int_hyperdrive_size5_class5";
    const ENG: &str = r#","Engineering":{"BlueprintName":"FSD_LongRange","Level":5,"Modifiers":[{"Label":"FSDOptimalMass","Value":1200.0,"OriginalValue":1050.0,"LessIsGood":0}]}"#;
    const BOOSTER: &str =
        r#",{"Slot":"GuardianFSDBooster","Item":"int_guardianfsdbooster_size3","Health":1.0}"#;

    /// Game-like range: unladen mass plus only the max fuel per jump aboard.
    fn game_like(line_fn: impl Fn(f64) -> String) -> String {
        let probe = line_fn(1.0);
        let EventKind::Loadout(lo) = parse_line(&probe).unwrap().kind else {
            panic!("Loadout")
        };
        let fsd = fsd_from_loadout(&lo, fsd_table()).unwrap();
        let mass = ShipMass {
            unladen: lo.unladen_mass.unwrap(),
            fuel_main: 5.0,
            fuel_reserve: 0.0,
            cargo: 0.0,
        };
        line_fn((jump_range_with(&fsd, &mass, true) * 1e4).round() / 1e4)
    }

    /// Builds a loadout whose `MaxJumpRange` is the capped range at full-tank mass (legacy convention).
    fn clamped(line_fn: impl Fn(f64) -> String) -> String {
        let probe = line_fn(1.0);
        line_fn((expected_range(&probe, true) * 1e4).round() / 1e4)
    }

    #[test]
    fn range_eval_counts_engineered_and_guardian_cases() {
        let plain = game_like(|r| loadout_line(1, FSD, "", "", r));
        let engineered = game_like(|r| loadout_line(2, FSD, ENG, "", r));
        let guardian = game_like(|r| loadout_line(3, FSD, "", BOOSTER, r));
        let report = eval_events(events(&[plain, engineered, guardian]).into_iter());
        assert_eq!(report.range.cases, 3);
        assert_eq!(report.range.engineered_cases, 1);
        assert_eq!(report.range.guardian_cases, 1);
        assert_eq!(report.range.within_tolerance, 3);
        assert!(report.range.max_abs_error < 0.05, "{:?}", report.range);
        assert!(report.missing_fsd.is_empty());
        assert_eq!(report.range.fsd_stats.len(), 1);
        assert_eq!(report.range.fsd_stats[0].loadouts, 3);
        assert_eq!(report.range.fsd_stats[0].distinct_masses, 1);
        assert!(!report.range.fsd_stats[0].fitted);
    }

    #[test]
    fn full_tank_and_max_fuel_hits_are_computed_independently() {
        let line = clamped(|r| loadout_line(1, FSD, "", "", r));
        let full = expected_range(&line, false);
        let capped = expected_range(&line, true);
        assert!(
            full - capped > 1.0,
            "tank of 32 t must exceed the 5 t jump cap"
        );
        let report = eval_events(events(&[line]).into_iter());
        assert_eq!(report.range.max_fuel_hits, 1);
        assert_eq!(report.range.full_tank_hits, 0);
        // The game-like reference (unladen + 5 t) does not match this legacy line.
        assert_eq!(report.range.max_fuel_mass_hits, 0);
    }

    #[test]
    fn unknown_fsd_is_a_miss_and_listed_by_item_name() {
        let line = loadout_line(1, "int_hyperdrive_size9_class9", "", "", 40.0);
        let report = eval_events(events(&[line]).into_iter());
        assert_eq!(report.range.cases, 1);
        assert_eq!(report.range.within_tolerance, 0);
        assert_eq!(
            report.missing_fsd,
            vec!["int_hyperdrive_size9_class9".to_owned()]
        );
    }

    fn star_scan(star_system: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-04-10T20:06:00Z","event":"Scan","ScanType":"AutoScan","BodyName":"{star_system} A","BodyID":0,"StarSystem":"{star_system}","SystemAddress":11,"DistanceFromArrivalLS":0.0,"StarType":"K","Subclass":3,"StellarMass":0.8,"WasDiscovered":true,"WasMapped":true}}"#
        )
    }

    #[test]
    fn value_eval_matches_fixture_sale_within_5_percent() {
        let EventKind::Scan(scan) = parse_line(&star_scan("Sys")).unwrap().kind else {
            panic!("Scan")
        };
        let computed = body_values(&scan, true).scanned;
        assert_eq!(computed, 1_214);
        let paid = 1_250;
        let lines = vec![
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"LoadGame","Odyssey":true}"#.to_owned(),
            r#"{"timestamp":"2026-04-10T20:05:00Z","event":"FSDJump","StarSystem":"Sys","SystemAddress":11,"StarPos":[0,0,0]}"#.to_owned(),
            r#"{"timestamp":"2026-04-10T20:05:40Z","event":"FSSDiscoveryScan","Progress":1.0,"BodyCount":1,"NonBodyCount":0,"SystemName":"Sys","SystemAddress":11}"#.to_owned(),
            star_scan("Sys"),
            format!(
                r#"{{"timestamp":"2026-04-10T20:45:10Z","event":"SellExplorationData","Systems":["Sys"],"Discovered":[],"BaseValue":{paid},"Bonus":0,"TotalEarnings":{paid}}}"#
            ),
        ];
        let report = eval_events(events(&lines).into_iter());
        assert_eq!(report.value.sales, 1);
        assert_eq!(report.value.computed, 1_214);
        assert_eq!(report.value.paid, 1_250);
        assert!((report.value.rel_error - 36.0 / 1_250.0).abs() < 1e-12);
        assert!(report.value.rel_error <= 0.05);
    }

    fn sale_lines(fss: bool, carrier: bool, paid: u64) -> Vec<JournalEvent> {
        let mut lines = vec![
            r#"{"timestamp":"2026-04-10T20:05:00Z","event":"FSDJump","StarSystem":"Sys","SystemAddress":11,"StarPos":[0,0,0]}"#.to_owned(),
            star_scan("Sys"),
        ];
        if fss {
            lines.insert(1, r#"{"timestamp":"2026-04-10T20:05:40Z","event":"FSSDiscoveryScan","Progress":1.0,"BodyCount":1,"NonBodyCount":0,"SystemName":"Sys","SystemAddress":11}"#.to_owned());
        }
        if carrier {
            lines.push(r#"{"timestamp":"2026-04-10T20:40:00Z","event":"Docked","StationName":"X","StationType":"FleetCarrier","MarketID":1,"StarSystem":"Sys","SystemAddress":11}"#.to_owned());
        }
        lines.push(format!(
            r#"{{"timestamp":"2026-04-10T20:45:10Z","event":"SellExplorationData","Systems":["Sys"],"Discovered":[],"BaseValue":1214,"Bonus":0,"TotalEarnings":{paid}}}"#
        ));
        events(&lines)
    }

    #[test]
    fn sales_of_partially_scanned_systems_are_not_covered() {
        let report = eval_events(sale_lines(false, false, 1_214).into_iter());
        assert_eq!(report.value.sales, 0);
        assert_eq!(report.value.uncovered_sales, 1);
    }

    #[test]
    fn carrier_sales_apply_the_observed_payout_factor_only_when_docked_at_a_carrier() {
        let carrier = eval_events(sale_lines(true, true, 1_068).into_iter());
        assert_eq!(carrier.value.carrier_sales, 1);
        assert_eq!(carrier.value.computed, 1_068);
        assert_eq!(carrier.value.computed_raw, 1_214);
        assert!(carrier.value.rel_error < 1e-12);
        let station = eval_events(sale_lines(true, false, 1_068).into_iter());
        assert_eq!(station.value.carrier_sales, 0);
        assert_eq!(station.value.computed, 1_214);
    }

    fn rocky_scan(body: u32, address: u64) -> String {
        format!(
            r#"{{"timestamp":"2026-04-10T20:06:0{body}Z","event":"Scan","ScanType":"Detailed","BodyName":"B{body}","BodyID":{body},"StarSystem":"S","SystemAddress":{address},"DistanceFromArrivalLS":200.0,"PlanetClass":"Rocky body","AtmosphereType":"CarbonDioxide","SurfaceGravity":1.2,"SurfaceTemperature":170.0,"SurfacePressure":1000.0,"MassEM":0.01,"Landable":true}}"#
        )
    }

    fn analyse(body: u32, address: u64, species: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-04-10T20:2{body}:00Z","event":"ScanOrganic","ScanType":"Analyse","Genus":"$Codex_Ent_Bacterial_Genus_Name;","Species":"{species}","Variant":"$V;","SystemAddress":{address},"Body":{body}}}"#
        )
    }

    #[test]
    fn prediction_eval_uses_only_past_history() {
        const AURASUS: &str = "$Codex_Ent_Bacterial_01_Name;";
        let rules = BioRules::from_json(
            r#"{"source":"test","license":"test","version":"test","rules":[
              {"species":"$Codex_Ent_Bacterial_01_Name;","body_class":["Rocky body"],
               "atmosphere_type":["CarbonDioxide"],"gravity_max":0.27,"source":"synthetic"}]}"#,
        )
        .unwrap();
        let lines = vec![
            rocky_scan(1, 5),
            rocky_scan(2, 5),
            analyse(1, 5, AURASUS),
            analyse(2, 5, AURASUS),
        ];
        let report = eval_with(
            events(&lines).into_iter(),
            fsd_table(),
            &rules,
            species_table(),
        );
        assert_eq!(report.prediction.bodies, 2);
        assert_eq!(report.prediction.hit_high_or_medium, 2);
        assert_eq!(
            report.prediction.likelihoods,
            vec![Some(Likelihood::Medium), Some(Likelihood::High)]
        );
        assert!((report.prediction.ratio - 1.0).abs() < 1e-12);
    }

    #[test]
    fn voucher_and_yield_reports_use_the_shared_ledgers() {
        let lines: Vec<String> = [
            r#"{"timestamp":"2026-10-18T10:00:00Z","event":"Bounty","Rewards":[{"Faction":"A","Reward":1000}],"TotalReward":1000}"#,
            r#"{"timestamp":"2026-10-18T10:01:00Z","event":"RedeemVoucher","Type":"bounty","Amount":400,"Factions":[{"Faction":"A","Amount":400}]}"#,
            r#"{"timestamp":"2026-10-18T10:02:00Z","event":"MiningRefined","Type":"$platinum_name;"}"#,
            r#"{"timestamp":"2026-10-18T10:03:00Z","event":"ProspectedAsteroid","Materials":[],"Content":"$AsteroidMaterialContent_High;"}"#,
            r#"{"timestamp":"2026-10-18T10:04:00Z","event":"MiningRefined","Type":"$platinum_name;"}"#,
            r#"{"timestamp":"2026-10-18T10:05:00Z","event":"MiningRefined","Type":"$platinum_name;"}"#,
        ]
        .iter()
        .map(|l| (*l).to_owned())
        .collect();
        let report = eval_events(events(&lines).into_iter());
        let b = report.vouchers.bounty;
        assert_eq!(
            (b.earned, b.redeemed, b.pending, b.difference),
            (1000, 400, 600, 0)
        );
        assert_eq!(report.mining.high.asteroids, 1);
        assert_eq!(report.mining.high.tons, 2);
        assert_eq!(report.mining.unattributed_tons, 1);
    }

    #[test]
    fn redeemed_uses_the_nominal_amount_of_the_ledger() {
        let lines: Vec<String> = [
            r#"{"timestamp":"2026-10-18T10:00:00Z","event":"Bounty","Rewards":[{"Faction":"A","Reward":1000}],"TotalReward":1000}"#,
            r#"{"timestamp":"2026-10-18T10:01:00Z","event":"Bounty","Rewards":[{"Faction":"B","Reward":500}],"TotalReward":500}"#,
            r#"{"timestamp":"2026-10-18T10:02:00Z","event":"RedeemVoucher","Type":"bounty","Amount":300,"BrokerPercentage":25.0,"Faction":"A"}"#,
            r#"{"timestamp":"2026-10-18T10:03:00Z","event":"RedeemVoucher","Type":"bounty","Amount":999,"Factions":[{"Faction":"A","Amount":200},{"Faction":"B","Amount":100}]}"#,
        ]
        .iter()
        .map(|l| (*l).to_owned())
        .collect();
        let b = eval_events(events(&lines).into_iter()).vouchers.bounty;
        // El corredor no cambia el importe (Amount tal cual) y las facciones
        // mandan sobre Amount: 300 + 300, no 300 + 999.
        assert_eq!(
            (b.earned, b.redeemed, b.pending, b.difference),
            (1500, 600, 900, 0)
        );
    }

    #[test]
    fn prediction_is_evaluated_per_species_not_once_per_body() {
        let rules = BioRules::from_json(
            r#"{"source":"t","license":"t","version":"t","rules":[
              {"species":"$Codex_Ent_Bacterial_01_Name;","body_class":["Rocky body"],
               "atmosphere_type":["CarbonDioxide"],"gravity_max":0.27,"source":"synthetic"}]}"#,
        )
        .unwrap();
        let lines = vec![
            rocky_scan(1, 5),
            analyse(1, 5, "$Codex_Ent_Bacterial_01_Name;"),
            analyse(1, 5, "$Codex_Ent_Bacterial_02_Name;"),
        ];
        let report = eval_with(
            events(&lines).into_iter(),
            fsd_table(),
            &rules,
            species_table(),
        );
        assert_eq!(report.prediction.bodies, 2);
        assert_eq!(report.prediction.hit_high_or_medium, 1);
    }
}

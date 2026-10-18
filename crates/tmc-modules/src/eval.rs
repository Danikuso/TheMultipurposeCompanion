//! Evaluación local de alcance de salto, valores de exploración y predicción
//! de especies sobre un historial de diarios (spec F1 §10.1). Solo produce
//! métricas agregadas; reutiliza las funciones públicas de los módulos.

use crate::data::{FsdTable, bio_rules, fsd_table, species_table};
use crate::exobiology::first_logged_estimate;
use crate::exobiology::predict::{
    BioRules, BodyProfileKey, HistoryStats, Likelihood, body_profile, is_primary_star, predict,
};
use crate::exobiology::species::SpeciesTable;
use crate::exploration::unsold::UnsoldLedger;
use crate::exploration::value::estimate_probes;
use crate::navigation::jump::{ShipMass, fsd_from_loadout, jump_range_with};
use std::collections::{BTreeSet, HashMap, HashSet};
use tmc_model::events::{
    EventKind, JournalEvent, Loadout, OrganicScanType, SaaScanComplete, Scan, ScanOrganic,
    SellOrganicData,
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvalReport {
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

pub(crate) fn eval_with(
    events: impl Iterator<Item = JournalEvent>,
    fsd: &FsdTable,
    rules: &BioRules,
    species: &SpeciesTable,
) -> EvalReport {
    let mut run = Run::new(fsd, rules, species);
    for event in events {
        run.apply(&event.kind);
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
    evaluated: HashSet<BodyKey>,
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
        }
    }

    fn apply(&mut self, kind: &EventKind) {
        match kind {
            EventKind::LoadGame(load) => self.odyssey = load.odyssey.unwrap_or(self.odyssey),
            EventKind::FsdJump(l) | EventKind::CarrierJump(l) | EventKind::Location(l) => {
                self.system = l.star_system.clone();
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
        let Some(spec) = fsd_from_loadout(lo, self.fsd) else {
            if let Some(item) = fsd_module.item.as_deref() {
                self.missing_fsd.insert(item.to_ascii_lowercase());
            }
            return;
        };
        let mass = ShipMass {
            unladen,
            fuel_main: lo
                .fuel_capacity
                .as_ref()
                .and_then(|f| f.main)
                .unwrap_or(0.0),
            fuel_reserve: 0.0,
            cargo: 0.0,
        };
        let capped = jump_range_with(&spec, &mass, true);
        let full = jump_range_with(&spec, &mass, false);
        let error = (capped - game).abs();
        if error <= RANGE_TOLERANCE_LY {
            self.range.within_tolerance += 1;
            self.range.max_fuel_hits += 1;
        }
        if (full - game).abs() <= RANGE_TOLERANCE_LY {
            self.range.full_tank_hits += 1;
        }
        let light = ShipMass {
            fuel_main: mass.fuel_main.min(spec.max_fuel_per_jump),
            ..mass
        };
        if (jump_range_with(&spec, &light, true) - game).abs() <= RANGE_TOLERANCE_LY {
            self.range.max_fuel_mass_hits += 1;
        }
        self.range_measured += 1;
        self.range_error_sum += error;
        self.range.max_abs_error = self.range.max_abs_error.max(error);
    }

    fn scan(&mut self, scan: &Scan) {
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
        let computed = if systems.is_empty() {
            self.ledger.total()
        } else {
            self.ledger.total_for(systems)
        };
        if computed == 0 {
            self.value.uncovered_sales += 1;
            return;
        }
        self.value.sales += 1;
        self.value.computed += computed;
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
        if self.evaluated.insert(key) {
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
            self.value.gross_rel_error = (self.value.computed as f64 - self.value.gross as f64)
                .abs()
                / self.value.gross as f64;
        }
        if self.probes.cases > 0 {
            self.probes.mean_abs_error = self.probe_error_sum / f64::from(self.probes.cases);
        }
        EvalReport {
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

    /// Builds a loadout whose `MaxJumpRange` is the game-like (capped) range.
    fn clamped(line_fn: impl Fn(f64) -> String) -> String {
        let probe = line_fn(1.0);
        line_fn((expected_range(&probe, true) * 1e4).round() / 1e4)
    }

    #[test]
    fn range_eval_counts_engineered_and_guardian_cases() {
        let plain = clamped(|r| loadout_line(1, FSD, "", "", r));
        let engineered = clamped(|r| loadout_line(2, FSD, ENG, "", r));
        let guardian = clamped(|r| loadout_line(3, FSD, "", BOOSTER, r));
        let report = eval_events(events(&[plain, engineered, guardian]).into_iter());
        assert_eq!(report.range.cases, 3);
        assert_eq!(report.range.engineered_cases, 1);
        assert_eq!(report.range.guardian_cases, 1);
        assert_eq!(report.range.within_tolerance, 3);
        assert!(report.range.max_abs_error < 0.05, "{:?}", report.range);
        assert!(report.missing_fsd.is_empty());
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
        assert_eq!(report.range.within_tolerance, 1);
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
}

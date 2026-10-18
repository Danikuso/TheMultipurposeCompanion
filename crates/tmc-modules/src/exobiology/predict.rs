//! Species prediction: project-owned rules, commander history and Canonn hints.

use super::species::{SpeciesInfo, SpeciesTable};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use tmc_core::StoreTask;
use tmc_model::events::Scan;
use tmc_model::net::{BodyProfile, RemoteBodyBio};

const STANDARD_GRAVITY: f64 = 9.806_65;
const STANDARD_ATMOSPHERE_PA: f64 = 101_325.0;
const GRAVITY_BIN_G: f64 = 0.05;
const TEMPERATURE_BIN_K: f64 = 20.0;
const HISTORY_STRONG: u32 = 3;
const CANONN_RAISE: f64 = 0.5;
const CANONN_LOWER: f64 = 0.1;

fn non_empty(text: Option<&String>) -> Option<String> {
    text.map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

/// Converts journal units (m/s², Pa) into the profile units (g, atm).
pub fn body_profile(scan: &Scan, star_class: Option<&str>) -> BodyProfile {
    BodyProfile {
        planet_class: non_empty(scan.planet_class.as_ref()),
        atmosphere_type: non_empty(scan.atmosphere_type.as_ref()),
        gravity_g: scan.surface_gravity.map(|g| g / STANDARD_GRAVITY),
        temperature_k: scan.surface_temperature,
        pressure_atm: scan.surface_pressure.map(|p| p / STANDARD_ATMOSPHERE_PA),
        volcanism: non_empty(scan.volcanism.as_ref()),
        star_class: star_class
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    }
}

/// Main star of a system: the first star `Scan` at 0 ls.
pub fn is_primary_star(scan: &Scan) -> bool {
    scan.star_type.as_deref().is_some_and(|s| !s.is_empty())
        && scan.distance_from_arrival_ls == Some(0.0)
}

/// Similarity key for the commander's history (bins of 0.05 g and 20 K).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BodyProfileKey {
    pub body_class: String,
    pub atmosphere: String,
    pub gravity_bin: Option<i64>,
    pub temperature_bin: Option<i64>,
    pub volcanic: bool,
    pub star_class: String,
}

impl From<&BodyProfile> for BodyProfileKey {
    fn from(profile: &BodyProfile) -> Self {
        let bin = |value: Option<f64>, width: f64| {
            value
                .filter(|v| v.is_finite())
                .map(|v| (v / width).floor() as i64)
        };
        Self {
            body_class: profile.planet_class.clone().unwrap_or_default(),
            atmosphere: atmosphere_of(profile),
            gravity_bin: bin(profile.gravity_g, GRAVITY_BIN_G),
            temperature_bin: bin(profile.temperature_k, TEMPERATURE_BIN_K),
            volcanic: profile.volcanism.as_deref().is_some_and(is_volcanic),
            star_class: profile.star_class.clone().unwrap_or_default(),
        }
    }
}

/// Lower-case atmosphere type; no atmosphere is `"none"`.
fn atmosphere_of(profile: &BodyProfile) -> String {
    profile
        .atmosphere_type
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map_or_else(|| "none".to_owned(), str::to_lowercase)
}

fn is_volcanic(volcanism: &str) -> bool {
    let text = volcanism.trim();
    !text.is_empty() && !text.eq_ignore_ascii_case("No volcanism")
}

/// Versioned prediction rules (`data/bio_rules.json`).
#[derive(Debug, Default, Deserialize)]
pub struct BioRules {
    pub source: String,
    pub license: String,
    pub version: String,
    pub rules: Vec<BioRule>,
}

/// Omitted clauses do not restrict; clauses are AND; rules of a species are OR.
#[derive(Debug, Clone, Deserialize)]
pub struct BioRule {
    pub species: String,
    pub body_class: Option<Vec<String>>,
    pub atmosphere_type: Option<Vec<String>>,
    pub gravity_min: Option<f64>,
    pub gravity_max: Option<f64>,
    pub temp_min: Option<f64>,
    pub temp_max: Option<f64>,
    pub pressure_max: Option<f64>,
    pub volcanism: Option<VolcanismRule>,
    pub star_class: Option<Vec<String>>,
    pub source: String,
}

/// `"any"` / `"none"` or a list of volcanism keywords (any of them).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum VolcanismRule {
    Keyword(String),
    AnyOf(Vec<String>),
}

impl BioRules {
    pub(crate) fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn matches(&self, species: &str, profile: &BodyProfile) -> bool {
        self.rules
            .iter()
            .any(|rule| rule.species == species && rule.matches(profile))
    }
}

fn in_range(value: Option<f64>, min: Option<f64>, max: Option<f64>) -> bool {
    if min.is_none() && max.is_none() {
        return true;
    }
    value.is_some_and(|v| min.is_none_or(|m| v >= m) && max.is_none_or(|m| v <= m))
}

fn star_matches(star: &str, pattern: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => star.starts_with(prefix),
        None => star == pattern,
    }
}

impl BioRule {
    pub fn matches(&self, profile: &BodyProfile) -> bool {
        let class_ok = self.body_class.as_ref().is_none_or(|classes| {
            profile
                .planet_class
                .as_deref()
                .is_some_and(|c| classes.iter().any(|x| x.eq_ignore_ascii_case(c)))
        });
        let atmosphere = atmosphere_of(profile);
        let atmosphere_ok = self
            .atmosphere_type
            .as_ref()
            .is_none_or(|types| types.iter().any(|t| t.eq_ignore_ascii_case(&atmosphere)));
        let star_ok = self.star_class.as_ref().is_none_or(|stars| {
            profile
                .star_class
                .as_deref()
                .is_some_and(|s| stars.iter().any(|p| star_matches(s, p)))
        });
        class_ok
            && atmosphere_ok
            && star_ok
            && in_range(profile.gravity_g, self.gravity_min, self.gravity_max)
            && in_range(profile.temperature_k, self.temp_min, self.temp_max)
            && in_range(profile.pressure_atm, None, self.pressure_max)
            && self.volcanism_matches(profile.volcanism.as_deref())
    }

    fn volcanism_matches(&self, volcanism: Option<&str>) -> bool {
        let text = volcanism.filter(|v| is_volcanic(v)).map(str::to_lowercase);
        match &self.volcanism {
            None => true,
            Some(VolcanismRule::Keyword(k)) if k.eq_ignore_ascii_case("any") => text.is_some(),
            Some(VolcanismRule::Keyword(k)) if k.eq_ignore_ascii_case("none") => text.is_none(),
            Some(VolcanismRule::Keyword(k)) => text.is_some_and(|t| t.contains(&k.to_lowercase())),
            Some(VolcanismRule::AnyOf(keys)) => {
                text.is_some_and(|t| keys.iter().any(|k| t.contains(&k.to_lowercase())))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Likelihood {
    High,
    Medium,
    Low,
    Unknown,
}

impl Likelihood {
    fn raised(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium | Self::High => Self::High,
            Self::Unknown => Self::Unknown,
        }
    }

    fn lowered(self) -> Self {
        match self {
            Self::High => Self::Medium,
            Self::Medium | Self::Low => Self::Low,
            Self::Unknown => Self::Unknown,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::High => 3,
            Self::Medium => 2,
            Self::Low => 1,
            Self::Unknown => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbSource {
    Rules,
    History,
    Canonn,
}

/// Predicted species; `species` is empty for an unknown-genus placeholder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub species: String,
    pub genus: String,
    pub value: u64,
    pub likelihood: Likelihood,
    pub sources: Vec<ProbSource>,
}

/// Species the commander completed, grouped by body similarity key.
#[derive(Debug, Clone, Default)]
pub struct HistoryStats {
    counts: HashMap<BodyProfileKey, HashMap<String, u32>>,
    totals: HashMap<BodyProfileKey, u32>,
}

impl HistoryStats {
    pub fn from_rows(rows: impl Iterator<Item = (BodyProfileKey, String)>) -> Self {
        let mut stats = Self::default();
        for (key, species) in rows {
            *stats
                .counts
                .entry(key.clone())
                .or_default()
                .entry(species)
                .or_default() += 1;
            *stats.totals.entry(key).or_default() += 1;
        }
        stats
    }

    pub fn count(&self, key: &BodyProfileKey, species: &str) -> u32 {
        self.counts
            .get(key)
            .and_then(|by_species| by_species.get(species))
            .copied()
            .unwrap_or(0)
    }

    pub fn total(&self, key: &BodyProfileKey) -> u32 {
        self.totals.get(key).copied().unwrap_or(0)
    }

    /// Parses the value produced by [`load_history`]; bad rows are skipped.
    pub fn from_task_value(value: &Value) -> Self {
        let rows = value["rows"].as_array().map_or(&[][..], Vec::as_slice);
        Self::from_rows(rows.iter().filter_map(|row| {
            let scan: Scan = serde_json::from_str(row[0].as_str()?).ok()?;
            let species = row[1].as_str()?.to_owned();
            let profile = body_profile(&scan, row[2].as_str());
            Some((BodyProfileKey::from(&profile), species))
        }))
    }
}

/// Bodies where the commander completed a species (`ScanOrganic` Analyse),
/// joined with their latest `Scan` (by `SystemAddress` + `BodyID`) and the
/// system's main star. Rows: `[scan_json, species, star_type|null]`.
pub fn load_history(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json(
            "WITH organic AS (
               SELECT DISTINCT json_extract(json, '$.SystemAddress') AS addr,
                      json_extract(json, '$.Body') AS body,
                      json_extract(json, '$.Species') AS species
               FROM event
               WHERE cmdr = ?1 AND name = 'ScanOrganic'
                 AND json_extract(json, '$.ScanType') = 'Analyse'
                 AND json_extract(json, '$.Species') IS NOT NULL),
             scans AS (
               SELECT id, ts, json,
                      json_extract(json, '$.SystemAddress') AS addr,
                      json_extract(json, '$.BodyID') AS body,
                      json_extract(json, '$.DistanceFromArrivalLS') AS dist,
                      json_extract(json, '$.StarType') AS star
               FROM event
               WHERE cmdr = ?1 AND name = 'Scan'
                 AND json_extract(json, '$.SystemAddress') IN (SELECT addr FROM organic))
             SELECT scan, species, star FROM (SELECT
               (SELECT b.json FROM scans b WHERE b.addr = o.addr AND b.body = o.body
                ORDER BY b.ts DESC, b.id DESC LIMIT 1) AS scan,
               o.species,
               (SELECT p.star FROM scans p WHERE p.addr = o.addr AND p.dist = 0
                  AND p.star IS NOT NULL AND p.star <> ''
                ORDER BY p.ts ASC, p.id ASC LIMIT 1) AS star,
               o.addr AS addr, o.body AS body
             FROM organic o)
             WHERE scan IS NOT NULL
             ORDER BY addr, body, species",
            &[json!(cmdr)],
        )?;
        Ok(json!({ "cmdr": cmdr, "rows": rows }))
    })
}

fn same_name(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Best Canonn weight for a species, matched by codex symbol or English name.
fn canonn_weight(canonn: Option<&RemoteBodyBio>, species: &SpeciesInfo) -> Option<f64> {
    canonn?
        .hints
        .iter()
        .filter(|h| same_name(&h.species, &species.symbol) || same_name(&h.species, &species.name))
        .map(|h| h.weight)
        .reduce(f64::max)
}

fn with_canonn(mut likelihood: Likelihood, weight: f64) -> Likelihood {
    if weight >= CANONN_RAISE {
        likelihood = likelihood.raised();
    } else if weight < CANONN_LOWER {
        likelihood = likelihood.lowered();
    }
    likelihood
}

/// Rules -> `Medium`; history and Canonn adjust. `allow_canonn_only` lets a
/// Canonn hint alone create a `Low` candidate (only inside DSS-confirmed genera).
fn rate_species(
    species: &SpeciesInfo,
    profile: &BodyProfile,
    key: &BodyProfileKey,
    history: &HistoryStats,
    canonn: Option<&RemoteBodyBio>,
    rules: &BioRules,
    allow_canonn_only: bool,
) -> Option<Candidate> {
    let mut sources = Vec::new();
    let mut likelihood = None;
    if rules.matches(&species.symbol, profile) {
        sources.push(ProbSource::Rules);
        likelihood = Some(Likelihood::Medium);
    }
    let cases = history.count(key, &species.symbol);
    if cases > 0 {
        sources.push(ProbSource::History);
        likelihood = Some(match (likelihood, cases >= HISTORY_STRONG) {
            (Some(_), true) => Likelihood::High,
            (Some(level), false) => level.raised(),
            (None, true) => Likelihood::Medium,
            (None, false) => Likelihood::Low,
        });
    }
    if let Some(weight) = canonn_weight(canonn, species) {
        let base = match likelihood {
            Some(level) => Some(level),
            None if allow_canonn_only => Some(Likelihood::Low),
            None => None,
        };
        if let Some(level) = base {
            sources.push(ProbSource::Canonn);
            likelihood = Some(with_canonn(level, weight));
        }
    }
    Some(Candidate {
        species: species.symbol.clone(),
        genus: species.genus.clone(),
        value: species.value,
        likelihood: likelihood?,
        sources,
    })
}

fn unknown_genus(genus: &str) -> Candidate {
    Candidate {
        species: String::new(),
        genus: genus.to_owned(),
        value: 0,
        likelihood: Likelihood::Unknown,
        sources: Vec::new(),
    }
}

/// Predicts the species of a body. Before the DSS (`confirmed_genera` is
/// `None`) every genus of the table is considered and only supported species
/// are returned; after it, only confirmed genera, and a confirmed genus with
/// no supported species becomes a single `Unknown` candidate (never invented).
/// Confirmed genera match by codex symbol or table name. Sorted by value desc.
pub fn predict(
    profile: &BodyProfile,
    confirmed_genera: Option<&[String]>,
    history: &HistoryStats,
    canonn: Option<&RemoteBodyBio>,
    rules: &BioRules,
    table: &SpeciesTable,
) -> Vec<Candidate> {
    let key = BodyProfileKey::from(profile);
    let rate = |species: &SpeciesInfo, canonn_only: bool| {
        rate_species(species, profile, &key, history, canonn, rules, canonn_only)
    };
    let mut candidates: Vec<Candidate> = match confirmed_genera {
        None => table
            .genera
            .iter()
            .flat_map(|genus| &genus.species)
            .filter_map(|species| rate(species, false))
            .collect(),
        Some(confirmed) => {
            let mut out = Vec::new();
            let mut seen: Vec<&str> = Vec::new();
            for wanted in confirmed {
                let genus = table
                    .genera
                    .iter()
                    .find(|g| same_name(&g.symbol, wanted) || same_name(&g.name, wanted));
                let symbol = genus.map_or(wanted.as_str(), |g| g.symbol.as_str());
                if seen.contains(&symbol) {
                    continue;
                }
                seen.push(symbol);
                let found: Vec<Candidate> = genus
                    .map(|g| g.species.iter().filter_map(|s| rate(s, true)).collect())
                    .unwrap_or_default();
                if found.is_empty() {
                    out.push(unknown_genus(symbol));
                } else {
                    out.extend(found);
                }
            }
            out
        }
    };
    candidates.sort_by(|a, b| {
        b.value
            .cmp(&a.value)
            .then(b.likelihood.rank().cmp(&a.likelihood.rank()))
            .then_with(|| a.genus.cmp(&b.genus))
            .then_with(|| a.species.cmp(&b.species))
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{bio_rules, species_table};
    use tmc_model::net::RemoteSpeciesHint;

    const AURASUS: &str = "$Codex_Ent_Bacterial_01_Name;";
    const BACTERIUM: &str = "$Codex_Ent_Bacterial_Genus_Name;";
    const ANEMONE: &str = "$Codex_Ent_Sphere_Genus_Name;";

    fn profile() -> BodyProfile {
        BodyProfile {
            planet_class: Some("Rocky body".into()),
            atmosphere_type: Some("CarbonDioxide".into()),
            gravity_g: Some(0.12),
            temperature_k: Some(170.0),
            pressure_atm: Some(0.01),
            volcanism: None,
            star_class: Some("K".into()),
        }
    }

    /// One rule only: Bacterium Aurasus on rocky CO2 bodies up to 0.27 g.
    fn single_rule() -> BioRules {
        BioRules::from_json(
            r#"{"source":"test","license":"test","version":"test","rules":[
              {"species":"$Codex_Ent_Bacterial_01_Name;","body_class":["Rocky body"],
               "atmosphere_type":["CarbonDioxide"],"gravity_max":0.27,"source":"synthetic"}]}"#,
        )
        .unwrap()
    }

    fn no_rules() -> BioRules {
        BioRules::from_json(r#"{"source":"t","license":"t","version":"t","rules":[]}"#).unwrap()
    }

    fn history(species: &str, cases: usize) -> HistoryStats {
        let key = BodyProfileKey::from(&profile());
        HistoryStats::from_rows((0..cases).map(|_| (key.clone(), species.to_owned())))
    }

    fn hint(weight: f64) -> RemoteBodyBio {
        RemoteBodyBio {
            hints: vec![RemoteSpeciesHint {
                genus: "Bacterium".into(),
                species: "Bacterium Aurasus".into(),
                weight,
            }],
        }
    }

    fn only<'a>(candidates: &'a [Candidate], species: &str) -> &'a Candidate {
        let found: Vec<_> = candidates.iter().filter(|c| c.species == species).collect();
        assert_eq!(found.len(), 1, "{candidates:?}");
        found[0]
    }

    #[test]
    fn body_profile_converts_units() {
        let scan: Scan = serde_json::from_str(
            r#"{"BodyName":"X 1","PlanetClass":"Rocky body","AtmosphereType":"CarbonDioxide",
                "SurfaceGravity":19.6133,"SurfacePressure":202650.0,"SurfaceTemperature":250.0,
                "Volcanism":""}"#,
        )
        .unwrap();
        let profile = body_profile(&scan, Some("K"));
        assert!((profile.gravity_g.unwrap() - 2.000).abs() < 0.001);
        assert!((profile.pressure_atm.unwrap() - 2.000).abs() < 0.001);
        assert_eq!(profile.temperature_k, Some(250.0));
        assert_eq!(profile.planet_class.as_deref(), Some("Rocky body"));
        assert_eq!(profile.atmosphere_type.as_deref(), Some("CarbonDioxide"));
        assert_eq!(profile.volcanism, None);
        assert_eq!(profile.star_class.as_deref(), Some("K"));
    }

    #[test]
    fn primary_star_is_star_scan_at_zero_ls() {
        let star: Scan =
            serde_json::from_str(r#"{"BodyName":"A","StarType":"K","DistanceFromArrivalLS":0.0}"#)
                .unwrap();
        let far: Scan =
            serde_json::from_str(r#"{"BodyName":"B","StarType":"M","DistanceFromArrivalLS":12.5}"#)
                .unwrap();
        let planet: Scan = serde_json::from_str(
            r#"{"BodyName":"A 1","PlanetClass":"Icy body","DistanceFromArrivalLS":0.0}"#,
        )
        .unwrap();
        assert!(is_primary_star(&star));
        assert!(!is_primary_star(&far));
        assert!(!is_primary_star(&planet));
    }

    #[test]
    fn profile_key_uses_bins() {
        let key = BodyProfileKey::from(&profile());
        assert_eq!(key.body_class, "Rocky body");
        assert_eq!(key.atmosphere, "carbondioxide");
        assert_eq!(key.gravity_bin, Some(2)); // 0.12 g / 0.05
        assert_eq!(key.temperature_bin, Some(8)); // 170 K / 20
        assert!(!key.volcanic);
        assert_eq!(key.star_class, "K");
        let mut near = profile();
        near.gravity_g = Some(0.14);
        near.temperature_k = Some(179.0);
        assert_eq!(BodyProfileKey::from(&near), key);
        near.gravity_g = Some(0.16);
        assert_ne!(BodyProfileKey::from(&near), key);
    }

    #[test]
    fn rule_match_gives_medium_from_rules() {
        let candidates = predict(
            &profile(),
            None,
            &HistoryStats::default(),
            None,
            &single_rule(),
            species_table(),
        );
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0],
            Candidate {
                species: AURASUS.into(),
                genus: BACTERIUM.into(),
                value: 1_000_000,
                likelihood: Likelihood::Medium,
                sources: vec![ProbSource::Rules],
            }
        );
        let mut heavy = profile();
        heavy.gravity_g = Some(0.5);
        let none = predict(
            &heavy,
            None,
            &HistoryStats::default(),
            None,
            &single_rule(),
            species_table(),
        );
        assert!(none.is_empty());
    }

    #[test]
    fn history_with_three_cases_gives_high() {
        let candidates = predict(
            &profile(),
            None,
            &history(AURASUS, 3),
            None,
            &single_rule(),
            species_table(),
        );
        let c = only(&candidates, AURASUS);
        assert_eq!(c.likelihood, Likelihood::High);
        assert_eq!(c.sources, vec![ProbSource::Rules, ProbSource::History]);
        // 1–2 cases raise one level (Medium -> High).
        let one = predict(
            &profile(),
            None,
            &history(AURASUS, 1),
            None,
            &single_rule(),
            species_table(),
        );
        assert_eq!(only(&one, AURASUS).likelihood, Likelihood::High);
    }

    #[test]
    fn history_only_without_rule_is_low_with_one_case() {
        let one = predict(
            &profile(),
            None,
            &history(AURASUS, 1),
            None,
            &no_rules(),
            species_table(),
        );
        let c = only(&one, AURASUS);
        assert_eq!(c.likelihood, Likelihood::Low);
        assert_eq!(c.sources, vec![ProbSource::History]);
        assert_eq!(c.value, 1_000_000);
        let three = predict(
            &profile(),
            None,
            &history(AURASUS, 3),
            None,
            &no_rules(),
            species_table(),
        );
        assert_eq!(only(&three, AURASUS).likelihood, Likelihood::Medium);
        assert_eq!(only(&three, AURASUS).sources, vec![ProbSource::History]);
    }

    #[test]
    fn canonn_hint_raises_one_level_and_is_listed_as_source() {
        let candidates = predict(
            &profile(),
            None,
            &HistoryStats::default(),
            Some(&hint(0.6)),
            &single_rule(),
            species_table(),
        );
        let c = only(&candidates, AURASUS);
        assert_eq!(c.likelihood, Likelihood::High);
        assert_eq!(c.sources, vec![ProbSource::Rules, ProbSource::Canonn]);
        // A middle weight keeps the level but still lists Canonn.
        let mid = predict(
            &profile(),
            None,
            &HistoryStats::default(),
            Some(&hint(0.3)),
            &single_rule(),
            species_table(),
        );
        assert_eq!(only(&mid, AURASUS).likelihood, Likelihood::Medium);
        assert_eq!(
            only(&mid, AURASUS).sources,
            vec![ProbSource::Rules, ProbSource::Canonn]
        );
    }

    #[test]
    fn canonn_low_weight_lowers_medium_to_low() {
        let candidates = predict(
            &profile(),
            None,
            &HistoryStats::default(),
            Some(&hint(0.05)),
            &single_rule(),
            species_table(),
        );
        let c = only(&candidates, AURASUS);
        assert_eq!(c.likelihood, Likelihood::Low);
        assert_eq!(c.sources, vec![ProbSource::Rules, ProbSource::Canonn]);
        // Lowering stops at Low: history-only Low stays Low.
        let floor = predict(
            &profile(),
            None,
            &history(AURASUS, 1),
            Some(&hint(0.05)),
            &no_rules(),
            species_table(),
        );
        assert_eq!(only(&floor, AURASUS).likelihood, Likelihood::Low);
    }

    #[test]
    fn without_canonn_layers_one_and_two_still_work() {
        let with_history = history(AURASUS, 2);
        let without = predict(
            &profile(),
            None,
            &with_history,
            None,
            &single_rule(),
            species_table(),
        );
        assert_eq!(
            without,
            vec![Candidate {
                species: AURASUS.into(),
                genus: BACTERIUM.into(),
                value: 1_000_000,
                likelihood: Likelihood::High,
                sources: vec![ProbSource::Rules, ProbSource::History],
            }]
        );
        // An empty Canonn answer behaves exactly like no Canonn.
        let empty = predict(
            &profile(),
            None,
            &with_history,
            Some(&RemoteBodyBio { hints: vec![] }),
            &single_rule(),
            species_table(),
        );
        assert_eq!(empty, without);
    }

    #[test]
    fn genus_without_rules_is_unknown_not_invented() {
        let confirmed = vec![
            ANEMONE.to_owned(),
            "$Codex_Ent_Thargoid_Genus_Name;".to_owned(),
        ];
        let candidates = predict(
            &profile(),
            Some(&confirmed),
            &HistoryStats::default(),
            None,
            bio_rules(),
            species_table(),
        );
        assert_eq!(
            candidates,
            vec![
                Candidate {
                    species: String::new(),
                    genus: ANEMONE.into(),
                    value: 0,
                    likelihood: Likelihood::Unknown,
                    sources: vec![],
                },
                Candidate {
                    species: String::new(),
                    genus: "$Codex_Ent_Thargoid_Genus_Name;".into(),
                    value: 0,
                    likelihood: Likelihood::Unknown,
                    sources: vec![],
                },
            ]
        );
    }

    #[test]
    fn dss_genuses_filter_candidates() {
        let all = predict(
            &profile(),
            None,
            &HistoryStats::default(),
            None,
            bio_rules(),
            species_table(),
        );
        assert!(all.iter().any(|c| c.genus != BACTERIUM), "{all:?}");
        for confirmed in [vec!["Bacterium".to_owned()], vec![BACTERIUM.to_owned()]] {
            let filtered = predict(
                &profile(),
                Some(&confirmed),
                &HistoryStats::default(),
                None,
                bio_rules(),
                species_table(),
            );
            assert!(!filtered.is_empty());
            assert!(
                filtered.iter().all(|c| c.genus == BACTERIUM),
                "{filtered:?}"
            );
            // Rocky CO2 at 170 K: Bacterium Aurasus is the bacterium rule that applies.
            assert_eq!(only(&filtered, AURASUS).likelihood, Likelihood::Medium);
        }
    }

    #[test]
    fn candidates_sorted_by_value_desc() {
        let table = SpeciesTable::from_json(
            r#"{"source":"s","license":"l","version":"v","genera":[
              {"genus":"$G;","name":"G","ccr_m":100,"provenance":"p","date":"2026-10-18","verified":false,
               "species":[
                 {"species":"$A;","name":"A","value":1000000,"provenance":"p","date":"2026-10-18","verified":false},
                 {"species":"$B;","name":"B","value":500000,"provenance":"p","date":"2026-10-18","verified":false},
                 {"species":"$C;","name":"C","value":2000000,"provenance":"p","date":"2026-10-18","verified":false}]}]}"#,
        )
        .unwrap();
        let rules = BioRules::from_json(
            r#"{"source":"t","license":"t","version":"t","rules":[
              {"species":"$A;","source":"s"},{"species":"$B;","source":"s"},
              {"species":"$C;","source":"s"},{"species":"$Missing;","source":"s"}]}"#,
        )
        .unwrap();
        let candidates = predict(
            &profile(),
            None,
            &HistoryStats::default(),
            None,
            &rules,
            &table,
        );
        let values: Vec<u64> = candidates.iter().map(|c| c.value).collect();
        assert_eq!(values, vec![2_000_000, 1_000_000, 500_000]);
    }

    #[test]
    fn bio_rules_have_source_for_every_rule() {
        let rules = bio_rules();
        assert_eq!(rules.version, "2026-10-18");
        assert!(rules.license.contains("PolyForm"));
        assert_eq!(rules.rules.len(), 90);
        for rule in &rules.rules {
            assert!(!rule.source.trim().is_empty(), "{}", rule.species);
            assert!(
                species_table().species(&rule.species).is_some(),
                "{}",
                rule.species
            );
        }
    }

    #[test]
    fn embedded_rules_cover_odyssey_genera_only() {
        let table = species_table();
        let covered: std::collections::BTreeSet<&str> = bio_rules()
            .rules
            .iter()
            .filter_map(|rule| table.species(&rule.species))
            .map(|species| species.genus.as_str())
            .collect();
        assert_eq!(covered.len(), 15);
        assert!(covered.contains(BACTERIUM));
        assert!(!covered.contains(ANEMONE));
    }

    #[test]
    fn rule_clauses_match_volcanism_star_and_ranges() {
        let rules = BioRules::from_json(
            r#"{"source":"t","license":"t","version":"t","rules":[
              {"species":"$Codex_Ent_Fumerolas_04_Name;","volcanism":["water"],"source":"s"},
              {"species":"$Codex_Ent_Bacterial_07_Name;","volcanism":"any","source":"s"},
              {"species":"$Codex_Ent_Bacterial_02_Name;","volcanism":"none","source":"s"},
              {"species":"$Codex_Ent_Electricae_01_Name;","star_class":["D*"],"source":"s"},
              {"species":"$Codex_Ent_Tussocks_01_Name;","temp_min":145,"temp_max":155,
               "gravity_min":0.1,"pressure_max":0.1,"source":"s"}]}"#,
        )
        .unwrap();
        let mut body = profile();
        body.volcanism = Some("major water geysers volcanism".into());
        body.star_class = Some("DA".into());
        body.temperature_k = Some(150.0);
        let species: Vec<String> = predict(
            &body,
            None,
            &HistoryStats::default(),
            None,
            &rules,
            species_table(),
        )
        .into_iter()
        .map(|c| c.species)
        .collect();
        let mut species = species;
        species.sort();
        assert_eq!(
            species,
            vec![
                "$Codex_Ent_Bacterial_07_Name;",
                "$Codex_Ent_Electricae_01_Name;",
                "$Codex_Ent_Fumerolas_04_Name;",
                "$Codex_Ent_Tussocks_01_Name;",
            ]
        );
        // Missing data never satisfies a clause that needs it.
        let mut blank = profile();
        blank.temperature_k = None;
        blank.star_class = None;
        let none = predict(
            &blank,
            None,
            &HistoryStats::default(),
            None,
            &rules,
            species_table(),
        );
        assert_eq!(
            none.iter().map(|c| c.species.as_str()).collect::<Vec<_>>(),
            vec!["$Codex_Ent_Bacterial_02_Name;"]
        );
    }

    #[test]
    fn history_stats_count_and_total() {
        let key = BodyProfileKey::from(&profile());
        let mut other = profile();
        other.planet_class = Some("Icy body".into());
        let other = BodyProfileKey::from(&other);
        let stats = HistoryStats::from_rows(
            [
                (key.clone(), AURASUS.to_owned()),
                (key.clone(), AURASUS.to_owned()),
                (key.clone(), "$Codex_Ent_Tussocks_01_Name;".to_owned()),
                (other.clone(), AURASUS.to_owned()),
            ]
            .into_iter(),
        );
        assert_eq!(stats.count(&key, AURASUS), 2);
        assert_eq!(stats.count(&key, "$Codex_Ent_Tussocks_01_Name;"), 1);
        assert_eq!(stats.count(&other, AURASUS), 1);
        assert_eq!(stats.total(&key), 3);
        assert_eq!(stats.total(&other), 1);
        assert_eq!(stats.count(&other, "$None;"), 0);
    }

    #[test]
    fn load_history_joins_scan_and_analysed_organics_for_cmdr() {
        let store = tmc_core::Store::open_in_memory().unwrap();
        store
            .exec_json(
                "INSERT INTO journal_file(id,name,size,offset) VALUES (1,'j.log',0,0)",
                &[],
            )
            .unwrap();
        let rows: [(&str, &str, &str); 8] = [
            (
                "Scan",
                "TESTER",
                r#"{"event":"Scan","BodyName":"S A","SystemAddress":7,"BodyID":0,"StarType":"K","DistanceFromArrivalLS":0.0}"#,
            ),
            (
                "Scan",
                "TESTER",
                r#"{"event":"Scan","BodyName":"S A 1","SystemAddress":7,"BodyID":3,"PlanetClass":"Rocky body","AtmosphereType":"CarbonDioxide","SurfaceGravity":1.1768,"SurfaceTemperature":170.0,"DistanceFromArrivalLS":300.0}"#,
            ),
            (
                "ScanOrganic",
                "TESTER",
                r#"{"event":"ScanOrganic","ScanType":"Log","SystemAddress":7,"Body":3,"Genus":"$Codex_Ent_Bacterial_Genus_Name;","Species":"$Codex_Ent_Bacterial_01_Name;"}"#,
            ),
            (
                "ScanOrganic",
                "TESTER",
                r#"{"event":"ScanOrganic","ScanType":"Analyse","SystemAddress":7,"Body":3,"Genus":"$Codex_Ent_Bacterial_Genus_Name;","Species":"$Codex_Ent_Bacterial_01_Name;"}"#,
            ),
            (
                "ScanOrganic",
                "TESTER",
                r#"{"event":"ScanOrganic","ScanType":"Analyse","SystemAddress":7,"Body":3,"Genus":"$Codex_Ent_Bacterial_Genus_Name;","Species":"$Codex_Ent_Bacterial_01_Name;"}"#,
            ),
            (
                "ScanOrganic",
                "TESTER",
                r#"{"event":"ScanOrganic","ScanType":"Sample","SystemAddress":7,"Body":3,"Genus":"$Codex_Ent_Tussocks_Genus_Name;","Species":"$Codex_Ent_Tussocks_01_Name;"}"#,
            ),
            (
                "ScanOrganic",
                "OTRO",
                r#"{"event":"ScanOrganic","ScanType":"Analyse","SystemAddress":7,"Body":3,"Genus":"$Codex_Ent_Tussocks_Genus_Name;","Species":"$Codex_Ent_Tussocks_02_Name;"}"#,
            ),
            (
                "ScanOrganic",
                "TESTER",
                r#"{"event":"ScanOrganic","ScanType":"Analyse","SystemAddress":8,"Body":1,"Genus":"$Codex_Ent_Bacterial_Genus_Name;","Species":"$Codex_Ent_Bacterial_02_Name;"}"#,
            ),
        ];
        for (line, (name, cmdr, raw)) in rows.iter().enumerate() {
            store
                .exec_json(
                    "INSERT INTO event(file_id,line,ts,name,cmdr,json) VALUES (1,?1,?2,?3,?4,?5)",
                    &[
                        json!(line as i64 + 1),
                        json!(format!("2026-10-18T10:00:{:02}.000000000Z", line)),
                        json!(name),
                        json!(cmdr),
                        json!(raw),
                    ],
                )
                .unwrap();
        }
        let value = load_history("TESTER".into())(&store).unwrap();
        assert_eq!(value["cmdr"], "TESTER");
        // Analyse twice on the same body counts once; Sample, other cmdr and
        // organics without a body Scan are ignored.
        assert_eq!(value["rows"].as_array().unwrap().len(), 1);
        let stats = HistoryStats::from_task_value(&value);
        let key = BodyProfileKey::from(&profile());
        assert_eq!(stats.count(&key, AURASUS), 1);
        assert_eq!(stats.total(&key), 1);
        assert_eq!(stats.count(&key, "$Codex_Ent_Tussocks_01_Name;"), 0);
    }
}

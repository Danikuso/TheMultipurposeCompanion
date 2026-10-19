use super::*;
use chrono::Duration;
use serde_json::json;
use tmc_model::events::parse_line;
use tmc_model::net::{NetFailure, NetRequest, NetSource, RemoteSpeciesHint};
use tmc_model::status::Status;

const SYSTEM: u64 = 3_932_277_478_106;
const BODY: u32 = 5;
const AURASUS: &str = "$Codex_Ent_Bacterial_01_Name;";
const NEBULUS: &str = "$Codex_Ent_Bacterial_02_Name;";
const BACTERIUM: &str = "$Codex_Ent_Bacterial_Genus_Name;";

fn t(seconds: i64) -> DateTime<Utc> {
    "2026-04-12T21:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
}

struct H {
    m: Exobiology,
    state: GameState,
    clock: i64,
    replaying: bool,
    requests: Vec<NetRequest>,
    tasks: Vec<String>,
    store_tasks: Vec<(String, tmc_core::StoreTask)>,
    next_id: u64,
}

impl H {
    fn new() -> Self {
        Self {
            m: Exobiology::new(),
            state: GameState::default(),
            clock: 0,
            replaying: false,
            requests: Vec::new(),
            tasks: Vec::new(),
            store_tasks: Vec::new(),
            next_id: 1,
        }
    }

    fn send(&mut self, ev: SourceEvent) -> TopicSet {
        match &ev {
            SourceEvent::Journal { ev, .. } => {
                self.state.apply_event(ev);
            }
            SourceEvent::Status(s) => {
                self.state.apply_status(s);
            }
            _ => {}
        }
        let mut ctx = ModuleCtx::new("exobiology", self.replaying, self.next_id);
        let dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        self.tasks.extend(tasks.iter().map(|(tag, _)| tag.clone()));
        self.store_tasks.extend(tasks);
        dirty
    }

    fn journal(&mut self, name: &str, extra: &str) -> TopicSet {
        self.clock += 1;
        let line = format!(
            r#"{{"timestamp":"{}","event":"{name}"{extra}}}"#,
            t(self.clock).to_rfc3339()
        );
        self.line(&line)
    }

    fn line(&mut self, line: &str) -> TopicSet {
        self.send(SourceEvent::Journal {
            ev: parse_line(line).unwrap(),
            file: "s.log".into(),
            line: 1,
            replaying: self.replaying,
        })
    }

    fn start(&mut self) {
        self.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
        self.journal("LoadGame", r#","Commander":"CMDR TESTER","Odyssey":true"#);
        self.journal(
            "Location",
            &format!(
                r#", "StarSystem":"Kappa Testis", "SystemAddress":{SYSTEM}, "StarPos":[0,0,0]"#
            ),
        );
    }

    fn scan_body(&mut self, class: &str, atmosphere: &str, footfalled: bool) {
        self.journal(
            "Scan",
            &format!(
                r#","ScanType":"Detailed","BodyName":"Kappa Testis A 5 a","BodyID":{BODY},"SystemAddress":{SYSTEM},"DistanceFromArrivalLS":512.3,"PlanetClass":"{class}","AtmosphereType":"{atmosphere}","SurfaceGravity":1.2,"SurfaceTemperature":170.0,"SurfacePressure":1000.0,"Landable":true,"WasDiscovered":true,"WasFootfalled":{footfalled}"#
            ),
        );
    }

    fn bio_signals(&mut self, count: u32) {
        self.journal(
            "FSSBodySignals",
            &format!(
                r#","BodyName":"Kappa Testis A 5 a","BodyID":{BODY},"SystemAddress":{SYSTEM},"Signals":[{{"Type":"$SAA_SignalType_Biological;","Count":{count}}}]"#
            ),
        );
    }

    fn dss(&mut self, genus: &str) {
        self.journal(
            "SAASignalsFound",
            &format!(
                r#","BodyName":"Kappa Testis A 5 a","BodyID":{BODY},"SystemAddress":{SYSTEM},"Signals":[{{"Type":"$SAA_SignalType_Biological;","Count":3}}],"Genuses":[{{"Genus":"{genus}","Genus_Localised":"Bacterium"}}]"#
            ),
        );
    }

    fn touchdown(&mut self) {
        self.journal(
            "Touchdown",
            &format!(
                r#","PlayerControlled":true,"Latitude":10.0,"Longitude":20.0,"Body":"Kappa Testis A 5 a","BodyID":{BODY},"SystemAddress":{SYSTEM},"OnPlanet":true"#
            ),
        );
    }

    fn organic(&mut self, kind: &str, species: &str) -> TopicSet {
        self.journal(
            "ScanOrganic",
            &format!(
                r#","ScanType":"{kind}","Genus":"{BACTERIUM}","Species":"{species}","SystemAddress":{SYSTEM},"Body":{BODY}"#
            ),
        )
    }

    fn complete(&mut self, species: &str) {
        for kind in ["Log", "Sample", "Analyse"] {
            self.organic(kind, species);
        }
    }

    fn status(&mut self, n: u8) -> TopicSet {
        let text = match n {
            1 => include_str!("../../../../tests/fixtures/status/exo_01.json"),
            2 => include_str!("../../../../tests/fixtures/status/exo_02.json"),
            _ => include_str!("../../../../tests/fixtures/status/exo_03.json"),
        };
        self.send(SourceEvent::Status(Status::parse(text).unwrap()))
    }

    fn topic(&self, name: &str) -> Value {
        self.m.topic_value(name).unwrap_or(Value::Null)
    }

    fn candidates(&self) -> Vec<Value> {
        self.topic(TOPIC_BODY)["candidates"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    fn response(&mut self, key: &str, outcome: NetOutcome) {
        self.send(SourceEvent::Net(NetResponse {
            id: 1,
            module: "exobiology".into(),
            key: key.into(),
            source: NetSource::Canonn,
            fetched_at: t(100),
            outcome,
        }));
    }
}

fn bio_key() -> String {
    format!("bio:{SYSTEM}:{BODY}")
}

#[test]
fn touchdown_on_bio_body_sets_active_relevance() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.bio_signals(3);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.touchdown();
    assert_eq!(h.state.location.mode, FlightMode::Landed);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    assert_eq!(h.topic(TOPIC_BODY)["bio_signals"], 3);
    assert_eq!(h.topic(TOPIC_BODY)["body"], "Kappa Testis A 5 a");
    // Taking off leaves the planet surface: back to Normal.
    h.journal("Liftoff", "");
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
}

#[test]
fn body_topic_has_kind_and_atmosphere_keys() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky ice body", "CarbonDioxideRich", false);
    h.bio_signals(3);
    h.touchdown();
    let body = h.topic(TOPIC_BODY);
    assert_eq!(body["kind"], "Rocky ice body");
    assert_eq!(body["kind_key"], "rocky-ice");
    assert_eq!(body["atmosphere"], "CarbonDioxideRich");
    assert_eq!(body["atmosphere_key"], "carbon-dioxide-rich");
}

#[test]
fn atmosphere_key_is_kebab_case_and_none_when_empty() {
    assert_eq!(
        super::atmosphere_key(Some("CarbonDioxide")),
        "carbon-dioxide"
    );
    assert_eq!(super::atmosphere_key(Some("EarthLike")), "earth-like");
    assert_eq!(super::atmosphere_key(Some("Neon")), "neon");
    assert_eq!(super::atmosphere_key(Some("")), "none");
    assert_eq!(super::atmosphere_key(Some("None")), "none");
    assert_eq!(super::atmosphere_key(None), "none");
}

#[test]
fn touchdown_without_signals_stays_normal_until_scan_organic() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.organic("Log", AURASUS);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    // Still in progress in orbit-side hover: the open sampling keeps it active.
    h.journal("Liftoff", "");
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.organic("Analyse", AURASUS);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
}

#[test]
fn exact_species_replaces_same_genus_candidates() {
    let mut h = H::new();
    h.start();
    h.scan_body("Gas giant class I", "Water", false);
    h.dss(BACTERIUM);
    h.touchdown();
    let genus_total = species_table().species_of_genus(BACTERIUM).len();
    assert!(genus_total >= 3);
    let bacteria = |h: &H| -> Vec<String> {
        h.candidates()
            .iter()
            .filter(|c| c["genus"] == BACTERIUM)
            .map(|c| c["species"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(bacteria(&h).len(), genus_total);
    let topic = h.topic(TOPIC_BODY);
    assert_eq!(topic["confirmed_genera"][0]["genus"], BACTERIUM);
    assert_eq!(topic["confirmed_genera"][0]["name"], "Bacterium");
    assert_eq!(topic["dss"], true);

    h.organic("Log", NEBULUS);
    assert_eq!(bacteria(&h), vec![NEBULUS.to_owned()]);
    let only = &h.candidates()[0];
    assert_eq!(only["status"], "in-progress");
    assert_eq!(only["progress"], 1);
    assert_eq!(only["species_name"], "Bacterium Nebulus");
    assert_eq!(only["genus_name"], "Bacterium");
    assert_eq!(only["min_m"], 500);
    h.organic("Sample", NEBULUS);
    h.organic("Analyse", NEBULUS);
    let done = &h.candidates()[0];
    assert_eq!(
        (done["status"].as_str(), done["progress"].is_null()),
        (Some("done"), true)
    );
    assert_eq!(h.topic(TOPIC_BODY)["completed"], 1);
}

#[test]
fn localised_names_from_the_journal_win_over_the_latin_name() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    h.journal(
        "ScanOrganic",
        &format!(
            r#","ScanType":"Log","Genus":"{BACTERIUM}","Genus_Localised":"Bacteria","Species":"{AURASUS}","Species_Localised":"Bacterio Aurasus","SystemAddress":{SYSTEM},"Body":{BODY}"#
        ),
    );
    let s = h.topic(TOPIC_SAMPLING);
    assert_eq!(s["species"], AURASUS);
    assert_eq!(s["species_name"], "Bacterio Aurasus");
    assert_eq!(s["genus_name"], "Bacteria");
}

fn footfalled_scan(value: Option<bool>) -> Scan {
    Scan {
        was_footfalled: value,
        ..Default::default()
    }
}

#[test]
fn first_logged_estimate_uses_footfall_then_discovery() {
    assert_eq!(
        first_logged_estimate(Some(&footfalled_scan(Some(false))), None),
        Some(true)
    );
    assert_eq!(
        first_logged_estimate(Some(&footfalled_scan(Some(true))), Some(false)),
        Some(false)
    );
    assert_eq!(
        first_logged_estimate(Some(&footfalled_scan(None)), Some(false)),
        Some(true)
    );
    assert_eq!(first_logged_estimate(None, Some(false)), Some(true));
    assert_eq!(first_logged_estimate(None, Some(true)), Some(false));
    assert_eq!(first_logged_estimate(None, None), None);
}

#[test]
fn unsold_bio_equals_following_sell_organic_data() {
    let mut h = H::new();
    let log = include_str!("../../../../tests/fixtures/journal/exobiology_session.log");
    let mut sold = None;
    let mut before = None;
    for line in log.lines() {
        let parsed = parse_line(line).unwrap();
        if let EventKind::SellOrganicData(s) = &parsed.kind {
            before = Some(h.topic(TOPIC_UNSOLD));
            sold = Some(s.bio_data.iter().map(|b| b.value + b.bonus).sum::<u64>());
        }
        h.line(line);
    }
    // Aurasus pays 1 000 000 and the footfall flag gives x5: 5 000 000.
    assert_eq!(sold, Some(5_000_000));
    assert_eq!(before.unwrap(), json!({"total": 5_000_000, "species": 1}));
    assert_eq!(h.topic(TOPIC_UNSOLD), json!({"total": 0, "species": 0}));
    // The estimate matched the real bonus.
    assert_eq!(h.m.estimate_accuracy(), (1, 1));
    // The Nebulus sampling (discarded nothing, still open) survives the sale.
    assert_eq!(h.topic(TOPIC_SAMPLING)["species"], NEBULUS);
}

#[test]
fn unsold_sums_value_plus_estimated_bonus_per_species() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", true);
    h.touchdown();
    h.complete(AURASUS);
    // Footfalled body: no bonus.
    assert_eq!(
        h.topic(TOPIC_UNSOLD),
        json!({"total": 1_000_000, "species": 1})
    );
    let nebulus = species_table().species(NEBULUS).unwrap().value;
    h.organic("Log", NEBULUS);
    let sampling = h.topic(TOPIC_SAMPLING);
    assert_eq!(sampling["unsold_after"], 1_000_000 + nebulus);
}

#[test]
fn died_zeroes_unsold_bio() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    h.complete(AURASUS);
    h.complete(NEBULUS);
    let nebulus = species_table().species(NEBULUS).unwrap().value;
    assert_eq!(
        h.topic(TOPIC_UNSOLD),
        json!({"total": 5 * (1_000_000 + nebulus), "species": 2})
    );
    h.journal("Died", "");
    assert_eq!(h.topic(TOPIC_UNSOLD), json!({"total": 0, "species": 0}));
    assert!(h.topic(TOPIC_SAMPLING).is_null());
}

#[test]
fn switching_species_sets_discarded_species_in_topic() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    h.organic("Log", AURASUS);
    assert!(h.topic(TOPIC_SAMPLING)["discarded_species"].is_null());
    h.organic("Sample", AURASUS);
    h.organic("Log", NEBULUS);
    let s = h.topic(TOPIC_SAMPLING);
    assert_eq!(s["discarded_species"], AURASUS);
    assert_eq!(s["discarded_name"], "Bacterium Aurasus");
    assert_eq!(
        (s["species"].as_str(), s["points"].as_u64()),
        (Some(NEBULUS), Some(1))
    );
}

#[test]
fn canonn_request_on_bio_signals_only_when_online() {
    let bio_requests = |h: &H| -> Vec<(String, NetQuery)> {
        h.requests
            .iter()
            .map(|r| (r.key.clone(), r.query.clone()))
            .collect()
    };
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.bio_signals(3);
    let sent = bio_requests(&h);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, bio_key());
    let NetQuery::BodyBio {
        system_address,
        star_system,
        body_id,
        body_name,
        profile,
    } = &sent[0].1
    else {
        panic!("BodyBio expected")
    };
    assert_eq!((*system_address, *body_id), (SYSTEM, BODY));
    assert_eq!(star_system, "Kappa Testis");
    assert_eq!(body_name, "Kappa Testis A 5 a");
    assert_eq!(profile.planet_class.as_deref(), Some("Rocky body"));
    // Landing on the same body does not repeat the request while in flight.
    h.touchdown();
    assert_eq!(h.requests.len(), 1);

    let mut offline = H::new();
    offline.send(SourceEvent::Settings(ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    }));
    offline.start();
    offline.scan_body("Rocky body", "CarbonDioxide", false);
    offline.bio_signals(3);
    assert!(offline.requests.is_empty());

    let mut legacy = H::new();
    legacy.state.galaxy = Galaxy::Legacy;
    legacy.start();
    legacy.state.galaxy = Galaxy::Legacy;
    legacy.scan_body("Rocky body", "CarbonDioxide", false);
    legacy.bio_signals(3);
    assert!(legacy.requests.is_empty());

    // No biological signals and not landed: nothing to ask.
    let mut quiet = H::new();
    quiet.start();
    quiet.scan_body("Rocky body", "CarbonDioxide", false);
    quiet.bio_signals(0);
    assert!(quiet.requests.is_empty());

    // Replay discards the request; the first live event repeats it.
    let mut replay = H::new();
    replay.replaying = true;
    replay.start();
    replay.scan_body("Rocky body", "CarbonDioxide", false);
    replay.bio_signals(3);
    assert!(replay.requests.is_empty());
    replay.replaying = false;
    replay.journal("Music", "");
    assert_eq!(replay.requests.len(), 1);
}

#[test]
fn canonn_response_for_current_body_adds_its_source() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.bio_signals(3);
    let before = h.candidates();
    let aurasus = before.iter().find(|c| c["species"] == AURASUS).unwrap();
    assert_eq!(aurasus["sources"], json!(["rules"]));
    h.response(
        &bio_key(),
        NetOutcome::BodyBio(RemoteBodyBio {
            hints: vec![RemoteSpeciesHint {
                genus: BACTERIUM.into(),
                species: AURASUS.into(),
                weight: 0.9,
            }],
        }),
    );
    let after = h.candidates();
    let aurasus = after.iter().find(|c| c["species"] == AURASUS).unwrap();
    assert_eq!(aurasus["sources"], json!(["rules", "canonn"]));
    assert_eq!(aurasus["likelihood"], "high");
}

#[test]
fn late_canonn_response_for_other_body_is_ignored() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.bio_signals(3);
    let before = h.candidates();
    h.response(
        &format!("bio:{SYSTEM}:9"),
        NetOutcome::BodyBio(RemoteBodyBio {
            hints: vec![RemoteSpeciesHint {
                genus: BACTERIUM.into(),
                species: AURASUS.into(),
                weight: 0.9,
            }],
        }),
    );
    let after = h.candidates();
    assert_eq!(before, after);
    assert!(
        after
            .iter()
            .all(|c| !c["sources"].as_array().unwrap().contains(&json!("canonn")))
    );
}

#[test]
fn disabled_or_failed_canonn_leaves_candidates_unchanged() {
    for outcome in [
        NetOutcome::Disabled,
        NetOutcome::NoData,
        NetOutcome::Failed(NetFailure::Down),
    ] {
        let mut h = H::new();
        h.start();
        h.scan_body("Rocky body", "CarbonDioxide", false);
        h.bio_signals(3);
        let before = h.candidates();
        assert!(!before.is_empty());
        h.response(&bio_key(), outcome);
        assert_eq!(h.candidates(), before);
    }
}

#[test]
fn disabled_canonn_is_not_asked_again_but_failed_is_after_cooldown() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.bio_signals(3);
    h.response(&bio_key(), NetOutcome::Disabled);
    h.journal("Music", "");
    assert_eq!(h.requests.len(), 1);
    let mut f = H::new();
    f.start();
    f.scan_body("Rocky body", "CarbonDioxide", false);
    f.bio_signals(3);
    f.response(&bio_key(), NetOutcome::Failed(NetFailure::Down));
    f.journal("Music", "");
    assert_eq!(f.requests.len(), 1);
    f.clock += 400;
    f.journal("Music", "");
    assert_eq!(f.requests.len(), 2);
}

#[test]
fn sampling_topic_reports_distance_direction_and_remaining() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    h.status(1);
    h.organic("Log", AURASUS);
    h.status(2);
    h.organic("Sample", AURASUS);
    let dirty = h.status(3);
    assert!(dirty.contains(TOPIC_SAMPLING));
    let s = h.topic(TOPIC_SAMPLING);
    assert_eq!(s["points"], 2);
    assert_eq!(s["min_m"], 500);
    assert_eq!(s["complete"], false);
    // Nearest previous point is the second sample at (10, 20.5).
    let distance = s["distance_m"].as_f64().unwrap();
    assert_eq!(distance, 12_891.0);
    assert_eq!(s["remaining_m"], 0.0);
    assert_eq!(s["direction"]["compass"], "W");
    assert_eq!(s["direction"]["rel"], "behind");
    // Same status again changes nothing.
    assert!(h.status(3).is_empty());
}

#[test]
fn remaining_distance_counts_down_to_the_genus_minimum() {
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    h.send(SourceEvent::Status(
        Status::parse(
            r#"{"timestamp":"2026-04-12T21:06:00Z","event":"Status","Flags":0,"Latitude":0.0,"Longitude":0.0,"Heading":0,"PlanetRadius":1500000.0}"#,
        )
        .unwrap(),
    ));
    h.organic("Log", AURASUS);
    // About 261.8 m east of the logged point (0.01 deg at 1500 km radius).
    h.send(SourceEvent::Status(
        Status::parse(
            r#"{"timestamp":"2026-04-12T21:06:05Z","event":"Status","Flags":0,"Latitude":0.0,"Longitude":0.01,"Heading":0,"PlanetRadius":1500000.0}"#,
        )
        .unwrap(),
    ));
    let s = h.topic(TOPIC_SAMPLING);
    assert_eq!(s["distance_m"], 262.0);
    assert_eq!(s["remaining_m"], 238.0);
    assert_eq!(s["direction"]["compass"], "W");
    assert_eq!(s["direction"]["rel"], "left");
}

#[test]
fn startup_requests_samples_history_and_unsold_once_per_commander() {
    let mut h = H::new();
    h.start();
    assert_eq!(h.tasks, vec![TAG_OPEN, TAG_HISTORY, TAG_UNSOLD]);
    h.journal("Music", "");
    assert_eq!(h.tasks.len(), 3);
}

#[test]
fn stored_results_for_another_commander_are_ignored() {
    let mut h = H::new();
    h.start();
    let rows = json!([[
        SYSTEM,
        BODY,
        AURASUS,
        "2026-04-12T21:00:30.000000000Z",
        null
    ]]);
    h.send(SourceEvent::Stored {
        module: "exobiology".into(),
        tag: TAG_UNSOLD.into(),
        value: json!({"cmdr": "CMDR OTHER", "rows": rows}),
    });
    assert_eq!(h.topic(TOPIC_UNSOLD), json!({"total": 0, "species": 0}));
    h.send(SourceEvent::Stored {
        module: "exobiology".into(),
        tag: TAG_UNSOLD.into(),
        value: json!({"cmdr": "CMDR TESTER", "rows": rows}),
    });
    // No Scan for the body: no bonus estimate, base value only.
    assert_eq!(
        h.topic(TOPIC_UNSOLD),
        json!({"total": 1_000_000, "species": 1})
    );
}

#[test]
fn stored_unsold_older_than_a_replayed_sale_is_dropped() {
    let mut h = H::new();
    h.start();
    h.journal("SellOrganicData", r#","BioData":[]"#);
    let rows = json!([[SYSTEM, BODY, AURASUS, t(0).to_rfc3339(), null]]);
    h.send(SourceEvent::Stored {
        module: "exobiology".into(),
        tag: TAG_UNSOLD.into(),
        value: json!({"cmdr": "CMDR TESTER", "rows": rows}),
    });
    assert_eq!(h.topic(TOPIC_UNSOLD), json!({"total": 0, "species": 0}));
}

#[test]
fn stored_open_samples_resume_the_sampling_and_activate_the_panel() {
    let mut h = H::new();
    h.start();
    let rows = json!([{
        "cmdr": "CMDR TESTER", "system_address": SYSTEM, "body_id": BODY,
        "species": AURASUS, "scan_type": "Log", "lat": 10.0, "lon": 20.0,
        "ts": "2026-04-12T21:06:00Z"
    }]);
    h.send(SourceEvent::Stored {
        module: "exobiology".into(),
        tag: TAG_OPEN.into(),
        value: json!({"cmdr": "CMDR TESTER", "rows": rows}),
    });
    let s = h.topic(TOPIC_SAMPLING);
    assert_eq!(
        (s["species"].as_str(), s["points"].as_u64()),
        (Some(AURASUS), Some(1))
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
}

#[test]
fn load_unsold_returns_only_species_completed_after_the_last_sale() {
    let store = tmc_core::Store::open_in_memory().unwrap();
    store.apply_migrations(persist::MIGRATIONS).unwrap();
    store
        .exec_json(
            "INSERT INTO journal_file(id,name,size,offset) VALUES (1,'j.log',0,0)",
            &[],
        )
        .unwrap();
    let scan = format!(
        r#"{{"event":"Scan","BodyID":{BODY},"SystemAddress":{SYSTEM},"BodyName":"B","WasFootfalled":false}}"#
    );
    let organic = |species: &str, kind: &str, body: u32| {
        format!(
            r#"{{"event":"ScanOrganic","ScanType":"{kind}","Species":"{species}","SystemAddress":{SYSTEM},"Body":{body}}}"#
        )
    };
    let events = [
        (
            "2026-04-12T21:00:00.000000000Z",
            "ScanOrganic",
            "CMDR TESTER",
            organic(NEBULUS, "Analyse", BODY),
        ),
        (
            "2026-04-12T21:01:00.000000000Z",
            "SellOrganicData",
            "CMDR TESTER",
            r#"{"event":"SellOrganicData"}"#.into(),
        ),
        (
            "2026-04-12T21:02:00.000000000Z",
            "Scan",
            "CMDR TESTER",
            scan,
        ),
        (
            "2026-04-12T21:03:00.000000000Z",
            "ScanOrganic",
            "CMDR TESTER",
            organic(AURASUS, "Log", BODY),
        ),
        (
            "2026-04-12T21:04:00.000000000Z",
            "ScanOrganic",
            "CMDR TESTER",
            organic(AURASUS, "Analyse", BODY),
        ),
        (
            "2026-04-12T21:05:00.000000000Z",
            "ScanOrganic",
            "CMDR OTHER",
            organic(NEBULUS, "Analyse", 9),
        ),
    ];
    for (line, (ts, name, cmdr, json)) in events.iter().enumerate() {
        store
            .exec_json(
                "INSERT INTO event(file_id,line,ts,name,cmdr,json) VALUES (1,?1,?2,?3,?4,?5)",
                &[
                    json!(line),
                    json!(ts),
                    json!(name),
                    json!(cmdr),
                    json!(json),
                ],
            )
            .unwrap();
    }
    let result = persist::load_unsold("CMDR TESTER".into())(&store).unwrap();
    let rows = result["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], SYSTEM);
    assert_eq!(rows[0][1], BODY);
    assert_eq!(rows[0][2], AURASUS);
    assert_eq!(rows[0][3], "2026-04-12T21:04:00.000000000Z");
    let scan: Scan = serde_json::from_str(rows[0][4].as_str().unwrap()).unwrap();
    assert_eq!(first_logged_estimate(Some(&scan), None), Some(true));
}

#[test]
fn registry_includes_exobiology_with_its_three_topics() {
    let modules = crate::all_modules();
    let exo = modules.iter().find(|m| m.id() == "exobiology").unwrap();
    assert_eq!(exo.topics(), [TOPIC_BODY, TOPIC_SAMPLING, TOPIC_UNSOLD]);
    assert_eq!(exo.migrations().len(), 1);
    assert_eq!(exo.migrations()[0].id, "exobiology-001");
}

fn history_store() -> tmc_core::Store {
    let store = tmc_core::Store::open_in_memory().unwrap();
    store.apply_migrations(persist::MIGRATIONS).unwrap();
    store
        .exec_json(
            "INSERT INTO journal_file(id,name,size,offset) VALUES(1,'history.log',0,0)",
            &[],
        )
        .unwrap();
    store
}

fn stored_outputs(h: &mut H, store: &tmc_core::Store) -> Vec<(String, Value)> {
    std::mem::take(&mut h.store_tasks)
        .into_iter()
        .map(|(tag, task)| (tag, task(store).unwrap()))
        .collect()
}

fn deliver_outputs(h: &mut H, outputs: Vec<(String, Value)>) {
    for (tag, value) in outputs {
        h.send(SourceEvent::Stored {
            module: "exobiology".into(),
            tag,
            value,
        });
    }
}

fn import_analysis(store: &tmc_core::Store, scan: &Scan) {
    let scan = serde_json::to_string(scan).unwrap();
    let organic =
        json!({"ScanType":"Analyse", "Species":AURASUS,"SystemAddress":SYSTEM,"Body":BODY})
            .to_string();
    for (line, name, json) in [(1, "Scan", scan), (2, "ScanOrganic", organic)] {
        store.exec_json("INSERT INTO event(file_id,line,ts,name,cmdr,json) VALUES(1,?1,?2,?3,'CMDR TESTER',?4)",
            &[json!(line),json!(t(line).to_rfc3339()),json!(name),json!(json)]).unwrap();
    }
}

#[test]
fn import_completion_refreshes_initial_empty_history_and_unsold_without_restart() {
    let store = history_store();
    let mut h = H::new();
    h.start();
    let initial = stored_outputs(&mut h, &store);
    deliver_outputs(&mut h, initial);
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.touchdown();
    let scan = h.m.scans[&(SYSTEM, BODY)].clone();
    let key = predict::BodyProfileKey::from(&body_profile(&scan, None));
    assert_eq!(h.m.history.count(&key, AURASUS), 0);
    import_analysis(&store, &scan);
    h.send(SourceEvent::ImportProgress { done: 1, total: 2 });
    assert!(h.store_tasks.is_empty());
    h.send(SourceEvent::ImportProgress { done: 2, total: 2 });
    let loaded = stored_outputs(&mut h, &store);
    assert_eq!(loaded.len(), 2);
    // The same completion is also seen live before the refreshed response.
    h.complete(AURASUS);
    deliver_outputs(&mut h, loaded);
    assert_eq!(h.m.history.count(&key, AURASUS), 1);
    assert_eq!(
        h.topic(TOPIC_UNSOLD),
        json!({"total":5_000_000,"species":1})
    );
}

#[test]
fn older_store_generation_cannot_replace_post_import_history() {
    let store = history_store();
    let mut h = H::new();
    h.start();
    let old = stored_outputs(&mut h, &store);
    h.scan_body("Rocky body", "CarbonDioxide", false);
    let scan = h.m.scans[&(SYSTEM, BODY)].clone();
    let key = predict::BodyProfileKey::from(&body_profile(&scan, None));
    import_analysis(&store, &scan);
    h.send(SourceEvent::ImportProgress { done: 2, total: 2 });
    let fresh = stored_outputs(&mut h, &store);
    deliver_outputs(&mut h, fresh);
    assert_eq!(h.m.history.count(&key, AURASUS), 1);
    deliver_outputs(&mut h, old);
    assert_eq!(h.m.history.count(&key, AURASUS), 1);
    assert_eq!(h.topic(TOPIC_UNSOLD)["species"], 1);
}

#[test]
fn sampling_coordinates_are_scoped_to_the_current_system_and_body() {
    for (other_system, other_body) in [(SYSTEM, BODY + 1), (SYSTEM + 1, BODY)] {
        let mut h = H::new();
        h.start();
        h.journal(
            "Location",
            &format!(
                r#", "StarSystem":"Kappa Testis", "SystemAddress":{SYSTEM}, "StarPos":[0,0,0]"#
            ),
        );
        h.scan_body("Rocky body", "CarbonDioxide", false);
        h.touchdown();
        h.status(1);
        h.organic("Log", AURASUS);
        h.status(2);
        h.organic("Sample", AURASUS);
        assert!(h.topic(TOPIC_SAMPLING)["distance_m"].is_number());
        h.journal(
            "FSDJump",
            &format!(
                r#", "StarSystem":"Other", "SystemAddress":{other_system}, "StarPos":[1,0,0]"#
            ),
        );
        h.journal("Touchdown", &format!(r#", "PlayerControlled":true,"Body":"Other B", "SystemAddress":{other_system},"BodyID":{other_body},"OnPlanet":true"#));
        h.send(SourceEvent::Status(Status::parse(r#"{"timestamp":"2026-04-12T22:00:00Z","event":"Status","Flags":0,"BodyName":"Other B","Latitude":-40,"Longitude":100,"PlanetRadius":1000000,"Heading":0}"#).unwrap()));
        for field in ["distance_m", "remaining_m", "direction"] {
            assert!(
                h.topic(TOPIC_SAMPLING)[field].is_null(),
                "{field}: {}",
                h.topic(TOPIC_SAMPLING)
            );
        }
        assert_eq!(h.topic(TOPIC_SAMPLING)["points"], 2);
        h.journal(
            "FSDJump",
            &format!(
                r#", "StarSystem":"Kappa Testis", "SystemAddress":{SYSTEM}, "StarPos":[0,0,0]"#
            ),
        );
        h.touchdown();
        // The previous Status still belongs to B until A sends a fresh one.
        assert!(h.topic(TOPIC_SAMPLING)["distance_m"].is_null());
        h.status(3);
        assert_eq!(h.topic(TOPIC_SAMPLING)["distance_m"], 12_891.0);
    }
}

#[test]
fn imported_sale_or_death_replaces_balance_and_preserves_live_overlay_and_open_samples() {
    for reset_name in ["SellOrganicData", "Died"] {
        let store = history_store();
        let mut h = H::new();
        h.start();
        let initial = stored_outputs(&mut h, &store);
        deliver_outputs(&mut h, initial);
        h.scan_body("Rocky body", "CarbonDioxide", false);
        h.touchdown();
        h.complete(AURASUS);
        import_analysis(&store, &h.m.scans[&(SYSTEM, BODY)]);
        h.send(SourceEvent::ImportProgress { done: 2, total: 2 });
        let old_snapshot = stored_outputs(&mut h, &store);
        deliver_outputs(&mut h, old_snapshot.clone());
        assert_eq!(h.topic(TOPIC_UNSOLD)["species"], 1);
        store.exec_json("INSERT INTO event(file_id,line,ts,name,cmdr,json) VALUES(1,3,?1,?2,'CMDR TESTER','{}')",
            &[json!(t(50).to_rfc3339()), json!(reset_name)]).unwrap();
        h.send(SourceEvent::ImportProgress { done: 3, total: 3 });
        let reset_snapshot = stored_outputs(&mut h, &store);
        // Neither the in-flight snapshot nor its imported reset includes this live completion.
        h.clock = 100;
        h.complete(NEBULUS);
        h.organic("Log", AURASUS);
        deliver_outputs(&mut h, reset_snapshot);
        assert_eq!(
            h.topic(TOPIC_UNSOLD),
            json!({
                "total":species_table().species(NEBULUS).unwrap().value * 5,"species":1
            }),
            "{reset_name}"
        );
        assert_eq!(h.topic(TOPIC_SAMPLING)["species"], AURASUS);
        assert_eq!(h.topic(TOPIC_SAMPLING)["points"], 1);
        assert_eq!(h.topic(TOPIC_SAMPLING)["complete"], false);
        let balance = h.topic(TOPIC_UNSOLD);
        deliver_outputs(&mut h, old_snapshot);
        assert_eq!(h.topic(TOPIC_UNSOLD), balance);
    }
}

#[test]
fn confirmed_completion_prunes_live_overlay_and_remains_idempotent() {
    let store = history_store();
    let mut h = H::new();
    h.start();
    let initial = stored_outputs(&mut h, &store);
    deliver_outputs(&mut h, initial);
    h.scan_body("Rocky body", "CarbonDioxide", false);
    h.complete(AURASUS);
    assert_eq!(h.m.live_completed.len(), 1);
    import_analysis(&store, &h.m.scans[&(SYSTEM, BODY)]);
    store
        .exec_json(
            "UPDATE event SET ts=?1 WHERE name='ScanOrganic'",
            &[json!(t(h.clock).to_rfc3339())],
        )
        .unwrap();
    h.send(SourceEvent::ImportProgress { done: 2, total: 2 });
    let loaded = stored_outputs(&mut h, &store);
    deliver_outputs(&mut h, loaded.clone());
    assert!(h.m.live_completed.is_empty());
    let balance = h.topic(TOPIC_UNSOLD);
    deliver_outputs(&mut h, loaded);
    assert_eq!(h.topic(TOPIC_UNSOLD), balance);
    assert_eq!(balance["species"], 1);
}

#[test]
fn history_generation_rejects_old_response_after_switching_away_and_back_to_commander() {
    let store = history_store();
    let mut h = H::new();
    h.start();
    h.scan_body("Rocky body", "CarbonDioxide", false);
    let scan = h.m.scans[&(SYSTEM, BODY)].clone();
    let key = predict::BodyProfileKey::from(&body_profile(&scan, None));
    import_analysis(&store, &scan);
    let old = stored_outputs(&mut h, &store);
    store.exec_json("DELETE FROM event", &[]).unwrap();
    h.journal("LoadGame", r#", "Commander":"CMDR OTHER","Odyssey":true"#);
    h.journal("LoadGame", r#", "Commander":"CMDR TESTER","Odyssey":true"#);
    let fresh = stored_outputs(&mut h, &store);
    deliver_outputs(&mut h, fresh);
    deliver_outputs(&mut h, old);
    assert_eq!(h.m.history.count(&key, AURASUS), 0);
    assert_eq!(h.topic(TOPIC_UNSOLD)["species"], 0);
}

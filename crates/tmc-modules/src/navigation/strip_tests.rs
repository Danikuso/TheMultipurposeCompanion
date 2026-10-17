use super::*;
use crate::navigation::jump::{fsd_from_loadout, jump_range};
use crate::navigation::route::PlanMode;
use chrono::Duration;
use serde_json::json;
use tmc_model::companion::{self, CompanionKind};
use tmc_model::events::parse_line;
use tmc_model::net::{NetFailure, NetRequest, NetSource, PlannedHop, RemoteSystem};
use tmc_model::status::Status;

fn t(seconds: i64) -> DateTime<Utc> {
    "2026-10-18T10:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
}

const FSD: &str = "int_hyperdrive_size5_class5";

struct Out {
    dirty: TopicSet,
    tasks: Vec<(String, tmc_core::StoreTask)>,
}

/// Replica lo que hace el actor: aplica el estado y luego llama al módulo.
struct Harness {
    nav: Navigation,
    state: GameState,
    next_id: u64,
    requests: Vec<NetRequest>,
    clock: i64,
}

impl Harness {
    fn new() -> Self {
        Self {
            nav: Navigation::new(),
            state: GameState::default(),
            next_id: 1,
            requests: Vec::new(),
            clock: 100,
        }
    }

    fn event(&mut self, ev: SourceEvent, replaying: bool) -> Out {
        match &ev {
            SourceEvent::Journal { ev, .. } => {
                self.state.apply_event(ev);
            }
            SourceEvent::Status(s) => {
                self.state.apply_status(s);
            }
            SourceEvent::Companion(c) => {
                self.state.apply_companion(c);
            }
            _ => {}
        }
        let mut ctx = ModuleCtx::new("navigation", replaying, self.next_id);
        let dirty = self.nav.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        Out { dirty, tasks }
    }

    fn journal_at(&mut self, seconds: i64, name: &str, extra: &str, replaying: bool) -> Out {
        let line = format!(
            r#"{{"timestamp":"{}","event":"{name}"{extra}}}"#,
            t(seconds).to_rfc3339()
        );
        self.event(
            SourceEvent::Journal {
                ev: parse_line(&line).unwrap(),
                file: "s.log".into(),
                line: 1,
                replaying,
            },
            replaying,
        )
    }

    fn journal(&mut self, name: &str, extra: &str) -> Out {
        self.clock += 1;
        self.journal_at(self.clock, name, extra, false)
    }

    fn load_game(&mut self) -> Out {
        self.load_game_as("TESTER")
    }

    fn load_game_as(&mut self, cmdr: &str) -> Out {
        self.journal_at(
            100,
            "LoadGame",
            &format!(
                r#","FID":"F1","Commander":"{cmdr}","Ship":"python","ShipID":7,"Credits":1,"gameversion":"4.1.0.0""#
            ),
            false,
        )
    }

    fn location(&mut self, name: &str, id: u64, x: f64) -> Out {
        self.journal(
            "Location",
            &format!(r#","StarSystem":"{name}","SystemAddress":{id},"StarPos":[{x},0,0]"#),
        )
    }

    fn jump_to(&mut self, name: &str, id: u64, x: f64) -> Out {
        self.clock += 1;
        self.jump_at(self.clock, name, id, x)
    }

    fn jump_at(&mut self, seconds: i64, name: &str, id: u64, x: f64) -> Out {
        self.journal_at(
            seconds,
            "FSDJump",
            &format!(
                r#","StarSystem":"{name}","SystemAddress":{id},"StarPos":[{x},0,0],"JumpDist":10.0"#
            ),
            false,
        )
    }

    /// Sesión viva en `S1` con una ruta `S1..S5` separada 10 al por tramo.
    fn with_route(&mut self) {
        self.load_game();
        self.location("S1", 1, 0.0);
        self.navroute();
    }

    fn navroute(&mut self) -> Out {
        let classes = ["K", "N", "DA", "M", "F"];
        let hops: Vec<String> = (1..=5)
            .map(|i| {
                format!(
                    r#"{{"StarSystem":"S{i}","SystemAddress":{i},"StarPos":[{}.0,0,0],"StarClass":"{}"}}"#,
                    (i - 1) * 10,
                    classes[i - 1]
                )
            })
            .collect();
        let file = companion::parse(
            CompanionKind::NavRoute,
            &format!(r#"{{"Route":[{}]}}"#, hops.join(",")),
        )
        .unwrap();
        self.event(SourceEvent::Companion(file), false)
    }

    fn loadout(&mut self, item: &str, max_range: &str) -> Out {
        self.journal(
            "Loadout",
            &format!(
                r#","Ship":"python","ShipID":7,"UnladenMass":468.0{max_range},"FuelCapacity":{{"Main":32.0,"Reserve":0.63}},"Modules":[{{"Slot":"FrameShiftDrive","Item":"{item}","Health":1.0}}]"#
            ),
        )
    }

    fn status(&mut self, fuel: f64, reservoir: f64) -> Out {
        let s = Status::parse(&format!(
            r#"{{"timestamp":"{}","Flags":0,"Flags2":0,"Fuel":{{"FuelMain":{fuel},"FuelReservoir":{reservoir}}}}}"#,
            t(self.clock).to_rfc3339()
        ))
        .unwrap();
        self.event(SourceEvent::Status(s), false)
    }

    fn settings(&mut self, online: bool) -> Out {
        let s = ModuleSettings {
            online_services: online,
            ..ModuleSettings::default()
        };
        self.event(SourceEvent::Settings(s), false)
    }

    fn net(&mut self, key: &str, at: i64, outcome: NetOutcome) -> Out {
        self.event(
            SourceEvent::Net(NetResponse {
                id: 1,
                module: "navigation".into(),
                key: key.into(),
                source: NetSource::Spansh,
                fetched_at: t(at),
                outcome,
            }),
            false,
        )
    }

    fn stored(&mut self, rows: Value) -> Out {
        self.stored_for("TESTER", rows)
    }

    fn stored_for(&mut self, cmdr: &str, rows: Value) -> Out {
        let rows = json!({"cmdr": cmdr, "rows": rows});
        self.event(
            SourceEvent::Stored {
                module: "navigation".into(),
                tag: "visits".into(),
                value: rows,
            },
            false,
        )
    }

    fn route(&self) -> Value {
        self.nav.topic_value(TOPIC_ROUTE).unwrap()
    }

    fn jump(&self) -> Value {
        self.nav.topic_value(TOPIC_JUMP).unwrap()
    }
}

fn remote(name: &str, id: u64) -> RemoteSystem {
    RemoteSystem {
        id64: id,
        name: name.into(),
        discovered: Some(true),
        body_count: Some(7),
        primary_star: Some("K".into()),
        allegiance: None,
        government: None,
        controlling_faction: None,
        faction_state: None,
        economy: None,
        second_economy: None,
        security: None,
        population: None,
        powers: vec![],
        powerplay_state: None,
    }
}

fn names(cards: &Value) -> Vec<String> {
    cards
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn strip_shows_next_and_three_following_from_navroute() {
    let mut h = Harness::new();
    h.with_route();
    let r = h.route();
    assert_eq!(r["source"], "nav-route");
    assert_eq!(r["current"]["name"], "S1");
    assert_eq!(r["next"]["name"], "S2");
    assert_eq!(names(&r["following"]), ["S3", "S4", "S5"]);
    assert_eq!(r["remaining_jumps"], 4);
    assert_eq!(r["total_ly"], 40.0);
    assert_eq!(r["next"]["jump_ly"], 10.0);
    assert_eq!(r["copy_target"], "S2");
    assert_eq!(r["off_route"], false);
}

#[test]
fn strip_advances_after_fsdjump() {
    let mut h = Harness::new();
    h.with_route();
    h.jump_to("S2", 2, 10.0);
    let r = h.route();
    assert_eq!(r["current"]["name"], "S2");
    assert_eq!(r["next"]["name"], "S3");
    assert_eq!(names(&r["following"]), ["S4", "S5"]);
    assert_eq!(r["remaining_jumps"], 3);
    assert_eq!(r["copy_seq"], 1);
    assert_eq!(r["copy_target"], "S3");
}

#[test]
fn strip_without_route_shows_current_system_card() {
    let mut h = Harness::new();
    h.load_game();
    h.location("S1", 1, 0.0);
    let r = h.route();
    assert_eq!(r["source"], "current");
    assert_eq!(r["current"]["name"], "S1");
    assert_eq!(r["current"]["id64"], 1);
    assert!(r["next"].is_null());
    assert!(r["following"].as_array().unwrap().is_empty());
    assert!(r["remaining_jumps"].is_null() && r["copy_target"].is_null());
}

#[test]
fn fuel_after_jump_uses_jump_model_and_hides_when_unknown() {
    let mut h = Harness::new();
    h.with_route();
    h.loadout(FSD, r#","MaxJumpRange":24.63"#);
    h.status(16.0, 0.5);
    let lo = tmc_model::events::Loadout {
        unladen_mass: Some(468.0),
        modules: vec![tmc_model::events::LoadoutModule {
            item: Some(FSD.into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let fsd = fsd_from_loadout(&lo, fsd_table()).unwrap();
    let burn = fuel_for_jump(&fsd, 468.0 + 16.0 + 0.5, 10.0).unwrap();
    let expected = (16.0 - burn) / 32.0 * 100.0;
    let card = &h.route()["next"];
    let got = card["fuel_after_pct"].as_f64().unwrap();
    assert!((got - expected).abs() < 0.1, "{got} vs {expected}");
    assert_eq!(card["reachable"], true);
    // El segundo salto parte de la masa ya reducida: queda menos combustible.
    let second = h.route()["following"][0]["fuel_after_pct"]
        .as_f64()
        .unwrap();
    assert!(second < got);

    h.loadout("int_hyperdrive_size9_class9_future", "");
    let r = h.route();
    assert!(r["next"]["fuel_after_pct"].is_null());
    assert!(r["next"]["reachable"].is_null());
    assert_eq!(r["next"]["jump_ly"], 10.0);
}

#[test]
fn scoopable_and_special_star_flags() {
    let mut h = Harness::new();
    h.with_route();
    let r = h.route();
    assert_eq!(r["current"]["star_class"], "K");
    assert_eq!(r["current"]["scoopable"], true);
    assert!(r["current"]["special"].is_null());
    assert_eq!(r["next"]["star_class"], "N");
    assert_eq!(r["next"]["special"], "neutron");
    assert_eq!(r["next"]["scoopable"], false);
    let dwarf = &r["following"][0];
    assert_eq!(dwarf["star_class"], "DA");
    assert_eq!(dwarf["special"], "white_dwarf");
    assert_eq!(dwarf["scoopable"], false);
    assert_eq!(r["following"][1]["scoopable"], true);
}

#[test]
fn requests_remote_for_current_next_and_three() {
    let mut h = Harness::new();
    h.with_route();
    let keys: Vec<_> = h.requests.iter().map(|r| r.key.clone()).collect();
    assert_eq!(
        keys,
        ["system:1", "system:2", "system:3", "system:4", "system:5"]
    );
    let ids: Vec<_> = h.requests.iter().map(|r| r.id).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5]);
    assert!(h.requests.iter().all(|r| r.module == "navigation"));
    assert_eq!(
        h.requests[1].query,
        NetQuery::System {
            id64: 2,
            name: "S2".into()
        }
    );
    // Sin respuesta no se repite la petición en vuelo.
    h.journal("Music", "");
    assert_eq!(h.requests.len(), 5);
    assert_eq!(h.route()["next"]["remote"]["status"], "pending");
}

#[test]
fn late_response_for_system_no_longer_in_strip_is_not_applied() {
    let mut h = Harness::new();
    h.with_route();
    h.net("system:99", 10, NetOutcome::System(remote("Zeta", 99)));
    let r = h.route();
    for card in [&r["current"], &r["next"], &r["following"][0]] {
        assert_eq!(card["remote"]["status"], "pending");
        assert!(card["remote"]["data"].is_null());
    }
    // S1 sale de la tira tras dos saltos: su respuesta tardía se ignora.
    h.jump_to("S2", 2, 10.0);
    h.jump_to("S3", 3, 20.0);
    h.net("system:1", 10, NetOutcome::System(remote("S1", 1)));
    assert!(!h.nav.remote.contains_key("system:1"));
    assert_eq!(h.route()["current"]["remote"]["status"], "pending");
}

#[test]
fn older_response_does_not_overwrite_newer() {
    let mut h = Harness::new();
    h.with_route();
    h.net("system:2", 20, NetOutcome::System(remote("S2 nuevo", 2)));
    let card = h.route()["next"].clone();
    assert_eq!(card["remote"]["status"], "ok");
    assert_eq!(card["remote"]["source"], "Spansh");
    assert_eq!(card["remote"]["data"]["name"], "S2 nuevo");
    h.net("system:2", 10, NetOutcome::System(remote("S2 viejo", 2)));
    h.net("system:2", 5, NetOutcome::Failed(NetFailure::Timeout));
    let card = h.route()["next"].clone();
    assert_eq!(card["remote"]["data"]["name"], "S2 nuevo");
    assert_eq!(
        card["remote"]["fetched_at"],
        json!(t(20).to_rfc3339_opts(SecondsFormat::AutoSi, true))
    );
    assert_eq!(card["remote"]["status"], "ok");
}

#[test]
fn failed_and_repeated_failed_mark_card_without_touching_local_fields() {
    let mut h = Harness::new();
    h.with_route();
    for _ in 0..2 {
        h.net("system:2", 30, NetOutcome::Failed(NetFailure::Down));
    }
    let next = h.route()["next"].clone();
    assert_eq!(next["remote"]["status"], "failed");
    assert_eq!(next["jump_ly"], 10.0);
    assert_eq!(next["star_class"], "N");
    h.net("system:3", 30, NetOutcome::NoData);
    h.net("system:4", 30, NetOutcome::Disabled);
    let r = h.route();
    assert_eq!(r["following"][0]["remote"]["status"], "no-data");
    assert_eq!(r["following"][1]["remote"]["status"], "disabled");
    assert_eq!(h.requests.len(), 5);
}

#[test]
fn strip_shows_local_card_and_no_data_when_services_disabled() {
    let mut h = Harness::new();
    h.settings(false);
    h.with_route();
    assert!(h.requests.is_empty());
    let next = h.route()["next"].clone();
    assert_eq!(next["remote"]["status"], "disabled");
    assert_eq!(next["jump_ly"], 10.0);
    assert_eq!(next["star_class"], "N");
    // Al reactivar los servicios se piden las fichas que faltan.
    h.settings(true);
    assert_eq!(h.requests.len(), 5);
    assert_eq!(h.route()["next"]["remote"]["status"], "pending");
}

#[test]
fn legacy_galaxy_never_requests() {
    let mut h = Harness::new();
    h.load_game();
    h.journal("Fileheader", r#","gameversion":"3.8.0.407""#);
    h.location("S1", 1, 0.0);
    h.navroute();
    assert_eq!(h.state.galaxy, Galaxy::Legacy);
    assert!(h.nav.settings.online_services);
    assert!(h.requests.is_empty());
    assert_eq!(h.route()["next"]["remote"]["status"], "disabled");
}

#[test]
fn requests_dropped_during_replay_are_made_after_first_live_event() {
    let mut h = Harness::new();
    h.load_game();
    h.journal_at(
        101,
        "Location",
        r#","StarSystem":"S1","SystemAddress":1,"StarPos":[0,0,0]"#,
        true,
    );
    assert!(h.requests.is_empty());
    h.status(10.0, 0.5);
    assert_eq!(h.requests.len(), 1);
    assert_eq!(h.requests[0].key, "system:1");
}

#[test]
fn visits_loaded_from_history_and_incremented_live() {
    let mut h = Harness::new();
    h.with_route();
    assert!(h.route()["current"]["visits"].is_null());
    h.stored(json!([[2, 2], [1, 1]]));
    assert_eq!(h.route()["current"]["visits"], 1);
    assert_eq!(h.route()["next"]["visits"], 2);
    h.jump_to("S2", 2, 10.0);
    assert_eq!(h.route()["current"]["visits"], 3);
    h.jump_to("S3", 3, 20.0);
    assert_eq!(h.route()["current"]["visits"], 1);
}

#[test]
fn visits_do_not_double_count_jumps_during_query() {
    // El corte es la marca de tiempo del primer evento (LoadGame, t=100).
    for (jump_time, expected) in [(101, 3), (100, 2)] {
        let mut h = Harness::new();
        let first = h.load_game();
        assert_eq!(first.tasks.len(), 2);
        assert_eq!(first.tasks[0].0, "visits");
        assert_eq!(first.tasks[1].0, "plan");
        h.jump_at(jump_time, "A", 7, 0.0);
        h.stored(json!([[7, 2]]));
        assert_eq!(h.route()["current"]["visits"], expected, "t={jump_time}");
    }
    // Un solo encargo por comandante.
    let mut h = Harness::new();
    h.load_game();
    assert!(h.journal("Music", "").tasks.is_empty());
}

#[test]
fn visits_query_counts_history_up_to_the_cut_for_the_commander() {
    let mut h = Harness::new();
    let mut out = h.load_game();
    let (tag, task) = out.tasks.remove(0);
    assert_eq!(tag, "visits");
    let store = tmc_core::Store::open_in_memory().unwrap();
    store
        .exec_json(
            "INSERT INTO journal_file(id,name,size,offset) VALUES (1,'j.log',0,0)",
            &[],
        )
        .unwrap();
    let rows = [
        (-50, "FSDJump", "TESTER", 10),
        (-40, "FSDJump", "TESTER", 10),
        (-30, "FSDJump", "TESTER", 11),
        (-20, "FSDJump", "OTRO", 10),
        (-10, "Docked", "TESTER", 10),
        (100, "FSDJump", "TESTER", 11),
        (101, "FSDJump", "TESTER", 10),
    ];
    for (line, (secs, name, cmdr, addr)) in rows.iter().enumerate() {
        store
            .exec_json(
                "INSERT INTO event(file_id,line,ts,name,cmdr,json) VALUES (1,?1,?2,?3,?4,?5)",
                &[
                    json!(line as i64 + 1),
                    json!(t(*secs).to_rfc3339_opts(SecondsFormat::Nanos, true)),
                    json!(name),
                    json!(cmdr),
                    json!(format!(r#"{{"SystemAddress":{addr}}}"#)),
                ],
            )
            .unwrap();
    }
    let value = task(&store).unwrap();
    assert_eq!(value["cmdr"], "TESTER");
    let mut counts: Vec<(u64, u64)> = value["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r[0].as_u64().unwrap(), r[1].as_u64().unwrap()))
        .collect();
    counts.sort();
    assert_eq!(counts, [(10, 2), (11, 2)]);
}

#[test]
fn replay_jumps_do_not_bump_copy_seq_but_live_jumps_do() {
    let mut h = Harness::new();
    h.with_route();
    h.journal_at(
        200,
        "FSDJump",
        r#","StarSystem":"S2","SystemAddress":2,"StarPos":[10,0,0]"#,
        true,
    );
    assert_eq!(h.route()["copy_seq"], 0);
    h.jump_to("S3", 3, 20.0);
    assert_eq!(h.route()["copy_seq"], 1);
}

#[test]
fn copy_seq_only_increases_on_live_jumps() {
    let mut h = Harness::new();
    h.load_game();
    h.location("S1", 1, 0.0);
    h.nav
        .set_plan(Some(plan_of(&[("S1", 1), ("S2", 2), ("S3", 3), ("S4", 4)])));
    // Replay de 3 FSDJump: el contador no se mueve.
    for (i, (name, id)) in [("S1", 1u64), ("S2", 2), ("S1", 1)].iter().enumerate() {
        h.journal_at(
            200 + i as i64,
            "FSDJump",
            &format!(
                r#","StarSystem":"{name}","SystemAddress":{id},"StarPos":[0,0,0],"JumpDist":10.0"#
            ),
            true,
        );
    }
    assert_eq!(h.route()["copy_seq"], 0);
    // Dos saltos en vivo sobre el plan S1 -> S2 -> S3 -> S4.
    h.jump_to("S2", 2, 10.0);
    assert_eq!(h.route()["copy_seq"], 1);
    assert_eq!(h.route()["copy_target"], "S3");
    h.jump_to("S3", 3, 20.0);
    let r = h.route();
    assert_eq!(r["copy_seq"], 2);
    assert_eq!(r["copy_target"], "S4");
}

fn loadout_model() -> (jump::FsdSpec, f64) {
    let lo = tmc_model::events::Loadout {
        unladen_mass: Some(468.0),
        modules: vec![tmc_model::events::LoadoutModule {
            item: Some(FSD.into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    (fsd_from_loadout(&lo, fsd_table()).unwrap(), 468.0)
}

#[test]
fn jump_topic_recomputes_on_loadout_status_and_cargo() {
    let (fsd, unladen) = loadout_model();
    let calc = |fuel: f64, cargo: f64| {
        jump_range(
            &fsd,
            &ShipMass {
                unladen,
                fuel_main: fuel,
                fuel_reserve: 0.5,
                cargo,
            },
        )
    };
    let mut h = Harness::new();
    h.load_game();
    let out = h.loadout(FSD, r#","MaxJumpRange":24.63"#);
    assert!(out.dirty.contains(TOPIC_JUMP));
    assert!(h.jump()["range_ly"].is_null());
    assert_eq!(h.jump()["source"], "formula");

    let out = h.status(5.0, 0.5);
    assert!(out.dirty.contains(TOPIC_JUMP));
    let full = h.jump()["range_ly"].as_f64().unwrap();
    assert!((full - calc(5.0, 0.0)).abs() < 0.01, "{full}");
    assert_eq!(h.jump()["fuel_main"], 5.0);
    assert_eq!(h.jump()["fuel_capacity"], 32.0);
    assert_eq!(h.jump()["max_fuel_per_jump"], 5.0);
    let range_full = h.jump()["range_full_ly"].as_f64().unwrap();
    assert!((range_full - 24.63).abs() < 0.01, "{range_full}");

    h.status(4.0, 0.5);
    let low = h.jump()["range_ly"].as_f64().unwrap();
    assert!((low - calc(4.0, 0.0)).abs() < 0.01, "{low}");
    assert!(low < full);

    h.journal("Cargo", r#","Vessel":"Ship","Count":40"#);
    let laden = h.jump()["range_ly"].as_f64().unwrap();
    assert!((laden - calc(4.0, 40.0)).abs() < 0.01, "{laden}");
    assert!(laden < low);

    h.loadout("int_hyperdrive_size9_class9_future", "");
    assert!(h.jump()["range_ly"].is_null());
    assert_eq!(h.jump()["source"], "unknown");
}

#[test]
fn jet_cone_boost_sets_supercharged_until_next_jump() {
    let mut h = Harness::new();
    h.load_game();
    assert!(h.jump()["supercharged"].is_null());
    h.journal("JetConeBoost", r#","BoostValue":4.0"#);
    assert_eq!(h.jump()["supercharged"], 4.0);
    h.jump_to("S2", 2, 10.0);
    assert!(h.jump()["supercharged"].is_null());
}

#[test]
fn local_politics_only_for_current_system() {
    let mut h = Harness::new();
    h.with_route();
    h.journal(
        "FSDJump",
        r#","StarSystem":"S2","SystemAddress":2,"StarPos":[10,0,0],"SystemAllegiance":"Federation","SystemEconomy_Localised":"Industrial","Population":1234,"SystemFaction":{"Name":"Faccion X","FactionState":"Boom"}"#,
    );
    let r = h.route();
    let p = &r["current"]["local_politics"];
    assert_eq!(p["allegiance"], "Federation");
    assert_eq!(p["economy"], "Industrial");
    assert_eq!(p["population"], 1234);
    assert_eq!(p["controlling_faction"], "Faccion X");
    assert_eq!(p["faction_state"], "Boom");
    assert!(r["next"]["local_politics"].is_null());
    // Sin datos políticos el campo queda vacío.
    h.jump_to("S3", 3, 20.0);
    assert!(h.route()["current"]["local_politics"].is_null());
}

fn plan_of(names: &[(&str, u64)]) -> ActivePlan {
    ActivePlan {
        created_at: t(0),
        origin: names[0].0.into(),
        destination: names[names.len() - 1].0.into(),
        mode: PlanMode::Neutron,
        params: json!({}),
        hops: names
            .iter()
            .enumerate()
            .map(|(i, (n, id))| PlannedHop {
                name: (*n).into(),
                id64: Some(*id),
                star_pos: None,
                distance_ly: 12.5 * i as f64,
                jumps: 2,
                neutron: false,
                refuel: None,
                star_class: None,
            })
            .collect(),
        current_index: 0,
        off_route: false,
    }
}

#[test]
fn active_plan_wins_over_navroute_and_reconcile_is_not_a_new_jump() {
    let mut h = Harness::new();
    h.with_route();
    h.nav
        .set_plan(Some(plan_of(&[("S1", 1), ("P2", 22), ("P3", 23)])));
    h.stored(json!([[1, 4]]));
    // Location sobre el hop actual: ni avanza ni cuenta como salto.
    h.location("S1", 1, 0.0);
    let r = h.route();
    assert_eq!(r["source"], "plan");
    assert_eq!(r["next"]["name"], "P2");
    assert_eq!(r["next"]["jump_ly"], 12.5);
    assert_eq!(r["remaining_jumps"], 4);
    assert_eq!(r["total_ly"], 37.5);
    assert_eq!(r["current"]["visits"], 4);
    assert_eq!(r["copy_seq"], 0);
    // Fuera de ruta: se conserva el plan.
    h.jump_to("Z", 99, 5.0);
    let r = h.route();
    assert_eq!(r["off_route"], true);
    assert_eq!(r["next"]["name"], "P2");
    assert_eq!(r["copy_seq"], 1);
}

#[test]
fn relevance_is_active_only_with_route_while_travelling() {
    let mut h = Harness::new();
    h.load_game();
    h.location("S1", 1, 0.0);
    assert_eq!(h.nav.relevance(&h.state), Relevance::Normal);
    h.navroute();
    assert_eq!(h.nav.relevance(&h.state), Relevance::Normal);
    h.jump_to("S2", 2, 10.0);
    assert_eq!(h.state.location.mode, FlightMode::Supercruise);
    assert_eq!(h.nav.relevance(&h.state), Relevance::Active);
    h.journal("NavRouteClear", "");
    assert_eq!(h.nav.relevance(&h.state), Relevance::Normal);
}

#[test]
fn route_topic_serializes_under_8_kib() {
    let mut h = Harness::new();
    h.with_route();
    for id in 1..=5u64 {
        let mut sys = remote(&format!("S{id}"), id);
        sys.powers = vec!["Poder largo de ejemplo".into(); 3];
        sys.controlling_faction = Some("Facción de ejemplo".into());
        h.net(&format!("system:{id}"), 10, NetOutcome::System(sys));
    }
    let topic = h.nav.topic_value(TOPIC_ROUTE).unwrap();
    assert_eq!(topic["next"]["remote"]["status"], "ok");
    assert!(serde_json::to_vec(&topic).unwrap().len() < 8192);
}

#[test]
fn registry_includes_navigation_with_its_three_topics() {
    let modules = crate::all_modules();
    let nav = modules.iter().find(|m| m.id() == "navigation").unwrap();
    assert_eq!(nav.topics(), [TOPIC_ROUTE, TOPIC_JUMP, TOPIC_PLAN]);
}

#[test]
fn visits_for_another_commander_are_ignored() {
    let mut h = Harness::new();
    h.with_route();
    h.stored_for("OTRO", json!([[1, 9]]));
    assert!(h.route()["current"]["visits"].is_null());
    h.stored(json!([[1, 2]]));
    assert_eq!(h.route()["current"]["visits"], 2);
}

#[test]
fn lost_request_is_retried_after_60_seconds() {
    let mut h = Harness::new();
    h.with_route();
    assert_eq!(h.requests.len(), 5);
    h.clock += 59;
    h.status(10.0, 0.5);
    assert_eq!(h.requests.len(), 5);
    h.clock += 2;
    h.status(10.0, 0.5);
    assert_eq!(h.requests.len(), 10);
    let ids: Vec<_> = h.requests[5..].iter().map(|r| r.id).collect();
    assert_eq!(ids, [6, 7, 8, 9, 10]);
    // La respuesta a la petición antigua se acepta; una más vieja que la aplicada, no.
    h.net("system:2", 500, NetOutcome::System(remote("S2", 2)));
    h.net("system:2", 400, NetOutcome::System(remote("viejo", 2)));
    assert_eq!(h.route()["next"]["remote"]["data"]["name"], "S2");
}

#[test]
fn inflight_of_systems_leaving_the_strip_is_dropped() {
    let mut h = Harness::new();
    h.with_route();
    h.jump_to("S2", 2, 10.0);
    h.jump_to("S3", 3, 20.0);
    assert!(!h.nav.inflight.contains_key("system:1"));
    assert!(h.nav.inflight.contains_key("system:5"));
}

#[test]
fn settings_change_purges_non_ok_states_and_rerequests() {
    let mut h = Harness::new();
    h.with_route();
    h.net("system:1", 10, NetOutcome::System(remote("S1", 1)));
    h.net("system:2", 10, NetOutcome::Disabled);
    h.net("system:3", 10, NetOutcome::NoData);
    assert_eq!(h.requests.len(), 5);
    h.settings(true);
    let keys: Vec<_> = h.requests[5..].iter().map(|r| r.key.clone()).collect();
    assert_eq!(keys, ["system:2", "system:3"]);
    let r = h.route();
    assert_eq!(r["current"]["remote"]["status"], "ok");
    assert_eq!(r["next"]["remote"]["status"], "pending");
}

#[test]
fn failed_is_retried_after_cooldown_but_no_data_is_not() {
    let mut h = Harness::new();
    h.with_route();
    let at = h.clock;
    h.net("system:2", at, NetOutcome::Failed(NetFailure::Down));
    h.net("system:3", at, NetOutcome::NoData);
    for id in [1, 4, 5] {
        h.net(
            &format!("system:{id}"),
            at,
            NetOutcome::System(remote("x", id)),
        );
    }
    h.clock = at + 299;
    h.status(10.0, 0.5);
    assert_eq!(h.requests.len(), 5);
    h.clock = at + 301;
    h.status(10.0, 0.5);
    assert_eq!(h.requests.len(), 6);
    assert_eq!(h.requests[5].key, "system:2");
    h.clock = at + 1000;
    h.status(10.0, 0.5);
    assert_eq!(h.route()["following"][0]["remote"]["status"], "no-data");
    assert!(h.requests[6..].iter().all(|r| r.key != "system:3"));
}

#[test]
fn irrelevant_journal_events_do_not_rebuild_topics() {
    let mut h = Harness::new();
    h.with_route();
    let out = h.journal("Music", "");
    assert!(out.dirty.is_empty());
    assert_eq!(h.route()["next"]["name"], "S2");
}

// ---- Planificador (Task 16) ----

use tmc_model::net::{GalaxyRouteParams, PlannedRoute};

fn planned(hops: &[(&str, u64)]) -> PlannedRoute {
    PlannedRoute {
        hops: hops
            .iter()
            .enumerate()
            .map(|(i, (n, id))| PlannedHop {
                name: (*n).into(),
                id64: Some(*id),
                star_pos: None,
                distance_ly: 10.0 * i as f64,
                jumps: 1,
                neutron: false,
                refuel: None,
                star_class: None,
            })
            .collect(),
        total_ly: 10.0 * (hops.len() as f64 - 1.0),
    }
}

impl Harness {
    fn route_requests(&self) -> Vec<&NetRequest> {
        self.requests.iter().filter(|r| r.key == "route").collect()
    }

    fn act(&mut self, name: &str, args: Value) -> Out {
        self.event(
            SourceEvent::Action {
                name: name.into(),
                args,
            },
            false,
        )
    }

    fn respond_id(&mut self, id: u64, outcome: NetOutcome) -> Out {
        self.event(
            SourceEvent::Net(NetResponse {
                id,
                module: "navigation".into(),
                key: "route".into(),
                source: NetSource::Spansh,
                fetched_at: t(200),
                outcome,
            }),
            false,
        )
    }

    fn respond(&mut self, outcome: NetOutcome) -> Out {
        let id = self.route_requests().last().unwrap().id;
        self.respond_id(id, outcome)
    }

    fn plan_topic(&self) -> Value {
        self.nav.topic_value(TOPIC_PLAN).unwrap()
    }

    /// Sesión en `Sol` con FSD conocida, lista para planificar.
    fn ready_ship(&mut self) {
        self.load_game();
        self.loadout(FSD, r#","MaxJumpRange":24.63"#);
        self.location("Sol", 10, 0.0);
    }

    /// Planifica, responde y acepta una ruta; devuelve la salida del accept.
    fn accept_route(&mut self, hops: &[(&str, u64)]) -> Out {
        let to = hops.last().unwrap().0;
        self.act("navigation.plan", json!({"to": to, "mode": "neutron"}));
        self.respond(NetOutcome::Route(planned(hops)));
        self.act("navigation.accept-plan", json!({}))
    }
}

fn run_tasks(
    store: &tmc_core::Store,
    tasks: Vec<(String, tmc_core::StoreTask)>,
) -> Vec<(String, Value)> {
    tasks
        .into_iter()
        .map(|(tag, task)| (tag, task(store).unwrap()))
        .collect()
}

fn only_load(tasks: Vec<(String, tmc_core::StoreTask)>) -> Vec<(String, tmc_core::StoreTask)> {
    tasks.into_iter().filter(|(t, _)| t == "plan").collect()
}

fn feed(h: &mut Harness, done: Vec<(String, Value)>) {
    for (tag, value) in done {
        h.event(
            SourceEvent::Stored {
                module: "navigation".into(),
                tag,
                value,
            },
            false,
        );
    }
}

fn plan_store() -> tmc_core::Store {
    let store = tmc_core::Store::open_in_memory().unwrap();
    store
        .apply_migrations(Navigation::new().migrations())
        .unwrap();
    store
}

fn sync_tasks(tasks: &[(String, tmc_core::StoreTask)]) -> usize {
    tasks.iter().filter(|(t, _)| t == "plan-sync").count()
}

fn saved_rows(store: &tmc_core::Store) -> Value {
    Value::Array(
        store
            .query_json(
                "SELECT cmdr, current_index FROM nav_route_plan ORDER BY cmdr",
                &[],
            )
            .unwrap(),
    )
}

#[test]
fn plan_action_requests_neutron_route_with_current_range() {
    let mut h = Harness::new();
    h.ready_ship();
    let expected = h
        .nav
        .model
        .as_ref()
        .unwrap()
        .range_now(&ShipMass {
            unladen: 468.0,
            fuel_main: 32.0,
            fuel_reserve: 0.0,
            cargo: 0.0,
        })
        .unwrap();
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "mode": "neutron"}),
    );
    assert_eq!(h.route_requests().len(), 1);
    assert_eq!(h.route_requests()[0].key, "route");
    let NetQuery::NeutronRoute(p) = &h.route_requests()[0].query else {
        panic!("{:?}", h.route_requests()[0].query);
    };
    assert_eq!(p.range_ly, expected);
    assert!((expected - 24.63).abs() < 0.05, "{expected}");
    assert_eq!(p.efficiency, 60);
    assert_eq!(p.from, "Sol");
    assert_eq!(p.to, "Colonia");
    assert_eq!(p.supercharge_multiplier, 4.0);
    assert_eq!(h.plan_topic()["status"], "computing");
    assert_eq!(h.plan_topic()["request"]["to"], "Colonia");
}

#[test]
fn route_response_moves_to_ready_and_accept_activates() {
    let mut h = Harness::new();
    h.ready_ship();
    h.act("navigation.plan", json!({"to": "S2", "mode": "neutron"}));
    let out = h.respond(NetOutcome::Route(planned(&[("Sol", 10), ("S2", 2)])));
    assert!(out.dirty.contains(TOPIC_PLAN));
    let topic = h.plan_topic();
    assert_eq!(topic["status"], "ready");
    assert_eq!(topic["result"]["hops"].as_array().unwrap().len(), 2);
    assert!(topic["active"].is_null());
    h.act("navigation.accept-plan", json!({}));
    let topic = h.plan_topic();
    assert_eq!(topic["status"], "active");
    assert_eq!(topic["active"]["total"], 2);
    assert_eq!(topic["active"]["current_index"], 0);
    assert_eq!(topic["active"]["destination"], "S2");
    assert_eq!(topic["active"]["off_route"], false);
    assert_eq!(h.route()["source"], "plan");
}

#[test]
fn planner_disabled_without_online_services() {
    let mut h = Harness::new();
    h.ready_ship();
    h.settings(false);
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "mode": "neutron"}),
    );
    assert_eq!(h.plan_topic()["status"], "disabled");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-disabled-offline");
    assert!(h.route_requests().is_empty());
}

#[test]
fn legacy_galaxy_planner_never_requests() {
    let mut h = Harness::new();
    h.load_game();
    h.journal("Fileheader", r#","gameversion":"3.8.0.407""#);
    h.location("Sol", 10, 0.0);
    assert_eq!(h.state.galaxy, Galaxy::Legacy);
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "mode": "neutron"}),
    );
    assert_eq!(h.plan_topic()["status"], "disabled");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-disabled-legacy");
    assert!(h.route_requests().is_empty());
}

#[test]
fn plan_without_known_range_fails_with_reason() {
    let mut h = Harness::new();
    h.load_game();
    h.loadout("int_hyperdrive_size9_class9_future", "");
    h.location("Sol", 10, 0.0);
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "mode": "neutron", "use_ship": true}),
    );
    assert_eq!(h.plan_topic()["status"], "failed");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-error-no-range");
    assert!(h.route_requests().is_empty());
}

#[test]
fn galaxy_plan_without_fsd_fails_with_reason_and_neutron_still_works() {
    let mut h = Harness::new();
    h.load_game();
    h.loadout(
        "int_hyperdrive_size9_class9_future",
        r#","MaxJumpRange":24.63"#,
    );
    h.location("Sol", 10, 0.0);
    assert!(h.nav.model.as_ref().unwrap().fsd.is_none());
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "to_id64": 20, "mode": "galaxy"}),
    );
    assert_eq!(h.plan_topic()["status"], "failed");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-error-no-fsd");
    assert!(h.route_requests().is_empty());
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "mode": "neutron"}),
    );
    assert_eq!(h.route_requests().len(), 1);
    let NetQuery::NeutronRoute(p) = &h.route_requests()[0].query else {
        panic!("{:?}", h.route_requests()[0].query);
    };
    assert!((p.range_ly - 24.63).abs() < 0.01, "{}", p.range_ly);
    assert_eq!(p.supercharge_multiplier, 4.0);
}

#[test]
fn galaxy_plan_uses_id64_and_fsd_params_and_needs_destination_id() {
    let mut h = Harness::new();
    h.ready_ship();
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "mode": "galaxy"}),
    );
    assert_eq!(h.plan_topic()["status"], "failed");
    assert_eq!(
        h.plan_topic()["error_key"],
        "nav-plan-error-no-destination-id"
    );
    assert!(h.route_requests().is_empty());
    h.act(
        "navigation.plan",
        json!({"to": "Colonia", "to_id64": 3_238_296_097_059u64, "mode": "galaxy", "use_injections": true}),
    );
    assert_eq!(h.route_requests().len(), 1);
    let NetQuery::GalaxyRoute(GalaxyRouteParams {
        from,
        to,
        ship,
        refuel_scoopable,
        use_injections,
    }) = &h.route_requests()[0].query
    else {
        panic!("{:?}", h.route_requests()[0].query);
    };
    assert_eq!(from, "10");
    assert_eq!(to, "3238296097059");
    assert_eq!(ship.unladen_mass, 468.0);
    assert_eq!(ship.fuel_main_capacity, 32.0);
    assert_eq!(ship.fuel_reserve, 0.63);
    assert_eq!(ship.max_fuel_per_jump, 5.0);
    assert_eq!(ship.supercharge_multiplier, 4.0);
    assert!(*refuel_scoopable && *use_injections);
}

#[test]
fn stale_and_repeated_responses_are_ignored_by_id() {
    let mut h = Harness::new();
    h.ready_ship();
    h.act("navigation.plan", json!({"to": "S2", "mode": "neutron"}));
    let first = h.route_requests()[0].id;
    h.act("navigation.plan", json!({"to": "S3", "mode": "neutron"}));
    let second = h.route_requests()[1].id;
    assert_ne!(first, second);
    h.respond_id(first, NetOutcome::Route(planned(&[("Sol", 10), ("S2", 2)])));
    assert_eq!(h.plan_topic()["status"], "computing");
    h.respond_id(second, NetOutcome::Failed(NetFailure::Timeout));
    h.respond_id(second, NetOutcome::Failed(NetFailure::Timeout));
    assert_eq!(h.plan_topic()["status"], "failed");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-error-service");
    h.respond_id(
        second,
        NetOutcome::Route(planned(&[("Sol", 10), ("S3", 3)])),
    );
    assert_eq!(h.plan_topic()["status"], "ready");
    h.respond_id(second, NetOutcome::Failed(NetFailure::Down));
    assert_eq!(h.plan_topic()["status"], "ready");
    assert_eq!(h.plan_topic()["result"]["hops"][1]["name"], "S3");
}

#[test]
fn recalculate_uses_current_system_as_origin() {
    let mut h = Harness::new();
    h.ready_ship();
    h.accept_route(&[("Sol", 10), ("X", 11), ("Colonia", 12)]);
    h.jump_to("X", 11, 10.0);
    h.act("navigation.recalculate", json!({}));
    assert_eq!(h.route_requests().len(), 2);
    let NetQuery::NeutronRoute(p) = &h.route_requests()[1].query else {
        panic!("{:?}", h.route_requests()[1].query);
    };
    assert_eq!(p.from, "X");
    assert_eq!(p.to, "Colonia");
    assert_eq!(h.plan_topic()["status"], "computing");
    assert_eq!(h.plan_topic()["active"]["current_index"], 1);
}

#[test]
fn active_plan_is_saved_and_restored_per_cmdr() {
    let store = plan_store();
    let mut h = Harness::new();
    h.ready_ship();
    let out = h.accept_route(&[("Sol", 10), ("X", 11), ("Colonia", 12)]);
    assert_eq!(sync_tasks(&out.tasks), 1);
    run_tasks(&store, out.tasks);
    assert_eq!(saved_rows(&store), json!([["TESTER", 0]]));
    let out = h.jump_to("X", 11, 10.0);
    run_tasks(&store, out.tasks);
    assert_eq!(saved_rows(&store), json!([["TESTER", 1]]));

    // Reinicio: otra instancia, mismo comandante, ya en X.
    let mut h2 = Harness::new();
    let out = h2.load_game();
    let load = only_load(out.tasks);
    h2.location("X", 11, 10.0);
    feed(&mut h2, run_tasks(&store, load));
    let topic = h2.plan_topic();
    assert_eq!(topic["status"], "active");
    assert_eq!(topic["active"]["destination"], "Colonia");
    assert_eq!(topic["active"]["current_index"], 1);
    assert_eq!(topic["active"]["total"], 3);
    assert_eq!(h2.route()["source"], "plan");

    // Otro comandante: sin plan.
    let mut h3 = Harness::new();
    let out = h3.load_game_as("OTHER");
    let load = only_load(out.tasks);
    feed(&mut h3, run_tasks(&store, load));
    assert!(h3.plan_topic()["active"].is_null());
    assert_eq!(h3.plan_topic()["status"], "idle");
}

fn stored_plan(index: usize) -> Value {
    let route = planned(&[("S1", 1), ("S2", 2), ("S3", 3), ("S4", 4)]);
    let plan = ActivePlan {
        created_at: t(0),
        origin: "S1".into(),
        destination: "S4".into(),
        mode: PlanMode::Neutron,
        params: json!({"to": "S4"}),
        hops: route.hops,
        current_index: index,
        off_route: false,
    };
    json!({"cmdr": "TESTER", "plan": plan})
}

#[test]
fn restored_plan_is_reconciled_with_current_system() {
    let mut h = Harness::new();
    h.load_game();
    h.location("S1", 1, 0.0);
    h.jump_to("S3", 3, 20.0);
    feed(&mut h, vec![("plan".into(), stored_plan(0))]);
    assert_eq!(h.plan_topic()["active"]["current_index"], 2);
    assert_eq!(h.plan_topic()["active"]["off_route"], false);

    let mut h = Harness::new();
    h.load_game();
    h.location("S1", 1, 0.0);
    h.jump_to("Zeta", 99, 5.0);
    feed(&mut h, vec![("plan".into(), stored_plan(0))]);
    assert_eq!(h.plan_topic()["active"]["current_index"], 0);
    assert_eq!(h.plan_topic()["active"]["off_route"], true);
}

#[test]
fn arrival_deletes_plan() {
    let store = plan_store();
    let mut h = Harness::new();
    h.ready_ship();
    let out = h.accept_route(&[("Sol", 10), ("Colonia", 12)]);
    run_tasks(&store, out.tasks);
    assert_eq!(saved_rows(&store), json!([["TESTER", 0]]));
    let out = h.jump_to("Colonia", 12, 10.0);
    assert_eq!(sync_tasks(&out.tasks), 1);
    run_tasks(&store, out.tasks);
    assert_eq!(saved_rows(&store), json!([]));
    assert!(h.plan_topic()["active"].is_null());
    assert_eq!(h.plan_topic()["status"], "idle");
    assert_eq!(h.route()["source"], "current");
}

#[test]
fn clear_plan_deletes_and_replay_jumps_do_not_write() {
    let store = plan_store();
    let mut h = Harness::new();
    h.ready_ship();
    let out = h.accept_route(&[("Sol", 10), ("X", 11), ("Y", 12), ("Colonia", 13)]);
    run_tasks(&store, out.tasks);
    // Saltos reproducidos: sin escrituras hasta el primer evento en vivo.
    let out = h.journal_at(
        150,
        "FSDJump",
        r#","StarSystem":"X","SystemAddress":11,"StarPos":[10,0,0],"JumpDist":10.0"#,
        true,
    );
    assert_eq!(sync_tasks(&out.tasks), 0);
    let out = h.journal_at(
        151,
        "FSDJump",
        r#","StarSystem":"Y","SystemAddress":12,"StarPos":[20,0,0],"JumpDist":10.0"#,
        true,
    );
    assert_eq!(sync_tasks(&out.tasks), 0);
    assert_eq!(saved_rows(&store), json!([["TESTER", 0]]));
    let out = h.status(10.0, 0.5);
    assert_eq!(sync_tasks(&out.tasks), 1);
    run_tasks(&store, out.tasks);
    assert_eq!(saved_rows(&store), json!([["TESTER", 2]]));

    let out = h.act("navigation.clear-plan", json!({}));
    run_tasks(&store, out.tasks);
    assert_eq!(saved_rows(&store), json!([]));
    assert_eq!(h.plan_topic()["status"], "idle");
    assert!(h.plan_topic()["active"].is_null());
}

// ---- Planificador: ronda de correcciones 1 ----

#[test]
fn recalculate_with_non_object_params_uses_defaults_without_panic() {
    let mut h = Harness::new();
    h.ready_ship();
    let mut v = stored_plan(0);
    v["plan"]["params"] = json!("x");
    h.nav.load_pending = true;
    h.nav.plan_cmdr = Some("TESTER".into());
    h.location("S1", 1, 0.0);
    feed(&mut h, vec![("plan".into(), v)]);
    assert_eq!(h.plan_topic()["status"], "active");
    h.act("navigation.recalculate", json!({}));
    let reqs = h.route_requests();
    assert_eq!(reqs.len(), 1);
    let NetQuery::NeutronRoute(p) = &reqs[0].query else {
        panic!("{:?}", reqs[0].query);
    };
    assert_eq!(p.from, "S1");
    assert_eq!(p.to, "S4");
    assert_eq!(p.efficiency, 60);
}

#[test]
fn settings_toggle_reenables_planner_and_invalidates_computing_request() {
    let mut h = Harness::new();
    h.ready_ship();
    h.settings(false);
    h.act("navigation.plan", json!({"to": "S2", "mode": "neutron"}));
    assert_eq!(h.plan_topic()["status"], "disabled");
    h.settings(true);
    assert_eq!(h.plan_topic()["status"], "idle");
    assert!(h.plan_topic()["error_key"].is_null());
    h.act("navigation.plan", json!({"to": "S2", "mode": "neutron"}));
    assert_eq!(h.plan_topic()["status"], "computing");
    let id = h.route_requests()[0].id;
    h.settings(false);
    assert_eq!(h.plan_topic()["status"], "disabled");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-disabled-offline");
    h.respond_id(id, NetOutcome::Route(planned(&[("Sol", 10), ("S2", 2)])));
    assert_eq!(h.plan_topic()["status"], "disabled");
    assert!(h.plan_topic()["result"].is_null());
}

#[test]
fn invalid_plan_arguments_fail_with_bad_request() {
    let long = "x".repeat(129);
    let cases = [
        json!({"mode": "neutron"}),
        json!({"to": "", "mode": "neutron"}),
        json!({"to": long, "mode": "neutron"}),
        json!({"to": "A", "from": long, "mode": "neutron"}),
        json!({"to": "A", "mode": "neutron", "efficiency": 0}),
        json!({"to": "A", "mode": "neutron", "efficiency": 101}),
        json!({"to": "A", "mode": "neutron", "efficiency": "x"}),
        json!({"to": "A"}),
        json!({"to": "A", "mode": "warp"}),
        json!({"to": 5, "mode": "neutron"}),
    ];
    for args in cases {
        let mut h = Harness::new();
        h.ready_ship();
        h.act("navigation.plan", args.clone());
        assert_eq!(h.plan_topic()["status"], "failed", "{args}");
        assert_eq!(
            h.plan_topic()["error_key"],
            "nav-plan-error-bad-request",
            "{args}"
        );
        assert!(h.route_requests().is_empty(), "{args}");
    }
    let mut h = Harness::new();
    h.ready_ship();
    h.act(
        "navigation.plan",
        json!({"to": "A", "mode": "neutron", "efficiency": 100}),
    );
    assert_eq!(h.plan_topic()["status"], "computing");
}

#[test]
fn oversized_route_is_rejected_and_topic_stays_small() {
    let mut h = Harness::new();
    h.ready_ship();
    let hops =
        |n: u64| -> Vec<(String, u64)> { (0..n).map(|i| (format!("System {i}"), i)).collect() };
    let big = hops(2000);
    let refs: Vec<(&str, u64)> = big.iter().map(|(n, i)| (n.as_str(), *i)).collect();
    h.act("navigation.plan", json!({"to": "Far", "mode": "neutron"}));
    h.respond(NetOutcome::Route(planned(&refs)));
    let topic = h.plan_topic();
    assert_eq!(topic["status"], "ready");
    assert_eq!(topic["result"]["hops_total"], 2000);
    assert_eq!(topic["result"]["hops"].as_array().unwrap().len(), 50);
    assert!(
        topic.to_string().len() < 16 * 1024,
        "{}",
        topic.to_string().len()
    );

    let huge = hops(2001);
    let refs: Vec<(&str, u64)> = huge.iter().map(|(n, i)| (n.as_str(), *i)).collect();
    h.act("navigation.plan", json!({"to": "Far", "mode": "neutron"}));
    h.respond(NetOutcome::Route(planned(&refs)));
    assert_eq!(h.plan_topic()["status"], "failed");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-error-route-too-long");
}

#[test]
fn clear_plan_before_stored_arrives_does_not_resurrect_it() {
    let mut h = Harness::new();
    h.load_game();
    h.location("S1", 1, 0.0);
    h.act("navigation.clear-plan", json!({}));
    feed(&mut h, vec![("plan".into(), stored_plan(0))]);
    assert!(h.plan_topic()["active"].is_null());
    assert_eq!(h.plan_topic()["status"], "idle");
}

#[test]
fn request_dropped_by_context_fails_with_service_error() {
    let mut h = Harness::new();
    h.ready_ship();
    h.event(
        SourceEvent::Action {
            name: "navigation.plan".into(),
            args: json!({"to": "A", "mode": "neutron"}),
        },
        true,
    );
    assert_eq!(h.plan_topic()["status"], "failed");
    assert_eq!(h.plan_topic()["error_key"], "nav-plan-error-service");
    assert!(h.nav.planner.req_id.is_none());
}

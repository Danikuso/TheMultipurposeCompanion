use super::*;
use chrono::TimeZone;
use std::collections::HashMap;
use tmc_core::{EventWriter, Store, StoredEvent};
use tmc_model::net::{NetRequest, NetSource, SystemCoords};

const MISSIONS: &str = include_str!("../../../../tests/fixtures/journal/missions_session.log");
const MASSACRE: &str = include_str!("../../../../tests/fixtures/journal/massacre_session.log");
const TOURISM: &str = include_str!("../../../../tests/fixtures/journal/tourism_session.log");

struct H {
    m: Missions,
    state: GameState,
    replaying: bool,
    exec: bool,
    store: Store,
    requests: Vec<NetRequest>,
    tags: Vec<String>,
    pending: Vec<(String, StoreTask)>,
    results: HashMap<String, Value>,
    next_id: u64,
    line_no: u64,
}

impl H {
    fn new() -> Self {
        Self {
            m: Missions::new(),
            state: GameState::default(),
            replaying: false,
            exec: false,
            store: Store::open_in_memory().unwrap(),
            requests: Vec::new(),
            tags: Vec::new(),
            pending: Vec::new(),
            results: HashMap::new(),
            next_id: 1,
            line_no: 0,
        }
    }

    fn with_store() -> Self {
        Self {
            exec: true,
            ..Self::new()
        }
    }

    fn send(&mut self, ev: SourceEvent) -> TopicSet {
        if let SourceEvent::Journal { ev, .. } = &ev {
            self.state.apply_event(ev);
        }
        self.dispatch(ev)
    }

    fn dispatch(&mut self, ev: SourceEvent) -> TopicSet {
        let mut ctx = ModuleCtx::new("missions", self.replaying, self.next_id);
        let mut dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        for (tag, task) in tasks {
            self.tags.push(tag.clone());
            if self.exec {
                dirty.extend(self.run(tag, task));
            } else {
                self.pending.push((tag, task));
            }
        }
        dirty
    }

    fn run(&mut self, tag: String, task: StoreTask) -> TopicSet {
        match task(&self.store) {
            Ok(value) => {
                self.results.insert(tag.clone(), value.clone());
                self.dispatch(SourceEvent::Stored {
                    module: "missions".into(),
                    tag,
                    value,
                })
            }
            Err(_) => TopicSet::new(),
        }
    }

    fn run_pending(&mut self) {
        for (tag, task) in std::mem::take(&mut self.pending) {
            self.run(tag, task);
        }
    }

    fn line(&mut self, line: &str) -> TopicSet {
        self.line_no += 1;
        self.send(SourceEvent::Journal {
            ev: parse_line(line).unwrap(),
            file: "m.log".into(),
            line: self.line_no,
            replaying: self.replaying,
        })
    }

    fn lines(&mut self, text: &str) {
        for l in text.lines().filter(|l| !l.trim().is_empty()) {
            self.line(l);
        }
    }

    fn journal(&mut self, ts: &str, name: &str, extra: &str) -> TopicSet {
        self.line(&format!(
            r#"{{"timestamp":"{ts}","event":"{name}"{extra}}}"#
        ))
    }

    fn val(&self, topic: &str) -> Value {
        self.m.topic_value(topic).unwrap_or(Value::Null)
    }

    fn coords(&mut self, name: &str, pos: [f64; 3]) {
        let key = format!("coords:{}", coord_key(name));
        self.dispatch(SourceEvent::Net(NetResponse {
            id: 0,
            module: "missions".into(),
            key,
            source: NetSource::Ardent,
            fetched_at: Utc::now(),
            outcome: NetOutcome::Coords(SystemCoords {
                name: name.into(),
                id64: None,
                star_pos: pos,
            }),
        }));
    }
}

/// Guarda en el almacén las líneas del diario como lo haría la importación.
fn write_events(store: &Store, text: &str) {
    let writer = EventWriter::spawn(store);
    for (i, raw) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let ev = parse_line(raw).unwrap();
        writer.send(StoredEvent {
            file_name: "Journal.2026-10-20T100000.01.log".into(),
            line: i as u64 + 1,
            ts: ev.timestamp,
            name: ev.name,
            cmdr: Some("TESTER".into()),
            json: raw.to_owned(),
        });
    }
    writer.flush_blocking().unwrap();
}

fn active_ids(h: &H) -> Vec<u64> {
    let mut ids: Vec<u64> = h.val(TOPIC_ACTIVE)["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["missions"].as_array().unwrap().clone())
        .map(|m| m["id"].as_u64().unwrap())
        .collect();
    ids.sort();
    ids
}

fn accept_line(id: u64, system: &str, expiry: &str) -> String {
    format!(
        r#"{{"timestamp":"2026-10-20T12:00:00Z","event":"MissionAccepted","Faction":"Alpha Union","Name":"Mission_Courier","LocalisedName":"Courier","DestinationSystem":"{system}","Expiry":"{expiry}","Reward":1000,"MissionID":{id}}}"#
    )
}

fn here(h: &mut H) {
    h.journal(
        "2026-10-20T11:59:00Z",
        "Commander",
        r#","FID":"F0000000","Name":"TESTER""#,
    );
    h.journal(
        "2026-10-20T11:59:01Z",
        "Location",
        r#","StarSystem":"Here","SystemAddress":9,"StarPos":[0.0,0.0,0.0],"Docked":true,"StationName":"Here Port","MarketID":1"#,
    );
}

#[test]
fn missions_fixture_matches_missions_event_after_restart() {
    let (first, second) = MISSIONS.split_at(
        MISSIONS
            .find("{\"timestamp\":\"2026-10-20T11:00:00Z\"")
            .unwrap(),
    );
    let mut live = H::new();
    live.lines(first);
    assert_eq!(active_ids(&live), vec![65000101, 65000102, 65000103]);
    // «Reinicio»: módulo nuevo, el historial en el almacén y la segunda mitad en vivo.
    let mut h = H::with_store();
    write_events(&h.store, first);
    h.lines(second);
    assert_eq!(active_ids(&h), vec![65000101, 65000102, 65000103]);
    let groups = h.val(TOPIC_ACTIVE)["groups"].as_array().unwrap().clone();
    assert_eq!(groups.len(), 2);
    let beta = groups.iter().find(|g| g["system"] == "Beta Reach").unwrap();
    assert_eq!(beta["station"], "Beta Port");
    assert_eq!(beta["missions"].as_array().unwrap().len(), 2);
    for g in &groups {
        for m in g["missions"].as_array().unwrap() {
            assert_eq!(m["details_missing"], false, "{m}");
        }
    }
}

fn massacre_missions_event() -> &'static str {
    r#"{"timestamp":"2026-10-20T12:45:00Z","event":"Missions","Active":[{"MissionID":65000001,"Name":"Mission_Massacre_name","Expires":600000},{"MissionID":65000002,"Name":"Mission_Massacre_name","Expires":600000},{"MissionID":65000003,"Name":"Mission_Massacre_name","Expires":600000},{"MissionID":65000004,"Name":"Mission_Massacre_name","Expires":600000},{"MissionID":65000005,"Name":"Mission_Massacre_name","Expires":600000}],"Failed":[],"Complete":[]}"#
}

fn restart_massacre(h: &mut H) {
    write_events(&h.store, MASSACRE);
    h.journal(
        "2026-10-20T12:44:00Z",
        "Commander",
        r#","FID":"F0000000","Name":"TESTER""#,
    );
    h.journal(
        "2026-10-20T12:44:01Z",
        "LoadGame",
        r#","FID":"F0000000","Commander":"TESTER","Ship":"python","ShipID":1"#,
    );
    h.line(massacre_missions_event());
}

#[test]
fn massacre_rebuilt_after_restart_matches_live() {
    let mut live = H::new();
    live.lines(MASSACRE);
    let mut rebuilt = H::with_store();
    restart_massacre(&mut rebuilt);
    let a = live.val(TOPIC_MASSACRE)["piles"].clone();
    let b = rebuilt.val(TOPIC_MASSACRE)["piles"].clone();
    assert!(!a.as_array().unwrap().is_empty());
    assert_eq!(a, b);
}

#[test]
fn bounty_rebuild_only_after_accepted_result() {
    let mut h = H::new();
    restart_massacre(&mut h);
    assert_eq!(h.tags, vec![TAG_ACCEPTED.to_owned()]);
    h.run_pending();
    assert!(h.tags.iter().any(|t| t == TAG_BOUNTIES), "{:?}", h.tags);
    h.run_pending();
    let since: DateTime<Utc> =
        serde_json::from_value(h.results[TAG_BOUNTIES]["since"].clone()).unwrap();
    assert_eq!(since, Utc.with_ymd_and_hms(2026, 10, 20, 12, 0, 0).unwrap());
}

#[test]
fn next_stop_is_nearest_then_earliest_expiry() {
    let mut h = H::new();
    here(&mut h);
    h.m.resolver.learn("Lave", [5.0, 0.0, 0.0]);
    h.m.resolver.learn("Diso", [0.0, 5.0, 0.0]);
    h.m.resolver.learn("Far", [50.0, 0.0, 0.0]);
    h.line(&accept_line(1, "Far", "2026-10-20T12:30:00Z"));
    h.line(&accept_line(2, "Lave", "2026-10-20T14:00:00Z"));
    h.line(&accept_line(3, "Diso", "2026-10-20T13:00:00Z"));
    let v = h.val(TOPIC_ACTIVE);
    assert_eq!(v["next_system"], "Diso");
    let order: Vec<&str> = v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["system"].as_str().unwrap())
        .collect();
    assert_eq!(order, vec!["Diso", "Lave", "Far"]);
    assert_eq!(v["groups"][0]["next"], true);
    assert_eq!(v["groups"][0]["distance_ly"], 5.0);
}

#[test]
fn groups_by_expiry_without_coords() {
    let mut h = H::new();
    here(&mut h);
    h.line(&accept_line(1, "Lave", "2026-10-20T14:00:00Z"));
    h.line(&accept_line(2, "Diso", "2026-10-20T13:00:00Z"));
    h.line(&accept_line(3, "Leesti", "2026-10-20T13:30:00Z"));
    let v = h.val(TOPIC_ACTIVE);
    let order: Vec<&str> = v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["system"].as_str().unwrap())
        .collect();
    assert_eq!(order, vec!["Diso", "Leesti", "Lave"]);
    assert_eq!(v["next_system"], "Diso");
    assert!(v["groups"][0]["distance_ly"].is_null());
}

#[test]
fn tourism_computed_on_accept_with_fake_coords() {
    let mut h = H::with_store();
    h.lines(TOURISM);
    assert!(h.tags.iter().any(|t| t == TAG_COORDS));
    let keys: Vec<&str> = h.requests.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(
        keys,
        vec!["coords:tour a", "coords:tour b", "coords:tour c"]
    );
    assert_eq!(h.val(TOPIC_TOURISM)["trips"][0]["status"], "loading");
    h.coords("Tour A", [10.0, 0.0, 0.0]);
    h.coords("Tour B", [-10.0, 0.0, 0.0]);
    h.coords("Tour C", [11.0, 0.0, 0.0]);
    let t = h.val(TOPIC_TOURISM)["trips"][0].clone();
    assert_eq!(t["status"], "ok");
    let p = &t["plan"];
    assert!((p["board_order_ly"].as_f64().unwrap() - 51.0).abs() < 1e-9);
    assert!((p["optimal_ly"].as_f64().unwrap() - 31.0).abs() < 1e-9);
    assert!((p["cr_per_ly"].as_f64().unwrap() - 10_000.0).abs() < 1e-6);
    assert_eq!(p["order"], json!(["Tour B", "Tour A", "Tour C"]));
    assert_eq!(p["can_abandon_now"], true);
    assert!(h.tags.iter().any(|t| t == TAG_STATS_WRITE));
}

#[test]
fn replay_emits_no_requests_or_store_writes() {
    let mut h = H::with_store();
    h.replaying = true;
    h.lines(TOURISM);
    h.lines(MISSIONS);
    assert!(h.requests.is_empty());
    assert!(
        !h.tags.iter().any(|t| t.ends_with(".write")),
        "{:?}",
        h.tags
    );
    assert!(h.tags.is_empty(), "{:?}", h.tags);
}

#[test]
fn no_coord_requests_in_legacy_or_offline() {
    let mut offline = H::with_store();
    offline.send(SourceEvent::Settings(ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    }));
    offline.lines(TOURISM);
    let mut legacy = H::with_store();
    legacy.state.galaxy = Galaxy::Legacy;
    legacy.lines(TOURISM);
    for h in [&offline, &legacy] {
        assert!(h.requests.is_empty());
        assert_eq!(h.val(TOPIC_TOURISM)["trips"][0]["status"], "no_coords");
    }
    assert!(!legacy.tags.iter().any(|t| t.ends_with(".write")));
}

#[test]
fn commander_switch_clears_missions() {
    let mut h = H::new();
    h.lines(
        MISSIONS
            .split("\n")
            .take(7)
            .collect::<Vec<_>>()
            .join("\n")
            .as_str(),
    );
    assert!(!active_ids(&h).is_empty());
    h.journal(
        "2026-10-20T10:30:00Z",
        "Commander",
        r#","FID":"F0000002","Name":"B""#,
    );
    assert!(active_ids(&h).is_empty());
    assert_eq!(h.val(TOPIC_ACTIVE)["count"], 0);
}

#[test]
fn stored_result_for_other_cmdr_is_ignored() {
    let mut h = H::new();
    let second = &MISSIONS[MISSIONS
        .find("{\"timestamp\":\"2026-10-20T11:00:00Z\"")
        .unwrap()..];
    h.lines(second);
    let before = h.val(TOPIC_ACTIVE);
    let accepted: Vec<Value> = MISSIONS
        .lines()
        .filter(|l| l.contains("MissionAccepted"))
        .map(|l| json!({"ts": "2026-10-20T10:05:00Z", "json": l}))
        .collect();
    let dirty = h.dispatch(SourceEvent::Stored {
        module: "missions".into(),
        tag: TAG_ACCEPTED.into(),
        value: json!({"cmdr": "OTRO", "accepted": accepted}),
    });
    assert!(dirty.is_empty());
    assert_eq!(h.val(TOPIC_ACTIVE), before);
    assert!(!h.tags.iter().any(|t| t == TAG_BOUNTIES));
}

#[test]
fn missions_topics_stay_under_16_kib() {
    let mut h = H::new();
    here(&mut h);
    for i in 0..20u64 {
        let target = format!("Pirate Clan Number {}", i % 5);
        h.line(&format!(
            r#"{{"timestamp":"2026-10-20T12:00:00Z","event":"MissionAccepted","Faction":"Issuer Faction Of Long Name {}","Name":"Mission_Massacre","LocalisedName":"Kill {target} pirates for the good of the system","TargetFaction":"{target}","KillCount":24,"DestinationSystem":"Destination System {i}","DestinationStation":"Destination Station {i}","Expiry":"2026-10-27T12:00:00Z","Reward":12000000,"MissionID":{}}}"#,
            i % 3,
            65_000_000 + i
        ));
    }
    let goals: Vec<Value> = (0..10)
        .map(|i| {
            json!({"CGID": i, "Title": format!("Community goal with a very long descriptive title {i}"),
                "SystemName": "Goal System", "MarketName": "Goal Market", "Expiry": "2026-10-30T12:00:00Z",
                "IsComplete": false, "CurrentTotal": 123456789, "PlayerContribution": 1234567,
                "NumContributors": 9999, "PlayerPercentileBand": 25, "TierReached": "Tier 4", "Bonus": 50000000})
        })
        .collect();
    h.line(
        &json!({"timestamp":"2026-10-20T12:10:00Z","event":"CommunityGoal","CurrentGoals": goals})
            .to_string(),
    );
    assert_eq!(h.val(TOPIC_MASSACRE)["piles"].as_array().unwrap().len(), 5);
    assert_eq!(h.val(TOPIC_CG)["goals"].as_array().unwrap().len(), 10);
    for t in TOPICS {
        let len = h.val(t).to_string().len();
        assert!(len < 16384, "{t}: {len}");
    }
}

#[test]
fn relevance_active_with_missions_docked_or_supercruise() {
    let mut h = H::new();
    here(&mut h);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.line(&accept_line(1, "Lave", "2026-10-20T14:00:00Z"));
    h.state.location.mode = FlightMode::Docked;
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.state.location.mode = FlightMode::Supercruise;
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.state.location.mode = FlightMode::NormalSpace;
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
}

#[test]
fn to_route_bumps_copy_seq_live_only_and_dismiss_removes_alerts() {
    let mut h = H::new();
    here(&mut h);
    h.send(SourceEvent::Action {
        name: ACTION_TO_ROUTE.into(),
        args: json!({"system": "Lave"}),
    });
    let v = h.val(TOPIC_ACTIVE);
    assert_eq!(v["copy_target"], "Lave");
    assert_eq!(v["copy_seq"], 1);
    // Caduca en 30 min: aviso; descartarlo lo quita y no vuelve.
    h.line(&accept_line(7, "Lave", "2026-10-20T12:30:00Z"));
    assert_eq!(h.val(TOPIC_ALERTS)["alerts"].as_array().unwrap().len(), 1);
    h.send(SourceEvent::Action {
        name: ACTION_DISMISS.into(),
        args: json!({"mission_id": 7}),
    });
    assert!(h.val(TOPIC_ALERTS)["alerts"].as_array().unwrap().is_empty());
    h.journal(
        "2026-10-20T12:01:00Z",
        "Music",
        r#","MusicTrack":"Exploration""#,
    );
    assert!(h.val(TOPIC_ALERTS)["alerts"].as_array().unwrap().is_empty());
}

#[test]
fn migrations_include_passenger_stats() {
    assert_eq!(Missions::migrations()[0].id, "missions-001");
    assert_eq!(Missions::new().id(), "missions");
}

fn net_outcome(h: &mut H, name: &str, outcome: NetOutcome) {
    h.dispatch(SourceEvent::Net(NetResponse {
        id: 0,
        module: "missions".into(),
        key: format!("coords:{}", coord_key(name)),
        source: NetSource::Ardent,
        fetched_at: Utc::now(),
        outcome,
    }));
}

fn music(h: &mut H, ts: &str) {
    h.journal(ts, "Music", r#","MusicTrack":"Exploration""#);
}

#[test]
fn tourism_trip_rebuilt_after_restart() {
    let mut h = H::with_store();
    write_events(&h.store, TOURISM);
    h.lines(
        &TOURISM
            .lines()
            .filter(|l| !l.contains("MissionAccepted"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    h.journal(
        "2026-10-20T15:00:00Z",
        "Missions",
        r#","Active":[{"MissionID":65000201,"Name":"Mission_Sightseeing_name","PassengerMission":true,"Expires":200000}],"Failed":[],"Complete":[]"#,
    );
    let t = h.val(TOPIC_TOURISM)["trips"][0].clone();
    assert_eq!(t["mission_id"], 65000201);
    assert_eq!(t["status"], "loading");
    h.coords("Tour A", [10.0, 0.0, 0.0]);
    h.coords("Tour B", [-10.0, 0.0, 0.0]);
    h.coords("Tour C", [11.0, 0.0, 0.0]);
    assert_eq!(h.val(TOPIC_TOURISM)["trips"][0]["status"], "ok");
    assert!(h.tags.iter().any(|t| t == TAG_STATS_WRITE));
}

#[test]
fn stored_redirect_older_than_live_is_skipped() {
    let accepted = MASSACRE
        .lines()
        .find(|l| l.contains("65000004") && l.contains("MissionAccepted"))
        .unwrap();
    let old = r#"{"timestamp":"2026-10-20T12:30:00Z","event":"MissionRedirected","MissionID":65000004,"Name":"Mission_Massacre_name","NewDestinationStation":"Old Port","NewDestinationSystem":"Old Place"}"#;
    let live = MASSACRE.lines().last().unwrap();
    let mut h = H::new();
    write_events(&h.store, &format!("{accepted}\n{old}\n{live}"));
    h.journal(
        "2026-10-20T12:00:00Z",
        "Commander",
        r#","FID":"F0000000","Name":"TESTER""#,
    );
    h.line(accepted);
    h.line(live);
    h.line(massacre_missions_event());
    h.run_pending();
    h.run_pending();
    assert!(h.results.contains_key(TAG_BOUNTIES));
    let m = h.m.book.get(65000004).unwrap();
    assert_eq!(m.destination_system.as_deref(), Some("Home"));
}

#[test]
fn coords_failed_retries_after_cooldown_and_no_data_never() {
    use tmc_model::net::NetFailure;
    let mut h = H::with_store();
    here(&mut h);
    h.line(&accept_line(1, "Lave", "2026-10-21T12:00:00Z"));
    assert_eq!(h.requests.len(), 1);
    net_outcome(&mut h, "Lave", NetOutcome::Failed(NetFailure::Timeout));
    music(&mut h, "2026-10-20T12:01:00Z");
    assert_eq!(h.requests.len(), 1);
    music(&mut h, "2026-10-20T12:06:00Z");
    assert_eq!(h.requests.len(), 2);
    net_outcome(&mut h, "Lave", NetOutcome::NoData);
    music(&mut h, "2026-10-20T12:30:00Z");
    assert_eq!(h.requests.len(), 2);
}

#[test]
fn discarded_local_coords_are_asked_again() {
    let mut h = H::new();
    here(&mut h);
    h.line(&accept_line(1, "Lave", "2026-10-21T12:00:00Z"));
    assert_eq!(h.tags.iter().filter(|t| *t == TAG_COORDS).count(), 1);
    h.dispatch(SourceEvent::Stored {
        module: "missions".into(),
        tag: TAG_COORDS.into(),
        value: json!({"cmdr": "OTRO", "names": ["Lave"], "coords": {}}),
    });
    music(&mut h, "2026-10-20T12:01:00Z");
    assert_eq!(h.tags.iter().filter(|t| *t == TAG_COORDS).count(), 2);
}

/// Instantánea JSON de los topics tal como los ve la UI (contrato serde Rust → TypeScript).
fn contract_snapshot() -> Value {
    let mut h = H::with_store();
    h.lines(MASSACRE);
    let ids = active_ids(&h);
    let ids: Vec<u64> = if ids.is_empty() { vec![1] } else { ids };
    let at = h.m.clock();
    let kinds = [
        AlertKind::ExpiringSoon,
        AlertKind::Redirected {
            system: "Redirected System".into(),
            station: Some("Redirected Port".into()),
        },
        AlertKind::Failed,
        AlertKind::CargoMissing {
            commodity: "gold".into(),
            missing: 14,
        },
    ];
    for (i, kind) in kinds.into_iter().enumerate() {
        h.m.alerts.push(MissionAlert {
            mission_id: ids[i % ids.len()],
            kind,
            at,
            bucket: i as i64,
        });
    }
    h.m.rewards.credits = 6_300_000;
    h.m.rewards.completed = 7;
    h.m.rewards.failed = 1;
    h.m.rewards.abandoned = 2;
    h.m.rewards.materials.insert("Modular Terminals".into(), 3);
    h.m.rewards.factions.insert(
        "Liberty Party".into(),
        FactionDelta {
            reputation: "UpGood".into(),
            influence: "Down".into(),
            reputation_score: 2,
            influence_score: -1,
        },
    );
    let state = h.state.clone();
    let active = h.m.render_active(&state);
    let massacre = h.m.stamp(json!({"piles": h.m.stack.view(&h.m.book)}));
    let alerts = h.m.render_alerts();
    let rewards = h.m.stamp(serde_json::to_value(&h.m.rewards).unwrap());

    let mut t = H::with_store();
    t.lines(TOURISM);
    t.coords("Tour A", [10.0, 0.0, 0.0]);
    t.coords("Tour B", [-10.0, 0.0, 0.0]);
    t.coords("Tour C", [11.0, 0.0, 0.0]);
    let profitable = t.val(TOPIC_TOURISM);
    let mut tourism = profitable.clone();
    let mut worse = tourism["trips"][0].clone();
    let trip =
        t.m.trips
            .values()
            .next()
            .and_then(|x| x.plan.clone())
            .unwrap();
    let not_worth = TourismPlan {
        verdict: Verdict::Profitable,
        ..trip
    };
    worse["mission_id"] = json!(99);
    worse["plan"] = serde_json::to_value(not_worth).unwrap();
    tourism["trips"].as_array_mut().unwrap().push(worse);
    // Estados sin plan: lo que publica `render_tourism` con cada `status`.
    for (id, status) in [
        (100, "loading"),
        (101, "no_coords"),
        (102, "stops_unavailable"),
    ] {
        tourism["trips"].as_array_mut().unwrap().push(json!({
            "mission_id": id, "name": "Tour", "stops": ["Tour A"], "status": status, "plan": null,
        }));
    }

    let mut cabins = Cabins::default();
    cabins.by_class.insert(
        crate::data::CabinClass::Economy,
        CabinOccupancy {
            occupied: Some(12),
            seats: 16,
        },
    );
    cabins.by_class.insert(
        crate::data::CabinClass::Business,
        CabinOccupancy {
            occupied: None,
            seats: 22,
        },
    );
    cabins.occupied_total = 12;
    cabins.seats_total = 38;
    let groups = vec![
        PassengerGroup {
            destination: Some("Sol".into()),
            count: 12,
            vip: false,
            wanted: true,
            mission_id: 1,
        },
        PassengerGroup {
            destination: None,
            count: 4,
            vip: true,
            wanted: false,
            mission_id: 2,
        },
    ];
    let passengers =
        h.m.stamp(json!({"cabins": cabins, "groups": groups, "wanted": true}));
    let cg = CgView {
        cgid: 1,
        title: "Goal".into(),
        system: Some("Sol".into()),
        station: None,
        contribution: 5000,
        percentile: Some(25),
        tier: Some("Tier 2".into()),
        expiry: Some(at),
        bonus: Some(1_000_000),
        complete: false,
    };
    let goals = h.m.stamp(json!({"goals": [cg]}));
    json!({
        "missions.active": active,
        "missions.massacre": massacre,
        "missions.alerts": alerts,
        "missions.rewards": rewards,
        "missions.passengers": passengers,
        "missions.tourism": tourism,
        "missions.community_goals": goals,
    })
}

#[test]
fn ui_contract_snapshot_is_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/tests/fixtures/missions-topics.json");
    let text = serde_json::to_string_pretty(&contract_snapshot()).unwrap() + "\n";
    if std::env::var_os("UPDATE_CONTRACTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        current.replace("\r\n", "\n"),
        text,
        "el contrato JSON de missions.* cambió: regenéralo con UPDATE_CONTRACTS=1 cargo test -p tmc-modules ui_contract_snapshot"
    );
}

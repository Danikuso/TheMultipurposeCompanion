use super::*;
use chrono::Duration;
use serde_json::json;
use tmc_model::events::parse_line;
use tmc_model::net::{NetOutcome, NetRequest, NetResponse, NetSource, PadSize, RemoteStation};
use tmc_model::status::{Flags, Status};

const FIXTURE: &str = include_str!("../../../../tests/fixtures/journal/combat_session.log");

fn t(seconds: i64) -> DateTime<Utc> {
    "2026-10-18T12:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
}

struct H {
    m: Combat,
    state: GameState,
    replaying: bool,
    requests: Vec<NetRequest>,
    tags: Vec<String>,
    next_id: u64,
}

impl H {
    fn new() -> Self {
        Self {
            m: Combat::new(),
            state: GameState::default(),
            replaying: false,
            requests: Vec::new(),
            tags: Vec::new(),
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
        let mut ctx = ModuleCtx::new("combat", self.replaying, self.next_id);
        let dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        self.tags.extend(tasks.into_iter().map(|(tag, _)| tag));
        dirty
    }

    fn line(&mut self, line: &str) -> TopicSet {
        self.send(SourceEvent::Journal {
            ev: parse_line(line).unwrap(),
            file: "s.log".into(),
            line: 1,
            replaying: self.replaying,
        })
    }

    fn journal(&mut self, at: i64, name: &str, extra: &str) -> TopicSet {
        let line = format!(
            r#"{{"timestamp":"{}","event":"{name}"{extra}}}"#,
            t(at).to_rfc3339()
        );
        self.line(&line)
    }

    fn status(&mut self, at: i64, flags: Flags) -> TopicSet {
        let text = format!(
            r#"{{"timestamp":"{}","event":"Status","Flags":{}}}"#,
            t(at).to_rfc3339(),
            flags.bits()
        );
        self.send(SourceEvent::Status(Status::parse(&text).unwrap()))
    }

    fn jump(&mut self, at: i64, system: &str, addr: u64) {
        self.journal(
            at,
            "FSDJump",
            &format!(r#","StarSystem":"{system}","SystemAddress":{addr},"StarPos":[1,0,0]"#),
        );
    }

    fn start(&mut self) {
        self.journal(0, "Commander", r#","FID":"F1","Name":"TESTER""#);
        self.journal(
            1,
            "LoadGame",
            r#","FID":"F1","Commander":"TESTER","Odyssey":true,"Ship":"python""#,
        );
        self.journal(
            2,
            "Location",
            r#","StarSystem":"Alpha Test","SystemAddress":1,"StarPos":[0,0,0]"#,
        );
    }

    fn bounty(&mut self, at: i64, faction: &str, reward: u64) {
        self.journal(
            at,
            "Bounty",
            &format!(
                r#","Rewards":[{{"Faction":"{faction}","Reward":{reward}}}],"Target":"asp","TotalReward":{reward},"VictimFaction":"Pirates""#
            ),
        );
    }

    fn lock(&mut self, at: i64, locked: bool) -> TopicSet {
        self.journal(
            at,
            "ShipTargeted",
            &format!(
                r#","TargetLocked":{locked},"Ship":"anaconda","ScanStage":3,"LegalStatus":"Wanted","Bounty":42000"#
            ),
        )
    }

    fn val(&self, topic: &str) -> serde_json::Value {
        self.m.topic_value(topic).unwrap()
    }

    fn if_requests(&self) -> Vec<&NetRequest> {
        self.requests
            .iter()
            .filter(|r| r.key.starts_with("if:"))
            .collect()
    }
}

fn station_response(id: u64, key: &str) -> SourceEvent {
    SourceEvent::Net(NetResponse {
        id,
        module: "combat".into(),
        key: key.into(),
        source: NetSource::Ardent,
        fetched_at: t(100),
        outcome: NetOutcome::Stations(vec![RemoteStation {
            station_name: "Hub".into(),
            system_name: "Near".into(),
            distance_ly: 7.5,
            distance_to_arrival_ls: Some(300.0),
            max_pad: Some(PadSize::Large),
            updated_at: None,
        }]),
    })
}

#[test]
fn combat_session_fixture_produces_expected_topics() {
    let mut h = H::new();
    let mut lines = FIXTURE.lines().collect::<Vec<_>>();
    let extra = r#"{ "timestamp":"2026-10-18T10:26:00Z", "event":"ShipTargeted", "TargetLocked":true, "Ship":"anaconda", "ScanStage":3, "LegalStatus":"Wanted", "Bounty":42000 }"#;
    lines.push(extra);
    for l in lines {
        h.line(l);
    }
    assert_eq!(
        h.val("combat.target")["badge"],
        json!({"kind":"wanted","bounty":42000})
    );
    let session = h.val("combat.session");
    let sum: u64 = session["vouchers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["amount"].as_u64().unwrap())
        .sum();
    assert_eq!(sum, 550_000);
    assert_eq!(session["if_fee_pct"], 25);
    assert_eq!(h.val("combat.conflict_zone")["kills"], 1);
    assert_eq!(h.val("combat.ax")["active"], false);
}

#[test]
fn replay_emits_no_requests_or_store_writes() {
    let mut h = H::new();
    h.replaying = true;
    for l in FIXTURE.lines() {
        h.line(l);
    }
    assert!(h.requests.is_empty());
    assert!(h.tags.is_empty());
    assert_eq!(h.val("combat.session")["kills"], 4);
}

#[test]
fn live_event_requests_ledger_rebuild_once_and_if_search() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    h.jump(20, "Beta", 2);
    assert_eq!(h.tags, vec!["combat.vouchers".to_string()]);
    let reqs = h.if_requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].key, "if:2");
    assert_eq!(
        reqs[0].query,
        NetQuery::NearestService {
            system_address: 2,
            service: NearbyService::InterstellarFactors,
            min_pad: PadSize::Medium,
        }
    );
}

#[test]
fn unknown_ship_searches_with_large_pad() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    h.state.ship = None;
    h.jump(20, "Beta", 2);
    let reqs = h.if_requests();
    assert_eq!(reqs.len(), 1);
    assert!(matches!(
        reqs[0].query,
        NetQuery::NearestService {
            min_pad: PadSize::Large,
            ..
        }
    ));
}

#[test]
fn no_requests_in_legacy_or_offline() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    h.state.galaxy = Galaxy::Legacy;
    h.jump(20, "Beta", 2);
    assert!(h.if_requests().is_empty());

    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    let settings = ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    };
    h.send(SourceEvent::Settings(settings));
    h.jump(20, "Beta", 2);
    assert!(h.if_requests().is_empty());
}

#[test]
fn stored_result_for_other_cmdr_is_ignored() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    h.send(SourceEvent::Stored {
        module: "combat".into(),
        tag: "combat.vouchers".into(),
        value: json!({"cmdr":"OTRO","ledger":{"entries":[]},"cut":9,"cut_ts":null}),
    });
    let rows = h.val("combat.session")["vouchers"].clone();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["amount"], 5000);
}

#[test]
fn stored_result_replaces_ledger_and_reapplies_newer_events_only() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000); // ts t(10): cubierto por el almacén
    h.bounty(20, "Navy", 700); // ts t(20): posterior al corte
    let entry = |amount: u64, at: i64| {
        json!({"kind":"Bounty","faction":"Navy","amount":amount,"system":"Alpha Test",
               "earned_at":t(at).to_rfc3339(),"ax":false})
    };
    h.send(SourceEvent::Stored {
        module: "combat".into(),
        tag: "combat.vouchers".into(),
        value: json!({"cmdr":"TESTER","cut":9,"cut_ts":t(10).to_rfc3339(),
            "ledger":{"entries":[entry(100_000, 5), entry(5000, 10)]}}),
    });
    let rows = h.val("combat.session")["vouchers"].clone();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["amount"], 105_700);
}

#[test]
fn late_if_response_for_previous_system_not_applied() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    h.jump(20, "X", 10);
    h.jump(21, "Y", 20);
    assert_eq!(h.if_requests().len(), 2);
    let x_id = h.if_requests()[0].id;
    h.send(station_response(x_id, "if:10"));
    assert_eq!(h.val("combat.session")["nearest_if"], json!(null));
    let y_id = h.if_requests()[1].id;
    h.send(station_response(y_id, "if:20"));
    assert_eq!(h.val("combat.session")["nearest_if"]["station_name"], "Hub");
}

#[test]
fn inflight_if_request_expires_after_60s() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    h.jump(20, "X", 10);
    assert_eq!(h.if_requests().len(), 1);
    h.jump(70, "X", 10);
    assert_eq!(h.if_requests().len(), 1, "aún en vuelo a los 50 s");
    h.jump(81, "X", 10);
    assert_eq!(h.if_requests().len(), 2, "caducada tras 60 s");
}

#[test]
fn relevance_active_with_locked_target() {
    let mut h = H::new();
    h.start();
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.lock(10, true);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.lock(12, false);
    h.status(23, Flags::empty());
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    assert_eq!(h.val("combat.target"), json!(null));
}

#[test]
fn relevance_under_attack_and_hardpoints_in_zone() {
    let mut h = H::new();
    h.start();
    h.journal(10, "UnderAttack", r#","Target":"You""#);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.status(41, Flags::empty());
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal(
        50,
        "FactionKillBond",
        r#","Reward":1000,"AwardingFaction":"A","VictimFaction":"B""#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.status(51, Flags::HARDPOINTS_DEPLOYED);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
}

#[test]
fn topics_stay_under_16_kib_with_200_factions() {
    let mut h = H::new();
    h.start();
    for i in 0..200u64 {
        h.bounty(10 + i as i64, &format!("Faction number {i}"), 1000 + i);
    }
    let session = h.val("combat.session");
    let rows = session["vouchers"].as_array().unwrap();
    assert_eq!(rows.len(), 33);
    assert_eq!(rows[32]["other"], true);
    let total: u64 = rows.iter().map(|r| r["amount"].as_u64().unwrap()).sum();
    assert_eq!(total, (0..200u64).map(|i| 1000 + i).sum::<u64>());
    assert!(serde_json::to_string(&session).unwrap().len() < 16384);
}

#[test]
fn reset_session_action_zeroes_session_only() {
    let mut h = H::new();
    h.start();
    h.bounty(10, "Navy", 5000);
    assert_eq!(h.val("combat.session")["kills"], 1);
    let dirty = h.send(SourceEvent::Action {
        name: "combat.reset_session".into(),
        args: json!({}),
    });
    assert!(dirty.contains("combat.session"));
    let session = h.val("combat.session");
    assert_eq!(session["kills"], 0);
    assert_eq!(session["credits"], 0);
    assert_eq!(session["vouchers"][0]["amount"], 5000);
}

#[test]
fn ship_topic_reports_overheating_and_identical_status_is_not_dirty() {
    let mut h = H::new();
    h.start();
    let dirty = h.status(5, Flags::OVERHEATING);
    assert!(dirty.contains("combat.ship"));
    assert_eq!(h.val("combat.ship")["heat"], "overheating");
    assert!(
        h.val("combat.ship")["alerts"]
            .to_string()
            .contains("overheating")
    );
    assert!(h.status(6, Flags::OVERHEATING).is_empty());
}

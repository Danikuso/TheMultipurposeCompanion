use super::*;
use chrono::Duration;
use serde_json::json;
use tmc_model::events::parse_line;
use tmc_model::net::{NetOutcome, NetRequest, NetResponse, NetSource, PadSize, RemoteMarket};

const FIXTURE: &str = include_str!("../../../../tests/fixtures/journal/mining_session.log");

fn t(seconds: i64) -> DateTime<Utc> {
    "2026-10-18T12:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
}

struct H {
    m: Mining,
    state: GameState,
    replaying: bool,
    requests: Vec<NetRequest>,
    tags: Vec<String>,
    next_id: u64,
    line_no: u64,
}

impl H {
    fn new() -> Self {
        Self {
            m: Mining::new(),
            state: GameState::default(),
            replaying: false,
            requests: Vec::new(),
            tags: Vec::new(),
            next_id: 1,
            line_no: 0,
        }
    }

    fn send(&mut self, ev: SourceEvent) -> TopicSet {
        if let SourceEvent::Journal { ev, .. } = &ev {
            self.state.apply_event(ev);
        }
        let mut ctx = ModuleCtx::new("mining", self.replaying, self.next_id);
        let dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        self.tags.extend(tasks.into_iter().map(|(tag, _)| tag));
        dirty
    }

    fn line(&mut self, line: &str) -> TopicSet {
        self.line_no += 1;
        self.send(SourceEvent::Journal {
            ev: parse_line(line).unwrap(),
            file: "s.log".into(),
            line: self.line_no,
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

    fn jump(&mut self, at: i64, system: &str, addr: u64) {
        self.journal(
            at,
            "FSDJump",
            &format!(r#","StarSystem":"{system}","SystemAddress":{addr},"StarPos":[1,0,0]"#),
        );
    }

    fn cargo(&mut self, at: i64, items: &[(&str, u32)]) -> TopicSet {
        let inv: Vec<String> = items
            .iter()
            .map(|(n, c)| format!(r#"{{"Name":"{n}","Count":{c}}}"#))
            .collect();
        let total: u32 = items.iter().map(|i| i.1).sum();
        self.journal(
            at,
            "Cargo",
            &format!(
                r#","Vessel":"Ship","Count":{total},"Inventory":[{}]"#,
                inv.join(",")
            ),
        )
    }

    fn val(&self, topic: &str) -> serde_json::Value {
        self.m.topic_value(topic).unwrap()
    }

    fn sell_requests(&self) -> Vec<&NetRequest> {
        self.requests
            .iter()
            .filter(|r| r.key.starts_with("sell:"))
            .collect()
    }
}

fn market(name: &str, price: u64) -> RemoteMarket {
    RemoteMarket {
        station_name: format!("{name} Station"),
        system_name: format!("{name} System"),
        market_id: None,
        distance_ly: 10.0,
        distance_to_arrival_ls: None,
        sell_price: price,
        demand: Some(100),
        max_pad: Some(PadSize::Large),
        station_type: None,
        updated_at: None,
    }
}

fn markets_response(key: &str, markets: Vec<RemoteMarket>) -> SourceEvent {
    SourceEvent::Net(NetResponse {
        id: 0,
        module: "mining".into(),
        key: key.into(),
        source: NetSource::Ardent,
        fetched_at: t(100),
        outcome: NetOutcome::Markets(markets),
    })
}

const REFINED: &str = r#","Type":"$platinum_name;","Type_Localised":"Platinum""#;

#[test]
fn mining_session_fixture_matches_refined_and_tph() {
    let mut h = H::new();
    for l in FIXTURE.lines() {
        h.line(l);
    }
    // El fixture (13:00-13:06) no incluye refinados: sesión vacía, 0 t/h a los 6 min.
    assert_eq!(h.val("mining.session")["refined"], json!({}));
    assert_eq!(h.val("mining.session")["tons_per_hour"], json!(0.0));
    assert_eq!(h.val("mining.session")["asteroids_prospected"], json!(2));
    // 3 refinados a las 14:00:00 (la sesión empezó a las 13:00:00): 3 t/h.
    // 13:00:00 es t(3600).
    for i in 0..3 {
        h.journal(7200 + i, "MiningRefined", REFINED);
    }
    let s = h.val("mining.session");
    assert_eq!(s["refined"], json!({"platinum": 3}));
    let tph = s["tons_per_hour"].as_f64().unwrap();
    assert!((tph - 3.0 / (3602.0 / 3600.0)).abs() < 1e-9, "{tph}");
}

#[test]
fn prospector_topic_shows_composition_and_verdict() {
    let mut h = H::new();
    for l in FIXTURE.lines() {
        h.line(l);
    }
    let p = h.val("mining.prospector");
    assert_eq!(p["current"]["materials"].as_array().unwrap().len(), 1);
    assert_eq!(p["current"]["materials"][0]["key"], json!("bauxite"));
    assert_eq!(p["current"]["motherlode"], json!("Monazite"));
    assert_eq!(p["current"]["verdict"]["kind"], json!("worth"));
    assert_eq!(p["history"].as_array().unwrap().len(), 2);
    // El primer asteroide tiene platino de interés sin precio: una consulta.
    let reqs = h.sell_requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].key, "sell:2:platinum");
}

#[test]
fn replay_emits_no_requests_or_store_writes() {
    let mut h = H::new();
    h.replaying = true;
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    for l in FIXTURE.lines() {
        h.line(l);
    }
    assert!(h.requests.is_empty());
    assert!(h.tags.is_empty());
    // En vivo sí se pide la reconstrucción de precios una vez.
    h.replaying = false;
    h.journal(8000, "Music", r#","MusicTrack":"Exploration""#);
    assert_eq!(h.tags, vec!["mining.prices".to_owned()]);
}

#[test]
fn no_requests_in_legacy_or_offline() {
    let mut h = H::new();
    h.state.galaxy = Galaxy::Legacy;
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert!(h.requests.is_empty());
    assert_eq!(h.val("mining.sell")["status"], json!("disabled"));

    let mut h = H::new();
    h.send(SourceEvent::Settings(ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    }));
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert!(h.requests.is_empty());
    assert_eq!(h.val("mining.sell")["status"], json!("disabled"));
}

#[test]
fn stored_result_for_other_cmdr_is_ignored() {
    let mut h = H::new();
    h.start();
    assert_eq!(h.tags, vec!["mining.prices".to_owned()]);
    let stored = |cmdr: &str| SourceEvent::Stored {
        module: "mining".into(),
        tag: "mining.prices".into(),
        value: json!({"cmdr": cmdr, "prices": {"platinum": {"price": 200000, "at": "2026-10-18T11:00:00Z"}}}),
    };
    h.send(stored("OTRO"));
    assert!(h.m.prices.best_price("platinum", t(10)).is_none());
    h.send(stored("TESTER"));
    assert_eq!(
        h.m.prices.best_price("platinum", t(10)).unwrap().price,
        200_000
    );
}

#[test]
fn late_markets_for_previous_system_not_applied() {
    let mut h = H::new();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert_eq!(h.sell_requests()[0].key, "sell:1:platinum");
    h.jump(10, "Beta Test", 2);
    assert_eq!(h.sell_requests().last().unwrap().key, "sell:2:platinum");
    assert_eq!(h.val("mining.sell")["status"], json!("loading"));

    h.send(markets_response(
        "sell:1:platinum",
        vec![market("X", 230_000)],
    ));
    let sell = h.val("mining.sell");
    assert_ne!(sell["status"], json!("ok"));
    assert!(
        sell["by_commodity"]
            .get("platinum")
            .is_none_or(|v| v.is_null())
    );
    assert_eq!(
        h.m.prices.best_price("platinum", t(20)).unwrap().price,
        230_000
    );

    h.send(markets_response(
        "sell:2:platinum",
        vec![market("Y", 250_000)],
    ));
    let sell = h.val("mining.sell");
    assert_eq!(sell["status"], json!("ok"));
    assert_eq!(
        sell["by_commodity"]["platinum"][0]["sell_price"],
        json!(250_000)
    );
}

#[test]
fn settings_change_recomputes_verdict() {
    let mut h = H::new();
    h.start();
    h.journal(
        3,
        "MarketSell",
        r#","Type":"platinum","Count":1,"SellPrice":220000,"TotalSale":220000"#,
    );
    h.journal(
        5,
        "ProspectedAsteroid",
        r#","Materials":[{"Name":"Platinum","Proportion":31.5}],"Content":"$AsteroidMaterialContent_High;""#,
    );
    let p = h.val("mining.prospector");
    assert_eq!(p["current"]["verdict"]["kind"], json!("worth"));
    assert_eq!(p["current"]["verdict"]["est_value"], json!(2_079_000));

    let dirty = h.send(SourceEvent::Settings(ModuleSettings {
        mining_threshold_cr: 5_000_000,
        ..ModuleSettings::default()
    }));
    assert!(dirty.contains("mining.prospector"));
    let p = h.val("mining.prospector");
    assert_eq!(p["current"]["verdict"]["kind"], json!("not_worth"));
    assert_eq!(p["history"].as_array().unwrap().len(), 1);
}

#[test]
fn sell_topic_stays_under_16_kib() {
    let mut h = H::new();
    h.start();
    let names: Vec<String> = (0..11).map(|i| format!("mineral{i}")).collect();
    let items: Vec<(&str, u32)> = names.iter().map(|n| (n.as_str(), 10)).collect();
    h.cargo(3, &items);
    // Máx. 10 consultas aunque haya 11 minerales.
    assert_eq!(h.sell_requests().len(), 10);
    let keys: Vec<String> = h.sell_requests().iter().map(|r| r.key.clone()).collect();
    for key in keys {
        let ms: Vec<RemoteMarket> = (0..50)
            .map(|i| market(&format!("{key} n{i}"), 100_000 + i))
            .collect();
        h.send(markets_response(&key, ms));
    }
    let sell = h.val("mining.sell");
    let bytes = serde_json::to_vec(&sell).unwrap().len();
    assert!(bytes < 16_384, "{bytes}");
    let by = sell["by_commodity"].as_object().unwrap();
    assert_eq!(by.len(), 10);
    assert!(by.values().all(|v| v.as_array().unwrap().len() == 5));
}

#[test]
fn relevance_active_after_prospector_launch() {
    let mut h = H::new();
    h.start();
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal(10, "LaunchDrone", r#","Type":"Prospector""#);
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.journal(10 + 11 * 60, "Music", r#","MusicTrack":"Exploration""#);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
}

#[test]
fn ship_pad_change_clears_remote_prices_and_requests_again() {
    let mut h = H::new();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    h.send(markets_response(
        "sell:1:platinum",
        vec![market("X", 230_000)],
    ));
    assert!(h.m.prices.best_price("platinum", t(20)).is_some());
    let before = h.sell_requests().len();
    h.journal(
        30,
        "LoadGame",
        r#","FID":"F1","Commander":"TESTER","Odyssey":true,"Ship":"anaconda""#,
    );
    assert!(h.m.prices.best_price("platinum", t(40)).is_none());
    assert_eq!(h.sell_requests().len(), before + 1);
}

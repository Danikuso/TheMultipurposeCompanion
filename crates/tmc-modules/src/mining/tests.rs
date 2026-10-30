use super::*;
use chrono::Duration;
use serde_json::json;
use tmc_model::events::parse_line;
use tmc_model::net::{
    NetOutcome, NetRequest, NetResponse, NetSource, PadSize, RemoteMarket, StockedItemPrice,
};

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
    store: Option<tmc_core::Store>,
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
            store: None,
        }
    }

    /// Ejecuta las tareas de almacén contra una base en memoria.
    fn with_store() -> Self {
        Self {
            store: Some(tmc_core::Store::open_in_memory().unwrap()),
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
        let mut ctx = ModuleCtx::new("mining", self.replaying, self.next_id);
        let mut dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        let mut follow = Vec::new();
        for (tag, task) in tasks {
            if let Some(store) = &self.store
                && let Ok(value) = task(store)
            {
                follow.push(SourceEvent::Stored {
                    module: "mining".into(),
                    tag: tag.clone(),
                    value,
                });
            }
            self.tags.push(tag);
        }
        for f in follow {
            dirty.extend(self.dispatch(f));
        }
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
        origin: None,
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
    let s = h.val("mining.session");
    // 4 platino + 2 painita + 1 bauxita refinados.
    assert_eq!(
        s["refined"],
        json!({"platinum": 4, "painite": 2, "bauxite": 1})
    );
    assert_eq!(s["asteroids_prospected"], json!(10));
    assert_eq!(
        s["limpets_launched"],
        json!({"Prospector": 1, "Collection": 1})
    );
    assert_eq!(s["limpets_left"], json!(2));
    // Sesión desde LoadGame (13:00:00) hasta el último evento (13:12:00): 720 s = 0,2 h.
    assert_eq!(s["started_at"], json!("2026-10-18T13:00:00Z"));
    assert!(s.get("tons_per_hour").is_none() && s.get("credits_per_hour").is_none());
    // Solo la bauxita tiene precio (MarketSell 450).
    assert_eq!(s["est_credits"], json!(450));
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
    // 10 asteroides prospectados; el historial conserva los 8 últimos.
    assert_eq!(p["history"].as_array().unwrap().len(), 8);
    // Una consulta por materia que estuvo en la carga (la bauxita, hasta venderla a las
    // 13:10:30) o que interesa del asteroide; nave mediana (python).
    let mut keys: Vec<_> = h.sell_requests().iter().map(|r| r.key.clone()).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(
        keys,
        vec!["sell:2:bauxite:m", "sell:2:painite:m", "sell:2:platinum:m"]
    );
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
    assert_eq!(h.sell_requests()[0].key, "sell:1:platinum:m");
    h.jump(10, "Beta Test", 2);
    assert_eq!(h.sell_requests().last().unwrap().key, "sell:2:platinum:m");
    assert_eq!(h.val("mining.sell")["status"], json!("loading"));

    h.send(markets_response(
        "sell:1:platinum:m",
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
        "sell:2:platinum:m",
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
        "sell:1:platinum:m",
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

#[test]
fn response_for_previous_pad_is_dropped() {
    let mut h = H::new();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert_eq!(h.sell_requests()[0].key, "sell:1:platinum:m");
    h.journal(
        30,
        "LoadGame",
        r#","FID":"F1","Commander":"TESTER","Odyssey":true,"Ship":"anaconda""#,
    );
    assert_eq!(h.sell_requests().last().unwrap().key, "sell:1:platinum:l");
    h.send(markets_response(
        "sell:1:platinum:m",
        vec![market("X", 230_000)],
    ));
    assert!(h.m.prices.best_price("platinum", t(40)).is_none());
    assert_eq!(h.val("mining.sell")["status"], json!("loading"));
}

#[test]
fn commander_change_resets_everything_but_load_game_keeps_prices() {
    let mut h = H::new();
    h.start();
    h.journal(
        3,
        "MarketSell",
        r#","Type":"platinum","Count":1,"SellPrice":220000,"TotalSale":220000"#,
    );
    h.cargo(4, &[("platinum", 5)]);
    h.journal(5, "MiningRefined", REFINED);
    h.send(markets_response(
        "sell:1:platinum:m",
        vec![market("X", 230_000)],
    ));
    // Misma partida recargada: sesión nueva y resultados borrados; precios se conservan.
    let n = h.sell_requests().len();
    h.journal(
        20,
        "LoadGame",
        r#","FID":"F1","Commander":"TESTER","Odyssey":true,"Ship":"python""#,
    );
    assert_eq!(h.val("mining.session")["refined"], json!({}));
    assert_eq!(h.sell_requests().len(), n + 1);
    assert!(h.m.prices.best_price("platinum", t(25)).is_some());
    h.journal(21, "MiningRefined", REFINED);
    // Otro comandante: todo limpio, incluidos los precios.
    h.journal(30, "Commander", r#","FID":"F2","Name":"OTRO""#);
    h.journal(
        31,
        "LoadGame",
        r#","FID":"F2","Commander":"OTRO","Odyssey":true,"Ship":"python""#,
    );
    assert_eq!(h.val("mining.session")["refined"], json!({}));
    assert!(h.m.prices.best_price("platinum", t(35)).is_none());
    assert!(h.m.inventory.is_empty());
    assert!(h.m.prospector.current().is_none());
}

#[test]
fn reset_session_action_clears_counters() {
    let mut h = H::new();
    h.start();
    h.journal(5, "MiningRefined", REFINED);
    assert_eq!(h.val("mining.session")["refined"], json!({"platinum": 1}));
    let dirty = h.send(SourceEvent::Action {
        name: "mining.reset_session".into(),
        args: json!({}),
    });
    assert!(dirty.contains("mining.session"));
    assert_eq!(h.val("mining.session")["refined"], json!({}));
}

#[test]
fn failed_is_retried_only_after_300_s() {
    use tmc_model::net::NetFailure;
    let mut h = H::new();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let n = h.sell_requests().len();
    h.send(SourceEvent::Net(NetResponse {
        id: 0,
        module: "mining".into(),
        key: "sell:1:platinum:m".into(),
        source: NetSource::Ardent,
        fetched_at: t(4),
        outcome: NetOutcome::Failed(NetFailure::Timeout),
    }));
    h.cargo(100, &[("platinum", 6)]);
    assert_eq!(h.sell_requests().len(), n);
    h.cargo(3 + 301, &[("platinum", 7)]);
    assert_eq!(h.sell_requests().len(), n + 1);
}

#[test]
fn radius_change_requests_again() {
    let mut h = H::new();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let n = h.sell_requests().len();
    h.send(SourceEvent::Settings(ModuleSettings {
        sell_search_radius_ly: 50,
        ..ModuleSettings::default()
    }));
    assert_eq!(h.sell_requests().len(), n + 1);
}

#[test]
fn session_topic_not_dirty_on_unrelated_events_until_30_s() {
    let mut h = H::new();
    h.start();
    h.journal(5, "MiningRefined", REFINED);
    let d = h.journal(10, "Music", r#","MusicTrack":"Exploration""#);
    assert!(!d.contains("mining.session"));
    // Sin campos dependientes del reloj, el paso del tiempo no ensucia nada.
    let d = h.journal(100, "Music", r#","MusicTrack":"Exploration""#);
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn as_of_does_not_dirty_topics_and_names_are_learned() {
    let mut h = H::new();
    h.start();
    h.journal(
        10,
        "MiningRefined",
        r#","Type":"$platinum_name;","Type_Localised":"Platino""#,
    );
    let s = h.val("mining.session");
    assert_eq!(s["names"]["platinum"], json!("Platino"));
    assert_eq!(s["as_of"], json!(t(10)));
    assert_eq!(h.val("mining.sell")["names"]["platinum"], json!("Platino"));
    // Un evento sin efecto avanza el reloj pero no ensucia ningún topic.
    let dirty = h.journal(50, "Music", r#","MusicTrack":"NoTrack""#);
    assert!(dirty.is_empty(), "{dirty:?}");
    assert_eq!(h.val("mining.session")["as_of"], json!(t(10)));
}

#[test]
fn as_of_is_null_before_any_event() {
    let m = Mining::new();
    assert!(m.topic_value("mining.session").is_none());
    let mut h = H::new();
    let ev = SourceEvent::Settings(ModuleSettings::default());
    h.send(ev);
    assert_eq!(h.val("mining.sell")["as_of"], json!(null));
}

fn eddn_seed(store: &tmc_core::Store, id: u64, price: u64, demand: u64, at: DateTime<Utc>) {
    use tmc_model::market::{CommodityPrice, MarketSnapshot, PriceOrigin};
    store
        .replace_markets(&[MarketSnapshot {
            market_id: id,
            station_name: format!("Buyer {id}"),
            system_name: format!("Buyer {id} System"),
            system_address: Some(900 + id),
            star_pos: Some([3.0, 0.0, 0.0]),
            max_pad: Some(PadSize::Large),
            station_type: Some("Coriolis".into()),
            updated_at: at,
            origin: PriceOrigin::Eddn,
            prices: vec![CommodityPrice {
                commodity: "platinum".into(),
                name_localised: None,
                buy_price: 0,
                sell_price: price,
                stock: 0,
                demand,
            }],
        }])
        .unwrap();
}

fn offline() -> H {
    let mut h = H::with_store();
    h.send(SourceEvent::Settings(ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    }));
    h
}

#[test]
fn irrelevant_event_dirties_no_mining_topic() {
    let mut h = H::with_store();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    h.journal(5, "MiningRefined", REFINED);
    for at in [100, 2000, 90_000] {
        let d = h.journal(at, "Music", r#","MusicTrack":"Exploration""#);
        assert!(d.is_empty(), "{at}: {d:?}");
    }
}

#[test]
fn sell_plan_includes_eddn_prices_when_ardent_disabled() {
    let mut h = offline();
    eddn_seed(h.store.as_ref().unwrap(), 5, 230_000, 100, t(-3600));
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert!(h.requests.is_empty());
    assert!(h.tags.iter().any(|t| t == "mining.local_sells"));
    let sell = h.val("mining.sell");
    assert_eq!(sell["status"], json!("ok"));
    let opts = sell["by_commodity"]["platinum"].as_array().unwrap();
    assert_eq!(opts.len(), 1);
    assert_eq!(opts[0]["origin"], json!("Eddn"));
    assert_eq!(opts[0]["sell_price"], json!(230_000));
    assert!(opts[0].get("age_s").is_none());
}

#[test]
fn eddn_and_ardent_same_market_newest_wins_even_if_cheaper() {
    let mut h = H::with_store();
    eddn_seed(h.store.as_ref().unwrap(), 5, 225_000, 100, t(-60));
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let key = h.sell_requests()[0].key.clone();
    let mut a = market("Buyer 5", 240_000);
    a.station_name = "Buyer 5".into();
    a.system_name = "Buyer 5 System".into();
    a.market_id = Some(5);
    a.updated_at = Some(t(-3600));
    h.send(markets_response(&key, vec![a]));
    let sell = h.val("mining.sell");
    let opts = sell["by_commodity"]["platinum"].as_array().unwrap();
    assert_eq!(opts.len(), 1, "{sell}");
    assert_eq!(opts[0]["sell_price"], json!(225_000));
    assert_eq!(opts[0]["origin"], json!("Eddn"));
}

#[test]
fn ardent_quote_older_than_48h_is_dropped() {
    let mut h = H::with_store();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let key = h.sell_requests()[0].key.clone();
    let mut old = market("Old", 240_000);
    old.updated_at = Some(t(3) - Duration::hours(49));
    let mut fresh = market("Fresh", 100_000);
    fresh.updated_at = Some(t(3) - Duration::hours(47));
    h.send(markets_response(&key, vec![old, fresh]));
    let sell = h.val("mining.sell");
    let opts = sell["by_commodity"]["platinum"].as_array().unwrap();
    assert_eq!(opts.len(), 1);
    assert_eq!(opts[0]["station_name"], json!("Fresh Station"));
}

#[test]
fn eddn_notice_requeries_at_most_every_10_s() {
    let mut h = H::with_store();
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let n = |s: i64| {
        SourceEvent::Eddn(tmc_model::source::EddnNotice {
            markets: vec![5],
            commodities: vec!["platinum".into()],
            connected: true,
            at: t(s),
        })
    };
    let count = |h: &H| h.tags.iter().filter(|t| *t == "mining.local_sells").count();
    let base = count(&h);
    h.send(n(10));
    assert_eq!(count(&h), base + 1);
    h.send(n(15));
    assert_eq!(count(&h), base + 1);
    h.send(n(25));
    assert_eq!(count(&h), base + 2);
}

#[test]
fn sell_topic_with_two_sources_stays_under_16_kib() {
    use tmc_model::market::{CommodityPrice, MarketSnapshot, PriceOrigin};
    let mut h = H::with_store();
    h.start();
    let names: Vec<String> = (0..10).map(|i| format!("mineral{i}")).collect();
    for (m, n) in names.iter().enumerate() {
        for i in 0..50u64 {
            h.store
                .as_ref()
                .unwrap()
                .replace_markets(&[MarketSnapshot {
                    market_id: 1000 + (m as u64) * 100 + i,
                    station_name: format!("T{m} {i}"),
                    system_name: format!("T{m} {i} Sys"),
                    system_address: Some(5000 + i),
                    star_pos: Some([3.0, 0.0, 0.0]),
                    max_pad: Some(PadSize::Large),
                    station_type: Some("Coriolis".into()),
                    updated_at: t(-60),
                    origin: PriceOrigin::Eddn,
                    prices: vec![CommodityPrice {
                        commodity: n.clone(),
                        name_localised: None,
                        buy_price: 0,
                        sell_price: 100_000 + i,
                        stock: 0,
                        demand: 10,
                    }],
                }])
                .unwrap();
        }
    }
    let items: Vec<(&str, u32)> = names.iter().map(|n| (n.as_str(), 10)).collect();
    h.cargo(3, &items);
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

fn seed_full(
    store: &tmc_core::Store,
    id: u64,
    name: &str,
    price: u64,
    pad: Option<PadSize>,
    station_type: &str,
) {
    use tmc_model::market::{CommodityPrice, MarketSnapshot, PriceOrigin};
    store
        .replace_markets(&[MarketSnapshot {
            market_id: id,
            station_name: name.into(),
            system_name: format!("{name} System"),
            system_address: Some(900 + id),
            star_pos: Some([3.0, 0.0, 0.0]),
            max_pad: pad,
            station_type: Some(station_type.into()),
            updated_at: t(-60),
            origin: PriceOrigin::Eddn,
            prices: vec![CommodityPrice {
                commodity: "platinum".into(),
                name_localised: None,
                buy_price: 0,
                sell_price: price,
                stock: 0,
                demand: 10,
            }],
        }])
        .unwrap();
}

fn platinum_options(h: &H) -> Vec<serde_json::Value> {
    h.val("mining.sell")["by_commodity"]["platinum"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn newer_closed_observation_removes_older_ardent_offer() {
    let mut h = H::with_store();
    // Cierre (demanda 0) más nuevo que la oferta remota del mismo mercado.
    eddn_seed(h.store.as_ref().unwrap(), 5, 225_000, 0, t(-60));
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let key = h.sell_requests()[0].key.clone();
    let mut a = market("Buyer 5", 240_000);
    a.station_name = "Buyer 5".into();
    a.system_name = "Buyer 5 System".into();
    a.market_id = Some(5);
    a.updated_at = Some(t(-3600));
    h.send(markets_response(&key, vec![a]));
    // El eco de candidatos llegó a la tarea y la lectura se aplicó.
    let sent = h.m.local_sent.clone().unwrap();
    assert_eq!(sent["candidates"]["platinum"][0]["market_id"], json!(5));
    assert!(h.m.local_rows["platinum"].iter().any(|r| r.market_id == 5));
    assert!(platinum_options(&h).is_empty());
}

#[test]
fn stale_local_stored_is_discarded() {
    let mut h = H::with_store();
    eddn_seed(h.store.as_ref().unwrap(), 5, 230_000, 100, t(-60));
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let good = {
        let mut v = h.m.local_sent.clone().unwrap();
        v["rows"] = json!({});
        v
    };
    let mut other_cmdr = good.clone();
    other_cmdr["cmdr"] = json!("OTHER");
    let mut other_params = good.clone();
    other_params["radius_ly"] = json!(1);
    for value in [other_cmdr, other_params] {
        let d = h.dispatch(SourceEvent::Stored {
            module: "mining".into(),
            tag: "mining.local_sells".into(),
            value,
        });
        assert!(d.is_empty());
        assert!(!h.m.local_rows["platinum"].is_empty());
    }
}

#[test]
fn no_local_task_in_replay_or_legacy() {
    let mut h = H::with_store();
    h.replaying = true;
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert!(!h.tags.iter().any(|t| t == "mining.local_sells"));

    let mut h = H::with_store();
    h.state.galaxy = Galaxy::Legacy;
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    assert!(!h.tags.iter().any(|t| t == "mining.local_sells"));
}

#[test]
fn local_sells_filter_pad_and_exclude_carriers() {
    let mut h = offline();
    let st = h.store.as_ref().unwrap();
    // Nave mediana: la plataforma pequeña se descarta, la desconocida se conserva.
    seed_full(st, 10, "Tiny", 300_000, Some(PadSize::Small), "Outpost");
    seed_full(st, 11, "Unknown", 200_000, None, "Outpost");
    seed_full(st, 12, "Big", 100_000, Some(PadSize::Large), "Coriolis");
    seed_full(
        st,
        3_700_000_001,
        "ABC-123",
        900_000,
        Some(PadSize::Large),
        "FleetCarrier",
    );
    h.start();
    h.cargo(3, &[("platinum", 5)]);
    let opts = platinum_options(&h);
    let names: Vec<_> = opts
        .iter()
        .map(|o| o["station_name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Unknown", "Big"]);
    assert_eq!(opts[0]["pad_ok"], json!(null));
    assert_eq!(opts[1]["pad_ok"], json!(true));
}

#[test]
fn offline_search_targets_resolves_instantly_with_sorted_distances() {
    let mut h = H::new();
    h.m.settings.online_services = false;
    h.state.location.system = "Sol".into();
    h.state.location.star_pos = [0.0, 0.0, 0.0];

    let d = h.send(SourceEvent::Action {
        name: "mining.search_targets".into(),
        args: json!({
            "commodity": "Platinum",
            "reference_system": "Sol",
            "min_hotspots": 1
        }),
    });

    assert!(d.contains("mining.targets"));
    assert!(d.contains("mining.loop"));

    let topic = h.m.topic_value("mining.targets").unwrap();
    assert_eq!(topic["status"], "ok");
    assert_eq!(topic["commodity"], "Platinum");

    let targets = topic["targets"].as_array().unwrap();
    assert!(!targets.is_empty());

    // Verificar que las distancias están ordenadas ascendentemente
    for i in 1..targets.len() {
        let prev = targets[i - 1]["distance_ly"].as_f64().unwrap();
        let curr = targets[i]["distance_ly"].as_f64().unwrap();
        assert!(curr >= prev, "targets deben estar ordenados por distancia");
    }

    // Verificar que Delkar está presente
    let delkar = targets
        .iter()
        .find(|t| t["system_name"] == "Delkar")
        .expect("Delkar debe estar en los resultados de Platino");
    assert_eq!(delkar["hotspot_count"], 2);
    assert_eq!(delkar["res_intensity"], "High");
}

#[test]
fn search_targets_require_res_combo_filters_correctly() {
    let mut h = H::new();
    h.state.location.system = "Sol".into();
    h.state.location.star_pos = [0.0, 0.0, 0.0];

    let _ = h.send(SourceEvent::Action {
        name: "mining.search_targets".into(),
        args: json!({
            "commodity": "Platinum",
            "require_res_combo": true
        }),
    });

    let topic = h.m.topic_value("mining.targets").unwrap();
    let targets = topic["targets"].as_array().unwrap();
    assert!(!targets.is_empty());

    // Todos deben tener res_intensity no nulo
    for t in targets {
        assert!(
            !t["res_intensity"].is_null(),
            "con require_res_combo todos los objetivos deben tener RES"
        );
    }
}

#[test]
fn select_target_and_loop_plan_with_demand_warning() {
    let mut h = H::new();
    h.state.location.system = "Sol".into();
    h.state.location.star_pos = [0.0, 0.0, 0.0];

    // Búsqueda inicial
    let _ = h.send(SourceEvent::Action {
        name: "mining.search_targets".into(),
        args: json!({ "commodity": "Platinum" }),
    });

    // Seleccionar Delkar 7
    let _ = h.send(SourceEvent::Action {
        name: "mining.select_target".into(),
        args: json!({
            "system_name": "Delkar",
            "body_name": "Delkar 7",
            "ring_name": "Delkar 7 A Ring"
        }),
    });

    // Añadir 30 t de platino en bodega
    h.m.inventory.insert("platinum".into(), 30);

    // Simular respuesta de estación compradora para el bucle (demanda 100 t -> 30 t es > 25 %, alerta activa)
    let hits = vec![tmc_model::net::MiningMultiSellHit {
        station_name: "Matthews Dock".into(),
        system_name: "Tofana".into(),
        distance_ly: 14.2,
        pad: PadSize::Large,
        item_prices: vec![StockedItemPrice {
            label: "Platinum".into(),
            price_cr: Some(285_400),
        }],
        total_revenue_cr: 285_400,
        updated_at: None,
    }];

    let d = h.send(SourceEvent::Net(NetResponse {
        id: 1,
        module: "mining".into(),
        key: "mining_loop_sell:1".into(),
        source: NetSource::Spansh,
        fetched_at: Utc::now(),
        outcome: NetOutcome::MiningMultiSell(hits),
    }));

    assert!(d.contains("mining.loop"));
    let loop_topic = h.m.topic_value("mining.loop").unwrap();
    assert_eq!(loop_topic["status"], "ok");

    let plan = &loop_topic["plan"];
    assert_eq!(plan["ring_target"]["system_name"], "Delkar");
    assert_eq!(plan["sell_station"]["station_name"], "Matthews Dock");
}

#[test]
fn search_multi_sell_and_clear_targets() {
    let mut h = H::new();
    h.state.location.system = "Sol".into();
    h.state.location.star_pos = [0.0, 0.0, 0.0];
    h.m.settings.online_services = true;

    // Con bodega con Platinum y Osmium
    h.m.inventory.insert("platinum".into(), 40);
    h.m.inventory.insert("osmium".into(), 20);

    let _ = h.send(SourceEvent::Action {
        name: "mining.search_multi_sell".into(),
        args: json!({}),
    });

    assert!(!h.requests.is_empty());
    let req = &h.requests[0];
    match &req.query {
        NetQuery::MiningMultiSell(q) => {
            assert!(q.commodities.contains(&"platinum".into()));
            assert!(q.commodities.contains(&"osmium".into()));
        }
        other => panic!("query inesperada: {other:?}"),
    }

    // Limpiar objetivos
    let _ = h.send(SourceEvent::Action {
        name: "mining.clear_targets".into(),
        args: json!({}),
    });

    let targets_topic = h.m.topic_value("mining.targets").unwrap();
    assert_eq!(targets_topic["status"], "idle");
    assert!(targets_topic["targets"].as_array().unwrap().is_empty());

    let loop_topic = h.m.topic_value("mining.loop").unwrap();
    assert_eq!(loop_topic["status"], "idle");
}

#[test]
fn approach_body_triggers_predictive_mining_relevance() {
    let mut h = H::new();
    h.start();
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);

    // Approaching ring at t=10
    h.journal(
        10,
        "ApproachBody",
        r#","Body":"HIP 21991 1 Ring A","BodyID":12,"SystemAddress":100"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);

    // Leaving body at t=15 resets predictive relevance
    h.journal(
        15,
        "LeaveBody",
        r#","Body":"HIP 21991 1 Ring A","BodyID":12,"SystemAddress":100"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
}

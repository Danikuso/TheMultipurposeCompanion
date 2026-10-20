use super::*;
use serde_json::json;
use tmc_core::Store;
use tmc_model::events::parse_line;
use tmc_model::market::{CommodityPrice, PriceOrigin};
use tmc_model::net::{NetRequest, NetSource, TradeCommodity, TradeRoutePlan, TradeStop};
use tmc_model::state::ShipState;

const FIXTURE: &str = include_str!("../../../../tests/fixtures/journal/trade_session.log");
const MARKET_JSON: &str = include_str!("../../../../tests/fixtures/companion/market_trade.json");

/// Instante del `Market.json` de la fixture y del `Docked` de las pruebas.
fn t(seconds: i64) -> DateTime<Utc> {
    "2026-10-19T18:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
}

struct H {
    m: Trade,
    state: GameState,
    replaying: bool,
    exec: bool,
    store: Store,
    requests: Vec<NetRequest>,
    tags: Vec<String>,
    next_id: u64,
    line_no: u64,
}

impl H {
    fn new() -> Self {
        Self {
            m: Trade::new(),
            state: GameState::default(),
            replaying: false,
            exec: false,
            store: Store::open_in_memory().unwrap(),
            requests: Vec::new(),
            tags: Vec::new(),
            next_id: 1,
            line_no: 0,
        }
    }

    /// Ejecuta las tareas de almacén contra una base en memoria y devuelve sus resultados.
    fn with_store() -> Self {
        Self {
            exec: true,
            ..Self::new()
        }
    }

    fn send(&mut self, ev: SourceEvent) -> TopicSet {
        match &ev {
            SourceEvent::Journal { ev, .. } => {
                self.state.apply_event(ev);
            }
            SourceEvent::Companion(c) => {
                self.state.apply_companion(c);
            }
            _ => {}
        }
        self.dispatch(ev)
    }

    fn dispatch(&mut self, ev: SourceEvent) -> TopicSet {
        let mut ctx = ModuleCtx::new("trade", self.replaying, self.next_id);
        let mut dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        let mut follow = Vec::new();
        for (tag, task) in tasks {
            if self.exec
                && let Ok(value) = task(&self.store)
            {
                follow.push(SourceEvent::Stored {
                    module: "trade".into(),
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

    fn action(&mut self, name: &str, args: Value) -> TopicSet {
        self.send(SourceEvent::Action {
            name: name.into(),
            args,
        })
    }

    fn fixture(&mut self) {
        for l in FIXTURE.lines() {
            self.line(l);
        }
    }

    /// Fixture de comercio, atraque en «Synthetic Port» (sistema 5) y `Market.json`.
    fn dock_with_market(&mut self) {
        if self.line_no == 0 {
            self.fixture();
        }
        self.journal(
            0,
            "Location",
            r#","StarSystem":"Synthetic System","SystemAddress":5,"StarPos":[0,0,0]"#,
        );
        self.journal(
            1,
            "Docked",
            r#","StationName":"Synthetic Port","StationType":"Coriolis","MarketID":128000001,"StarSystem":"Synthetic System","SystemAddress":5,"StationServices":["commodities"],"LandingPads":{"Small":4,"Medium":4,"Large":2}"#,
        );
        self.market_json();
    }

    fn market_json(&mut self) {
        self.send(SourceEvent::Companion(CompanionFile::Raw(
            CompanionKind::Market,
            serde_json::from_str(MARKET_JSON).unwrap(),
        )));
    }

    fn val(&self, topic: &str) -> Value {
        self.m.topic_value(topic).unwrap()
    }

    fn requests_with(&self, prefix: &str) -> Vec<&NetRequest> {
        self.requests
            .iter()
            .filter(|r| r.key.starts_with(prefix))
            .collect()
    }

    fn count_tag(&self, tag: &str) -> usize {
        self.tags.iter().filter(|x| x.as_str() == tag).count()
    }

    fn ship(&mut self) {
        self.state.ship = Some(ShipState {
            ship_type: "type9".into(),
            cargo_capacity: Some(256),
            max_jump_range: Some(22.5),
            ..self.state.ship.clone().unwrap_or_default()
        });
    }
}

fn remote(id: u64, system: &str, price: u64, at: DateTime<Utc>) -> RemoteMarket {
    RemoteMarket {
        station_name: format!("Station {id}"),
        system_name: system.into(),
        market_id: Some(id),
        distance_ly: 10.0,
        distance_to_arrival_ls: None,
        sell_price: price,
        demand: Some(100),
        max_pad: Some(PadSize::Large),
        station_type: None,
        updated_at: Some(at),
        origin: None,
    }
}

fn net(key: &str, outcome: NetOutcome) -> SourceEvent {
    SourceEvent::Net(NetResponse {
        id: 0,
        module: "trade".into(),
        key: key.into(),
        source: NetSource::Ardent,
        fetched_at: t(100),
        outcome,
    })
}

fn export(id: u64, price: u64) -> RemoteExport {
    RemoteExport {
        station_name: format!("Seller {id}"),
        system_name: "Elsewhere".into(),
        market_id: Some(id),
        distance_ly: 7.0,
        distance_to_arrival_ls: None,
        buy_price: price,
        stock: Some(50),
        max_pad: Some(PadSize::Large),
        station_type: None,
        updated_at: Some(t(50)),
    }
}

fn plan() -> TradeRoutePlan {
    TradeRoutePlan {
        stops: [("Synthetic System", "B"), ("B", "C")]
            .into_iter()
            .map(|(from, to)| TradeStop {
                from_system: from.into(),
                from_station: format!("Port {from}"),
                to_system: to.into(),
                to_station: format!("Port {to}"),
                from_market_id: None,
                to_market_id: None,
                commodities: (0..3)
                    .map(|i| TradeCommodity {
                        commodity: format!("c{i}"),
                        name_localised: Some(format!("Commodity {i}")),
                        amount: 10,
                        buy_price: 100,
                        sell_price: 200,
                        profit_per_ton: 100,
                    })
                    .collect(),
                stop_profit: 1000,
                distance_ly: 20.0,
                data_updated_at: None,
            })
            .collect(),
        total_profit: 2000,
    }
}

/// Mercado vecino que compra las mercancías de la fixture a precios conocidos.
fn seed_buyer(store: &Store, id: u64, system: &str, demand: u64, at: DateTime<Utc>) {
    let prices = [
        ("gold", 11000),
        ("silver", 5000),
        ("palladium", 13900),
        ("cobalt", 1800),
        ("copper", 2700),
        ("aluminium", 1100),
        ("lavianbrandy", 3500),
    ]
    .into_iter()
    .map(|(c, p)| CommodityPrice {
        commodity: c.into(),
        name_localised: None,
        buy_price: 0,
        sell_price: p,
        stock: 0,
        demand,
    })
    .collect();
    store
        .replace_markets(&[tmc_model::market::MarketSnapshot {
            market_id: id,
            station_name: format!("Buyer {id}"),
            system_name: system.into(),
            system_address: Some(900 + id),
            star_pos: Some([3.0, 0.0, 0.0]),
            max_pad: Some(PadSize::Large),
            station_type: Some("Coriolis".into()),
            updated_at: at,
            origin: PriceOrigin::Eddn,
            prices,
        }])
        .unwrap();
}

#[test]
fn trade_fixture_current_market_top5() {
    let mut h = H::with_store();
    seed_buyer(&h.store, 2, "Other", 100, t(-3600));
    h.dock_with_market();
    let market = h.val("trade.market");
    assert_eq!(market["rows"].as_array().unwrap().len(), 5);
    assert_eq!(market["rows"][0]["commodity"], json!("gold"));
    assert_eq!(market["rows"][0]["profit_per_ton"], json!(1600));
    assert_eq!(market["station"], json!("Synthetic Port"));
    // El Market.json propio se escribe en el almacén en vivo.
    assert_eq!(h.count_tag("trade.market.write"), 1);
    assert!(!h.store.market_prices(128000001).unwrap().is_empty());
    // La sesión y el coste de la fixture: 60 t vendidas a coste medio 9500.
    let session = h.val("trade.session");
    assert_eq!(session["tons_sold"], json!(60));
    assert_eq!(session["profit"], json!(660000 - 60 * 9500));
    let cargo = h.val("trade.cargo");
    let gold = &cargo["lines"][0];
    assert_eq!(gold["commodity"], json!("gold"));
    assert_eq!(gold["paid_avg"], json!(9500));
    assert_eq!(gold["price_here"], json!(9000));
    assert_eq!(
        h.val("trade.commodities")["items"][0][0],
        json!("aluminium")
    );
}

#[test]
fn replay_emits_no_requests_or_store_writes() {
    let mut h = H::new();
    h.replaying = true;
    h.dock_with_market();
    h.action("trade.search", json!({"commodity": "gold"}));
    h.action("trade.route_request", json!({"max_hops": 3}));
    assert!(h.requests.is_empty());
    assert!(h.tags.is_empty(), "{:?}", h.tags);
    h.replaying = false;
    h.journal(500, "Music", r#","MusicTrack":"Exploration""#);
    assert_eq!(h.count_tag("trade.cost"), 1);
    assert!(h.tags.iter().all(|t| !t.ends_with(".write")));
}

#[test]
fn ardent_rows_without_coords_stay_in_memory_only() {
    let rows_for = |ms: &[RemoteMarket]| ardent_rows(ms, &[], "Gold", t(0));
    let markets = vec![
        remote(11, "Known", 11000, t(-3600)),
        remote(12, "Unknown", 12000, t(-3600)),
    ];
    let rows = rows_for(&markets);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1.sell_price, 11000);
    assert_eq!(rows[0].1.demand, 100);
    assert_eq!(rows[0].1.commodity, "gold");
    assert_eq!(rows[0].3, PriceOrigin::Ardent);
    let store = Store::open_in_memory().unwrap();
    store
        .upsert_coords(&[("Known".into(), None, [1.0, 0.0, 0.0])], t(0))
        .unwrap();
    ardent_write_task(rows)(&store).unwrap();
    assert_eq!(store.market_prices(11).unwrap().len(), 1);
    assert!(store.market_prices(12).unwrap().is_empty());

    // El módulo usa las dos para el mercado actual.
    let mut h = H::with_store();
    h.dock_with_market();
    let key = h.requests_with("sells:")[0].key.clone();
    assert!(key.starts_with("sells:5:"));
    h.send(net(&key, NetOutcome::Markets(markets)));
    let gold = &h.val("trade.market")["rows"][0];
    assert_eq!(gold["commodity"], json!("gold"));
    assert_eq!(gold["best_sell"]["sell_price"], json!(12000));
    assert!(h.count_tag("trade.ardent.write") >= 1);
}

#[test]
fn no_requests_in_legacy_or_offline() {
    let mut h = H::with_store();
    h.fixture();
    h.state.galaxy = Galaxy::Legacy;
    h.dock_with_market();
    h.action("trade.search", json!({"commodity": "gold"}));
    h.action("trade.route_request", json!({}));
    assert!(h.requests.is_empty());
    assert_eq!(h.count_tag("trade.market.write"), 0);
    assert_eq!(h.val("trade.route")["status"], json!("disabled"));

    let mut h = H::with_store();
    h.send(SourceEvent::Settings(ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    }));
    h.dock_with_market();
    h.action("trade.search", json!({"commodity": "gold"}));
    h.action("trade.route_request", json!({}));
    assert!(h.requests.is_empty());
    assert_eq!(h.val("trade.route")["status"], json!("disabled"));
}

#[test]
fn stored_result_for_other_cmdr_is_ignored() {
    let mut h = H::new();
    h.dock_with_market();
    let sent = h.m.sells_sent.clone().unwrap();
    let mut other = sent.clone();
    other["cmdr"] = json!("OTRO");
    other["rows"] = json!({});
    let dirty = h.dispatch(SourceEvent::Stored {
        module: "trade".into(),
        tag: "trade.best_sells".into(),
        value: other,
    });
    assert!(dirty.is_empty());
    assert!(!h.m.sells_loaded);
    let dirty = h.dispatch(SourceEvent::Stored {
        module: "trade".into(),
        tag: "trade.cost".into(),
        value: json!({"cmdr":"OTRO","basis":CostBasis::default(),"cut":0,"cut_pos":null}),
    });
    assert!(dirty.is_empty());
    assert!(!h.m.cost_loaded);
    let mut ok = sent;
    ok["rows"] = json!({});
    h.dispatch(SourceEvent::Stored {
        module: "trade".into(),
        tag: "trade.best_sells".into(),
        value: ok,
    });
    assert!(h.m.sells_loaded);
}

#[test]
fn commander_switch_clears_cost_basis_and_route() {
    let mut h = H::new();
    h.fixture();
    assert_eq!(h.val("trade.cargo")["lines"][0]["paid_avg"], json!(9500));
    h.m.route.on_plan(plan());
    h.m.route_status = "ok";
    h.journal(
        600,
        "LoadGame",
        r#","FID":"F9","Commander":"OTRO","Ship":"type9","Credits":1000"#,
    );
    let cargo = h.val("trade.cargo");
    assert!(cargo["lines"][0]["paid_avg"].is_null());
    let route = h.val("trade.route");
    assert!(route["plan"].is_null());
    assert_eq!(route["status"], json!("idle"));
    assert_eq!(h.val("trade.session")["tons_sold"], json!(0));
}

#[test]
fn late_exports_for_previous_system_not_applied() {
    let mut h = H::new();
    h.dock_with_market();
    h.action("trade.search", json!({"commodity": "Gold"}));
    let old = h.requests_with("exports:")[0].key.clone();
    assert!(old.starts_with("exports:5:gold:"));
    h.journal(
        10,
        "FSDJump",
        r#","StarSystem":"Elsewhere","SystemAddress":6,"StarPos":[5,0,0]"#,
    );
    h.send(net(&old, NetOutcome::Exports(vec![export(30, 9000)])));
    assert!(h.val("trade.search")["rows"].as_array().unwrap().is_empty());
    assert!(h.m.search_exports.is_empty());
    // La búsqueda se repite en el sistema nuevo y su respuesta sí vale.
    let fresh = h.requests_with("exports:6:")[0].key.clone();
    h.send(net(&fresh, NetOutcome::Exports(vec![export(30, 9000)])));
    let search = h.val("trade.search");
    assert_eq!(search["rows"][0]["buy_price"], json!(9000));
    assert_eq!(search["status"], json!("ok"));
}

#[test]
fn eddn_burst_requeries_at_most_every_10s() {
    let mut h = H::new();
    h.dock_with_market();
    let before = h.count_tag("trade.best_sells");
    assert_eq!(before, 1);
    for i in 0..50 {
        h.send(SourceEvent::Eddn(EddnNotice {
            markets: vec![2],
            commodities: vec!["gold".into()],
            connected: true,
            at: t(1000) + Duration::milliseconds(i * 60),
        }));
    }
    assert_eq!(h.count_tag("trade.best_sells"), before + 1);
    assert_eq!(h.val("trade.market")["eddn_connected"], json!(true));
    // Pasados 10 s vuelve a consultar; sin mercancías de la vista, no.
    h.send(SourceEvent::Eddn(EddnNotice {
        markets: vec![2],
        commodities: vec!["unobtainium".into()],
        connected: true,
        at: t(1100),
    }));
    assert_eq!(h.count_tag("trade.best_sells"), before + 1);
    h.send(SourceEvent::Eddn(EddnNotice {
        markets: vec![2],
        commodities: vec!["gold".into()],
        connected: true,
        at: t(1100),
    }));
    assert_eq!(h.count_tag("trade.best_sells"), before + 2);
}

#[test]
fn route_advance_copies_next_system() {
    let mut h = H::new();
    h.dock_with_market();
    h.ship();
    h.action("trade.route_request", json!({"max_hops": 3}));
    let reqs = h.requests_with("route:");
    assert_eq!(reqs.len(), 1);
    assert!(matches!(reqs[0].query, NetQuery::TradeRoute(_)));
    assert_eq!(h.val("trade.route")["status"], json!("loading"));
    let key = reqs[0].key.clone();
    h.send(net(&key, NetOutcome::TradeRoute(plan())));
    let route = h.val("trade.route");
    assert_eq!(route["status"], json!("ok"));
    assert_eq!(route["copy_target"], json!("B"));
    // 20 al con alcance 22,5 al: un salto (ceil(distancia / alcance)).
    assert_eq!(route["plan"]["stops"][0]["jumps"], json!(1));
    let seq = route["copy_seq"].as_u64().unwrap();
    h.journal(
        20,
        "Docked",
        r#","StationName":"Port B","MarketID":7,"StarSystem":"B","SystemAddress":8,"StationServices":["commodities"]"#,
    );
    let route = h.val("trade.route");
    assert_eq!(route["copy_target"], json!("C"));
    assert_eq!(route["copy_seq"], json!(seq + 1));
    // Una ruta distinta o fuera de ruta no es abandono; limpiar sí vacía.
    h.action("trade.route_clear", json!({}));
    let route = h.val("trade.route");
    assert!(route["plan"].is_null());
    assert_eq!(route["status"], json!("idle"));
}

#[test]
fn route_copy_seq_is_frozen_without_auto_copy_and_stale_plan_is_dropped() {
    let mut h = H::new();
    h.send(SourceEvent::Settings(ModuleSettings {
        auto_copy_next_system: false,
        ..ModuleSettings::default()
    }));
    h.dock_with_market();
    h.ship();
    h.action("trade.route_request", json!({}));
    let key = h.requests_with("route:")[0].key.clone();
    h.send(net("route:5:x", NetOutcome::TradeRoute(plan())));
    assert!(h.val("trade.route")["plan"].is_null());
    h.send(net(&key, NetOutcome::TradeRoute(plan())));
    let route = h.val("trade.route");
    assert_eq!(route["copy_target"], json!("B"));
    assert_eq!(route["copy_seq"], json!(0));
}

#[test]
fn route_request_is_blocked_without_ship_data() {
    let mut h = H::new();
    h.dock_with_market();
    h.state.ship = None;
    h.action("trade.route_request", json!({}));
    assert!(h.requests_with("route:").is_empty());
    assert_eq!(h.val("trade.route")["status"], json!("blocked"));
}

#[test]
fn trade_topics_stay_under_16_kib() {
    let mut h = H::new();
    h.dock_with_market();
    let items: Vec<(String, String)> = (0..400)
        .map(|i| (format!("commodity{i:03}"), format!("Commodity name {i:03}")))
        .collect();
    h.m.commodities = items.into_iter().map(|(k, n)| (k, Some(n))).collect();
    let inv: Vec<String> = (0..30)
        .map(|i| format!(r#"{{"Name":"Item{i}","Count":5}}"#))
        .collect();
    h.journal(
        30,
        "Cargo",
        &format!(
            r#","Vessel":"Ship","Count":150,"Inventory":[{}]"#,
            inv.join(",")
        ),
    );
    let mut big = plan();
    big.stops = (0..10).flat_map(|_| plan().stops).take(10).collect();
    h.m.route.on_plan(big);
    h.action("trade.search", json!({"commodity": "gold"}));
    h.journal(40, "Music", r#","MusicTrack":"Exploration""#);
    for topic in TOPICS {
        let size = serde_json::to_string(&h.val(topic)).unwrap().len();
        assert!(size < 16384, "{topic} ocupa {size} bytes");
    }
    assert_eq!(h.val("trade.cargo")["lines"].as_array().unwrap().len(), 30);
    assert!(
        h.val("trade.commodities")["items"]
            .as_array()
            .unwrap()
            .len()
            <= 400
    );
}

#[test]
fn relevance_active_when_docked_with_market() {
    let mut h = H::new();
    h.journal(0, "Commander", r#","FID":"F1","Name":"TESTER""#);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal(
        1,
        "Docked",
        r#","StationName":"Synthetic Port","MarketID":5,"StarSystem":"S","SystemAddress":5,"StationServices":["commodities"]"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
    h.journal(2, "Undocked", r#","StationName":"Synthetic Port""#);
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal(
        3,
        "Cargo",
        r#","Vessel":"Ship","Count":4,"Inventory":[{"Name":"Gold","Count":4}]"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
}

#[test]
fn relevance_ignores_limpets_mission_and_stolen_cargo() {
    let mut h = H::new();
    h.journal(0, "Commander", r#","FID":"F1","Name":"TESTER""#);
    h.journal(
        1,
        "Cargo",
        r#","Vessel":"Ship","Count":5,"Inventory":[{"Name":"drones","Count":5}]"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal(
        2,
        "Cargo",
        r#","Vessel":"Ship","Count":3,"Inventory":[{"Name":"Gold","Count":3,"MissionID":77}]"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Normal);
    h.journal(
        3,
        "Cargo",
        r#","Vessel":"Ship","Count":1,"Inventory":[{"Name":"Gold","Count":1}]"#,
    );
    assert_eq!(h.m.relevance(&h.state), Relevance::Active);
}

#[test]
fn ardent_prices_older_than_48h_are_ignored() {
    let mut h = H::with_store();
    h.dock_with_market();
    let key = h.requests_with("sells:")[0].key.clone();
    let clock = h.m.clock();
    let old = remote(40, "Far", 12000, clock - Duration::hours(49));
    h.send(net(&key, NetOutcome::Markets(vec![old])));
    assert!(h.val("trade.market")["rows"][0]["best_sell"].is_null());
    let fresh = remote(40, "Far", 12000, clock - Duration::hours(47));
    h.send(net(&key, NetOutcome::Markets(vec![fresh])));
    assert_eq!(
        h.val("trade.market")["rows"][0]["best_sell"]["sell_price"],
        json!(12000)
    );
}

#[test]
fn closed_offer_from_newer_full_snapshot_blocks_ardent_sale() {
    let mut h = H::with_store();
    // Mercado 40 cerrado (demanda 0) por un snapshot completo más nuevo que Ardent.
    seed_buyer(&h.store, 40, "Far", 0, t(-600));
    h.dock_with_market();
    let keys: Vec<String> = h
        .requests_with("sells:")
        .iter()
        .map(|r| r.key.clone())
        .collect();
    let key = keys[0].clone();
    for other in &keys[1..] {
        h.send(net(other, NetOutcome::NoData));
    }
    let mut ardent = remote(40, "Far", 15000, t(-7200));
    ardent.station_name = "Buyer 40".into();
    h.send(net(&key, NetOutcome::Markets(vec![ardent])));
    assert!(h.val("trade.market")["rows"][0]["best_sell"].is_null());
    assert!(h.count_tag("trade.best_sells") >= 2);
}

#[test]
fn irrelevant_events_dirty_no_trade_topic() {
    let mut h = H::with_store();
    seed_buyer(&h.store, 2, "Other", 100, t(-3600));
    h.dock_with_market();
    assert!(!h.val("trade.market")["rows"][0]["best_sell"].is_null());
    let dirty = h.journal(900, "Music", r#","MusicTrack":"Exploration""#);
    assert!(dirty.is_empty(), "{dirty:?}");
    let row = &h.val("trade.market")["rows"][0];
    assert!(row.get("age_s").is_none());
    assert!(h.val("trade.session").get("credits_per_hour").is_none());
}

#[test]
fn copy_seq_is_monotonic_across_clear_and_new_plan() {
    let mut h = H::new();
    h.dock_with_market();
    h.ship();
    h.action("trade.route_request", json!({}));
    let key = h.requests_with("route:")[0].key.clone();
    h.send(net(&key, NetOutcome::TradeRoute(plan())));
    assert_eq!(h.val("trade.route")["copy_seq"], json!(1));
    h.action("trade.route_clear", json!({}));
    assert_eq!(h.val("trade.route")["copy_seq"], json!(1));
    h.action("trade.route_request", json!({}));
    let key = h.requests_with("route:")[1].key.clone();
    h.send(net(&key, NetOutcome::TradeRoute(plan())));
    assert_eq!(h.val("trade.route")["copy_seq"], json!(2));
}

#[test]
fn market_json_before_docked_and_when_starting_docked() {
    let mut h = H::new();
    h.fixture();
    // La fixture termina atracada en Synthetic Port: se desatraca antes del caso a probar.
    h.journal(
        -1,
        "Undocked",
        r#","StationName":"Synthetic Port","MarketID":128000001"#,
    );
    h.journal(
        0,
        "Location",
        r#","StarSystem":"Synthetic System","SystemAddress":5,"StarPos":[0,0,0]"#,
    );
    h.market_json();
    assert!(h.val("trade.market")["station"].is_null());
    h.journal(
        1,
        "Docked",
        r#","StationName":"Synthetic Port","StationType":"Coriolis","MarketID":128000001,"StarSystem":"Synthetic System","SystemAddress":5,"StationServices":["commodities"]"#,
    );
    assert_eq!(h.val("trade.market")["station"], json!("Synthetic Port"));

    let mut h = H::new();
    h.fixture();
    h.journal(
        0,
        "Location",
        r#","Docked":true,"StationName":"Synthetic Port","StationType":"Coriolis","MarketID":128000001,"StarSystem":"Synthetic System","SystemAddress":5,"StarPos":[0,0,0]"#,
    );
    h.market_json();
    assert_eq!(h.val("trade.market")["station"], json!("Synthetic Port"));
}

#[test]
fn late_response_from_previous_system_keeps_new_pending_imports() {
    let mut h = H::with_store();
    h.dock_with_market();
    let old = h.requests_with("sells:")[0].key.clone();
    h.journal(
        10,
        "FSDJump",
        r#","StarSystem":"Elsewhere","SystemAddress":6,"StarPos":[5,0,0]"#,
    );
    h.m.pending_imports.insert("gold".into());
    h.send(net(&old, NetOutcome::NoData));
    assert!(h.m.pending_imports.contains("gold"));
}

#[test]
fn shipyard_swap_and_load_game_clear_inventory_and_switch_resets_dock() {
    let mut h = H::new();
    h.dock_with_market();
    assert!(!h.val("trade.cargo")["lines"].as_array().unwrap().is_empty());
    h.journal(20, "ShipyardSwap", r#","ShipType":"python","ShipID":2"#);
    assert!(h.val("trade.cargo")["lines"].as_array().unwrap().is_empty());
    h.journal(
        30,
        "LoadGame",
        r#","FID":"F9","Commander":"OTRO","Ship":"type9","Credits":1000"#,
    );
    assert!(h.m.docked.is_none() && h.m.market.is_none());
}

#[test]
fn offline_writes_market_but_no_ardent_and_legacy_writes_neither() {
    let mut h = H::with_store();
    h.dock_with_market();
    let key = h.requests_with("sells:")[0].key.clone();
    h.send(SourceEvent::Settings(ModuleSettings {
        online_services: false,
        ..ModuleSettings::default()
    }));
    h.send(net(
        &key,
        NetOutcome::Markets(vec![remote(11, "Known", 11000, t(-3600))]),
    ));
    assert_eq!(h.count_tag("trade.market.write"), 1);
    assert_eq!(h.count_tag("trade.ardent.write"), 0);

    let mut h = H::with_store();
    h.fixture();
    h.state.galaxy = Galaxy::Legacy;
    h.dock_with_market();
    h.send(net(
        "sells:5:gold:l",
        NetOutcome::Markets(vec![remote(11, "Known", 11000, t(-3600))]),
    ));
    assert_eq!(h.count_tag("trade.market.write"), 0);
    assert_eq!(h.count_tag("trade.ardent.write"), 0);
}

/// Criterio 3: coste medio pagado y beneficio estimado de la carga; misión y robada, fuera.
#[test]
fn trade_fixture_cargo_avg_and_profit() {
    let mut h = H::with_store();
    seed_buyer(&h.store, 2, "Other", 100, t(-3600));
    h.dock_with_market();
    let cargo = h.val("trade.cargo");
    let lines = cargo["lines"].as_array().unwrap();
    let gold = lines.iter().find(|l| l["commodity"] == "gold").unwrap();
    // (100 × 9400 + 50 × 9700) / 150 = 9500.
    assert_eq!(gold["paid_avg"], json!(9500));
    assert_eq!(gold["kind"], json!("Trade"));
    // 90 t × (11000 − 9500).
    assert_eq!(gold["est_profit"], json!(135_000));
    for l in lines.iter().filter(|l| l["commodity"] != "gold") {
        assert_ne!(l["kind"], json!("Trade"), "{l}");
        assert!(l["est_profit"].is_null(), "{l}");
    }
    assert_eq!(lines.len(), 3);
}

/// Criterio 5: Σ MarketSell − coste medio de lo vendido == beneficio de la sesión (±0).
#[test]
fn trade_session_profit_matches_fixture() {
    let mut h = H::new();
    h.fixture();
    let session = h.val("trade.session");
    let sold: i64 = 660_000;
    let cost = 60 * 9_500;
    assert_eq!(session["profit"], json!(sold - cost));
    assert_eq!(session["profit"], json!(90_000));
    assert_eq!(session["tons_sold"], json!(60));
}

fn contract_snapshot() -> Value {
    let mut h = H::with_store();
    seed_buyer(&h.store, 2, "Other", 100, t(-3600));
    h.dock_with_market();
    h.ship();
    h.action("trade.route_request", json!({"max_hops": 3}));
    let key = h.requests_with("route:")[0].key.clone();
    h.send(net(&key, NetOutcome::TradeRoute(plan())));
    json!({
        "trade.market": h.val("trade.market"),
        "trade.cargo": h.val("trade.cargo"),
        "trade.session": h.val("trade.session"),
        "trade.route": h.val("trade.route"),
        "trade.search": h.val("trade.search"),
        "trade.commodities": h.val("trade.commodities"),
    })
}

#[test]
fn ui_contract_snapshot_is_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/tests/fixtures/trade-topics.json");
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
        "el contrato JSON de trade.* cambió: regenéralo con UPDATE_CONTRACTS=1 cargo test -p tmc-modules ui_contract_snapshot"
    );
}

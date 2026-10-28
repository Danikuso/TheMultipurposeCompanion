use super::*;
use chrono::Utc;
use serde_json::json;
use tmc_core::ModuleCtx;
use tmc_model::events::parse_line;
use tmc_model::net::{
    NetOutcome, NetQuery, NetResponse, NetSource, OutfittingFallbackEntry, OutfittingSearchResult,
    OutfittingStationHit, PadSize, StockedItemPrice,
};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{Galaxy, GameState};

fn apply_journal(m: &mut ShipModule, state: &mut GameState, raw_json: &str) -> TopicSet {
    let ev = parse_line(raw_json).expect("evento de diario válido");
    state.apply_event(&ev);
    let src = SourceEvent::Journal {
        ev,
        file: "Journal.2026-10-28T120000.01.log".into(),
        line: 1,
        replaying: false,
    };
    let mut ctx = ModuleCtx::new("ship", false, 1);
    m.on_event(&src, state, &mut ctx)
}

#[test]
fn embedded_catalog_contains_48_ships_153_modules_and_permits() {
    let cat = crate::data::outfitting_catalog();
    assert_eq!(cat.ships.len(), 48);
    assert_eq!(cat.modules.len(), 153);
    assert_eq!(cat.classes, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(cat.ratings.len(), 9);
    let founders = cat
        .permit_for("shinrarta dezhra")
        .expect("Shinrarta Dezhra");
    assert_eq!(founders.permit_key, "permit-founders");
    assert!(founders.requires_elite);
    assert!(cat.permit_for("Sol").is_some());
    assert!(cat.permit_for("Diaguandri").is_none());
}

#[test]
fn syncs_commander_system_pad_elite_rank_and_pledged_power_into_catalog_topic() {
    let mut m = ShipModule::new();
    let mut state = GameState::default();

    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:00Z","event":"LoadGame","Commander":"Test","FID":"F1","Ship":"mandalay","Credits":50000000}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:01Z","event":"Loadout","Ship":"mandalay","ShipID":1,"Rebuy":1200000,"Modules":[]}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:02Z","event":"Rank","Combat":3,"Trade":8,"Explore":5,"Empire":12,"Federation":4}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:03Z","event":"Location","Docked":false,"StarSystem":"Cubeo","SystemAddress":12345,"StarPos":[10.5,-22.0,114.25]}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:04Z","event":"Powerplay","Power":"Aisling Duval","Rank":4,"Merits":12000,"TimePledged":900000}"#,
    );

    let v = m.topic_value(TOPIC_CATALOG).unwrap();
    assert_eq!(v["ships"].as_array().unwrap().len(), 48);
    assert_eq!(v["commander"]["current_system"], "Cubeo");
    assert_eq!(v["commander"]["current_ship_pad"], "Medium");
    assert_eq!(v["commander"]["pledged_power"], "Aisling Duval");
    assert_eq!(v["commander"]["has_elite_rank"], true);
    assert_eq!(v["commander"]["credits"], 50_000_000);
    assert_eq!(v["commander"]["rebuy"], 1_200_000);

    // Al abandonar la potencia, `pledged_power` pasa a null.
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:05:00Z","event":"PowerplayLeave","Power":"Aisling Duval"}"#,
    );
    let v2 = m.topic_value(TOPIC_CATALOG).unwrap();
    assert!(v2["commander"]["pledged_power"].is_null());
}

#[test]
fn search_action_propagates_stronghold_power_coords_and_enriches_permits_and_rebuy() {
    let mut m = ShipModule::new();
    let mut state = GameState {
        galaxy: Galaxy::Live,
        ..Default::default()
    };

    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:00Z","event":"LoadGame","Commander":"Novice","FID":"F2","Ship":"mandalay","Credits":21000000}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:01Z","event":"Loadout","Ship":"mandalay","ShipID":1,"Rebuy":2000000,"Modules":[]}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:02Z","event":"Rank","Combat":2,"Trade":4,"Explore":5}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:03Z","event":"Location","Docked":false,"StarSystem":"Deep Sector A-1","SystemAddress":999,"StarPos":[-500.0,12.0,880.0]}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:04Z","event":"PowerplayJoin","Power":"Li Yong-Rui"}"#,
    );

    // Lanzar búsqueda sin indicar `reference_system` (toma el sistema actual + StarPos) y con `include_own_stronghold = true`.
    // Como el comandante NO tiene rango Élite, `exclude_permits` por defecto es `false` solo si lo desactiva explícitamente;
    // primero probamos con `exclude_permits: false` para verificar la anotación de permiso y el semáforo de recompra.
    let mut ctx = ModuleCtx::new("ship", false, 42);
    let changed = m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({
                "ship": "Mandalay",
                "modules": [{ "name": "Frame Shift Drive (SCO)", "class": 5, "rating": "A" }],
                "include_own_stronghold": true,
                "exclude_permits": false
            }),
        },
        &state,
        &mut ctx,
    );
    assert!(changed.contains(TOPIC_SEARCH));

    let (reqs, _, _) = ctx.take_outputs();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].id, 42);
    let NetQuery::OutfittingSearch(ref p) = reqs[0].query else {
        panic!("se esperaba OutfittingSearch");
    };
    assert_eq!(p.reference_system, "Deep Sector A-1");
    assert_eq!(p.reference_coords, Some([-500.0, 12.0, 880.0]));
    // Al comprar una nave en modo pad automático, exige plataforma Grande (`L`).
    assert_eq!(p.min_pad, PadSize::Large);
    assert!(p.include_own_stronghold);
    assert_eq!(p.pledged_power.as_deref(), Some("Li Yong-Rui"));

    // Simular respuesta con 3 estaciones con distintos costes frente a 21M cr y 2M de recompra:
    // 1. Ray Gateway (18M cr -> sobran 3M >= 2M recompra -> "ok")
    // 2. Jameson Memorial en Shinrarta Dezhra (20M cr -> sobra 1M < 2M recompra -> "tight_rebuy" + permit-founders)
    // 3. I Sola Prospect (25M cr -> > 21M -> "insufficient")
    let mk_hit = |station: &str, system: &str, total: u64| OutfittingStationHit {
        station_name: station.into(),
        system_name: system.into(),
        station_type: Some("Coriolis Starport".into()),
        market_id: Some(1),
        distance_ly: 10.0,
        distance_to_arrival_ls: Some(300.0),
        max_pad: Some(PadSize::Large),
        is_planetary: false,
        is_carrier: false,
        is_stronghold_carrier: false,
        controlling_power: None,
        discount_tag: None,
        updated_at: Some(Utc::now()),
        item_prices: vec![StockedItemPrice {
            label: "Mandalay".into(),
            price_cr: Some(total),
        }],
        total_price_cr: Some(total),
    };

    let resp = NetResponse {
        id: 42,
        module: "ship".into(),
        key: reqs[0].key.clone(),
        source: NetSource::Spansh,
        fetched_at: Utc::now(),
        outcome: NetOutcome::OutfittingStations(OutfittingSearchResult {
            stations: vec![
                mk_hit("Ray Gateway", "Diaguandri", 18_000_000),
                mk_hit("Jameson Memorial", "Shinrarta Dezhra", 20_000_000),
                mk_hit("I Sola Prospect", "Brestla", 25_000_000),
            ],
            fallback: Vec::new(),
        }),
    };
    let mut ctx2 = ModuleCtx::new("ship", false, 43);
    m.on_event(&SourceEvent::Net(resp.clone()), &state, &mut ctx2);

    let s = m.topic_value(TOPIC_SEARCH).unwrap();
    assert_eq!(s["status"], "ok");
    let rows = s["stations"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["affordability"], "ok");
    assert!(rows[0]["permit_key"].is_null());
    assert_eq!(rows[1]["affordability"], "tight_rebuy");
    assert_eq!(rows[1]["permit_key"], "permit-founders");
    assert_eq!(rows[2]["affordability"], "insufficient");

    // Si el comandante no-Élite busca sin especificar `exclude_permits`, se autoactiva `true`
    // y oculta Shinrarta Dezhra contabilizándola en `hidden_permit_count`.
    let mut ctx3 = ModuleCtx::new("ship", false, 50);
    m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({ "ship": "Mandalay" }),
        },
        &state,
        &mut ctx3,
    );
    let mut resp50 = resp;
    resp50.id = 50;
    m.on_event(&SourceEvent::Net(resp50), &state, &mut ctx3);
    let s2 = m.topic_value(TOPIC_SEARCH).unwrap();
    assert_eq!(s2["stations"].as_array().unwrap().len(), 2);
    assert_eq!(s2["hidden_permit_count"], 1);
}

#[test]
fn search_blocks_network_in_replay_legacy_and_offline_and_supports_fallback_and_clear() {
    let mut m = ShipModule::new();
    let mut state = GameState::default();
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:03Z","event":"Location","Docked":false,"StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0]}"#,
    );

    // 1. Replay: no emite petición
    let mut replay_ctx = ModuleCtx::new("ship", true, 1);
    m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({ "ship": "Mandalay" }),
        },
        &state,
        &mut replay_ctx,
    );
    assert!(replay_ctx.take_outputs().0.is_empty());

    // 2. Galaxia Legacy: bloqueado -> estado "offline"
    state.galaxy = Galaxy::Legacy;
    let mut legacy_ctx = ModuleCtx::new("ship", false, 1);
    m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({ "ship": "Mandalay" }),
        },
        &state,
        &mut legacy_ctx,
    );
    assert!(legacy_ctx.take_outputs().0.is_empty());
    assert_eq!(m.topic_value(TOPIC_SEARCH).unwrap()["status"], "offline");

    // 3. `online_services = false`: bloqueado -> estado "offline"
    state.galaxy = Galaxy::Live;
    let off_settings = ModuleSettings {
        online_services: false,
        ..Default::default()
    };
    let mut off_ctx = ModuleCtx::new("ship", false, 1);
    m.on_event(&SourceEvent::Settings(off_settings), &state, &mut off_ctx);
    m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({ "ship": "Mandalay" }),
        },
        &state,
        &mut off_ctx,
    );
    assert!(off_ctx.take_outputs().0.is_empty());
    assert_eq!(m.topic_value(TOPIC_SEARCH).unwrap()["status"], "offline");

    // 4. Online activo + respuesta de fallback -> estado "fallback", y `ship.outfitting_clear` vuelve a "idle"
    let mut on_ctx = ModuleCtx::new("ship", false, 77);
    m.on_event(
        &SourceEvent::Settings(ModuleSettings::default()),
        &state,
        &mut on_ctx,
    );
    m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({
                "ship": "Mandalay",
                "modules": [{ "name": "Rail Gun", "class": 8, "rating": "A" }]
            }),
        },
        &state,
        &mut on_ctx,
    );
    let (reqs, _, _) = on_ctx.take_outputs();
    assert_eq!(reqs.len(), 1);
    m.on_event(
        &SourceEvent::Net(NetResponse {
            id: 77,
            module: "ship".into(),
            key: reqs[0].key.clone(),
            source: NetSource::Spansh,
            fetched_at: Utc::now(),
            outcome: NetOutcome::OutfittingStations(OutfittingSearchResult {
                stations: Vec::new(),
                fallback: vec![OutfittingFallbackEntry {
                    item_label: "8A Rail Gun".into(),
                    nearest_station: None,
                }],
            }),
        }),
        &state,
        &mut on_ctx,
    );
    assert_eq!(m.topic_value(TOPIC_SEARCH).unwrap()["status"], "fallback");

    m.on_event(
        &SourceEvent::Action {
            name: ACTION_CLEAR.into(),
            args: json!({}),
        },
        &state,
        &mut on_ctx,
    );
    assert_eq!(m.topic_value(TOPIC_SEARCH).unwrap()["status"], "idle");
}

#[test]
fn multi_component_basket_search_aggregates_items_at_same_station() {
    let mut m = ShipModule::new();
    let mut state = GameState::default();
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:00Z","event":"LoadGame","Commander":"Cmdr","FID":"F3","Ship":"krait_mkii","Credits":60000000}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:01Z","event":"Loadout","Ship":"krait_mkii","ShipID":1,"Rebuy":2500000,"Modules":[]}"#,
    );
    apply_journal(
        &mut m,
        &mut state,
        r#"{"timestamp":"2026-10-28T10:00:03Z","event":"Location","Docked":false,"StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0]}"#,
    );

    let mut ctx = ModuleCtx::new("ship", false, 99);
    let changed = m.on_event(
        &SourceEvent::Action {
            name: ACTION_SEARCH.into(),
            args: json!({
                "modules": [
                    { "name": "Power Plant", "class": 5, "rating": "A" },
                    { "name": "Thrusters", "class": 5, "rating": "A" },
                    { "name": "Frame Shift Drive (SCO)", "class": 5, "rating": "A" },
                    { "name": "Power Distributor", "class": 5, "rating": "A" },
                    { "name": "Shield Generator", "class": 5, "rating": "A" }
                ],
                "only_lyr_discount": true
            }),
        },
        &state,
        &mut ctx,
    );
    assert!(changed.contains(TOPIC_SEARCH));
    let (reqs, _, _) = ctx.take_outputs();
    assert_eq!(reqs.len(), 1);
    let NetQuery::OutfittingSearch(ref p) = reqs[0].query else {
        panic!("esperaba OutfittingSearch");
    };
    assert_eq!(p.modules.len(), 5);
    assert!(p.ship.is_none());
    assert!(p.only_lyr_discount);

    // Simular que una estación tiene los 5 componentes en la misma ubicación
    let hit_kyriazi = OutfittingStationHit {
        station_name: "Kyriazi Survey".into(),
        system_name: "Tascheter Sector IX-K a8-3".into(),
        station_type: Some("Orbis Starport".into()),
        market_id: Some(999),
        distance_ly: 36.93,
        distance_to_arrival_ls: Some(3501.0),
        max_pad: Some(PadSize::Large),
        is_planetary: false,
        is_carrier: false,
        is_stronghold_carrier: false,
        controlling_power: None,
        discount_tag: Some("lyr_15".into()),
        updated_at: Some(Utc::now()),
        item_prices: vec![
            StockedItemPrice {
                label: "5A Power Plant".into(),
                price_cr: Some(3_675_145),
            },
            StockedItemPrice {
                label: "5A Thrusters".into(),
                price_cr: Some(4_338_358),
            },
            StockedItemPrice {
                label: "5A Frame Shift Drive (SCO)".into(),
                price_cr: Some(5_206_032),
            },
            StockedItemPrice {
                label: "5A Power Distributor".into(),
                price_cr: Some(1_055_122),
            },
            StockedItemPrice {
                label: "5A Shield Generator".into(),
                price_cr: Some(4_338_358),
            },
        ],
        total_price_cr: Some(18_613_015),
    };

    let resp = NetResponse {
        id: 99,
        module: "ship".into(),
        key: reqs[0].key.clone(),
        source: NetSource::Spansh,
        fetched_at: Utc::now(),
        outcome: NetOutcome::OutfittingStations(OutfittingSearchResult {
            stations: vec![hit_kyriazi],
            fallback: Vec::new(),
        }),
    };

    let mut ctx2 = ModuleCtx::new("ship", false, 100);
    m.on_event(&SourceEvent::Net(resp), &state, &mut ctx2);

    let s = m.topic_value(TOPIC_SEARCH).unwrap();
    assert_eq!(s["status"], "ok");
    let stations = s["stations"].as_array().unwrap();
    assert_eq!(stations.len(), 1);
    let st = &stations[0];
    assert_eq!(st["station_name"], "Kyriazi Survey");
    assert_eq!(st["system_name"], "Tascheter Sector IX-K a8-3");
    assert_eq!(st["total_price_cr"], 18_613_015);
    assert_eq!(st["item_prices"].as_array().unwrap().len(), 5);
    // 60M - 18.6M = 41.4M > 2.5M de seguro -> "ok"
    assert_eq!(st["affordability"], "ok");
}

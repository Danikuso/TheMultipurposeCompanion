//! Contrato HTTP de páginas sobre el estado calculado del actor real.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tmc_core::{CoreHandle, spawn_core};
use tmc_model::events::parse_line;
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_modules::data::{blueprints, grade_cap, materials};
use tmc_modules::engineering::Engineering;
use tmc_modules::engineering::goals::Goal;
use tmc_modules::engineering::plan::{PlanRequest, plan_cost};
use tmc_server::{ServerConfig, build_router};
use tower::ServiceExt;

async fn journal(core: &CoreHandle, event: Value) {
    core.events
        .send(SourceEvent::Journal {
            ev: parse_line(&event.to_string()).unwrap(),
            file: "Journal.synthetic.log".into(),
            line: 1,
            replaying: false,
        })
        .await
        .unwrap();
}

async fn get(router: &Router, uri: &str) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .uri(uri)
        .header(header::HOST, "127.0.0.1:47320")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:55000".parse::<SocketAddr>().unwrap(),
    ));
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    assert!(bytes.len() < 16384, "HTTP page is {} bytes", bytes.len());
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn section(router: &Router, resource: &str, revision: u64, name: &str) -> Vec<Value> {
    let mut cursor = 0;
    let mut rows = Vec::new();
    loop {
        let uri = format!(
            "/api/engineering/{resource}?revision={revision}&cursor={cursor}&section={name}"
        );
        let (status, page) = get(router, &uri).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert_eq!(page["revision"], revision);
        assert_eq!(page["section"], name);
        assert_eq!(page["cursor"], cursor);
        rows.extend(page["items"].as_array().unwrap().iter().cloned());
        match page["next_cursor"].as_u64() {
            Some(next) => {
                assert!(next > cursor);
                cursor = next;
            }
            None => {
                assert_eq!(rows.len() as u64, page["total"]);
                return rows;
            }
        }
    }
}

#[tokio::test]
async fn engineering_http_pages_are_lossless_and_invalidate_with_live_state() {
    let core = spawn_core(
        vec![Box::new(Engineering::new())],
        None,
        Duration::from_millis(100),
    );
    core.events
        .send(SourceEvent::Settings(ModuleSettings {
            online_services: false,
            ..Default::default()
        }))
        .await
        .unwrap();
    journal(
        &core,
        json!({"timestamp":"2026-10-21T12:00:00Z","event":"Commander","Name":"CMDR TESTER"}),
    )
    .await;
    let request = PlanRequest {
        module_type: "engine".into(),
        blueprint: "engine_dirty".into(),
        experimental: Some("special_engine_overloaded".into()),
        target_grade: 5,
        rolls: [10; 5],
    };
    let catalog = blueprints();
    let cost = plan_cost(catalog, "int_engine_size5_class5", None, &request);
    let mut categories: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for (key, material) in materials() {
        let count = if cost.total.contains_key(key) {
            0
        } else {
            grade_cap(material.grade)
        };
        categories
            .entry(format!("{:?}", material.category))
            .or_default()
            .push(json!({"Name":key,"Count":count}));
    }
    journal(&core, json!({
        "timestamp":"2026-10-21T12:00:01Z","event":"Materials",
        "Raw":categories["Raw"],"Manufactured":categories["Manufactured"],"Encoded":categories["Encoded"],
    })).await;
    let goals: Vec<_> = (0..20)
        .map(|i| {
            Goal::new(
                Some(100 + i),
                format!("LargeModuleSlot{i}"),
                "int_engine_size8_class5".into(),
                request.clone(),
                None,
            )
        })
        .collect();
    let ids: Vec<_> = goals.iter().map(|goal| goal.id.clone()).collect();
    core.events
        .send(SourceEvent::Stored {
            module: "engineering".into(),
            tag: "engineering.goals".into(),
            value: json!({"cmdr":"CMDR TESTER","goals":goals}),
        })
        .await
        .unwrap();
    let locker_section = |prefix: &str| {
        (0..50)
            .map(|i| {
                json!({
                    "Name":format!("{prefix}_resource_{i:03}"),
                    "Name_Localised":format!("Recurso localizado {i:03}"),"Count":10,
                })
            })
            .collect::<Vec<_>>()
    };
    journal(
        &core,
        json!({
            "timestamp":"2026-10-21T12:00:02Z","event":"ShipLocker",
            "Items":locker_section("item"),"Components":locker_section("component"),
            "Consumables":locker_section("consumable"),"Data":locker_section("data"),
        }),
    )
    .await;
    let snapshot = core.snapshot().await;
    let revision = snapshot["engineering.goals"]["revision"].as_u64().unwrap();
    assert_eq!(snapshot["engineering.goals"]["total"], 20);
    assert_eq!(snapshot["engineering.locker"]["counts"]["components"], 50);
    assert_eq!(snapshot["engineering.locker"]["revision"], revision);
    for topic in ["engineering.goals", "engineering.locker"] {
        assert!(serde_json::to_vec(&snapshot[topic]).unwrap().len() < 16384);
    }
    let router = build_router(
        ServerConfig::default(),
        "127.0.0.1".into(),
        core.clone(),
        tmc_core::Store::open_in_memory().unwrap(),
        Arc::new(crate::AppActions::new()),
    );
    let first_uri = format!("/api/engineering/goals?revision={revision}");
    let (a, b) = tokio::join!(get(&router, &first_uri), get(&router, &first_uri));
    assert_eq!(a, b);
    assert_eq!(a.0, StatusCode::OK);
    assert!(
        a.1["next_cursor"].as_u64().is_some(),
        "worst case must span pages"
    );
    let rows = section(&router, "goals", revision, "goals").await;
    assert_eq!(
        rows.iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ids.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(
        rows.iter()
            .map(|row| row["trades"].as_array().unwrap().len())
            .sum::<usize>(),
        215
    );
    let needs = section(&router, "goals", revision, "needs").await;
    assert_eq!(
        needs
            .iter()
            .map(|row| row["count"].as_u64().unwrap())
            .sum::<u64>(),
        2060
    );
    for name in ["missing", "surplus"] {
        assert!(!section(&router, "goals", revision, name).await.is_empty());
    }
    for (name, prefix) in [
        ("items", "item"),
        ("components", "component"),
        ("consumables", "consumable"),
        ("data", "data"),
    ] {
        let rows = section(&router, "locker", revision, name).await;
        assert_eq!(rows.len(), 50);
        assert_eq!(rows[0]["key"], format!("{prefix}_resource_000"));
        assert_eq!(rows[49]["key"], format!("{prefix}_resource_049"));
        assert!(rows.iter().all(|row| row["count"] == 10));
    }
    assert_eq!(
        core.snapshot().await["engineering.goals"]["revision"],
        revision,
        "queries must not mutate the revision"
    );
    journal(
        &core,
        json!({"timestamp":"2026-10-21T12:00:03Z","event":"MaterialCollected",
        "Category":"Raw","Name":"carbon","Count":1}),
    )
    .await;
    let changed = core.snapshot().await["engineering.goals"]["revision"]
        .as_u64()
        .unwrap();
    assert!(changed > revision);
    assert_eq!(get(&router, &first_uri).await.0, StatusCode::CONFLICT);
    assert_eq!(
        section(&router, "goals", changed, "goals").await[0]["missing"]["carbon"],
        9
    );
    for (offset, name) in [(4, "OTHER TESTER"), (5, "CMDR TESTER")] {
        journal(&core,json!({"timestamp":format!("2026-10-21T12:00:0{offset}Z"),"event":"Commander","Name":name})).await;
        assert_eq!(core.snapshot().await["engineering.goals"]["total"], 0);
        let stale_uri = format!("/api/engineering/locker?revision={changed}&section=components");
        assert_eq!(get(&router, &stale_uri).await.0, StatusCode::CONFLICT);
    }
}

#[tokio::test]
async fn catalogue_engineers_and_flow_pages_are_lossless_and_independent_per_client() {
    use tmc_modules::data::engineers;
    let core = spawn_core(
        vec![Box::new(Engineering::new())],
        None,
        Duration::from_millis(100),
    );
    core.events
        .send(SourceEvent::Settings(ModuleSettings {
            online_services: false,
            ..Default::default()
        }))
        .await
        .unwrap();
    journal(
        &core,
        json!({"timestamp":"2026-10-22T12:00:00Z","event":"Commander","Name":"CMDR TESTER"}),
    )
    .await;
    journal(
        &core,
        json!({"timestamp":"2026-10-22T12:00:01Z","event":"Loadout","Ship":"asp","ShipID":7,
        "Modules":[{"Slot":"MainEngines","Item":"int_engine_size5_class5","On":true,"Priority":0}]}),
    )
    .await;
    core.events
        .send(SourceEvent::Action {
            name: "engineering.select_module".into(),
            args: json!({"slot":"MainEngines"}),
        })
        .await
        .unwrap();
    core.events
        .send(SourceEvent::Action {
            name: "engineering.preview".into(),
            args: json!({"blueprint":"engine_dirty","experimental":"special_engine_overloaded","grade":5}),
        })
        .await
        .unwrap();
    let snapshot = core.snapshot().await;
    let revision = snapshot["engineering.catalogue"]["revision"]
        .as_u64()
        .unwrap();
    for topic in [
        "engineering.catalogue",
        "engineering.engineers",
        "engineering.flow",
    ] {
        assert_eq!(snapshot[topic]["revision"], revision, "{topic}");
        assert!(serde_json::to_vec(&snapshot[topic]).unwrap().len() < 16384);
    }
    let router = build_router(
        ServerConfig::default(),
        "127.0.0.1".into(),
        core.clone(),
        tmc_core::Store::open_in_memory().unwrap(),
        Arc::new(crate::AppActions::new()),
    );
    let recipes = section(&router, "catalogue", revision, "recipes").await;
    assert_eq!(
        recipes.len() as u64,
        snapshot["engineering.catalogue"]["counts"]["recipes"]
    );
    assert_eq!(
        section(&router, "engineers", revision, "engineers")
            .await
            .len(),
        engineers().len()
    );
    let lines = section(&router, "flow", revision, "lines").await;
    assert_eq!(
        lines.len() as u64,
        snapshot["engineering.flow"]["counts"]["lines"]
    );
    assert!(snapshot["engineering.flow"].get("lines").is_none());
    // Two clients keep independent cursors and receive identical bytes.
    let uri = format!("/api/engineering/catalogue?revision={revision}&section=recipes&cursor=1");
    let (a, b) = tokio::join!(get(&router, &uri), get(&router, &uri));
    assert_eq!(a, b);
    assert_eq!(a.1["cursor"], 1);
    // engineer_id filters the engineers resource; garbage is a 400 and other resources reject it.
    let base = format!("/api/engineering/engineers?revision={revision}&section=capabilities");
    let (status, filtered) = get(&router, &format!("{base}&engineer_id=300100")).await;
    assert_eq!(status, 200);
    assert!(
        filtered["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["engineer_id"] == 300100)
    );
    assert_eq!(
        get(&router, &format!("{base}&engineer_id=1")).await.1["total"],
        0
    );
    assert_eq!(
        get(&router, &format!("{base}&engineer_id=abc")).await.0,
        400
    );
    let flow =
        format!("/api/engineering/flow?revision={revision}&section=lines&engineer_id=300100");
    assert_eq!(get(&router, &flow).await.0, 400);
    // GET never mutates the revision or broadcasts topics.
    assert_eq!(core.snapshot().await, snapshot);
    // A flow change invalidates every resource, not only flow.
    core.events
        .send(SourceEvent::Action {
            name: "engineering.close_flow".into(),
            args: json!({}),
        })
        .await
        .unwrap();
    let changed = core.snapshot().await["engineering.flow"]["revision"]
        .as_u64()
        .unwrap();
    assert!(changed > revision);
    for resource in ["catalogue", "engineers", "flow"] {
        let stale = format!("/api/engineering/{resource}?revision={revision}");
        assert_eq!(get(&router, &stale).await.0, StatusCode::CONFLICT);
    }
    assert!(section(&router, "flow", changed, "costs").await.is_empty());
}

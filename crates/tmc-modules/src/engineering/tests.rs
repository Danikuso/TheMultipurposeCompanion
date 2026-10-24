use super::*;
use tmc_core::Store;
use tmc_model::events::parse_line;
use tmc_model::net::{NetFailure, NetRequest, NetSource, RemoteStation, TraderKind};

const FIXTURE: &str = include_str!("../../../../tests/fixtures/journal/engineering_session.log");

/// Instante base de la fixture; los eventos propios de las pruebas lo avanzan.
fn t(seconds: i64) -> DateTime<Utc> {
    "2026-10-21T12:00:00Z".parse::<DateTime<Utc>>().unwrap() + chrono::Duration::seconds(seconds)
}

struct H {
    m: Engineering,
    state: GameState,
    replaying: bool,
    exec: bool,
    store: Store,
    requests: Vec<NetRequest>,
    tags: Vec<String>,
    results: Vec<(String, Value)>,
    next_id: u64,
    line_no: u64,
    clock: i64,
    max_page_size: std::cell::Cell<usize>,
}

impl H {
    fn pages(&self, resource: &str, section: &str) -> Vec<Value> {
        let mut rows = Vec::new();
        let mut cursor = 0;
        loop {
            let page = self
                .m
                .query(
                    resource,
                    &ModulePageQuery {
                        revision: self.m.revision,
                        cursor,
                        section: Some(section.into()),
                        engineer_id: None,
                    },
                )
                .unwrap();
            let size = serde_json::to_vec(&page).unwrap().len();
            self.max_page_size.set(self.max_page_size.get().max(size));
            assert!(size < 16384, "{resource}/{section}: {size} bytes");
            rows.extend(page["items"].as_array().unwrap().iter().cloned());
            match page["next_cursor"].as_u64() {
                Some(next) => {
                    assert!(next as usize > cursor);
                    cursor = next as usize;
                }
                None => {
                    assert_eq!(rows.len(), page["total"].as_u64().unwrap() as usize);
                    return rows;
                }
            }
        }
    }
    fn details(&self, topic: &str) -> Value {
        let mut value = self.val(topic);
        let resource = topic.strip_prefix("engineering.").unwrap();
        let sections: &[&str] = if resource == "goals" {
            &["goals", "needs", "missing", "surplus"]
        } else {
            &["items", "components", "consumables", "data"]
        };
        for section in sections {
            let rows = self.pages(resource, section);
            value[*section] = if resource == "goals" && *section != "goals" {
                Value::Object(
                    rows.into_iter()
                        .map(|r| (r["key"].as_str().unwrap().to_owned(), r["count"].clone()))
                        .collect(),
                )
            } else {
                json!(rows)
            };
        }
        value
    }
    fn new() -> Self {
        Self {
            m: Engineering::new(),
            state: GameState::default(),
            replaying: false,
            exec: false,
            store: Store::open_in_memory().unwrap(),
            requests: Vec::new(),
            tags: Vec::new(),
            results: Vec::new(),
            next_id: 1,
            line_no: 0,
            clock: 600,
            max_page_size: std::cell::Cell::new(0),
        }
    }

    /// Ejecuta las tareas de almacén contra una base en memoria y reenvía su resultado.
    fn with_store() -> Self {
        let mut h = Self::new();
        h.exec = true;
        h.store.apply_migrations(&[ENGINEERING_MIGRATION]).unwrap();
        h
    }

    /// Segundo módulo sobre la misma base (reinicio de la app).
    fn restart(old: &H) -> Self {
        let mut h = Self::new();
        h.exec = true;
        h.store = old.store.clone();
        h
    }

    fn send(&mut self, mut ev: SourceEvent) -> TopicSet {
        // Las respuestas abreviadas se correlacionan con la petición real más reciente.
        if let SourceEvent::Net(r) = &mut ev
            && r.id == 0
            && let Some(q) = self.requests.iter().rev().find(|q| q.key == r.key)
        {
            r.id = q.id;
        }
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
        let mut ctx = ModuleCtx::new("engineering", self.replaying, self.next_id);
        let mut dirty = self.m.on_event(&ev, &self.state, &mut ctx);
        let (requests, tasks, next) = ctx.take_outputs();
        self.next_id = next;
        self.requests.extend(requests);
        let mut follow = Vec::new();
        for (tag, task) in tasks {
            if self.exec
                && let Ok(value) = task(&self.store)
            {
                self.results.push((tag.clone(), value.clone()));
                follow.push(SourceEvent::Stored {
                    module: "engineering".into(),
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

    /// Evento propio con una marca de tiempo posterior a la fixture.
    fn journal(&mut self, name: &str, extra: &str) -> TopicSet {
        self.clock += 1;
        let line = format!(
            r#"{{"timestamp":"{}","event":"{name}"{extra}}}"#,
            t(self.clock).to_rfc3339()
        );
        self.line(&line)
    }

    fn fixture(&mut self) {
        for l in FIXTURE.lines() {
            self.line(l);
        }
    }

    fn action(&mut self, name: &str, args: Value) -> TopicSet {
        self.send(SourceEvent::Action {
            name: name.into(),
            args,
        })
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

    fn offline(&mut self) {
        self.send(SourceEvent::Settings(ModuleSettings {
            online_services: false,
            ..ModuleSettings::default()
        }));
    }

    /// Flujo del contrato: motores, Dirty Drive Tuning G5 con Drag Drives.
    fn preview_engines(&mut self) {
        self.action("engineering.select_module", json!({"slot": "MainEngines"}));
        self.action("engineering.preview", engine_preview());
    }

    /// Fija el objetivo del FSD (`fsd_longrange` G5, sin experimental).
    fn pin_fsd(&mut self) {
        self.action(
            "engineering.select_module",
            json!({"slot": "FrameShiftDrive"}),
        );
        self.action(
            "engineering.preview",
            json!({"blueprint": "fsd_longrange", "grade": 5}),
        );
        self.action("engineering.pin_goal", json!({}));
    }
}

fn engine_preview() -> Value {
    json!({"blueprint": "engine_dirty", "experimental": "special_engine_overloaded", "grade": 5})
}

fn station(name: &str, ly: f64, pad: PadSize) -> RemoteStation {
    RemoteStation {
        station_name: name.into(),
        system_name: "Synthetic Neighbour".into(),
        distance_ly: ly,
        distance_to_arrival_ls: Some(1200.0),
        max_pad: Some(pad),
        updated_at: None,
    }
}

fn net(key: &str, outcome: NetOutcome) -> SourceEvent {
    SourceEvent::Net(NetResponse {
        id: 0,
        module: "engineering".into(),
        key: key.into(),
        source: NetSource::Spansh,
        fetched_at: t(700),
        outcome,
    })
}

fn line_of<'a>(inv: &'a Value, cat: &str, grade: &str, key: &str) -> &'a Value {
    inv["by_category"][cat][grade]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["key"] == key)
        .unwrap_or_else(|| panic!("{key} no está en {cat} G{grade}"))
}

fn flow_line<'a>(lines: &'a [Value], key: &str) -> &'a Value {
    lines
        .iter()
        .find(|l| l["key"] == key)
        .unwrap_or_else(|| panic!("línea {key}"))
}

/// Campos pesados que ya no viajan en los topics (se sirven por REST paginado).
fn assert_no_heavy_fields(topic: &Value, fields: &[&str]) {
    for field in fields {
        assert!(topic.get(*field).is_none(), "{field} sigue en el topic");
    }
}

fn deliver_goals(h: &mut H, cmdr: &str, goals: &[Goal]) -> TopicSet {
    h.send(SourceEvent::Stored {
        module: "engineering".into(),
        tag: TAG_GOALS_READ.into(),
        value: json!({"cmdr": cmdr, "goals": goals}),
    })
}

#[test]
fn pending_read_preserves_updated_pin_and_unpin() {
    for remove in [false, true] {
        let mut h = H::new();
        h.fixture();
        h.pin_fsd();
        let stored = h.m.goals.clone();
        h.action(
            ACTION_PREVIEW,
            json!({"blueprint":"fsd_longrange", "grade":5, "rolls":[2,2,2,2,9]}),
        );
        h.action(ACTION_PIN_GOAL, json!({}));
        if remove {
            h.action(ACTION_UNPIN_GOAL, json!({"id":stored[0].id}));
        }
        assert_eq!(h.count_tag(TAG_GOALS_WRITE), 0);
        deliver_goals(&mut h, "CMDR TESTER", &stored);
        if remove {
            assert_eq!(h.m.goals.len(), 0);
        } else {
            assert_eq!(h.m.goals[0].request.rolls, [2, 2, 2, 2, 9]);
        }
        assert_eq!(h.count_tag(TAG_GOALS_WRITE), 1);
    }
}

#[test]
fn pending_read_preserves_move_and_ignores_duplicate_results() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    h.preview_engines();
    h.action(ACTION_PIN_GOAL, json!({}));
    let stored = h.m.goals.clone();
    let id = stored[1].id.clone();
    h.action(ACTION_MOVE_GOAL, json!({"id":id,"to":0}));
    deliver_goals(&mut h, "CMDR TESTER", &stored);
    assert_eq!(h.m.goals[0].id, id);
    h.action(ACTION_MOVE_GOAL, json!({"id":id,"to":1}));
    let before = h.val(TOPIC_GOALS);
    assert!(deliver_goals(&mut h, "CMDR TESTER", &stored).is_empty());
    assert_eq!(h.val(TOPIC_GOALS), before);
    assert_eq!(h.m.goals[1].id, id);
}

#[test]
fn pending_read_is_cleared_on_commander_switch() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    let old = h.m.goals.clone();
    h.journal("Commander", r#", "Name":"B""#);
    assert!(deliver_goals(&mut h, "CMDR TESTER", &old).is_empty());
    deliver_goals(&mut h, "B", &[]);
    assert_eq!(h.m.goals.len(), 0);
    assert_eq!(h.count_tag(TAG_GOALS_WRITE), 0);
}

#[test]
fn pending_read_keeps_limit_and_replaces_existing_at_capacity() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    let mut stored = h.m.goals.clone();
    for i in 1..goals::MAX_GOALS {
        let mut g = stored[0].clone();
        g.id = format!("saved-{i}");
        g.slot = format!("Slot{i}");
        stored.push(g);
    }
    h.action(
        ACTION_PREVIEW,
        json!({"blueprint":"fsd_longrange", "grade":5,"rolls":[2,2,2,2,9]}),
    );
    h.action(ACTION_PIN_GOAL, json!({}));
    h.preview_engines();
    h.action(ACTION_PIN_GOAL, json!({}));
    deliver_goals(&mut h, "CMDR TESTER", &stored);
    assert_eq!(h.m.goals.len(), 20);
    assert_eq!(h.m.goals[0].request.rolls, [2, 2, 2, 2, 9]);
    assert_eq!(h.val(TOPIC_GOALS)["status"], "too_many");
}

#[test]
fn incompatible_known_blueprint_rejected_without_changes() {
    let mut h = H::new();
    h.fixture();
    h.preview_engines();
    let before = h.val(TOPIC_FLOW);
    assert!(
        h.action(
            ACTION_PREVIEW,
            json!({"blueprint":"fsd_longrange","grade":5})
        )
        .is_empty()
    );
    assert_eq!(h.val(TOPIC_FLOW), before);
    let mut req = h.m.flow.as_ref().unwrap().request.clone().unwrap();
    req.blueprint = "fsd_longrange".into();
    req.experimental = None;
    assert!(
        h.m.cost_for("int_engine_size8_class5", &req, None)
            .has_resolution_blocker()
    );
    h.m.flow.as_mut().unwrap().request = Some(req);
    assert!(h.action(ACTION_PIN_GOAL, json!({})).is_empty());
    assert_eq!(h.m.goals.len(), 0);
    h.action(
        ACTION_PREVIEW,
        json!({"blueprint":"future_unknown","grade":5}),
    );
    assert_eq!(h.val(TOPIC_FLOW)["status"], "no_data");
    assert!(h.pages("flow", "costs").is_empty());
}

#[test]
fn loadout_replacing_same_slot_closes_flow() {
    for item in ["int_hyperdrive_size5_class5", "int_unknownmodule"] {
        let mut h = H::new();
        h.fixture();
        h.preview_engines();
        h.journal("Loadout", &format!(r#", "Ship":"asp", "ShipID":7,"Modules":[{{"Slot":"MainEngines","Item":"{item}","On":true,"Priority":0}}]"#));
        assert_eq!(h.val(TOPIC_FLOW)["status"], "idle");
        assert!(h.m.flow.is_none());
        assert!(h.action(ACTION_PIN_GOAL, json!({})).is_empty());
    }
}

#[test]
fn pad_change_invalidates_station_and_rejects_late_response() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    let request = h.requests_with("trader:Encoded:")[0].clone();
    let response = |id, name: &str| {
        SourceEvent::Net(NetResponse {
            id,
            module: "engineering".into(),
            key: request.key.clone(),
            source: NetSource::Spansh,
            fetched_at: t(700),
            outcome: NetOutcome::Stations(vec![station(name, 8.2, PadSize::Medium)]),
        })
    };
    h.send(response(request.id, "Medium Port"));
    assert_eq!(
        h.val(TOPIC_SOURCES)["traders"]["Encoded"]["Ok"]["station_name"],
        "Medium Port"
    );
    h.journal("Loadout", r#", "Ship":"anaconda","ShipID":8,"Modules":[]"#);
    assert_eq!(h.val(TOPIC_SOURCES)["traders"]["Encoded"], "Loading");
    let new = (*h.requests_with("trader:Encoded:").last().unwrap()).clone();
    assert_ne!(new.id, request.id);
    assert!(matches!(
        new.query,
        NetQuery::NearestService {
            min_pad: PadSize::Large,
            ..
        }
    ));
    let before = h.val(TOPIC_SOURCES);
    assert!(h.send(response(request.id, "Late Medium")).is_empty());
    assert_eq!(h.val(TOPIC_SOURCES), before);
    h.send(SourceEvent::Net(NetResponse {
        id: new.id,
        module: "engineering".into(),
        key: new.key.clone(),
        source: NetSource::Spansh,
        fetched_at: t(701),
        outcome: NetOutcome::Stations(vec![station("Large Port", 12.0, PadSize::Large)]),
    }));
    assert_eq!(
        h.val(TOPIC_SOURCES)["traders"]["Encoded"]["Ok"]["station_name"],
        "Large Port"
    );
}

#[test]
fn source_retry_same_id_and_online_cycle_correlation() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    let request = h.requests_with("trader:Encoded:")[0].clone();
    h.send(net(&request.key, NetOutcome::Failed(NetFailure::Timeout)));
    assert_eq!(h.val(TOPIC_SOURCES)["traders"]["Encoded"], "NoData");
    h.send(net(
        &request.key,
        NetOutcome::Stations(vec![station("Retry Port", 8.2, PadSize::Large)]),
    ));
    assert_eq!(
        h.val(TOPIC_SOURCES)["traders"]["Encoded"]["Ok"]["station_name"],
        "Retry Port"
    );
    // Otra búsqueda pendiente pierde validez al desactivar servicios.
    h.journal(
        "FSDJump",
        r#", "StarSystem":"Other","SystemAddress":777,"StarPos":[1,2,3]"#,
    );
    let old = (*h.requests_with("trader:Encoded:").last().unwrap()).clone();
    h.offline();
    h.send(SourceEvent::Settings(ModuleSettings::default()));
    let new = (*h.requests_with("trader:Encoded:").last().unwrap()).clone();
    assert_ne!(new.id, old.id);
    let before = h.val(TOPIC_SOURCES);
    assert!(
        h.send(SourceEvent::Net(NetResponse {
            id: old.id,
            module: "engineering".into(),
            key: old.key,
            source: NetSource::Spansh,
            fetched_at: t(700),
            outcome: NetOutcome::Stations(vec![station("Obsolete Port", 1.0, PadSize::Large)])
        }))
        .is_empty()
    );
    assert_eq!(h.val(TOPIC_SOURCES), before);
}

// ----- Criterios de aceptación -----

/// Criterio 2: la fixture reproduce la instantánea más los cambios.
#[test]
fn fixture_inventory_matches_snapshot_plus_changes() {
    let mut h = H::new();
    h.fixture();
    let inv = h.val("engineering.inventory");
    // 120 + 3 recogidos − 6 intercambiados.
    assert_eq!(line_of(&inv, "Raw", "1", "iron")["count"], json!(117));
    assert_eq!(
        line_of(&inv, "Raw", "1", "iron")["name_localised"],
        "Hierro"
    );
    // 150 + 2 recogidos, pero el máximo de G4 es 150.
    assert_eq!(line_of(&inv, "Raw", "4", "polonium")["count"], json!(150));
    assert_eq!(inv["at_max"], json!(["polonium"]));
    assert_eq!(inv["caps"]["1"], json!(300));
    // Encoded 12 + 2; el material no catalogado conserva su nombre del diario.
    assert_eq!(
        line_of(&inv, "Encoded", "4", "shieldpatternanalysis")["count"],
        json!(14)
    );
    let unknown = line_of(&inv, "Encoded", "0", "foo_new");
    assert_eq!(unknown["count"], json!(4));
    assert_eq!(unknown["known"], json!(false));
    // Sin nombre localizado: respaldo en inglés del catálogo.
    assert_eq!(
        line_of(&inv, "Raw", "1", "carbon")["name_en"],
        json!("Carbon")
    );
}

#[test]
fn locker_and_session_from_fixture() {
    let mut h = H::new();
    h.fixture();
    let locker = h.details("engineering.locker");
    assert_eq!(locker["components"][0]["key"], "graphene");
    assert_eq!(locker["components"][0]["count"], json!(8));
    let s = h.val("engineering.session");
    assert_eq!(s["totals"]["Scan"], json!({"in": 2, "out": 0}));
    assert_eq!(s["totals"]["Trade"], json!({"in": 1, "out": 6}));
    assert_eq!(s["recent"].as_array().unwrap().len(), 9);
    h.action("engineering.reset_session", json!({}));
    let s = h.val("engineering.session");
    assert_eq!(s["totals"], json!({}));
    assert_eq!(s["recent"], json!([]));
}

/// `ShipLocker.json` (archivo compañero) sustituye al evento sin listas del diario.
#[test]
fn ship_locker_companion_file_updates_the_locker() {
    let mut h = H::new();
    h.fixture();
    assert_eq!(
        h.details("engineering.locker")["components"][0]["count"],
        json!(8)
    );
    let file = json!({
        "timestamp": "2026-10-21T12:06:00Z", "event": "ShipLocker",
        "Items": [{"Name": "healthpack", "Name_Localised": "Medkit", "OwnerID": 0, "Count": 2}],
        "Components": [{"Name": "graphene", "Name_Localised": "Graphene", "OwnerID": 0, "Count": 11}],
        "Consumables": [], "Data": []
    });
    let dirty = h.send(SourceEvent::Companion(CompanionFile::Raw(
        CompanionKind::ShipLocker,
        file,
    )));
    assert!(dirty.contains("engineering.locker"));
    let locker = h.details("engineering.locker");
    assert_eq!(locker["components"][0]["count"], json!(11));
    assert_eq!(locker["items"][0]["key"], "healthpack");
    // El mismo archivo otra vez no cambia nada.
    let again = h.send(SourceEvent::Companion(CompanionFile::Raw(
        CompanionKind::ShipLocker,
        json!({"Items": [{"Name": "healthpack", "Name_Localised": "Medkit", "OwnerID": 0, "Count": 2}],
               "Components": [{"Name": "graphene", "Name_Localised": "Graphene", "OwnerID": 0, "Count": 11}],
               "Consumables": [], "Data": []}),
    )));
    assert!(again.is_empty());
}

#[test]
fn engineers_topic_has_rank_and_distance() {
    let mut h = H::new();
    h.fixture();
    let e = h.val("engineering.engineers");
    assert_no_heavy_fields(&e, &["engineers", "selected"]);
    let rows = h.pages("engineers", "engineers");
    assert_eq!(e["total"], json!(rows.len()));
    let f = rows.iter().find(|r| r["id"] == 300100).unwrap();
    assert_eq!(f["status"], "Unlocked");
    assert_eq!(f["rank"], json!(5));
    let d = f["distance_ly"].as_f64().unwrap();
    assert!((d - 12.3).abs() < 1e-6, "{d}");
    assert!(e["selected_id"].is_null());
    assert!(e["next_step"].is_null());
    // Elvira se desbloqueó con un `EngineerProgress` suelto.
    let el = rows.iter().find(|r| r["id"] == 300160).unwrap();
    assert_eq!(el["status"], "Unlocked");
}

#[test]
fn select_engineer_shows_next_step_and_blueprints() {
    let mut h = H::new();
    h.fixture();
    // Tod McQuinn (Known): requisito de invitación.
    h.action("engineering.select_engineer", json!({"id": 300260}));
    let sel = h.val("engineering.engineers");
    assert_eq!(sel["selected_id"], json!(300260));
    assert!(sel["next_step"]["Requirement"]["key"].is_string());
    h.action("engineering.select_engineer", json!({"id": 300100}));
    let sel = h.val("engineering.engineers");
    assert_eq!(sel["next_step"], json!("Done"));
    let capabilities = h.pages("engineers", "capabilities");
    let grades: Vec<u64> = capabilities
        .iter()
        .filter(|c| c["engineer_id"] == 300100 && c["blueprint"] == "engine_dirty")
        .map(|c| c["grade"].as_u64().unwrap())
        .collect();
    assert_eq!(grades, vec![1, 2, 3]);

    // Volver a seleccionar el mismo ingeniero lo repliega/deselecciona.
    h.action("engineering.select_engineer", json!({"id": 300100}));
    let sel_toggled = h.val("engineering.engineers");
    assert_eq!(sel_toggled["selected_id"], serde_json::Value::Null);

    // Enviar id null también deselecciona.
    h.action("engineering.select_engineer", json!({"id": 300100}));
    assert_eq!(h.val("engineering.engineers")["selected_id"], json!(300100));
    h.action("engineering.select_engineer", json!({"id": null}));
    assert_eq!(
        h.val("engineering.engineers")["selected_id"],
        serde_json::Value::Null
    );

    // Un id desconocido no cambia nada.
    let before = h.val("engineering.engineers");
    let dirty = h.action("engineering.select_engineer", json!({"id": 1}));
    assert!(dirty.is_empty());
    assert_eq!(h.val("engineering.engineers"), before);
}

#[test]
fn ship_topic_carries_current_for_every_module() {
    let mut h = H::new();
    h.fixture();
    let ship = h.val("engineering.ship");
    let fsd = ship["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["slot"] == "FrameShiftDrive")
        .unwrap();
    // Tras la `EngineerCraft` del diario el FSD está en G4 con calidad completa.
    assert_eq!(fsd["current"]["level"], json!(4));
    assert_eq!(fsd["current"]["quality"], json!(1.0));
    assert_eq!(fsd["engineerable"], json!(true));
    // Un módulo no catalogado con ingeniería sigue mostrando la actual.
    h.journal(
        "Loadout",
        r#","Ship":"asp","ShipID":7,"Modules":[{"Slot":"Misc1","Item":"int_weirdthing","On":true,"Priority":0,"Engineering":{"BlueprintName":"Misc_Strange","Level":2,"Quality":0.5}}]"#,
    );
    let m = &h.val("engineering.ship")["modules"][0];
    assert_eq!(m["engineerable"], json!(false));
    assert_eq!(m["current"]["level"], json!(2));
    // Y uno de serie, `current: null`.
    h.journal(
        "Loadout",
        r#","Ship":"asp","ShipID":7,"Modules":[{"Slot":"MainEngines","Item":"int_engine_size5_class5","On":true,"Priority":0}]"#,
    );
    assert!(h.val("engineering.ship")["modules"][0]["current"].is_null());
}

/// Flujo con la nave de la fixture: motores de serie → Dirty Drive Tuning G5 + Drag Drives.
#[test]
fn preview_flow_with_fixture_ship() {
    let mut h = H::new();
    h.fixture();
    h.preview_engines();
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "ok");
    assert_eq!(flow["slot"], "MainEngines");
    assert_eq!(flow["module_type"], "engine");
    assert!(flow["current"].is_null());
    assert_eq!(flow["request"]["target_grade"], json!(5));
    assert_eq!(flow["request"]["rolls"], json!([1, 1, 1, 1, 4]));
    assert_no_heavy_fields(&flow, &["lines", "cost", "options"]);
    let lines = h.pages("flow", "lines");
    assert_eq!(lines.len(), 13);
    // 9 de 30 unidades cubiertas: 9 × 100 / 30.
    assert_eq!(flow["percent"], json!(30));
    let pharma = flow_line(&lines, "pharmaceuticalisolators");
    assert_eq!(
        (pharma["need"].clone(), pharma["have"].clone()),
        (json!(4), json!(8))
    );
    assert_eq!(pharma["state"], "ok");
    let hybrid = flow_line(&lines, "hybridcapacitors");
    assert_eq!(hybrid["state"], "trade");
    assert_eq!(hybrid["missing"], json!(0));
    assert_eq!(hybrid["need"], json!(3));
    assert_eq!(hybrid["have"], json!(0));
    // Felicity solo llega a G3 en engine_dirty: el G5 lo hace el Profesor Palin (sin desbloquear).
    assert!(flow["engineer"]["nearest_unlocked"].is_null());
    assert_eq!(flow["engineer"]["nearest_any"]["id"], json!(300220));
    assert_eq!(
        flow["engineer"]["next_step"]["Requirement"]["key"],
        "unlock-professor-palin"
    );
    let costs = h.pages("flow", "costs");
    assert!(costs.iter().any(|c| {
        c["grade"] == json!(5)
            && c["material"] == "pharmaceuticalisolators"
            && c["count"] == json!(4)
    }));
    let options = h.pages("flow", "blueprints");
    assert_eq!(
        options.iter().map(|o| o["key"].clone()).collect::<Vec<_>>(),
        vec![
            json!("engine_dirty"),
            json!("engine_reinforced"),
            json!("engine_tuned")
        ]
    );
    // Cerrar el flujo lo vacía.
    h.action("engineering.close_flow", json!({}));
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "idle");
    assert!(flow["slot"].is_null());
}

#[test]
fn blocked_flow_keeps_its_reason_instead_of_plain_no_data() {
    let mut h = H::new();
    h.fixture();
    h.action("engineering.select_module", json!({"slot": "MainEngines"}));
    h.action(
        "engineering.preview",
        json!({"blueprint": "nope", "grade": 3}),
    );
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "no_data");
    assert_eq!(flow["blockers"], json!(["unknown_blueprint"]));
    assert!(h.pages("flow", "costs").is_empty());
    // un flujo sin bloqueo los publica vacios
    h.preview_engines();
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "ok");
    assert_eq!(flow["blockers"], json!([]));
}

#[test]
fn pin_rejection_distinguishes_incompatible_from_no_data() {
    use super::plan::PlanBlocker as B;
    let with = |b: B| PlanCost {
        blockers: vec![b],
        ..PlanCost::default()
    };
    assert_eq!(
        pin_rejection(&with(B::IncompatibleItem)),
        Some("incompatible")
    );
    for b in [
        B::UnknownBlueprint,
        B::MissingBaseMapping,
        B::BaseRequired,
        B::AmbiguousRecipe,
        B::UnknownExperimental,
    ] {
        assert_eq!(pin_rejection(&with(b)), Some("no_data"), "{b:?}");
    }
    // un hueco de receta no impide fijar; el objetivo queda bloqueado en la vista
    assert_eq!(pin_rejection(&with(B::MissingRecipe { grade: 2 })), None);
    assert_eq!(pin_rejection(&PlanCost::default()), None);
}

#[test]
fn unknown_blueprint_is_no_data() {
    let mut h = H::new();
    h.fixture();
    h.action("engineering.select_module", json!({"slot": "MainEngines"}));
    h.action(
        "engineering.preview",
        json!({"blueprint": "nope", "grade": 3}),
    );
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "no_data");
    assert!(h.pages("flow", "costs").is_empty());
    assert!(h.pages("flow", "lines").is_empty());
    // Y no se puede fijar como objetivo.
    let dirty = h.action("engineering.pin_goal", json!({}));
    assert!(dirty.is_empty());
    assert_eq!(h.details("engineering.goals")["goals"], json!([]));
}

#[test]
fn preview_rejects_invalid_arguments_without_changes() {
    let mut h = H::new();
    h.fixture();
    h.action("engineering.select_module", json!({"slot": "MainEngines"}));
    h.action("engineering.preview", engine_preview());
    let before = h.val("engineering.flow");
    for args in [
        // tiradas fuera de 1..=10
        json!({"blueprint": "engine_dirty", "grade": 5, "rolls": [1, 1, 1, 1, 0]}),
        json!({"blueprint": "engine_dirty", "grade": 5, "rolls": [1, 1, 1, 1, 11]}),
        json!({"blueprint": "engine_dirty", "grade": 5, "rolls": [1, 1, 1, 1]}),
        // experimental de otro tipo de módulo
        json!({"blueprint": "engine_dirty", "grade": 5, "experimental": "nope"}),
        // grado y blueprint inválidos
        json!({"blueprint": "engine_dirty", "grade": 6}),
        json!({"blueprint": "engine_dirty", "grade": 0}),
        json!({"grade": 3}),
    ] {
        let dirty = h.action("engineering.preview", args.clone());
        assert!(dirty.is_empty(), "{args}");
        assert_eq!(h.val("engineering.flow"), before, "{args}");
    }
    // Tiradas válidas en el límite sí se aplican.
    h.action(
        "engineering.preview",
        json!({"blueprint": "engine_dirty", "grade": 5, "rolls": [10, 10, 10, 10, 10]}),
    );
    assert_eq!(
        h.val("engineering.flow")["request"]["rolls"],
        json!([10, 10, 10, 10, 10])
    );
    // Sin flujo abierto no hay vista previa.
    h.action("engineering.close_flow", json!({}));
    assert!(h.action("engineering.preview", engine_preview()).is_empty());
    // Una ranura inexistente o sin módulo no abre el flujo.
    assert!(
        h.action("engineering.select_module", json!({"slot": "Nope"}))
            .is_empty()
    );
    assert!(h.action("engineering.select_module", json!({})).is_empty());
}

// ----- Red, replay, Legacy -----

#[test]
fn replay_emits_no_requests_or_store_writes() {
    let mut h = H::with_store();
    h.replaying = true;
    h.fixture();
    h.preview_engines();
    h.pin_fsd();
    assert_eq!(h.requests.len(), 0);
    assert_eq!(
        h.tags.iter().filter(|t| t.ends_with(".write")).count(),
        0,
        "{:?}",
        h.tags
    );
    // Tampoco se lee el almacén en el replay.
    assert!(h.tags.is_empty(), "{:?}", h.tags);
    // El objetivo sí queda en memoria y las fuentes están desactivadas.
    assert_eq!(
        h.details("engineering.goals")["goals"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let sources = h.val("engineering.sources");
    assert!(
        sources["traders"]
            .as_object()
            .unwrap()
            .values()
            .all(|s| s == "Disabled")
    );
}

#[test]
fn no_requests_in_legacy_or_offline() {
    let mut h = H::with_store();
    h.state.galaxy = Galaxy::Legacy;
    h.fixture();
    // `LoadGame` de la fixture no trae versión: Legacy se mantiene al forzarlo después.
    h.state.galaxy = Galaxy::Legacy;
    h.preview_engines();
    h.pin_fsd();
    assert_eq!(h.requests.len(), 0);
    assert_eq!(h.count_tag("engineering.goals.write"), 0);
    let traders = h.val("engineering.sources")["traders"].clone();
    assert!(!traders.as_object().unwrap().is_empty());
    assert!(
        traders
            .as_object()
            .unwrap()
            .values()
            .all(|s| s == "Disabled")
    );

    let mut h = H::with_store();
    h.offline();
    h.fixture();
    h.preview_engines();
    assert_eq!(h.requests.len(), 0);
    let traders = h.val("engineering.sources")["traders"].clone();
    assert!(
        traders
            .as_object()
            .unwrap()
            .values()
            .all(|s| s == "Disabled")
    );
}

/// Criterio 6: Spansh/Ardent falso con un comerciante cercano.
#[test]
fn fake_ardent_trader_shows_nearest() {
    let mut h = H::with_store();
    h.fixture();
    h.pin_fsd();
    let req = h
        .requests_with("trader:Encoded:")
        .first()
        .map(|r| (r.key.clone(), r.query.clone()))
        .expect("petición de comerciante Encoded");
    assert_eq!(req.0, "trader:Encoded:900100");
    // Plataforma de la nave (asp = M) y solo el sistema actual.
    assert_eq!(
        req.1,
        NetQuery::NearestService {
            system_address: 900100,
            system_name: "Synthetic Neighbour".into(),
            service: NearbyService::MaterialTrader(Some(TraderKind::Encoded)),
            min_pad: PadSize::Medium,
        }
    );
    assert_eq!(
        h.val("engineering.sources")["traders"]["Encoded"],
        json!("Loading")
    );
    let dirty = h.send(net(
        &req.0,
        NetOutcome::Stations(vec![
            station("Far Port", 30.0, PadSize::Large),
            station("Fake Port", 8.2, PadSize::Large),
        ]),
    ));
    assert!(dirty.contains("engineering.sources"));
    let ok = &h.val("engineering.sources")["traders"]["Encoded"]["Ok"];
    assert_eq!(ok["station_name"], "Fake Port");
    assert_eq!(ok["distance_ly"], json!(8.2));
    assert_eq!(ok["max_pad"], "Large");
}

#[test]
fn requests_are_deduplicated_while_in_flight() {
    let mut h = H::with_store();
    h.fixture();
    // Menos de `INFLIGHT_TTL_S` después de la fixture: la petición sigue en vuelo.
    h.clock = 300;
    h.pin_fsd();
    let first = h.requests_with("trader:Encoded:").len();
    assert_eq!(first, 1);
    // Más eventos con las mismas necesidades no repiten la petición.
    h.journal("Scan", r#","BodyName":"x""#);
    h.preview_engines();
    assert_eq!(h.requests_with("trader:Encoded:").len(), 1);
}

#[test]
fn failed_or_empty_response_becomes_no_data_not_loading() {
    for outcome in [
        NetOutcome::Failed(NetFailure::Timeout),
        NetOutcome::NoData,
        NetOutcome::Stations(Vec::new()),
    ] {
        let mut h = H::with_store();
        h.fixture();
        h.pin_fsd();
        h.send(net("trader:Encoded:900100", outcome));
        assert_eq!(
            h.val("engineering.sources")["traders"]["Encoded"],
            json!("NoData")
        );
        // No se repite la petición ya respondida.
        h.journal("Scan", r#","BodyName":"x""#);
        assert_eq!(h.requests_with("trader:Encoded:").len(), 1);
    }
}

#[test]
fn no_typed_request_with_empty_or_unknown_system_name() {
    let mut h = H::with_store();
    h.fixture();
    // Sistema con dirección pero sin nombre: nada tipado que pedir.
    h.journal(
        "Location",
        r#","StarSystem":"","SystemAddress":555,"StarPos":[1,2,3]"#,
    );
    let before = h.requests.len();
    h.pin_fsd();
    h.preview_engines();
    assert_eq!(h.requests.len(), before);
    assert!(
        h.requests
            .iter()
            .skip(before)
            .all(|r| !matches!(&r.query, NetQuery::NearestService { system_name, .. } if system_name.trim().is_empty()))
    );
    // Sin dirección de sistema tampoco.
    let mut h = H::with_store();
    h.fixture();
    h.journal("Location", r#","StarSystem":"Somewhere","StarPos":[1,2,3]"#);
    let before = h.requests.len();
    h.pin_fsd();
    assert_eq!(h.requests.len(), before);
}

#[test]
fn no_services_shows_no_data_and_rest_works() {
    let mut h = H::with_store();
    h.offline();
    h.fixture();
    h.preview_engines();
    let sources = h.val("engineering.sources");
    assert!(
        sources["traders"]
            .as_object()
            .unwrap()
            .values()
            .all(|s| s == "Disabled")
    );
    assert_eq!(h.val("engineering.flow")["status"], "ok");
    let rows = h.pages("engineers", "engineers");
    let f = rows.iter().find(|r| r["id"] == 300100).unwrap().clone();
    assert!((f["distance_ly"].as_f64().unwrap() - 12.3).abs() < 1e-6);
    assert_eq!(h.requests.len(), 0);
    // Al activar los servicios vuelven a pedirse.
    h.send(SourceEvent::Settings(ModuleSettings::default()));
    assert!(!h.requests_with("trader:").is_empty());
}

#[test]
fn late_trader_for_previous_system_not_applied() {
    let mut h = H::with_store();
    h.fixture();
    h.pin_fsd();
    let key = h.requests_with("trader:Encoded:")[0].key.clone();
    assert_eq!(key, "trader:Encoded:900100");
    h.journal(
        "FSDJump",
        r#","StarSystem":"Elsewhere","SystemAddress":777,"StarPos":[1,2,3]"#,
    );
    let before = h.val("engineering.sources");
    assert_eq!(before["traders"]["Encoded"], json!("Loading"));
    // Pedido nuevo para el sistema actual.
    assert!(!h.requests_with("trader:Encoded:777").is_empty());
    let dirty = h.send(net(
        &key,
        NetOutcome::Stations(vec![station("Late Port", 1.0, PadSize::Large)]),
    ));
    assert!(dirty.is_empty());
    assert_eq!(h.val("engineering.sources"), before);
}

// ----- Comandante y almacén -----

#[test]
fn commander_switch_clears_and_reloads_goals() {
    let mut h = H::with_store();
    h.fixture();
    h.pin_fsd();
    assert_eq!(
        h.details("engineering.goals")["goals"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(h.count_tag("engineering.goals.write") >= 1);
    h.results.clear();
    h.journal("Commander", r#","Name":"B""#);
    assert_eq!(h.details("engineering.goals")["goals"], json!([]));
    assert_eq!(h.val("engineering.inventory")["at_max"], json!([]));
    assert_eq!(h.details("engineering.locker")["components"], json!([]));
    assert_eq!(h.val("engineering.flow")["status"], "idle");
    assert_eq!(h.val("engineering.session")["totals"], json!({}));
    let read = h
        .results
        .iter()
        .find(|(tag, _)| tag == "engineering.goals")
        .expect("lectura de objetivos del nuevo comandante");
    assert_eq!(read.1["cmdr"], "B");
    assert_eq!(read.1["goals"], json!([]));
}

#[test]
fn stored_goals_for_other_cmdr_ignored() {
    let mut h = H::with_store();
    h.fixture();
    let before = h.details("engineering.goals");
    let dirty = h.send(SourceEvent::Stored {
        module: "engineering".into(),
        tag: "engineering.goals".into(),
        value: json!({"cmdr": "OTRO", "goals": [{
            "id": "7:FrameShiftDrive:fsd_longrange:-:5",
            "ship_id": 7, "slot": "FrameShiftDrive", "item": "x",
            "request": {"module_type": "fsd", "blueprint": "fsd_longrange", "experimental": null, "target_grade": 5, "rolls": [1,1,1,1,4]},
            "base": null
        }]}),
    });
    assert!(dirty.is_empty());
    assert_eq!(h.details("engineering.goals"), before);
}

/// Criterio 7: la lista sobrevive a un reinicio y avisa al poder completarse.
#[test]
fn goals_survive_restart_and_alert_completable() {
    let mut h = H::with_store();
    h.fixture();
    h.pin_fsd();
    let goals = h.details("engineering.goals");
    assert_eq!(goals["goals"].as_array().unwrap().len(), 1);
    let writes = h.count_tag("engineering.goals.write");
    assert_eq!(writes, 1);

    let mut h2 = H::restart(&h);
    h2.fixture();
    assert_eq!(h2.details("engineering.goals")["goals"], goals["goals"]);
    // Releer no escribe nada.
    assert_eq!(h2.count_tag("engineering.goals.write"), 0);

    // Escenario propio: faltan 1 dataminedwake y no hay excedente que intercambiar.
    let mut h = H::with_store();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "Materials",
        r#","Raw":[{"Name":"arsenic","Count":4}],"Manufactured":[{"Name":"chemicalmanipulators","Count":4}],"Encoded":[{"Name":"dataminedwake","Count":3}]"#,
    );
    h.journal(
        "EngineerProgress",
        r#","Engineers":[{"Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":5}]"#,
    );
    h.journal(
        "LoadGame",
        r#","Commander":"CMDR TESTER","Ship":"asp","ShipID":7,"GameMode":"Solo","Credits":10"#,
    );
    h.journal(
        "Loadout",
        r#","Ship":"asp","ShipID":7,"Modules":[{"Slot":"FrameShiftDrive","Item":"int_hyperdrive_size5_class5","On":true,"Priority":0,"Engineering":{"BlueprintName":"FSD_LongRange","Level":4,"Quality":1.0}}]"#,
    );
    h.pin_fsd();
    let g = &h.details("engineering.goals")["goals"][0];
    assert_eq!(g["completable"], json!(false));
    assert_eq!(h.val("engineering.alerts")["alerts"], json!([]));
    h.journal(
        "MaterialCollected",
        r#","Category":"Encoded","Name":"dataminedwake","Count":1"#,
    );
    let g = &h.details("engineering.goals")["goals"][0];
    assert_eq!(g["completable"], json!(true));
    assert_eq!(g["engineer"], "Felicity Farseer");
    let alerts = h.val("engineering.alerts");
    let kinds: Vec<&str> = alerts["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"goal_completable"), "{kinds:?}");
    let a = alerts["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["kind"] == "goal_completable")
        .unwrap();
    assert_eq!(a["engineer"], "Felicity Farseer");
    assert_eq!(a["id"], json!("goal:7:FrameShiftDrive:fsd_longrange:-:5"));
}

#[test]
fn unpin_and_move_goals_rewrite_the_list() {
    let mut h = H::with_store();
    h.fixture();
    h.pin_fsd();
    h.action("engineering.select_module", json!({"slot": "MainEngines"}));
    h.action(
        "engineering.preview",
        json!({"blueprint": "engine_dirty", "grade": 3}),
    );
    h.action("engineering.pin_goal", json!({}));
    let ids: Vec<String> = h.details("engineering.goals")["goals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids.len(), 2);
    h.action("engineering.move_goal", json!({"id": ids[1], "to": 0}));
    let moved: Vec<String> = h.details("engineering.goals")["goals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(moved, vec![ids[1].clone(), ids[0].clone()]);
    // Movimientos inválidos: sin cambios.
    for args in [
        json!({"id": "nope", "to": 0}),
        json!({"id": ids[0], "to": 5}),
        json!({"id": ids[0]}),
        json!({"id": ids[0], "to": -1}),
    ] {
        assert!(h.action("engineering.move_goal", args).is_empty());
    }
    h.action("engineering.unpin_goal", json!({"id": ids[1]}));
    assert_eq!(
        h.details("engineering.goals")["goals"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        h.action("engineering.unpin_goal", json!({"id": "nope"}))
            .is_empty()
    );
    // 1 fijar + 1 fijar + mover + quitar.
    assert_eq!(h.count_tag("engineering.goals.write"), 4);
    // Lo escrito es lo que queda.
    let (_, last) = h
        .results
        .iter()
        .rfind(|(t, _)| t == "engineering.goals.write")
        .unwrap();
    assert_eq!(last["count"], json!(1));
}

#[test]
fn pin_too_many_is_reported_and_list_intact() {
    let mut h = H::new();
    h.fixture();
    for i in 0..goals::MAX_GOALS {
        h.m.goals.push(Goal::new(
            Some(7),
            format!("Slot{i}"),
            "x".into(),
            PlanRequest {
                module_type: "fsd".into(),
                blueprint: "fsd_longrange".into(),
                experimental: None,
                target_grade: 5,
                rolls: [1, 1, 1, 1, 4],
            },
            None,
        ));
    }
    h.pin_fsd();
    let g = h.details("engineering.goals");
    assert_eq!(g["status"], "too_many");
    assert_eq!(g["goals"].as_array().unwrap().len(), goals::MAX_GOALS);
}

// ----- Avisos -----

#[test]
fn startup_snapshots_raise_no_alerts() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "LoadGame",
        r#","Commander":"CMDR TESTER","Ship":"asp","ShipID":7,"GameMode":"Solo","Credits":10"#,
    );
    h.journal(
        "Materials",
        r#","Raw":[{"Name":"polonium","Count":150},{"Name":"iron","Count":300}],"Manufactured":[],"Encoded":[]"#,
    );
    h.journal(
        "EngineerProgress",
        r#","Engineers":[{"Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":5},{"Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Invited"}]"#,
    );
    h.journal(
        "Loadout",
        r#","Ship":"asp","ShipID":7,"Modules":[{"Slot":"FrameShiftDrive","Item":"int_hyperdrive_size5_class5","On":true,"Priority":0}]"#,
    );
    assert_eq!(h.val("engineering.alerts")["alerts"], json!([]));
    assert_eq!(
        h.val("engineering.inventory")["at_max"],
        json!(["iron", "polonium"])
    );
    // Una resincronización posterior de lo mismo tampoco avisa.
    h.journal(
        "Materials",
        r#","Raw":[{"Name":"polonium","Count":150},{"Name":"iron","Count":300}],"Manufactured":[],"Encoded":[]"#,
    );
    assert_eq!(h.val("engineering.alerts")["alerts"], json!([]));
}

#[test]
fn live_transitions_raise_alerts_and_respect_settings() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "Materials",
        r#","Raw":[{"Name":"polonium","Count":149}],"Manufactured":[],"Encoded":[]"#,
    );
    h.journal(
        "EngineerProgress",
        r#","Engineers":[{"Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Known"}]"#,
    );
    h.journal(
        "MaterialCollected",
        r#","Category":"Raw","Name":"polonium","Count":1"#,
    );
    h.journal(
        "EngineerProgress",
        r#","Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Invited""#,
    );
    let kinds: Vec<String> = h.val("engineering.alerts")["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["material_at_max", "engineer_invited"]);
    // Descartar un aviso lo quita; un índice fuera de rango no hace nada.
    assert!(
        h.action("engineering.dismiss_alert", json!({"index": 9}))
            .is_empty()
    );
    h.action("engineering.dismiss_alert", json!({"index": 0}));
    assert_eq!(
        h.val("engineering.alerts")["alerts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Con los avisos desactivados no se registra ninguno.
    let mut h = H::new();
    h.send(SourceEvent::Settings(ModuleSettings {
        alerts_enabled: false,
        ..ModuleSettings::default()
    }));
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "Materials",
        r#","Raw":[{"Name":"polonium","Count":149}],"Manufactured":[],"Encoded":[]"#,
    );
    h.journal(
        "MaterialCollected",
        r#","Category":"Raw","Name":"polonium","Count":1"#,
    );
    assert_eq!(h.val("engineering.alerts")["alerts"], json!([]));
    // Ni en replay.
    let mut h = H::new();
    h.replaying = true;
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "Materials",
        r#","Raw":[{"Name":"polonium","Count":149}],"Manufactured":[],"Encoded":[]"#,
    );
    h.journal(
        "MaterialCollected",
        r#","Category":"Raw","Name":"polonium","Count":1"#,
    );
    assert_eq!(h.val("engineering.alerts")["alerts"], json!([]));
}

#[test]
fn alerts_are_capped_at_twenty_without_duplicate_ids() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    // 25 materiales catalogados de grado 1 a una unidad de su máximo (300).
    let keys: Vec<&str> = [
        "carbon",
        "phosphorus",
        "sulphur",
        "iron",
        "nickel",
        "rhenium",
        "lead",
        "salvagedalloys",
        "gridresistors",
        "chemicalstorageunits",
        "compactcomposites",
        "basicconductors",
        "crystalshards",
        "heatconductionwiring",
        "mechanicalscrap",
        "wornshieldemitters",
        "temperedalloys",
        "bulkscandata",
        "scrambledemissiondata",
        "legacyfirmware",
        "encryptedfiles",
        "disruptedwakeechoes",
        "shieldcyclerecordings",
        "vanadium",
        "chromium",
    ]
    .to_vec();
    let mats = |cat: &str| -> Vec<String> {
        keys.iter()
            .filter(|k| {
                crate::data::materials()
                    .get(**k)
                    .is_some_and(|d| format!("{:?}", d.category) == cat)
            })
            .map(|k| {
                let cap = crate::data::grade_cap(crate::data::materials()[*k].grade);
                format!(r#"{{"Name":"{k}","Count":{}}}"#, cap - 1)
            })
            .collect()
    };
    h.journal(
        "Materials",
        &format!(
            r#","Raw":[{}],"Manufactured":[{}],"Encoded":[{}]"#,
            mats("Raw").join(","),
            mats("Manufactured").join(","),
            mats("Encoded").join(",")
        ),
    );
    for k in &keys {
        let d = &crate::data::materials()[*k];
        h.journal(
            "MaterialCollected",
            &format!(r#","Category":"{:?}","Name":"{k}","Count":1"#, d.category),
        );
    }
    let alerts = h.val("engineering.alerts")["alerts"].clone();
    let alerts = alerts.as_array().unwrap();
    assert_eq!(alerts.len(), 20);
    // Se descartaron los más antiguos.
    assert_eq!(
        alerts[19]["id"],
        json!(format!("at_max:{}", {
            let mut sorted: Vec<&str> = keys.clone();
            sorted.sort_unstable();
            // el último recogido es `chromium` (orden de las recogidas)
            let _ = sorted;
            "chromium"
        }))
    );
    let ids: BTreeSet<String> = alerts
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids.len(), 20);
    // Recoger de nuevo lo que ya estaba al máximo no duplica nada.
    h.journal(
        "MaterialDiscarded",
        r#","Category":"Raw","Name":"chromium","Count":1"#,
    );
    h.journal(
        "MaterialCollected",
        r#","Category":"Raw","Name":"chromium","Count":1"#,
    );
    let again = h.val("engineering.alerts")["alerts"].clone();
    let again: Vec<&str> = again
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(again.iter().filter(|i| **i == "at_max:chromium").count(), 1);
    assert_eq!(again.len(), 20);
}

// ----- Relevancia -----

fn relevance(h: &H) -> Relevance {
    h.m.relevance(&h.state)
}

#[test]
fn startup_engineer_list_does_not_activate_layout() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "EngineerProgress",
        r#","Engineers":[{"Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":5}]"#,
    );
    assert_eq!(relevance(&h), Relevance::Normal);
    h.journal(
        "EngineerCraft",
        r#","Slot":"FrameShiftDrive","Module":"int_hyperdrive_size5_class5","Ingredients":[],"Engineer":"Felicity Farseer","EngineerID":300100,"BlueprintName":"FSD_LongRange","Level":4,"Quality":1.0"#,
    );
    assert_eq!(relevance(&h), Relevance::Active);
    h.journal("Undocked", r#","StationName":"X","StationType":"Outpost""#);
    assert_eq!(relevance(&h), Relevance::Normal);
    // Una actualización suelta también activa; saltar al hiperespacio lo limpia.
    h.journal(
        "EngineerProgress",
        r#","Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Unlocked""#,
    );
    assert_eq!(relevance(&h), Relevance::Active);
    h.journal(
        "FSDJump",
        r#","StarSystem":"Elsewhere","SystemAddress":5,"StarPos":[1,2,3]"#,
    );
    assert_eq!(relevance(&h), Relevance::Normal);
    // La contribución a un ingeniero también.
    h.journal(
        "EngineerContribution",
        r#","Engineer":"Elvira Martuuk","EngineerID":300160,"Type":"Commodity","Commodity":"soontillrelics","Quantity":3,"TotalQuantity":3"#,
    );
    assert_eq!(relevance(&h), Relevance::Active);
}

#[test]
fn relevance_active_docked_at_workshop() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "Location",
        r#","StarSystem":"Deciat","SystemAddress":6681123623626,"StarPos":[122.625,-0.8125,-47.28125]"#,
    );
    assert_eq!(relevance(&h), Relevance::Normal);
    h.journal(
        "Docked",
        r#","StationName":"Farseer Inc","StationType":"Outpost","MarketID":128999001,"StarSystem":"Deciat","SystemAddress":6681123623626"#,
    );
    assert_eq!(relevance(&h), Relevance::Active);
    h.journal(
        "Undocked",
        r#","StationName":"Farseer Inc","MarketID":128999001"#,
    );
    assert_eq!(relevance(&h), Relevance::Normal);
    // Atracado en otra estación del mismo sistema: no es un taller.
    h.journal(
        "Docked",
        r#","StationName":"Other Port","StationType":"Outpost","MarketID":1,"StarSystem":"Deciat","SystemAddress":6681123623626"#,
    );
    assert_eq!(relevance(&h), Relevance::Normal);
    // Abrir el flujo también activa.
    h.journal("Loadout", r#","Ship":"asp","ShipID":7,"Modules":[{"Slot":"MainEngines","Item":"int_engine_size5_class5","On":true,"Priority":0}]"#);
    h.action("engineering.select_module", json!({"slot": "MainEngines"}));
    assert_eq!(relevance(&h), Relevance::Active);
    h.action("engineering.close_flow", json!({}));
    assert_eq!(relevance(&h), Relevance::Normal);
}

// ----- Posición -----

#[test]
fn location_without_star_pos_does_not_set_position() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    for ev in ["Location", "FSDJump", "CarrierJump"] {
        h.journal(ev, r#","StarSystem":"Nowhere","SystemAddress":9"#);
        let rows = h.pages("engineers", "engineers");
        assert!(rows.iter().all(|r| r["distance_ly"].is_null()), "{ev}");
    }
    // Con posición válida sí hay distancias.
    h.journal(
        "FSDJump",
        r#","StarSystem":"Somewhere","SystemAddress":10,"StarPos":[0,0,0]"#,
    );
    let rows = h.pages("engineers", "engineers");
    assert!(rows.iter().all(|r| r["distance_ly"].is_number()));
}

#[test]
fn commodity_contribution_have_comes_from_cargo() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "EngineerProgress",
        r#","Engineers":[{"Engineer":"Elvira Martuuk","EngineerID":300160,"Progress":"Invited"}]"#,
    );
    h.state.ship = Some(tmc_model::state::ShipState {
        cargo: vec![tmc_model::companion::CargoItem {
            name: "SoontillRelics".into(),
            count: 2,
            ..Default::default()
        }],
        ..Default::default()
    });
    h.action("engineering.select_engineer", json!({"id": 300160}));
    let step = h.val("engineering.engineers")["next_step"].clone();
    assert_eq!(step["Contribute"]["have"], json!(2));
    assert_eq!(step["Contribute"]["need"], json!(3));
}

// ----- Topics -----

#[test]
fn topics_dirty_only_on_change() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    let materials = r#","Raw":[{"Name":"iron","Count":10}],"Manufactured":[],"Encoded":[]"#;
    let first = h.journal("Materials", materials);
    assert!(first.contains("engineering.inventory"));
    // El mismo `Materials` más tarde: solo cambia `as_of`.
    let second = h.journal("Materials", materials);
    assert!(!second.contains("engineering.inventory"), "{second:?}");
    assert!(second.is_empty(), "{second:?}");
    let third = h.journal(
        "Materials",
        r#","Raw":[{"Name":"iron","Count":11}],"Manufactured":[],"Encoded":[]"#,
    );
    assert!(third.contains("engineering.inventory"));
    // El valor publicado sí avanza su `as_of`... pero no cuenta como cambio.
    assert!(h.val("engineering.inventory")["as_of"].is_string());
}

#[test]
fn worst_case_engine_goals_size() {
    let mut h = H::new();
    h.fixture();
    h.journal("Materials", r#", "Raw":[],"Manufactured":[],"Encoded":[]"#);
    h.m.goals.clear();
    for i in 0..20 {
        h.m.goals.push(Goal::new(
            Some(8),
            format!("LargeModuleSlot{i}"),
            "int_engine_size8_class5".into(),
            PlanRequest {
                module_type: "engine".into(),
                blueprint: "engine_dirty".into(),
                experimental: Some("special_engine_overloaded".into()),
                target_grade: 5,
                rolls: [10, 10, 10, 10, 10],
            },
            None,
        ));
    }
    h.action(ACTION_RESET_SESSION, json!({}));
    let mut maximum = 0;
    let mut max_trades = 0;
    for bp in &blueprints().blueprints {
        for exp in [
            None,
            blueprints()
                .experimentals_for(bp.module_types()[0])
                .first()
                .copied(),
        ] {
            let req = PlanRequest {
                module_type: bp.module_types()[0].to_owned(),
                blueprint: bp.key.clone(),
                experimental: exp.map(|e| e.key.clone()),
                target_grade: 5,
                rolls: [10; 5],
            };
            let prefix = blueprints()
                .module_types
                .iter()
                .find(|t| t.id == req.module_type)
                .and_then(|t| t.item_prefixes.first().map(String::as_str))
                .unwrap_or("int_module_");
            let probe = format!("{prefix}size8_class5");
            let cost = h.m.cost_for(&probe, &req, None);
            h.m.goals = (0..20)
                .map(|i| {
                    Goal::new(
                        Some(100 + i),
                        format!("LargeModuleSlot{i}"),
                        "int_engine_size8_class5".into(),
                        req.clone(),
                        None,
                    )
                })
                .collect();
            for surplus in [false, true] {
                let mut cats: BTreeMap<String, Vec<Value>> = BTreeMap::new();
                for (key, def) in materials() {
                    let count = if surplus && !cost.total.contains_key(key) {
                        grade_cap(def.grade)
                    } else {
                        0
                    };
                    cats.entry(format!("{:?}", def.category))
                        .or_default()
                        .push(json!({"Name":key,"Count":count}));
                }
                h.journal(
                    "Materials",
                    &format!(
                        r#", "Raw":{},"Manufactured":{},"Encoded":{}"#,
                        json!(cats["Raw"]),
                        json!(cats["Manufactured"]),
                        json!(cats["Encoded"])
                    ),
                );
                let v = h.details(TOPIC_GOALS);
                assert_eq!(v["goals"].as_array().unwrap().len(), 20);
                let trades: usize = v["goals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g["trades"].as_array().unwrap().len())
                    .sum();
                max_trades = max_trades.max(trades);
                let size = serde_json::to_vec(&h.val(TOPIC_GOALS)).unwrap().len();
                assert_eq!(v["goals"], json!(h.m.goal_pages["goals"]));
                for section in ["goals", "needs", "missing", "surplus"] {
                    h.pages("goals", section);
                }
                maximum = maximum.max(size);
                eprintln!(
                    "goals {} experimental={} surplus={surplus}: {size} B, {trades} trades",
                    bp.key,
                    exp.is_some()
                );
            }
        }
    }
    eprintln!(
        "largest goals summary: {maximum} bytes; largest page: {} bytes; trades: {max_trades}",
        h.max_page_size.get()
    );
    assert_eq!(max_trades, 244);
    assert!(maximum < 16384, "engineering.goals: {maximum} bytes");
}

#[test]
fn engineering_topics_stay_under_16_kib() {
    let mut h = H::new();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    // Catálogo entero al máximo con nombres localizados (≈ 22 caracteres).
    let mut by_cat: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, def) in crate::data::materials() {
        by_cat
            .entry(format!("{:?}", def.category))
            .or_default()
            .push(format!(
                r#"{{"Name":"{key}","Name_Localised":"Material {:03} localizado","Count":{}}}"#,
                key.len() * 7 % 1000,
                crate::data::grade_cap(def.grade)
            ));
    }
    let list = |c: &str| by_cat.get(c).cloned().unwrap_or_default().join(",");
    h.journal(
        "Materials",
        &format!(
            r#","Raw":[{}],"Manufactured":[{}],"Encoded":[{}]"#,
            list("Raw"),
            list("Manufactured"),
            list("Encoded")
        ),
    );
    let actual_count: usize = h.val(TOPIC_INVENTORY)["by_category"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|c| c.as_object().unwrap().values())
        .map(|g| g.as_array().unwrap().len())
        .sum();
    assert_eq!(actual_count, materials().len());
    let section = |prefix: &str| {
        (0..50).map(|i|json!({"Name":format!("{prefix}_resource_{i:03}"),"Name_Localised":format!("Recurso localizado {i:03}"),"Count":10})).collect::<Vec<_>>()
    };
    h.journal(
        "ShipLocker",
        &format!(
            r#", "Items":{},"Components":{},"Consumables":{},"Data":{}"#,
            json!(section("item")),
            json!(section("component")),
            json!(section("consumable")),
            json!(section("data"))
        ),
    );
    for section in ["items", "components", "consumables", "data"] {
        assert_eq!(
            h.details(TOPIC_LOCKER)[section].as_array().unwrap().len(),
            50
        );
    }
    let expected = serde_json::to_value(h.m.locker.view()).unwrap();
    for section in ["items", "components", "consumables", "data"] {
        assert_eq!(json!(h.pages("locker", section)), expected[section]);
    }
    eprintln!(
        "locker summary: {} bytes; largest locker page: {} bytes; 200 rows",
        serde_json::to_vec(&h.val(TOPIC_LOCKER)).unwrap().len(),
        h.max_page_size.get()
    );
    for _ in 0..25 {
        h.journal(
            "MaterialDiscarded",
            r#", "Category":"Raw","Name":"iron","Count":1"#,
        );
        h.journal(
            "MaterialCollected",
            r#", "Category":"Raw","Name":"iron","Count":1"#,
        );
    }
    assert_eq!(h.val(TOPIC_SESSION)["recent"].as_array().unwrap().len(), 50);
    // 60 módulos con ingeniería.
    let modules: Vec<String> = (0..60)
        .map(|i| {
            format!(
                r#"{{"Slot":"Slot{i}","Item":"int_hyperdrive_size5_class5","On":true,"Priority":0,"Engineering":{{"BlueprintName":"FSD_LongRange","Level":4,"Quality":0.5,"ExperimentalEffect":"special_fsd_fastboot"}}}}"#
            )
        })
        .collect();
    h.journal(
        "Loadout",
        &format!(
            r#","Ship":"asp","ShipID":7,"Modules":[{}]"#,
            modules.join(",")
        ),
    );
    // 20 objetivos distintos y el flujo abierto con todas las líneas posibles.
    for i in 0..goals::MAX_GOALS {
        h.m.goals.push(Goal::new(
            Some(7),
            format!("Slot{i}"),
            "int_hyperdrive_size5_class5".into(),
            PlanRequest {
                module_type: "fsd".into(),
                blueprint: "fsd_longrange".into(),
                experimental: None,
                target_grade: 5,
                rolls: [10, 10, 10, 10, 10],
            },
            None,
        ));
    }
    h.m.flow = Some(Flow {
        ship_id: Some(7),
        slot: "Slot0".into(),
        request: None,
    });
    h.action("engineering.select_module", json!({"slot": "Slot0"}));
    h.action(
        "engineering.preview",
        json!({"blueprint": "fsd_longrange", "grade": 5, "rolls": [10, 10, 10, 10, 10]}),
    );
    h.action("engineering.select_engineer", json!({"id": 300160}));
    h.journal("Scan", r#","BodyName":"x""#);
    h.m.alerts.clear();
    for a in 0..MAX_ALERTS {
        h.m.alerts.push(json!({"id": format!("at_max:long-material-key-{a}"), "kind": "material_at_max", "key": format!("long-material-key-{a}"), "grade": 5, "max": 100, "at": h.m.now}));
    }
    h.send(SourceEvent::Settings(ModuleSettings::default()));
    let sizes: Vec<(&str, usize)> = TOPICS
        .iter()
        .map(|t| (*t, serde_json::to_vec(&h.val(t)).unwrap().len()))
        .collect();
    assert_eq!(h.val(TOPIC_ALERTS)["alerts"].as_array().unwrap().len(), 20);
    assert_eq!(h.val(TOPIC_SHIP)["modules"].as_array().unwrap().len(), 60);
    assert_eq!(h.val(TOPIC_GOALS)["goals"].as_array().unwrap().len(), 20);
    // Los topics de ingenieros y flujo son resúmenes: sin filas pesadas y de tamaño
    // acotado; las filas se sirven por páginas (cada una comprobada < 16 KiB en `pages`).
    let engineers = h.val(TOPIC_ENGINEERS);
    assert_no_heavy_fields(&engineers, &["engineers", "selected"]);
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "ok", "{flow}");
    assert_no_heavy_fields(&flow, &["lines", "cost", "options"]);
    let projected_engineers_size = serde_json::to_vec(&engineers).unwrap().len();
    let projected_flow_size = serde_json::to_vec(&flow).unwrap().len();
    assert!(!h.pages("engineers", "engineers").is_empty());
    assert!(!h.pages("flow", "lines").is_empty());
    eprintln!(
        "published topics: {sizes:?}; {actual_count} catalog materials; engineers summary: {projected_engineers_size} B; flow summary: {projected_flow_size} B"
    );
    for (topic, size) in &sizes {
        assert!(*size < 16384, "{topic}: {size} B ({sizes:?})");
    }
    assert!(
        projected_engineers_size < 1024,
        "{projected_engineers_size}"
    );
    assert!(projected_flow_size < 4096, "{projected_flow_size}");
}

// ----- Contrato Rust → UI -----

/// Escenario del contrato: la fixture en vivo con un comerciante falso, el FSD fijado
/// y la vista previa de los motores.
fn contract_snapshot() -> Value {
    let mut h = H::with_store();
    h.fixture();
    h.pin_fsd();
    let key = h.requests_with("trader:Encoded:")[0].key.clone();
    h.send(net(
        &key,
        NetOutcome::Stations(vec![station("Fake Port", 8.2, PadSize::Large)]),
    ));
    h.preview_engines();
    json!({
        "engineering.pages": {"goals":h.m.query("goals", &ModulePageQuery {revision:h.m.revision,cursor:0,section:None, engineer_id: None}).unwrap(), "locker_components":h.m.query("locker", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("components".into()), engineer_id: None}).unwrap(), "catalogue_recipes":h.m.query("catalogue", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("recipes".into()), engineer_id: None}).unwrap(), "engineers_capabilities":h.m.query("engineers", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("capabilities".into()), engineer_id: None}).unwrap(), "flow_costs":h.m.query("flow", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("costs".into()), engineer_id: None}).unwrap(), "flow_lines":h.m.query("flow", &ModulePageQuery {revision:h.m.revision,cursor:0,section:None, engineer_id: None}).unwrap(), "engineers_engineers":h.m.query("engineers", &ModulePageQuery {revision:h.m.revision,cursor:0,section:None, engineer_id: None}).unwrap(), "engineers_unlock":h.m.query("engineers", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("unlock".into()), engineer_id: None}).unwrap(), "flow_blueprints":h.m.query("flow", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("blueprints".into()), engineer_id: None}).unwrap(), "flow_experimentals":h.m.query("flow", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("experimentals".into()), engineer_id: None}).unwrap(), "flow_grades":h.m.query("flow", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("grades".into()), engineer_id: None}).unwrap()},
        "engineering.catalogue": h.val("engineering.catalogue"),
        "engineering.inventory": h.val("engineering.inventory"),
        "engineering.locker": h.val("engineering.locker"),
        "engineering.session": h.val("engineering.session"),
        "engineering.engineers": h.val("engineering.engineers"),
        "engineering.ship": h.val("engineering.ship"),
        "engineering.flow": h.val("engineering.flow"),
        "engineering.goals": h.val("engineering.goals"),
        "engineering.sources": h.val("engineering.sources"),
        "engineering.alerts": h.val("engineering.alerts"),
    })
}

/// El escenario vigente del contrato sigue cabiendo; las dos pruebas ignoradas
/// conservan los incumplimientos demostrados del peor caso para su decisión.
#[test]
fn fixture_topics_stay_under_16_kib() {
    let snapshot = contract_snapshot();
    assert_eq!(snapshot.as_object().unwrap().len(), 11);
    for (topic, value) in snapshot.as_object().unwrap() {
        if topic == "engineering.pages" {
            for (page_name, page_val) in value.as_object().unwrap() {
                let size = serde_json::to_vec(page_val).unwrap().len();
                assert!(size < 16384, "page {page_name}: {size} B");
            }
            continue;
        }
        let size = serde_json::to_vec(value).unwrap().len();
        assert!(size < 16384, "{topic}: {size} B");
    }
}

#[test]
fn contract_snapshot_has_the_values_the_ui_asserts() {
    let c = contract_snapshot();
    assert_eq!(
        line_of(&c["engineering.inventory"], "Raw", "1", "iron")["count"],
        json!(117)
    );
    assert_eq!(c["engineering.inventory"]["at_max"], json!(["polonium"]));
    assert_eq!(
        c["engineering.pages"]["locker_components"]["items"][0]["count"],
        json!(8)
    );
    let goal = &c["engineering.pages"]["goals"]["items"][0];
    assert_eq!(goal["completable"], json!(true));
    assert_eq!(goal["engineer"], "Felicity Farseer");
    assert_eq!(goal["trades"].as_array().unwrap().len(), 3);
    let missing3 = c["engineering.pages"]["flow_lines"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["state"] == "missing" && l["missing"] == json!(3));
    assert!(missing3);
    let ok = &c["engineering.sources"]["traders"]["Encoded"]["Ok"];
    assert_eq!(ok["station_name"], "Fake Port");
    assert_eq!(ok["max_pad"], "Large");
}

#[cfg(debug_assertions)]
#[test]
fn real_engineering_topics_preserve_public_diagnostic_fields() {
    use tmc_model::diagnostics::{DiagnosticContext, redact_diagnostic};
    let snapshot = contract_snapshot();
    let flow = redact_diagnostic(
        &snapshot[TOPIC_FLOW],
        DiagnosticContext::Topic { name: TOPIC_FLOW },
    );
    assert_eq!(flow["status"], "ok");
    assert_eq!(flow["request"]["blueprint"], "engine_dirty");
    assert_eq!(flow["request"]["experimental"], "special_engine_overloaded");
    assert_eq!(flow["module_type"], "engine");
    let inv = redact_diagnostic(
        &snapshot[TOPIC_INVENTORY],
        DiagnosticContext::Topic {
            name: TOPIC_INVENTORY,
        },
    );
    assert!(
        inv["by_category"]["Raw"]["1"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["count"] == 117)
    );
    assert!(
        inv["by_category"]["Raw"]["1"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["key"] == "[redacted]")
    );
    let session = redact_diagnostic(
        &snapshot[TOPIC_SESSION],
        DiagnosticContext::Topic {
            name: TOPIC_SESSION,
        },
    );
    assert_eq!(
        session["recent"][0]["origin"],
        snapshot[TOPIC_SESSION]["recent"][0]["origin"]
    );
}

#[test]
fn ui_contract_snapshot_is_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/tests/fixtures/engineering-topics.json");
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
        "el contrato JSON de engineering.* cambió: regenéralo con UPDATE_CONTRACTS=1 cargo test -p tmc-modules ui_contract_snapshot"
    );
}

/// Los eventos ajenos a la ingeniería (combate, exploración…) no recalculan ni republican nada.
#[test]
fn unrelated_journal_events_do_not_recompute() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    let before: Vec<Value> = TOPICS.iter().map(|t| h.val(t)).collect();
    for _ in 0..50 {
        let dirty = h.journal("Scan", r#","BodyName":"x""#);
        assert!(dirty.is_empty());
        h.journal("FuelScoop", r#","Scooped":1.0,"Total":20.0"#);
    }
    let after: Vec<Value> = TOPICS.iter().map(|t| h.val(t)).collect();
    assert_eq!(before, after);
}

#[test]
fn pagination_revisions_are_semantic_monotonic_and_queries_read_only() {
    let mut h = H::new();
    h.fixture();
    let revision = h.m.revision;
    let query = ModulePageQuery {
        revision,
        cursor: 0,
        section: None,
        engineer_id: None,
    };
    let before = h.m.values.clone();
    let relevance = h.m.relevance(&h.state);
    let first = h.m.query("goals", &query).unwrap();
    assert_eq!(first, h.m.query("goals", &query).unwrap());
    h.pages("locker", "components");
    assert_eq!(before, h.m.values);
    assert_eq!(relevance, h.m.relevance(&h.state));
    assert_eq!(revision, h.m.revision);
    // The timestamp of an otherwise identical snapshot does not invalidate pages.
    let snapshot = h.m.locker.view();
    let contents = json!({"Items":snapshot.items.iter().map(|r|json!({"Name":r.key,"Name_Localised":r.name_localised,"Count":r.count})).collect::<Vec<_>>(),"Components":snapshot.components.iter().map(|r|json!({"Name":r.key,"Name_Localised":r.name_localised,"Count":r.count})).collect::<Vec<_>>(),"Consumables":[],"Data":[]});
    let mut raw = contents.to_string();
    raw.remove(0);
    raw.pop();
    h.journal("ShipLocker", &format!(",{raw}"));
    assert_eq!(revision, h.m.revision);
    h.journal(
        "MaterialCollected",
        r#", "Category":"Raw","Name":"iron","Count":1"#,
    );
    assert!(h.m.revision > revision);
    assert_eq!(
        h.m.query("goals", &query),
        Err(ModuleQueryError::StaleRevision)
    );
    let before_engineer = h.m.revision;
    h.journal("EngineerProgress", r#", "Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":4,"RankProgress":50"#);
    assert!(h.m.revision > before_engineer);
    h.action(ACTION_SELECT_MODULE, json!({"slot":"MainEngines"}));
    h.action(
        ACTION_PREVIEW,
        json!({"blueprint":"engine_dirty","grade":5,"rolls":[1,1,1,1,4]}),
    );
    let before_goal = h.m.revision;
    h.action(ACTION_PIN_GOAL, json!({}));
    assert!(h.m.revision > before_goal);
    let before_switch = h.m.revision;
    h.journal("Commander", r#", "Name":"B","FID":"F2""#);
    assert!(h.m.revision > before_switch);
    let empty_b = h.m.revision;
    h.journal("Commander", r#", "Name":"CMDR TESTER","FID":"F1""#);
    assert!(h.m.revision > empty_b);
    let empty_a = h.m.revision;
    h.journal("Commander", r#", "Name":"B","FID":"F2""#);
    assert!(h.m.revision > empty_a);
    assert_eq!(
        h.m.query(
            "locker",
            &ModulePageQuery {
                revision: empty_b,
                cursor: 0,
                section: None,
                engineer_id: None,
            }
        ),
        Err(ModuleQueryError::StaleRevision)
    );
}

#[test]
fn pagination_validates_sections_cursor_and_preserves_escaped_utf8() {
    let mut h = H::new();
    h.fixture();
    let query = |cursor, section: &str| ModulePageQuery {
        revision: h.m.revision,
        cursor,
        section: Some(section.into()),
        engineer_id: None,
    };
    assert_eq!(
        h.m.query("missing", &query(0, "items")),
        Err(ModuleQueryError::NotFound)
    );
    assert_eq!(
        h.m.query("locker", &query(0, "CANARY")),
        Err(ModuleQueryError::InvalidQuery)
    );
    assert_eq!(
        h.m.query("locker", &query(usize::MAX, "items")),
        Err(ModuleQueryError::InvalidQuery)
    );
    let rows: Vec<Value> = (0..200)
        .map(|i| json!({"key":i,"name":"\u{e9}\u{6f22}\u{5b57}\"\n".repeat(10)}))
        .collect();
    let mut cursor = 0;
    let mut actual = Vec::new();
    let mut max_size = 0;
    loop {
        let page = pagination::page(
            42,
            "items",
            &rows,
            &ModulePageQuery {
                revision: 42,
                cursor,
                section: None,
                engineer_id: None,
            },
        )
        .unwrap();
        let bytes = serde_json::to_vec(&page).unwrap();
        max_size = max_size.max(bytes.len());
        assert!(bytes.len() < 16384);
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), page);
        actual.extend(page["items"].as_array().unwrap().iter().cloned());
        match page["next_cursor"].as_u64() {
            Some(next) => {
                assert!(next as usize > cursor);
                cursor = next as usize;
            }
            None => break,
        }
    }
    assert_eq!(actual, rows);
    assert_eq!(
        pagination::page(
            42,
            "items",
            &[json!({"name":"\u{6f22}".repeat(6000)})],
            &ModulePageQuery {
                revision: 42,
                cursor: 0,
                section: None,
                engineer_id: None,
            }
        ),
        Err(ModuleQueryError::ItemTooLarge)
    );
    let end = pagination::page(
        42,
        "items",
        &rows,
        &ModulePageQuery {
            revision: 42,
            cursor: rows.len(),
            section: None,
            engineer_id: None,
        },
    )
    .unwrap();
    assert_eq!(end["items"], json!([]));
    assert!(end["next_cursor"].is_null());
    eprintln!("largest escaped UTF8 page: {max_size} bytes");
}

/// `engineer_for_goal` resuelve el `Goal` real: el id es opaco, los experimentales no cuentan
/// y un objetivo ya conseguido (partida por encima del objetivo) no pide ingeniero.
#[test]
fn engineer_for_goal_resolves_the_real_goal() {
    let mut h = H::with_store();
    h.journal("Commander", r#","FID":"F1","Name":"CMDR TESTER""#);
    h.journal(
        "EngineerProgress",
        r#","Engineers":[{"Engineer":"Felicity Farseer","EngineerID":300100,"Progress":"Unlocked","Rank":5}]"#,
    );
    h.journal(
        "LoadGame",
        r#","Commander":"CMDR TESTER","Ship":"asp","ShipID":7,"GameMode":"Solo","Credits":10"#,
    );
    h.journal(
        "Loadout",
        r#","Ship":"asp","ShipID":7,"Modules":[{"Slot":"FrameShiftDrive","Item":"int_hyperdrive_size5_class5","On":true,"Priority":0,"Engineering":{"BlueprintName":"FSD_LongRange","Level":4,"Quality":1.0}}]"#,
    );
    h.pin_fsd();
    let real = h.m.goals[0].clone();
    assert_eq!(
        h.m.engineer_for_goal(&real.id).as_deref(),
        Some("Felicity Farseer")
    );
    // Un id que no es de ningún objetivo no se interpreta: sin ingeniero.
    assert_eq!(
        h.m.engineer_for_goal("7:FrameShiftDrive:fsd_longrange:-:5x"),
        None
    );
    assert_eq!(h.m.engineer_for_goal("garbage"), None);
    // Un experimental no altera la ruta de grados.
    let mut exp = real.clone();
    exp.request.experimental = Some("fsd_fastboot".into());
    exp.id = "exp".into();
    h.m.goals.push(exp);
    assert_eq!(
        h.m.engineer_for_goal("exp").as_deref(),
        Some("Felicity Farseer")
    );
    // Instalado G4 completo y objetivo G4: nada que tirar, nadie que recomendar.
    let mut done = real;
    done.request.target_grade = 4;
    done.id = "done".into();
    h.m.goals.push(done);
    assert_eq!(h.m.engineer_for_goal("done"), None);
}

#[test]
fn catalogue_pages_serve_every_static_row_flat() {
    use crate::data::{blueprints, engineers};
    let mut h = H::new();
    h.fixture();
    let cat = blueprints();
    let summary = h.val("engineering.catalogue");
    assert_eq!(summary["revision"], json!(h.m.revision));
    let recipes: usize = cat
        .blueprints
        .iter()
        .flat_map(|b| &b.recipes)
        .map(|r| r.grades.len())
        .sum();
    let expected = [
        ("module_types", cat.module_types.len()),
        ("blueprints", cat.blueprints.len()),
        ("recipes", recipes),
        ("experimentals", cat.experimentals.len()),
        ("variants", cat.variants.len()),
    ];
    for (section, n) in expected {
        assert_eq!(h.pages("catalogue", section).len(), n, "{section}");
        assert_eq!(summary["counts"][section], json!(n), "{section}");
    }
    assert_eq!(h.pages("catalogue", "aliases").len(), cat.aliases.len());
    let first = &h.pages("catalogue", "recipes")[0];
    for key in [
        "blueprint",
        "module_type",
        "grade",
        "materials",
        "merc_coins",
        "entry",
    ] {
        assert!(first.get(key).is_some(), "{key}: {first}");
    }
    let default =
        h.m.query(
            "catalogue",
            &ModulePageQuery {
                revision: h.m.revision,
                cursor: 0,
                section: None,
                engineer_id: None,
            },
        )
        .unwrap();
    assert_eq!(default["section"], "blueprints");
    assert!(!engineers().is_empty());
}

#[test]
fn engineer_pages_flatten_capabilities_and_unlock_steps() {
    use crate::data::engineers;
    let mut h = H::new();
    h.fixture();
    let defs = engineers();
    assert_eq!(h.pages("engineers", "engineers").len(), defs.len());
    let caps: usize = defs
        .iter()
        .flat_map(|e| &e.blueprints)
        .map(|b| b.grades.len())
        .sum();
    let capabilities = h.pages("engineers", "capabilities");
    assert_eq!(capabilities.len(), caps);
    for key in ["engineer_id", "module_type", "blueprint", "grade"] {
        assert!(capabilities[0].get(key).is_some(), "{key}");
    }
    let unlock = h.pages("engineers", "unlock");
    assert!(defs.iter().all(|e| {
        unlock
            .iter()
            .any(|r| r["engineer_id"] == json!(e.id) && r["field"] == "requirement")
    }));
    let before = h.m.revision;
    let summary = h.val("engineering.engineers");
    assert_eq!(summary["total"], json!(defs.len()));
    assert_eq!(summary["revision"], json!(before));
    h.action(ACTION_SELECT_ENGINEER, json!({"id": 300100}));
    assert_eq!(h.val("engineering.engineers")["selected_id"], json!(300100));
    assert_eq!(h.m.revision, before);
    let default =
        h.m.query(
            "engineers",
            &ModulePageQuery {
                revision: h.m.revision,
                cursor: 0,
                section: None,
                engineer_id: None,
            },
        )
        .unwrap();
    assert_eq!(default["section"], "engineers");
}

#[test]
fn flow_pages_expose_options_grades_costs_and_lines() {
    let mut h = H::new();
    h.fixture();
    for section in ["blueprints", "experimentals", "grades", "costs", "lines"] {
        assert!(h.pages("flow", section).is_empty(), "idle {section}");
    }
    let before = h.m.revision;
    h.preview_engines();
    assert!(h.m.revision > before);
    let flow = h.val("engineering.flow");
    assert_eq!(flow["revision"], json!(h.m.revision));
    assert_no_heavy_fields(&flow, &["lines", "cost", "options"]);
    let lines = h.pages("flow", "lines");
    assert_eq!(lines.len(), 13);
    assert_eq!(flow["counts"]["lines"], json!(lines.len()));
    let blueprints = h.pages("flow", "blueprints");
    assert!(blueprints.iter().any(|b| b["key"] == "engine_dirty"
        && b["available"] == json!(true)
        && b["grades"] == json!([1, 2, 3, 4, 5])));
    assert_eq!(h.pages("flow", "experimentals").len(), 5);
    let grades: Vec<u64> = h
        .pages("flow", "grades")
        .iter()
        .map(|g| g["grade"].as_u64().unwrap())
        .collect();
    assert_eq!(grades, vec![1, 2, 3, 4, 5]);
    let total: u64 = lines.iter().map(|l| l["need"].as_u64().unwrap()).sum();
    let costs = h.pages("flow", "costs");
    let materials: u64 = costs
        .iter()
        .filter(|c| c["kind"] == "material")
        .map(|c| c["count"].as_u64().unwrap())
        .sum();
    assert_eq!(materials, total);
    assert!(
        costs.iter().any(|c| c["grade"].is_null()),
        "experimental cost row"
    );
    let default =
        h.m.query(
            "flow",
            &ModulePageQuery {
                revision: h.m.revision,
                cursor: 0,
                section: None,
                engineer_id: None,
            },
        )
        .unwrap();
    assert_eq!(default["section"], "lines");
    // Changing the selection invalidates the previous revision.
    let old = h.m.revision;
    h.action(ACTION_CLOSE_FLOW, json!({}));
    assert!(h.m.revision > old);
    assert_eq!(
        h.m.query(
            "flow",
            &ModulePageQuery {
                revision: old,
                cursor: 0,
                section: None,
                engineer_id: None,
            }
        ),
        Err(ModuleQueryError::StaleRevision)
    );
}

#[test]
fn new_resources_validate_sections_and_queries_are_read_only() {
    let mut h = H::new();
    h.fixture();
    h.preview_engines();
    let q = |section: &str| ModulePageQuery {
        revision: h.m.revision,
        cursor: 0,
        section: Some(section.into()),
        engineer_id: None,
    };
    for (resource, section) in [
        ("catalogue", "goals"),
        ("catalogue", "lines"),
        ("engineers", "lines"),
        ("flow", "items"),
        ("flow", "CANARY"),
    ] {
        assert_eq!(
            h.m.query(resource, &q(section)),
            Err(ModuleQueryError::InvalidQuery),
            "{resource}/{section}"
        );
    }
    let values = h.m.values.clone();
    let revision = h.m.revision;
    for (resource, section) in [
        ("catalogue", "recipes"),
        ("engineers", "capabilities"),
        ("flow", "costs"),
    ] {
        h.pages(resource, section);
    }
    assert_eq!(values, h.m.values);
    assert_eq!(revision, h.m.revision);
}

/// El ingeniero recomendado es el mismo en el flujo, en el objetivo fijado y en el panel
/// de ingenieros (las tres vistas salen de la misma función, sin recalcular en la UI).
#[test]
fn recommended_engineer_matches_between_flow_goal_and_engineer_rows() {
    let mut h = H::new();
    h.fixture();
    h.pin_fsd();
    let flow = h.val(TOPIC_FLOW);
    assert_eq!(flow["status"], "ok", "{flow}");
    let goals = h.pages("goals", "goals");
    assert_eq!(goals.len(), 1);
    let name = goals[0]["engineer"].clone();
    assert_eq!(name, json!("Felicity Farseer"));
    assert_eq!(flow["engineer"]["nearest_unlocked"]["name"], name);
    let row = h
        .pages("engineers", "engineers")
        .into_iter()
        .find(|r| r["name"] == name)
        .expect("el ingeniero recomendado aparece en las filas");
    assert_eq!(row["id"], flow["engineer"]["nearest_unlocked"]["id"]);
    assert_eq!(row["status"], "Unlocked");
    let capable = h.pages("engineers", "capabilities").iter().any(|c| {
        c["engineer_id"] == row["id"] && c["blueprint"] == "fsd_longrange" && c["grade"] == 5
    });
    assert!(capable);
}

#[test]
fn engineer_id_filter_returns_only_that_engineer_and_is_bounded() {
    let mut h = H::new();
    h.fixture();
    let ask = |h: &H, resource: &str, section: &str, id: Option<u64>| {
        h.m.query(
            resource,
            &ModulePageQuery {
                revision: h.m.revision,
                cursor: 0,
                section: Some(section.into()),
                engineer_id: id,
            },
        )
    };
    let all = h.pages("engineers", "capabilities");
    let one = ask(&h, "engineers", "capabilities", Some(300100)).unwrap();
    let rows = one["items"].as_array().unwrap();
    assert!(!rows.is_empty() && rows.len() < all.len());
    assert!(rows.iter().all(|r| r["engineer_id"] == json!(300100)));
    assert_eq!(
        rows.len(),
        all.iter()
            .filter(|r| r["engineer_id"] == json!(300100))
            .count()
    );
    let named = ask(&h, "engineers", "engineers", Some(300160)).unwrap();
    assert_eq!(named["total"], json!(1));
    assert_eq!(named["items"][0]["id"], json!(300160));
    // Un id desconocido da una lista vacía, no un error.
    let none = ask(&h, "engineers", "capabilities", Some(1)).unwrap();
    assert_eq!(
        (none["total"].clone(), none["items"].clone()),
        (json!(0), json!([]))
    );
    // Fuera de los recursos de ingenieros el filtro es inválido; la revisión se sigue validando.
    assert_eq!(
        ask(&h, "flow", "lines", Some(300100)).unwrap_err(),
        ModuleQueryError::InvalidQuery
    );
    let stale = ModulePageQuery {
        revision: h.m.revision + 1,
        cursor: 0,
        section: Some("capabilities".into()),
        engineer_id: Some(300100),
    };
    assert_eq!(
        h.m.query("engineers", &stale).unwrap_err(),
        ModuleQueryError::StaleRevision
    );
}

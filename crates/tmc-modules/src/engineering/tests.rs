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

fn flow_line<'a>(flow: &'a Value, key: &str) -> &'a Value {
    flow["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["key"] == key)
        .unwrap_or_else(|| panic!("línea {key}"))
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
    assert_eq!(h.val(TOPIC_FLOW)["cost"], Value::Null);
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
    let rows = e["engineers"].as_array().unwrap();
    let f = rows.iter().find(|r| r["id"] == 300100).unwrap();
    assert_eq!(f["status"], "Unlocked");
    assert_eq!(f["rank"], json!(5));
    let d = f["distance_ly"].as_f64().unwrap();
    assert!((d - 12.3).abs() < 1e-6, "{d}");
    assert!(e["selected"].is_null());
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
    let sel = h.val("engineering.engineers")["selected"].clone();
    assert_eq!(sel["id"], json!(300260));
    assert!(sel["next_step"]["Requirement"]["key"].is_string());
    h.action("engineering.select_engineer", json!({"id": 300100}));
    let sel = h.val("engineering.engineers")["selected"].clone();
    assert_eq!(sel["next_step"], json!("Done"));
    assert_eq!(sel["blueprints"][0]["blueprint"], "engine_dirty");
    assert_eq!(sel["blueprints"][0]["max_grade"], json!(3));
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
    assert_eq!(flow["lines"].as_array().unwrap().len(), 13);
    // 9 de 19 unidades cubiertas (reservas): 9 × 100 / 19.
    assert_eq!(flow["percent"], json!(47));
    let polonium = flow_line(&flow, "polonium");
    assert_eq!(
        (polonium["need"].clone(), polonium["have"].clone()),
        (json!(4), json!(150))
    );
    assert_eq!(polonium["state"], "ok");
    let heat = flow_line(&flow, "protoheatradiators");
    assert_eq!(heat["state"], "missing");
    assert_eq!(heat["missing"], json!(2));
    assert_eq!(heat["need"], json!(4));
    assert_eq!(heat["have"], json!(2));
    // Felicity solo llega a G3 en engine_dirty: el G5 lo hace el Profesor Palin (sin desbloquear).
    assert!(flow["engineer"]["nearest_unlocked"].is_null());
    assert_eq!(flow["engineer"]["nearest_any"]["id"], json!(300220));
    assert_eq!(
        flow["engineer"]["next_step"]["Requirement"]["key"],
        "unlock-professor-palin"
    );
    assert_eq!(flow["cost"]["total"]["polonium"], json!(4));
    assert_eq!(flow["options"]["blueprints"], json!(["engine_dirty"]));
    // Cerrar el flujo lo vacía.
    h.action("engineering.close_flow", json!({}));
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "idle");
    assert!(flow["slot"].is_null());
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
    assert!(flow["cost"].is_null());
    assert_eq!(flow["lines"], json!([]));
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
    let rows = h.val("engineering.engineers");
    let f = rows["engineers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == 300100)
        .unwrap()
        .clone();
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
        r#","Raw":[],"Manufactured":[{"Name":"biotechconductors","Count":4}],"Encoded":[{"Name":"dataminedwake","Count":3}]"#,
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
        let rows = h.val("engineering.engineers");
        assert!(
            rows["engineers"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["distance_ly"].is_null()),
            "{ev}"
        );
    }
    // Con posición válida sí hay distancias.
    h.journal(
        "FSDJump",
        r#","StarSystem":"Somewhere","SystemAddress":10,"StarPos":[0,0,0]"#,
    );
    let rows = h.val("engineering.engineers");
    assert!(
        rows["engineers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["distance_ly"].is_number())
    );
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
    let step = h.val("engineering.engineers")["selected"]["next_step"].clone();
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
            let prefix = &blueprints()
                .module_types
                .iter()
                .find(|t| t.id == req.module_type)
                .unwrap()
                .item_prefixes[0];
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
    assert_eq!(max_trades, 215);
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
    // Cuarenta filas completas, con rangos/distancias y nombres largos, conservando
    // el envoltorio real y el siguiente paso del ingeniero seleccionado.
    let mut engineers = h.val(TOPIC_ENGINEERS);
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(json!(engineers::EngineerRow {
            id: 900000 + i,
            name: format!("Synthetic Engineer Long Name {i}"),
            system: format!("Synthetic Engineer Home System {i}"),
            distance_ly: Some(12345.123456789),
            status: EngineerStatus::Unlocked,
            rank: Some(5),
            rank_progress: Some(100),
            on_foot: i % 2 == 0,
            verified: false,
        }));
    }
    engineers["engineers"] = json!(rows);
    assert_eq!(engineers["engineers"].as_array().unwrap().len(), 40);
    let projected_engineers_size = serde_json::to_vec(&engineers).unwrap().len();
    let flow = h.val("engineering.flow");
    assert_eq!(flow["status"], "ok", "{flow}");
    // Treinta líneas con propuestas completas (no la primera línea sin intercambio).
    let mut projected_flow = flow.clone();
    let lines: Vec<FlowLine> = (0..30)
        .map(|i| FlowLine {
            key: format!("long_material_key_{i}"),
            name_en: Some(format!("Long Material Display Name {i}")),
            grade: 5,
            need: 40,
            have: 0,
            state: "trade",
            missing: 0,
            trade: Some(TradeProposal {
                give_key: "long_surplus_material_key".into(),
                give: 240,
                get_key: format!("long_material_key_{i}"),
                get: 40,
                trader: TraderKind::Manufactured,
                same_family: false,
            }),
        })
        .collect();
    projected_flow["lines"] = json!(lines);
    assert_eq!(projected_flow["lines"].as_array().unwrap().len(), 30);
    let projected_flow_size = serde_json::to_vec(&projected_flow).unwrap().len();
    eprintln!(
        "published topics: {sizes:?}; {actual_count} catalog materials; 40 engineers: {projected_engineers_size} B; 30 trade lines: {projected_flow_size} B"
    );
    for (topic, size) in &sizes {
        assert!(*size < 16384, "{topic}: {size} B ({sizes:?})");
    }
    assert!(projected_engineers_size < 16384);
    assert!(projected_flow_size < 16384);
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
        "engineering.pages": {"goals":h.m.query("goals", &ModulePageQuery {revision:h.m.revision,cursor:0,section:None}).unwrap(), "locker_components":h.m.query("locker", &ModulePageQuery {revision:h.m.revision,cursor:0,section:Some("components".into())}).unwrap()},
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
    assert_eq!(snapshot.as_object().unwrap().len(), 10);
    for (topic, value) in snapshot.as_object().unwrap() {
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
    assert_eq!(goal["trades"][0]["give"], json!(6));
    assert_eq!(goal["trades"][0]["get"], json!(1));
    assert_eq!(goal["trades"].as_array().unwrap().len(), 1);
    let missing2 = c["engineering.flow"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["state"] == "missing" && l["missing"] == json!(2));
    assert!(missing2);
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
    assert_eq!(
        flow["lines"][0]["state"],
        snapshot[TOPIC_FLOW]["lines"][0]["state"]
    );
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
                section: None
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
                section: None
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
        },
    )
    .unwrap();
    assert_eq!(end["items"], json!([]));
    assert!(end["next_cursor"].is_null());
    eprintln!("largest escaped UTF8 page: {max_size} bytes");
}

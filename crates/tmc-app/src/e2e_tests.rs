//! Pruebas de extremo a extremo con `start()` (F1, criterios 2 y 5).

use crate::{
    Config, Running, ServerSection, start, start_custom, start_full, start_with_transport,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tmc_core::CoreMessage;
use tmc_net::{BoxFut, HttpReply, Transport, TransportError};

/// Serializa las pruebas e2e del archivo: comparten el interruptor global de «juego en marcha».
static E2E_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const ENGINEERING_FIXTURE: &str =
    include_str!("../../../tests/fixtures/journal/engineering_session.log");

#[test]
fn every_line_of_f4_fixture_is_typed() {
    use tmc_model::events::{EventKind, parse_line};
    let mut other = Vec::new();
    for (index, line) in ENGINEERING_FIXTURE.lines().enumerate() {
        let event = parse_line(line).unwrap_or_else(|e| panic!("line {}: {e:?}", index + 1));
        if matches!(event.kind, EventKind::Other) {
            other.push(index + 1);
        }
    }
    const EXPECTED_OTHER: &[usize] = &[];
    assert_eq!(other, EXPECTED_OTHER);
    assert_eq!(ENGINEERING_FIXTURE.lines().count(), 21);
}

#[test]
fn fixture_inventory_criterion_2() {
    use tmc_model::events::{MaterialCategory, parse_line};
    use tmc_modules::engineering::inventory::{ChangeOrigin, MaterialInventory};
    let mut inventory = MaterialInventory::default();
    let mut changes = Vec::new();
    for line in ENGINEERING_FIXTURE.lines() {
        let event = parse_line(line).unwrap();
        changes.extend(inventory.apply_event(&event.kind, event.timestamp));
    }
    // Snapshot + collect - trade - synthesis + mission - craft, calculated by hand.
    use MaterialCategory::{Encoded, Manufactured, Raw};
    let expected = BTreeMap::from([
        ((Raw, 1, "iron"), 117),
        ((Raw, 4, "polonium"), 150),
        ((Raw, 1, "carbon"), 1),
        ((Manufactured, 3, "conductiveceramics"), 26),
        ((Manufactured, 5, "pharmaceuticalisolators"), 8),
        ((Manufactured, 4, "heatvanes"), 3),
        ((Manufactured, 5, "protoheatradiators"), 2),
        ((Manufactured, 5, "biotechconductors"), 4),
        ((Encoded, 4, "shieldpatternanalysis"), 14),
        ((Encoded, 0, "foo_new"), 4),
        ((Encoded, 5, "dataminedwake"), 3),
        ((Encoded, 4, "hyperspacetrajectories"), 6),
    ]);
    let rows = inventory.by_category();
    let actual: BTreeMap<_, _> = rows
        .iter()
        .flat_map(|(category, grades)| {
            grades.iter().flat_map(move |(grade, rows)| {
                rows.iter()
                    .map(move |row| ((*category, *grade, row.key.as_str()), row.count))
            })
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(
        rows[&tmc_model::events::MaterialCategory::Raw][&1][1].key,
        "iron"
    );
    assert_eq!(
        rows[&tmc_model::events::MaterialCategory::Encoded][&0][0].key,
        "foo_new"
    );
    assert_eq!(inventory.at_max(), ["polonium"]);
    let overflow = changes.iter().find(|c| c.key == "polonium").unwrap();
    assert_eq!(
        (overflow.delta, overflow.lost, overflow.origin),
        (0, 2, ChangeOrigin::Mining)
    );
}

#[test]
fn fixture_engineers_criterion_3() {
    use tmc_model::events::{EngineerStatus, parse_line};
    use tmc_modules::engineering::engineers::EngineerBook;
    let mut book = EngineerBook::default();
    for line in ENGINEERING_FIXTURE.lines() {
        book.apply_journal(&parse_line(line).unwrap().kind, line);
    }
    assert_eq!(book.position(), Some([134.925, -0.8125, -47.28125]));
    let rows = book.rows();
    let farseer = rows.iter().find(|r| r.id == 300100).unwrap();
    assert_eq!(
        (farseer.status, farseer.rank, farseer.rank_progress),
        (EngineerStatus::Unlocked, Some(5), Some(0))
    );
    assert!((farseer.distance_ly.unwrap() - 12.3).abs() < 0.000001);
    assert_eq!(
        book.status(300160).unwrap().progress,
        EngineerStatus::Unlocked
    );
    assert_eq!(book.status(300260).unwrap().progress, EngineerStatus::Known);
}

#[test]
fn engineering_inventory_parked_journal_edges() {
    use tmc_model::events::{EventKind, parse_line};
    use tmc_modules::engineering::inventory::{ChangeOrigin, MaterialInventory};
    let mut inventory = MaterialInventory::default();
    let mut apply = |body: &str| {
        let event =
            parse_line(&format!(r#"{{"timestamp":"2026-10-21T12:00:00Z",{body}}}"#)).unwrap();
        assert!(!matches!(event.kind, EventKind::Other));
        inventory.apply_event(&event.kind, event.timestamp)
    };
    apply(r#""event":"Materials","Raw":[{"Name":"iron","Count":2},{"Name":"iron","Count":3}]"#);
    let discarded =
        apply(r#""event":"MaterialDiscarded","Name":"iron","Category":"Raw","Count":1"#);
    assert_eq!(
        (discarded[0].delta, discarded[0].origin),
        (-1, ChangeOrigin::Discarded)
    );
    let donation = apply(
        r#""event":"EngineerContribution","Engineer":"Synthetic Engineer","Type":"Material","Material":"iron","Quantity":2"#,
    );
    assert_eq!(
        (donation[0].delta, donation[0].origin),
        (-2, ChangeOrigin::Engineering)
    );
    let alias = apply(
        r#""event":"Synthesis","Name":"Synthetic recipe","Materials":[{"Material":"iron","Count":1}]"#,
    );
    assert_eq!((alias[0].key.as_str(), alias[0].delta), ("iron", -1));
    assert_eq!(inventory.count("iron"), 1);
}

#[test]
fn engineering_trade_parked_rounding_and_caps() {
    use tmc_modules::{
        data::{TraderRules, materials},
        engineering::trades::{Rate, plan_trades, rate},
    };
    let rules = TraderRules::default();
    // Different families, down two grades: 6:9 reduces to 2:3.
    assert_eq!(
        rate(
            &materials()["pharmaceuticalisolators"],
            &materials()["heatexchangers"],
            &rules
        ),
        Some(Rate { give: 2, get: 3 })
    );
    let missing = BTreeMap::from([("carbon".into(), 1)]);
    let surplus = BTreeMap::from([("vanadium".into(), 1)]);
    let overshoot = plan_trades(&missing, &surplus, &|_| 0, &rules);
    assert_eq!(
        (overshoot.proposals[0].give, overshoot.proposals[0].get),
        (1, 3)
    );
    assert!(overshoot.uncovered.is_empty());
    // Only two spaces remain; a 1:3 lot cannot fit and must never overfill.
    let capped = plan_trades(&missing, &surplus, &|_| 298, &rules);
    assert!(capped.proposals.is_empty());
    assert_eq!(capped.uncovered, missing);
}

#[test]
fn engineering_plan_parked_quality_and_experimental() {
    use tmc_modules::{
        data::blueprints,
        engineering::{
            plan::{PlanRequest, plan_cost, start_grade},
            ship::CurrentEng,
        },
    };
    let mut current = CurrentEng {
        blueprint: "engine_dirty".into(),
        level: 3,
        quality: 0.998,
        experimental: Some("another_experimental".into()),
    };
    let request = PlanRequest {
        module_type: "engine".into(),
        blueprint: "engine_dirty".into(),
        experimental: Some("special_engine_overloaded".into()),
        target_grade: 5,
        rolls: [1, 1, 1, 1, 4],
    };
    assert_eq!(
        (
            start_grade(Some(&current), "engine_dirty", &request.rolls).grade,
            start_grade(Some(&current), "engine_dirty", &request.rolls).partial_rolls
        ),
        (3, Some(1))
    );
    current.quality = 0.999;
    assert_eq!(
        start_grade(Some(&current), "engine_dirty", &request.rolls).grade,
        4
    );
    let catalog = blueprints();
    let cost = plan_cost(catalog, "int_engine_size5_class5", Some(&current), &request);
    assert_eq!(cost.per_grade.keys().copied().collect::<Vec<_>>(), [4, 5]);
    assert_eq!(cost.experimental.unwrap().values().sum::<u32>(), 3);
}

/// Simula el proceso del juego mientras vive y restaura el valor anterior al soltarse.
struct GameRunning(bool);
impl GameRunning {
    fn on() -> Self {
        Self(tmc_journal::process::force_game_running_for_e2e(true))
    }
}
impl Drop for GameRunning {
    fn drop(&mut self) {
        tmc_journal::process::force_game_running_for_e2e(self.0);
    }
}

const HEAD: &str = concat!(
    r#"{"timestamp":"2026-10-18T10:00:00Z","event":"Fileheader","part":1,"language":"English/UK","Odyssey":true,"gameversion":"4.1.0.0","build":"r1"}"#,
    "\n",
    r#"{"timestamp":"2026-10-18T10:00:01Z","event":"Commander","FID":"F0000001","Name":"CMDR TESTER"}"#,
    "\n",
    r#"{"timestamp":"2026-10-18T10:00:02Z","event":"LoadGame","FID":"F0000001","Commander":"CMDR TESTER","Ship":"python","ShipID":7,"FuelLevel":20.0,"FuelCapacity":32.0,"GameMode":"Solo","Credits":1,"Loan":0}"#,
    "\n",
    r#"{"timestamp":"2026-10-18T10:00:03Z","event":"Location","StarSystem":"Start","SystemAddress":100,"StarPos":[0,0,0],"Docked":false}"#,
    "\n",
);

fn config(jdir: &Path, ddir: &Path, online: bool) -> Config {
    Config {
        journal_dir: Some(jdir.to_path_buf()),
        data_dir: Some(ddir.to_path_buf()),
        server: ServerSection {
            port: 0,
            ..Default::default()
        },
        auto_copy_next_system: false,
        online_services: online,
        ..Default::default()
    }
}

fn append(path: &Path, text: &str) {
    let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(f, "{text}").unwrap();
}

async fn wait_for<F: Fn(&BTreeMap<String, Value>) -> bool>(
    running: &Running,
    what: &str,
    cond: F,
) -> BTreeMap<String, Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let s = running.core.snapshot().await;
        if cond(&s) {
            return s;
        }
        assert!(
            Instant::now() < deadline,
            "timeout waiting for {what}: {}",
            serde_json::to_string(&s).unwrap_or_default()
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn route_strip_updates_within_500ms_p95() {
    let _serial = E2E_LOCK.lock().await;
    let (jdir, ddir) = (tempdir().unwrap(), tempdir().unwrap());
    let journal = jdir.path().join("Journal.2026-10-18T100000.01.log");
    std::fs::write(&journal, HEAD).unwrap();
    // Emula «juego en marcha»: sondeo activo (250 ms) también sin proceso del juego.
    let running = start_custom(
        config(jdir.path(), ddir.path(), false),
        Arc::new(FakeTransport),
        Some(Duration::from_millis(250)),
    )
    .await
    .unwrap();
    wait_for(&running, "start", |s| {
        s.get("navigation.route")
            .is_some_and(|r| r["current"]["name"] == "Start")
    })
    .await;
    let mut rx = running.core.subscribe();

    let mut latencies = Vec::new();
    for i in 1..=20u32 {
        let name = format!("Sys {i}");
        let line = format!(
            r#"{{"timestamp":"2026-10-18T10:{:02}:00Z","event":"FSDJump","StarSystem":"{name}","SystemAddress":{},"StarPos":[{i},0,0],"JumpDist":1.0}}"#,
            i,
            1000 + i
        );
        let t0 = Instant::now();
        append(&journal, &line);
        let route = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(CoreMessage::Update { topics, .. }) = rx.recv().await
                    && let Some(r) = topics.get("navigation.route")
                    && r["current"]["name"] == name.as_str()
                {
                    return r.clone();
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("no navigation.route update for jump {i}"));
        latencies.push(t0.elapsed());
        assert_eq!(route["current"]["remote"]["status"], "disabled");
    }
    latencies.sort();
    // 20 muestras: el percentil 95 (nearest-rank) es la 19.ª, índice 18.
    let p95 = latencies[18];
    eprintln!("route strip p95 = {p95:?}, max = {:?}", latencies[19]);
    assert!(p95 < Duration::from_millis(500), "p95 = {p95:?}");
    running.server.shutdown();
}

struct FakeTransport;
impl Transport for FakeTransport {
    fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        Box::pin(async {
            Ok(HttpReply {
                status: 200,
                body: include_str!("../../../tests/fixtures/net/spansh_system_ok.json").to_owned(),
            })
        })
    }
    fn post_form(
        &self,
        _url: &str,
        _form: &[(String, String)],
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.get("")
    }
}

#[tokio::test]
async fn strip_shows_remote_when_fake_service_answers() {
    let _serial = E2E_LOCK.lock().await;
    let (jdir, ddir) = (tempdir().unwrap(), tempdir().unwrap());
    let journal = jdir.path().join("Journal.2026-10-18T100000.01.log");
    std::fs::write(&journal, HEAD).unwrap();
    let running = start_with_transport(
        config(jdir.path(), ddir.path(), true),
        Arc::new(FakeTransport),
    )
    .await
    .unwrap();
    wait_for(&running, "start", |s| s.contains_key("navigation.route")).await;
    append(
        &journal,
        r#"{"timestamp":"2026-10-18T10:01:00Z","event":"FSDJump","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0,0,0],"JumpDist":1.0}"#,
    );
    let s = wait_for(&running, "remote ok", |s| {
        s.get("navigation.route").is_some_and(|r| {
            r["current"]["name"] == "Sol" && r["current"]["remote"]["status"] == "ok"
        })
    })
    .await;
    let remote = &s["navigation.route"]["current"]["remote"];
    assert_eq!(remote["data"]["name"], "Sol");
    assert_eq!(remote["data"]["id64"], 10477373803u64);
    running.server.shutdown();
}

fn tp<'a>(s: &'a BTreeMap<String, Value>, key: &str) -> &'a Value {
    s.get(key).unwrap_or(&Value::Null)
}

fn haversine_m(a: (f64, f64), b: (f64, f64), r: f64) -> f64 {
    let (p1, p2) = (a.0.to_radians(), b.0.to_radians());
    let dp = p2 - p1;
    let dl = (b.1 - a.1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

#[tokio::test]
async fn exobiology_fixture_end_to_end() {
    let _serial = E2E_LOCK.lock().await;
    // Desviación (plan T30): fixture sintético hasta tener una sesión real de exobiología.
    let log = include_str!("../../../tests/fixtures/journal/exobiology_session.log");
    let lines: Vec<&str> = log.lines().collect();
    // Los Status de los fixtures llevan Flags 0; el juego marca LANDED (bit 1) en tierra.
    let statuses = [
        include_str!("../../../tests/fixtures/status/exo_01.json"),
        include_str!("../../../tests/fixtures/status/exo_02.json"),
        include_str!("../../../tests/fixtures/status/exo_03.json"),
    ]
    .map(|t| t.replace(r#""Flags":0"#, r#""Flags":2"#));
    let (jdir, ddir) = (tempdir().unwrap(), tempdir().unwrap());
    let journal = jdir.path().join("Journal.2026-04-12T210000.01.log");
    // Hasta Disembark (índice 6) se reproduce; el resto entra en vivo.
    std::fs::write(&journal, format!("{}\n", lines[..7].join("\n"))).unwrap();
    let status_path = jdir.path().join("Status.json");
    std::fs::write(&status_path, &statuses[0]).unwrap();
    let running = start(config(jdir.path(), ddir.path(), false))
        .await
        .unwrap();

    let s = wait_for(&running, "panel active", |s| {
        s.get("core.modules").is_some_and(|m| {
            m.as_array().is_some_and(|a| {
                a.iter()
                    .any(|e| e["id"] == "exobiology" && e["relevance"] == "active")
            })
        })
    })
    .await;
    let body = &s["exobiology.body"];
    assert_eq!(body["body"], "Kappa Testis A 5 a");
    assert_eq!(body["bio_signals"], 3);
    assert!(!body["candidates"].as_array().unwrap().is_empty());

    let radius = 1_500_000.0;
    let positions = [(10.0, 20.0), (10.0, 20.5), (10.0, 21.0)];
    wait_for(&running, "status1", |s| {
        tp(s, "core.status")["Latitude"] == 10.0 && tp(s, "core.status")["Longitude"] == 20.0
    })
    .await;
    append(&journal, lines[7]);
    wait_for(&running, "log", |s| {
        tp(s, "exobiology.sampling")["points"] == 1
    })
    .await;
    std::fs::write(&status_path, &statuses[1]).unwrap();
    let expected = haversine_m(positions[0], positions[1], radius);
    wait_for(&running, "dist1", |s| {
        tp(s, "exobiology.sampling")["distance_m"]
            .as_f64()
            .is_some_and(|d| (d - expected).abs() <= 1.0)
    })
    .await;
    append(&journal, lines[8]);
    wait_for(&running, "sample", |s| {
        tp(s, "exobiology.sampling")["points"] == 2
    })
    .await;
    std::fs::write(&status_path, &statuses[2]).unwrap();
    let expected = haversine_m(positions[1], positions[2], radius);
    let s = wait_for(&running, "dist2", |s| {
        tp(s, "exobiology.sampling")["distance_m"]
            .as_f64()
            .is_some_and(|d| (d - expected).abs() <= 1.0)
    })
    .await;
    assert_eq!(
        tp(&s, "exobiology.sampling")["species"],
        "$Codex_Ent_Bacterial_01_Name;"
    );
    append(&journal, lines[9]);
    let s = wait_for(&running, "complete", |s| {
        tp(s, "exobiology.unsold")["species"] == 1
    })
    .await;
    assert_eq!(tp(&s, "exobiology.unsold")["total"], 5_000_000);
    let sold: Value = serde_json::from_str(lines[11]).unwrap();
    let sold_total: u64 = sold["BioData"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["Value"].as_u64().unwrap() + b["Bonus"].as_u64().unwrap())
        .sum();
    assert_eq!(sold_total, 5_000_000);
    append(&journal, lines[10]);
    append(&journal, lines[11]);
    let s = wait_for(&running, "sold", |s| {
        tp(s, "exobiology.unsold")["species"] == 0
    })
    .await;
    assert_eq!(tp(&s, "exobiology.unsold")["total"], 0);
    running.server.shutdown();
}

#[tokio::test]
async fn real_exploration_fixture_replays_end_to_end() {
    let _serial = E2E_LOCK.lock().await;
    let log = include_str!("../../../tests/fixtures/journal/real_explo_01.log");
    let jumps = log
        .lines()
        .filter(|l| l.contains(r#""event":"FSDJump""#))
        .count();
    assert_eq!(jumps, 6);
    let (jdir, ddir) = (tempdir().unwrap(), tempdir().unwrap());
    std::fs::write(jdir.path().join("Journal.2026-10-18T014025.01.log"), log).unwrap();
    let running = start(config(jdir.path(), ddir.path(), false))
        .await
        .unwrap();
    let s = wait_for(&running, "replay", |s| {
        tp(s, "core.location")["system"] != ""
            && tp(s, "core.game").is_object()
            && tp(s, "core.session")["jumps"] == 6
    })
    .await;
    assert_eq!(tp(&s, "core.location")["system"], "Crucis Sector GH-V b2-6");
    assert_eq!(tp(&s, "core.location")["system_address"], 13865362466233u64);
    assert_eq!(tp(&s, "core.session")["jumps"], 6);
    let route = tp(&s, "navigation.route");
    assert_eq!(route["current"]["name"], "Crucis Sector GH-V b2-6");
    assert_eq!(route["current"]["visits"], 1);
    assert_eq!(route["current"]["remote"]["status"], "disabled");
    // La venta final (340 774 cr) deja las exploraciones sin vender a cero.
    assert_eq!(
        *tp(&s, "exploration.unsold"),
        serde_json::json!({"at_risk": false, "systems": 0, "total": 0})
    );
    running.server.shutdown();
}

#[tokio::test]
async fn burst_of_1000_combat_events_stays_under_10hz_and_500ms() {
    let _serial = E2E_LOCK.lock().await;
    let (jdir, ddir) = (tempdir().unwrap(), tempdir().unwrap());
    let journal = jdir.path().join("Journal.2026-10-18T100000.01.log");
    std::fs::write(&journal, HEAD).unwrap();
    let running = start_custom(
        config(jdir.path(), ddir.path(), false),
        Arc::new(FakeTransport),
        Some(Duration::from_millis(250)),
    )
    .await
    .unwrap();
    wait_for(&running, "start", |s| s.contains_key("combat.ship")).await;
    let mut rx = running.core.subscribe();

    // 20 ráfagas de ~100 eventos (ShipTargeted y UnderAttack alternados). Solo el último
    // evento de cada ráfaga lleva un casco único (50 + n), que identifica la ráfaga.
    const BURSTS: u32 = 20;
    let mut latencies = Vec::new();
    let mut arrivals = Vec::new();
    for burst in 0..BURSTS {
        let marker = 50.0 + f64::from(burst);
        let mut text = String::new();
        for i in 0..50u32 {
            let ts = format!("2026-10-18T10:{:02}:{:02}Z", 10 + burst, i);
            let hull = 10.0 + f64::from(i % 7);
            text.push_str(&format!(
                r#"{{"timestamp":"{ts}","event":"ShipTargeted","TargetLocked":true,"Ship":"anaconda","ScanStage":3,"LegalStatus":"Wanted","Bounty":42000,"HullHealth":{hull}}}"#
            ));
            text.push('\n');
            text.push_str(&format!(
                r#"{{"timestamp":"{ts}","event":"UnderAttack","Target":"You"}}"#
            ));
            text.push('\n');
        }
        text.push_str(&format!(
            r#"{{"timestamp":"2026-10-18T10:{:02}:59Z","event":"ShipTargeted","TargetLocked":true,"Ship":"anaconda","ScanStage":3,"LegalStatus":"Wanted","Bounty":42000,"HullHealth":{marker}}}"#,
            10 + burst
        ));
        text.push('\n');
        let t0 = Instant::now();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(CoreMessage::Update { topics, .. }) = rx.recv().await {
                    arrivals.push(Instant::now());
                    if let Some(t) = topics.get("combat.target")
                        && t["hull_pct"] == marker
                    {
                        return;
                    }
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("burst {burst} not reflected"));
        latencies.push(t0.elapsed());
    }
    latencies.sort();
    // 20 muestras: p95 por rango más cercano = la 19.ª (índice 18).
    let p95 = latencies[18];
    eprintln!("combat burst p95 = {p95:?}, updates = {}", arrivals.len());
    assert!(p95 < Duration::from_millis(500), "p95 = {p95:?}");
    // El difusor emite como mucho 10/s; se mide por hora de recepción en el receptor, donde
    // la planificación de tokio puede agrupar un envío extra en el borde de la ventana: se
    // permite 1 de margen (≤ 11 por ventana deslizante de 1 s).
    for (i, a) in arrivals.iter().enumerate() {
        let n = arrivals[i..]
            .iter()
            .take_while(|b| b.duration_since(*a) < Duration::from_secs(1))
            .count();
        assert!(n <= 11, "{n} difusiones en 1 s");
    }
    running.server.shutdown();
}

/// Relay EDDN falso en proceso: re-publica `burst.jsonl` (500 `commodity/3` + 20 corruptos) cada segundo.
async fn spawn_fake_relay(
    stop: Arc<std::sync::atomic::AtomicBool>,
) -> (String, tokio::task::JoinHandle<()>) {
    use std::io::Write as _;
    use zeromq::{PubSocket, Socket, SocketSend, ZmqMessage};
    let mut sock = PubSocket::new();
    let ep = sock.bind("tcp://127.0.0.1:0").await.unwrap();
    let zeromq::Endpoint::Tcp(_, port) = ep else {
        panic!("endpoint inesperado")
    };
    let text = include_str!("../../../tests/fixtures/eddn/burst.jsonl");
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        // Termina siempre al final de una pasada completa (el recuento de corruptos es múltiplo de 20).
        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let mut e =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(line.replace("@NOW@", &now).as_bytes()).unwrap();
                if sock
                    .send(ZmqMessage::from(e.finish().unwrap()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    (format!("tcp://127.0.0.1:{port}"), handle)
}

#[tokio::test]
async fn burst_of_500_eddn_markets_keeps_10hz_and_one_requery() {
    let _serial = E2E_LOCK.lock().await;
    let (jdir, ddir) = (tempdir().unwrap(), tempdir().unwrap());
    let journal = jdir.path().join("Journal.2026-10-19T120000.01.log");
    // Se empieza con la cabecera y el resto de la fixture llega en vivo (si no, sería reproducción).
    let fixture = include_str!("../../../tests/fixtures/journal/trade_session.log");
    let mut lines = fixture.lines();
    std::fs::write(
        &journal,
        format!(
            "{}
{}
",
            HEAD.lines().next().unwrap(),
            lines.next().unwrap()
        ),
    )
    .unwrap();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Coordenadas de `Sistema Burst i` (como las de un diario previo) para que el filtro acepte mercados.
    {
        let store = tmc_store::Store::open(&ddir.path().join("tmc.db")).unwrap();
        let coords: Vec<(String, Option<u64>, [f64; 3])> = (0..500)
            .map(|i| {
                (
                    format!("sistema burst {i}"),
                    None,
                    [f64::from(i % 20), 0.0, 0.0],
                )
            })
            .collect();
        store.upsert_coords(&coords, chrono::Utc::now()).unwrap();
    }
    let (url, relay) = spawn_fake_relay(stop.clone()).await;
    // Sin juego instalado: el sondeo de procesos (solo depuración) simula EliteDangerous64.
    let _game = GameRunning::on();
    crate::eddn_service::E2E_CORRUPT_DROPS.store(0, std::sync::atomic::Ordering::Relaxed);
    let running = start_full(
        config(jdir.path(), ddir.path(), true),
        Arc::new(FakeTransport),
        Some(Duration::from_millis(100)),
        Some(url),
    )
    .await
    .unwrap();
    wait_for(&running, "start", |s| s.contains_key("trade.market")).await;
    for l in lines {
        append(&journal, l);
    }
    std::fs::write(
        jdir.path().join("Market.json"),
        include_str!("../../../tests/fixtures/companion/market_trade.json"),
    )
    .unwrap();
    append(
        &journal,
        r#"{"timestamp":"2026-10-19T12:01:30Z","event":"Market","MarketID":128000001,"StationName":"Synthetic Port","StationType":"Coriolis","StarSystem":"Synthetic System"}"#,
    );
    wait_for(&running, "trade market", |s| {
        s.get("trade.market")
            .is_some_and(|m| m["station"] == "Synthetic Port")
    })
    .await;
    let mut rx = running.core.subscribe();
    // La lectura inicial por el `Market` ya ocurrió: se cuentan solo las motivadas por EDDN.
    let t_start = Instant::now();
    // La conexión al relay y la ráfaga llegan por su cuenta; mientras, ráfagas de eventos de diario.
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut arrivals = Vec::new();
    let mut latencies = Vec::new();
    let mut n = 0u32;
    while latencies.len() < 120 && Instant::now() < deadline {
        n += 1;
        let marker = 100 + n;
        let ts = format!("2026-10-19T12:{:02}:{:02}Z", 2 + n / 60, n % 60);
        append(
            &journal,
            &format!(
                r#"{{"timestamp":"{ts}","event":"Cargo","Count":{marker},"Inventory":[{{"Name":"Gold","Count":{marker},"Stolen":0}}]}}"#
            ),
        );
        let t0 = Instant::now();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(CoreMessage::Update { topics, .. }) = rx.recv().await {
                    arrivals.push(Instant::now());
                    if topics
                        .get("trade.cargo")
                        .is_some_and(|c| c["used"] == marker)
                    {
                        return;
                    }
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("cargo {marker} not reflected"));
        latencies.push(t0.elapsed());
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert!(latencies.len() >= 100, "solo {} muestras", latencies.len());
    latencies.sort();
    let p95 = latencies[(latencies.len() * 95).div_ceil(100) - 1];
    eprintln!("eddn burst lat = {latencies:?}");
    eprintln!("eddn burst p95 = {p95:?}, updates = {}", arrivals.len());
    assert!(p95 < Duration::from_millis(500), "p95 = {p95:?}");
    for (i, a) in arrivals.iter().enumerate() {
        let c = arrivals[i..]
            .iter()
            .take_while(|b| b.duration_since(*a) < Duration::from_secs(1))
            .count();
        assert!(c <= 11, "{c} difusiones en 1 s");
    }
    // A lo sumo una relectura `trade.best_sells` por ventana de 10 s desde que empieza la ráfaga.
    let queries: Vec<Instant> = tmc_core::debug_hooks::store_task_times("trade.best_sells")
        .into_iter()
        .filter(|t| *t >= t_start)
        .collect();
    eprintln!("eddn burst best_sells tasks = {}", queries.len());
    for (i, a) in queries.iter().enumerate() {
        let c = queries[i..]
            .iter()
            .take_while(|b| b.duration_since(*a) < Duration::from_secs(10))
            .count();
        assert!(c <= 1, "{c} relecturas best_sells en 10 s");
    }
    // Se detiene el relay al final de una pasada y se espera al último lote (≤ 5 s): el estado
    // final cuenta todos los corruptos recibidos, siempre de 20 en 20.
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = relay.await;
    tokio::time::sleep(Duration::from_secs(7)).await;
    let corrupt = crate::eddn_service::E2E_CORRUPT_DROPS.load(std::sync::atomic::Ordering::Relaxed);
    eprintln!("eddn burst corrupt drops = {corrupt}");
    assert!(
        corrupt >= 20 && corrupt.is_multiple_of(20),
        "corrupt = {corrupt}"
    );
    // El suscriptor llegó a conectarse.
    wait_for(&running, "eddn connected", |s| {
        s.get("trade.market")
            .is_some_and(|m| m["eddn_connected"] == true)
    })
    .await;
    running.server.shutdown();
}

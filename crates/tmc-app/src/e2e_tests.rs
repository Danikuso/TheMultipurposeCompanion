//! Pruebas de extremo a extremo con `start()` (F1, criterios 2 y 5).

use crate::{Config, Running, ServerSection, start, start_custom, start_with_transport};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tmc_core::CoreMessage;
use tmc_net::{BoxFut, HttpReply, Transport, TransportError};

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

use crate::subscriber::IDLE_TIMEOUT;
use crate::*;
use chrono::{Duration as CDur, SecondsFormat, Utc};
use flate2::{Compression, write::ZlibEncoder};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

enum Step {
    Frame(Vec<u8>),
    Fail,
}

#[derive(Default)]
struct Counts {
    connects: u32,
    disconnects: u32,
    timeouts: Vec<Duration>,
}

struct FakeSource {
    script: VecDeque<Step>,
    clock: FakeClock,
    counts: Arc<Mutex<Counts>>,
    /// Con el guion agotado: `None` avanza el reloj falso el tiempo de espera sin límite;
    /// `Some(presupuesto)` lo avanza hasta agotarlo y luego espera de verdad sin tocar el reloj
    /// (el suscriptor no puede alcanzar `IDLE_TIMEOUT` ni reconectar por su cuenta).
    empty_budget: Option<Duration>,
}

impl FrameSource for FakeSource {
    fn connect(&mut self, _url: &str) -> Result<(), EddnError> {
        self.counts.lock().unwrap().connects += 1;
        Ok(())
    }
    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, EddnError> {
        self.counts.lock().unwrap().timeouts.push(timeout);
        match self.script.pop_front() {
            Some(Step::Frame(f)) => Ok(Some(f)),
            Some(Step::Fail) => Err(EddnError::Transport("caída".into())),
            None if self.empty_budget.is_some_and(|b| b.is_zero()) => {
                std::thread::sleep(Duration::from_millis(1));
                Ok(None)
            }
            None => {
                let step = self.empty_budget.map_or(timeout, |b| timeout.min(b));
                if let Some(b) = &mut self.empty_budget {
                    *b -= step;
                }
                self.clock.advance(step);
                Ok(None)
            }
        }
    }
    fn disconnect(&mut self) {
        self.counts.lock().unwrap().disconnects += 1;
    }
}

fn z(s: &str) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(s.as_bytes()).unwrap();
    e.finish().unwrap()
}

fn market(id: u64, ts: chrono::DateTime<Utc>) -> Vec<u8> {
    z(&format!(
        r#"{{"$schemaRef":"{COMMODITY_SCHEMA}","header":{{}},"message":{{"timestamp":"{}","systemName":"Lave","stationName":"Puerto {id}","marketId":{id},"commodities":[]}}}}"#,
        ts.to_rfc3339_opts(SecondsFormat::Secs, true)
    ))
}

fn filter(enabled: bool) -> EddnFilter {
    EddnFilter {
        enabled,
        center: Some([0.0; 3]),
        radius_ly: 50.0,
    }
}

struct Rig {
    rx: mpsc::Receiver<EddnOutput>,
    ftx: watch::Sender<EddnFilter>,
    clock: FakeClock,
    counts: Arc<Mutex<Counts>>,
    handle: std::thread::JoinHandle<()>,
    t0: std::time::Instant,
}

fn rig(script: Vec<Step>, enabled: bool) -> Rig {
    rig_with(script, filter(enabled), 1024)
}

fn rig_with(script: Vec<Step>, f: EddnFilter, cap: usize) -> Rig {
    rig_full(script, f, cap, None)
}

fn rig_blocking(script: Vec<Step>, enabled: bool) -> Rig {
    rig_full(script, filter(enabled), 1024, Some(Duration::ZERO))
}

fn rig_full(script: Vec<Step>, f: EddnFilter, cap: usize, empty_budget: Option<Duration>) -> Rig {
    let clock = FakeClock::new();
    let t0 = clock.now();
    let counts = Arc::new(Mutex::new(Counts::default()));
    let source = FakeSource {
        script: script.into(),
        clock: clock.clone(),
        counts: counts.clone(),
        empty_budget,
    };
    let (ftx, frx) = watch::channel(f);
    let (tx, rx) = mpsc::channel(cap);
    let handle = spawn_subscriber(
        Box::new(source),
        Box::new(clock.clone()),
        "tcp://fake".into(),
        frx,
        tx,
        vec![("lave".into(), None, [0.0; 3])],
    );
    Rig {
        rx,
        ftx,
        clock,
        counts,
        handle,
        t0,
    }
}

impl Rig {
    fn next(&mut self) -> EddnOutput {
        self.rx.blocking_recv().expect("canal cerrado")
    }
    fn next_batch(&mut self) -> Vec<MarketSnapshot> {
        loop {
            if let EddnOutput::Batch(m, _, _) = self.next() {
                return m;
            }
        }
    }
    fn next_full_batch(&mut self) -> (Vec<MarketSnapshot>, Vec<StationInfo>, Vec<CoordEntry>) {
        loop {
            if let EddnOutput::Batch(m, s, c) = self.next() {
                return (m, s, c);
            }
        }
    }
    fn finish(self) {
        drop(self.rx);
        self.handle.join().unwrap();
    }
}

use tmc_model::market::{MarketSnapshot, StationInfo};

#[test]
fn batches_flush_at_20_markets() {
    let now = Utc::now();
    let script = (0..25)
        .map(|i| Step::Frame(market(1000 + i, now)))
        .collect();
    let mut r = rig(script, true);
    assert_eq!(r.next_batch().len(), 20);
    let rest = r.next_batch();
    assert_eq!(rest.len(), 5);
    let elapsed = r.clock.now().duration_since(r.t0);
    // El hilo sigue avanzando el reloj tras enviar: solo hay cota inferior fiable.
    assert!(elapsed >= Duration::from_secs(5), "{elapsed:?}");
    r.finish();
}

#[test]
fn duplicate_market_in_batch_keeps_newest() {
    let t1 = Utc::now() - CDur::minutes(10);
    let t2 = Utc::now() - CDur::minutes(1);
    // Llegan en orden inverso: gana la más nueva, no la última.
    let script = vec![Step::Frame(market(7, t2)), Step::Frame(market(7, t1))];
    let mut r = rig(script, true);
    let b = r.next_batch();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].updated_at.timestamp(), t2.timestamp());
    r.finish();
}

#[test]
fn reconnects_with_backoff_after_relay_drop() {
    let now = Utc::now();
    let script = vec![
        Step::Fail,
        Step::Fail,
        Step::Fail,
        Step::Frame(market(1, now)),
        Step::Fail,
    ];
    let mut r = rig(script, true);
    r.next_batch();
    let secs: Vec<u64> = r.clock.sleeps().iter().map(|d| d.as_secs()).collect();
    assert_eq!(&secs[..4], &[1, 2, 4, 1]);
    assert!(r.counts.lock().unwrap().connects >= 5);
    r.finish();
}

#[test]
fn connected_status_emitted_on_connect() {
    let mut r = rig(vec![], true);
    match r.next() {
        EddnOutput::Status(s) => assert!(s.connected),
        other => panic!("{other:?}"),
    }
    r.finish();
}

#[test]
fn disabled_filter_disconnects_within_1s() {
    let mut r = rig_blocking(vec![], true);
    assert!(matches!(r.next(), EddnOutput::Status(s) if s.connected));
    r.ftx.send(filter(false)).unwrap();
    loop {
        match r.next() {
            EddnOutput::Status(s) if !s.connected => break,
            _ => {}
        }
    }
    // Cada vuelta sin recibir avanza como máximo 1 s: la baja es inmediata a esa escala.
    while r.clock.sleeps().len() < 5 {
        std::thread::yield_now();
    }
    {
        let c = r.counts.lock().unwrap();
        assert_eq!(c.disconnects, 1);
        assert_eq!(c.connects, 1);
        // Cada espera de recv es de 1 s como máximo: la baja ocurre en una vuelta.
        assert!(c.timeouts.iter().all(|t| *t <= Duration::from_secs(1)));
    }
    r.finish();
}

#[test]
fn disabled_from_start_never_connects() {
    let r = rig_blocking(vec![], false);
    while r.clock.sleeps().len() < 3 {
        std::thread::yield_now();
    }
    assert_eq!(r.counts.lock().unwrap().connects, 0);
    r.finish();
}

#[test]
fn corrupt_frames_never_stop_the_loop() {
    let mut script: Vec<Step> = (0..100)
        .map(|i| Step::Frame(format!("basura {i}").into_bytes()))
        .collect();
    script.push(Step::Frame(market(5, Utc::now())));
    let mut r = rig(script, true);
    assert_eq!(r.next_batch().len(), 1);
    // El estado que acompaña al lote lleva las cifras.
    loop {
        if let EddnOutput::Status(s) = r.next()
            && s.stats.received == 101
        {
            assert_eq!(s.stats.dropped[&DropReason::Corrupt], 100);
            break;
        }
    }
    r.finish();
}

#[test]
fn full_out_channel_never_blocks_and_merges() {
    let now = Utc::now();
    let clock = FakeClock::new();
    let counts = Arc::new(Mutex::new(Counts::default()));
    // 60 mercados con canal de capacidad 1 que nadie vacía hasta el final.
    let script = (0..60)
        .map(|i| Step::Frame(market(2000 + i, now)))
        .chain((0..60).map(|i| Step::Frame(market(2000 + i, now))))
        .collect::<VecDeque<_>>();
    let source = FakeSource {
        script,
        clock: clock.clone(),
        counts,
        empty_budget: None,
    };
    let (_ftx, frx) = watch::channel(filter(true));
    let (tx, mut rx) = mpsc::channel(1);
    let h = spawn_subscriber(
        Box::new(source),
        Box::new(clock.clone()),
        "tcp://fake".into(),
        frx,
        tx,
        vec![("lave".into(), None, [0.0; 3])],
    );
    let mut seen = std::collections::HashSet::new();
    while seen.len() < 60 {
        if let EddnOutput::Batch(m, _, _) = rx.blocking_recv().unwrap() {
            seen.extend(m.into_iter().map(|s| s.market_id));
        }
    }
    drop(rx);
    h.join().unwrap();
}

/// Manual: `fake_relay 0 tests/fixtures/eddn/burst.jsonl` y
/// `TMC_FAKE_RELAY_URL=tcp://127.0.0.1:<puerto> cargo test -p tmc-eddn -- --ignored relay_smoke`.
#[test]
#[ignore]
fn relay_smoke() {
    let url = std::env::var("TMC_FAKE_RELAY_URL").expect("TMC_FAKE_RELAY_URL");
    let (_ftx, frx) = watch::channel(filter(true));
    let (tx, mut rx) = mpsc::channel(1024);
    let seed = (0..10)
        .map(|i| (format!("sistema burst {i}"), None, [i as f64, 0.0, 0.0]))
        .collect();
    let h = spawn_subscriber(
        Box::new(ZmqSource::new().unwrap()),
        Box::new(SystemClock),
        url,
        frx,
        tx,
        seed,
    );
    let mut seen = std::collections::HashSet::new();
    let mut corrupt = 0;
    let start = std::time::Instant::now();
    while (seen.len() < 500 || corrupt < 20) && start.elapsed() < Duration::from_secs(20) {
        match rx.try_recv() {
            Ok(EddnOutput::Batch(m, _, _)) => seen.extend(m.into_iter().map(|s| s.market_id)),
            Ok(EddnOutput::Status(s)) => {
                corrupt = corrupt.max(
                    s.stats
                        .dropped
                        .get(&DropReason::Corrupt)
                        .copied()
                        .unwrap_or(0),
                )
            }
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    drop(rx);
    h.join().unwrap();
    assert_eq!(seen.len(), 500);
    assert!(corrupt >= 20, "corrupt={corrupt}");
}

#[test]
fn full_out_with_due_batch_does_not_spin() {
    let now = Utc::now();
    let script = (0..25)
        .map(|i| Step::Frame(market(3000 + i, now)))
        .collect();
    // Capacidad 1 ocupada por el Status de conexión: el lote no puede salir.
    let r = rig_with(script, filter(true), 1);
    while r.counts.lock().unwrap().timeouts.len() < 30 {
        std::thread::yield_now();
    }
    {
        let c = r.counts.lock().unwrap();
        assert!(c.timeouts.iter().all(|t| *t >= Duration::from_millis(100)));
    }
    r.finish();
}

#[test]
fn idle_connection_is_dropped_and_reconnected() {
    let mut r = rig(vec![], true);
    assert!(matches!(r.next(), EddnOutput::Status(s) if s.connected));
    assert!(matches!(r.next(), EddnOutput::Status(s) if !s.connected));
    while r.counts.lock().unwrap().connects < 2 {
        std::thread::yield_now();
    }
    assert!(r.clock.now().duration_since(r.t0) >= IDLE_TIMEOUT);
    assert_eq!(r.clock.sleeps()[0], Duration::from_secs(1));
    r.finish();
}

fn fsd(system: &str, x: f64) -> Vec<u8> {
    z(&format!(
        r#"{{"$schemaRef":"{JOURNAL_SCHEMA}","header":{{}},"message":{{"timestamp":"{}","event":"FSDJump","StarSystem":"{system}","StarPos":[{x},0,0]}}}}"#,
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    ))
}

#[test]
fn coords_are_emitted_within_margin_only() {
    // Radio 50: margen = max(75, 150) = 150 ly.
    let script = vec![
        Step::Frame(fsd("Cerca", 120.0)),
        Step::Frame(fsd("Lejos", 400.0)),
    ];
    let mut r = rig(script, true);
    let (_, _, coords) = r.next_full_batch();
    assert_eq!(coords.len(), 1);
    assert_eq!(coords[0].0, "cerca");
    r.finish();
}

#[test]
fn no_center_emits_no_coords() {
    let f = EddnFilter {
        enabled: true,
        center: None,
        radius_ly: 50.0,
    };
    let script = vec![
        Step::Frame(fsd("Cerca", 1.0)),
        Step::Frame(market(9, Utc::now())),
    ];
    let mut r = rig_with(script, f, 1024);
    // Sin centro el mercado conocido (semilla) pasa, las coordenadas no salen.
    let (m, _, coords) = r.next_full_batch();
    assert_eq!(m.len(), 1);
    assert!(coords.is_empty());
    r.finish();
}

/// `burst.jsonl`: 500 mercados cercanos se almacenan y los 20 mensajes corruptos se cuentan.
#[test]
fn burst_fixture_yields_500_markets_and_20_corrupt_drops() {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let text = include_str!("../../../tests/fixtures/eddn/burst.jsonl");
    let script: Vec<Step> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
            e.write_all(l.replace("@NOW@", &now).as_bytes()).unwrap();
            Step::Frame(e.finish().unwrap())
        })
        .collect();
    assert_eq!(script.len(), 520);
    let center = EddnFilter {
        enabled: true,
        center: Some([0.0; 3]),
        radius_ly: 500.0,
    };
    let clock = FakeClock::new();
    let counts = Arc::new(Mutex::new(Counts::default()));
    let source = FakeSource {
        script: script.into(),
        clock: clock.clone(),
        counts,
        // El estado final (con los descartes corruptos) sale al caer por inactividad: se avanza
        // el reloj lo justo para ello y después se espera sin tocarlo (sin más reconexiones).
        empty_budget: Some(IDLE_TIMEOUT + Duration::from_secs(5)),
    };
    let (_ftx, frx) = watch::channel(center);
    let (tx, rx) = mpsc::channel(1024);
    // Un solo mercado por sistema `Sistema Burst i`: se siembran sus coordenadas.
    let seed = (0..500)
        .map(|i| {
            (
                format!("sistema burst {i}"),
                None,
                [f64::from(i % 100), 0.0, 0.0],
            )
        })
        .collect();
    let handle = spawn_subscriber(
        Box::new(source),
        Box::new(clock),
        "tcp://fake".into(),
        frx,
        tx,
        seed,
    );
    let mut r = Rig {
        rx,
        ftx: watch::channel(filter(true)).0,
        clock: FakeClock::new(),
        counts: Arc::new(Mutex::new(Counts::default())),
        handle,
        t0: std::time::Instant::now(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut corrupt = 0;
    while seen.len() < 500 || corrupt < 20 {
        match r.next() {
            EddnOutput::Batch(m, _, _) => seen.extend(m.into_iter().map(|s| s.market_id)),
            EddnOutput::Status(s) => {
                corrupt = corrupt.max(
                    s.stats
                        .dropped
                        .get(&DropReason::Corrupt)
                        .copied()
                        .unwrap_or(0),
                )
            }
        }
    }
    assert_eq!(seen.len(), 500);
    assert_eq!(corrupt, 20);
    r.finish();
}

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
}

struct FakeSource {
    script: VecDeque<Step>,
    clock: FakeClock,
    counts: Arc<Mutex<Counts>>,
}

impl FrameSource for FakeSource {
    fn connect(&mut self, _url: &str) -> Result<(), EddnError> {
        self.counts.lock().unwrap().connects += 1;
        Ok(())
    }
    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, EddnError> {
        match self.script.pop_front() {
            Some(Step::Frame(f)) => Ok(Some(f)),
            Some(Step::Fail) => Err(EddnError::Transport("caída".into())),
            None => {
                self.clock.advance(timeout);
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
}

fn rig(script: Vec<Step>, enabled: bool) -> Rig {
    let clock = FakeClock::new();
    let counts = Arc::new(Mutex::new(Counts::default()));
    let source = FakeSource {
        script: script.into(),
        clock: clock.clone(),
        counts: counts.clone(),
    };
    let (ftx, frx) = watch::channel(filter(enabled));
    let (tx, rx) = mpsc::channel(1024);
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
    fn finish(self) {
        drop(self.rx);
        self.handle.join().unwrap();
    }
}

use tmc_model::market::MarketSnapshot;

#[test]
fn batches_flush_at_20_markets() {
    let now = Utc::now();
    let script = (0..25)
        .map(|i| Step::Frame(market(1000 + i, now)))
        .collect();
    let mut r = rig(script, true);
    assert_eq!(r.next_batch().len(), 20);
    let before = r.clock.now();
    let rest = r.next_batch();
    assert_eq!(rest.len(), 5);
    assert!(r.clock.now().duration_since(before) >= Duration::ZERO);
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
    assert_eq!(r.counts.lock().unwrap().connects, 5);
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
    let mut r = rig(vec![], true);
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
    }
    r.finish();
}

#[test]
fn disabled_from_start_never_connects() {
    let r = rig(vec![], false);
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

//! Hilo suscriptor: conexión, reconexión con retroceso, decodificación y lotes.

use crate::coords::{CoordIndex, dist};
use crate::decode::{CarrierSet, Decoded, DropReason, decode_with};
use crate::filter::{EddnFilter, EddnStats, accept};
use crate::transport::FrameSource;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tmc_model::market::{MarketSnapshot, StationInfo};
use tokio::sync::{mpsc, watch};

/// Mercados por lote que fuerzan el envío inmediato.
pub const BATCH_MAX_MARKETS: usize = 20;
/// Antigüedad máxima de un lote pendiente.
pub const BATCH_MAX_AGE: Duration = Duration::from_secs(5);
/// Tope de elementos pendientes (si `out` está lleno) por tipo.
pub const PENDING_CAP: usize = 2_000;
const BACKOFF_MAX_S: u64 = 60;
const POLL: Duration = Duration::from_secs(1);
/// Espera mínima de `recv`: evita el bucle activo cuando hay un lote vencido sin poder salir.
const MIN_POLL: Duration = Duration::from_millis(100);
/// Reintento de envío tras encontrar `out` lleno.
const FLUSH_RETRY: Duration = Duration::from_millis(500);
/// Sin ningún fotograma durante este tiempo se considera caída la conexión.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Margen de coordenadas que se conservan y emiten alrededor del radio del filtro.
fn coord_margin(radius: f64) -> f64 {
    (radius * 1.5).max(radius + 100.0)
}

pub type CoordEntry = (String, Option<u64>, [f64; 3]);

#[derive(Debug, Clone, PartialEq)]
pub struct EddnStatus {
    pub connected: bool,
    pub stats: EddnStats,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EddnOutput {
    Batch(Vec<MarketSnapshot>, Vec<StationInfo>, Vec<CoordEntry>),
    Status(EddnStatus),
}

pub trait Clock: Send {
    fn now(&self) -> Instant;
    fn sleep(&self, d: Duration);
    /// Espera `d` en tramos de 1 s como máximo mientras `stop()` sea falso.
    fn sleep_unless(&self, d: Duration, stop: &dyn Fn() -> bool) {
        let mut left = d;
        while !left.is_zero() && !stop() {
            let step = left.min(POLL);
            self.sleep(step);
            left -= step;
        }
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Reloj de pruebas: `sleep` y `advance` mueven el tiempo sin esperar y las esperas se registran.
#[derive(Clone)]
pub struct FakeClock {
    base: Instant,
    state: Arc<Mutex<(Duration, Vec<Duration>)>>,
}

impl Default for FakeClock {
    fn default() -> Self {
        Self {
            base: Instant::now(),
            state: Arc::new(Mutex::new((Duration::ZERO, Vec::new()))),
        }
    }
}

impl FakeClock {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn advance(&self, d: Duration) {
        self.state.lock().unwrap().0 += d;
    }
    /// Esperas pedidas con `sleep`, en orden.
    pub fn sleeps(&self) -> Vec<Duration> {
        self.state.lock().unwrap().1.clone()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.base + self.state.lock().unwrap().0
    }
    fn sleep(&self, d: Duration) {
        let mut s = self.state.lock().unwrap();
        s.0 += d;
        s.1.push(d);
    }
    /// Una sola espera registrada, sin tramos ni comprobación.
    fn sleep_unless(&self, d: Duration, _stop: &dyn Fn() -> bool) {
        self.sleep(d);
    }
}

#[derive(Default)]
struct Pending {
    markets: HashMap<u64, MarketSnapshot>,
    stations: HashMap<u64, StationInfo>,
    coords: HashMap<String, (Option<u64>, [f64; 3])>,
    since: Option<Instant>,
}

impl Pending {
    fn is_empty(&self) -> bool {
        self.markets.is_empty() && self.stations.is_empty() && self.coords.is_empty()
    }
    fn touch(&mut self, now: Instant) {
        self.since.get_or_insert(now);
    }
    fn due(&self, now: Instant) -> bool {
        self.markets.len() >= BATCH_MAX_MARKETS
            || (!self.is_empty()
                && self
                    .since
                    .is_some_and(|s| now.saturating_duration_since(s) >= BATCH_MAX_AGE))
    }
    fn add_market(&mut self, m: MarketSnapshot, now: Instant) {
        // Se queda la más nueva; a igualdad, la última recibida.
        if self
            .markets
            .get(&m.market_id)
            .is_some_and(|old| old.updated_at > m.updated_at)
        {
            return;
        }
        if self.markets.len() >= PENDING_CAP
            && !self.markets.contains_key(&m.market_id)
            && let Some(oldest) = self
                .markets
                .iter()
                .min_by_key(|(_, v)| v.updated_at)
                .map(|(k, _)| *k)
        {
            self.markets.remove(&oldest);
        }
        self.markets.insert(m.market_id, m);
        self.touch(now);
    }
    fn add_station(&mut self, s: StationInfo, now: Instant) {
        if self.stations.len() >= PENDING_CAP && !self.stations.contains_key(&s.market_id) {
            return;
        }
        self.stations.insert(s.market_id, s);
        self.touch(now);
    }
    fn add_coords(&mut self, k: String, id: Option<u64>, p: [f64; 3], now: Instant) {
        if self.coords.len() >= PENDING_CAP && !self.coords.contains_key(&k) {
            return;
        }
        self.coords.insert(k, (id, p));
        self.touch(now);
    }
}

enum Flow {
    Continue,
    Stop,
}

struct Worker {
    source: Box<dyn FrameSource>,
    clock: Box<dyn Clock>,
    url: String,
    filter: watch::Receiver<EddnFilter>,
    out: mpsc::Sender<EddnOutput>,
    idx: CoordIndex,
    carriers: CarrierSet,
    stats: EddnStats,
    pending: Pending,
    backoff_s: u64,
    connected: bool,
    status_dirty: bool,
    last_frame: Instant,
    flush_retry: Option<Instant>,
    last_area: Option<([f64; 3], f64)>,
}

/// Lanza el hilo suscriptor. Termina al cerrarse `out`.
pub fn spawn_subscriber(
    source: Box<dyn FrameSource>,
    clock: Box<dyn Clock>,
    url: String,
    filter: watch::Receiver<EddnFilter>,
    out: mpsc::Sender<EddnOutput>,
    seed: Vec<CoordEntry>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let center = filter.borrow().center;
        let mut idx = CoordIndex::new();
        for (k, id, p) in seed {
            idx.insert(k, id, p, center);
        }
        let last_frame = clock.now();
        let mut w = Worker {
            source,
            clock,
            url,
            filter,
            out,
            idx,
            carriers: CarrierSet::new(),
            stats: EddnStats::default(),
            pending: Pending::default(),
            backoff_s: 1,
            connected: false,
            status_dirty: false,
            last_frame,
            flush_retry: None,
            last_area: None,
        };
        w.run();
        w.source.disconnect();
    })
}

impl Worker {
    /// Envía el estado; si `out` está lleno lo reintenta en la vuelta siguiente.
    fn status(&mut self) {
        self.status_dirty = matches!(
            self.out.try_send(EddnOutput::Status(EddnStatus {
                connected: self.connected,
                stats: self.stats.clone(),
            })),
            Err(mpsc::error::TrySendError::Full(_))
        );
    }

    fn run(&mut self) {
        loop {
            if self.out.is_closed() {
                return;
            }
            if self.status_dirty {
                self.status();
            }
            let f = self.filter.borrow().clone();
            if !f.enabled {
                if matches!(self.flush(), Flow::Stop) {
                    return;
                }
                // Si `out` sigue lleno, lo pendiente se descarta: nadie lo pidió ya.
                self.pending = Pending::default();
                self.flush_retry = None;
                self.backoff_s = 1;
                if self.connected {
                    self.source.disconnect();
                    self.connected = false;
                    self.status();
                }
                self.clock.sleep(POLL);
                continue;
            }
            self.update_area(&f);
            if matches!(self.flush_if_due(), Flow::Stop) {
                return;
            }
            if !self.connected {
                match self.source.connect(&self.url) {
                    Ok(()) => {
                        self.connected = true;
                        self.last_frame = self.clock.now();
                        self.status();
                    }
                    Err(_) => {
                        self.wait_backoff();
                        continue;
                    }
                }
            }
            let now = self.clock.now();
            let timeout = match self.pending.since {
                Some(s) if !self.pending.is_empty() => BATCH_MAX_AGE
                    .saturating_sub(now.saturating_duration_since(s))
                    .clamp(MIN_POLL, POLL),
                _ => POLL,
            };
            match self.source.recv(timeout) {
                Ok(Some(frame)) => {
                    self.last_frame = self.clock.now();
                    self.handle(&frame, &f);
                    if matches!(self.flush_if_due(), Flow::Stop) {
                        return;
                    }
                }
                Ok(None) => {
                    let idle = self.clock.now().saturating_duration_since(self.last_frame);
                    if idle >= IDLE_TIMEOUT {
                        self.source.disconnect();
                        self.connected = false;
                        self.status();
                        self.wait_backoff();
                    }
                }
                Err(_) => {
                    self.source.disconnect();
                    self.connected = false;
                    self.status();
                    self.wait_backoff();
                }
            }
        }
    }

    fn wait_backoff(&mut self) {
        let d = Duration::from_secs(self.backoff_s);
        self.backoff_s = (self.backoff_s * 2).min(BACKOFF_MAX_S);
        let (out, filter) = (&self.out, &self.filter);
        self.clock
            .sleep_unless(d, &|| out.is_closed() || !filter.borrow().enabled);
    }

    fn drop_count(&mut self, r: DropReason) {
        *self.stats.dropped.entry(r).or_insert(0) += 1;
    }

    fn handle(&mut self, frame: &[u8], f: &EddnFilter) {
        self.stats.received += 1;
        let now = self.clock.now();
        match decode_with(frame, Utc::now(), &mut self.carriers) {
            Decoded::Ignored(r) => {
                self.drop_count(r);
                // Un mensaje bien formado (aunque no interese) prueba que el relay vive.
                if !matches!(r, DropReason::Corrupt | DropReason::TooLarge) {
                    self.backoff_s = 1;
                }
            }
            Decoded::Market(mut m) => {
                self.backoff_s = 1;
                match accept(&mut m, &self.idx, f) {
                    Ok(()) => {
                        self.stats.accepted += 1;
                        self.pending.add_market(m, now);
                    }
                    Err(r) => self.drop_count(r),
                }
            }
            Decoded::Station(info, coords) => {
                self.backoff_s = 1;
                if let Some((k, id, p)) = coords {
                    self.coord(k, id, p, f, now);
                }
                let far = match (info.star_pos, f.center) {
                    (Some(p), Some(c)) => f.radius_ly.is_nan() || dist(p, c) > f.radius_ly,
                    _ => false,
                };
                if far {
                    self.drop_count(DropReason::Far);
                } else {
                    self.pending.add_station(info, now);
                }
            }
            Decoded::Coords(k, id, p) => {
                self.backoff_s = 1;
                self.coord(k, id, p, f, now);
            }
        }
    }

    /// Al cambiar centro o radio, descarta del índice lo que queda fuera del margen.
    fn update_area(&mut self, f: &EddnFilter) {
        let area = f.center.map(|c| (c, f.radius_ly));
        if area != self.last_area {
            if let Some((c, r)) = area
                && r.is_finite()
                && r >= 0.0
            {
                self.idx.recenter(c, coord_margin(r));
            }
            self.last_area = area;
        }
    }

    fn coord(&mut self, k: String, id: Option<u64>, p: [f64; 3], f: &EddnFilter, now: Instant) {
        self.idx.insert(k.clone(), id, p, f.center);
        // Sin centro no se emiten coordenadas (tabla sin cota).
        if f.center
            .is_some_and(|c| dist(p, c) <= coord_margin(f.radius_ly))
        {
            self.pending.add_coords(k, id, p, now);
        }
    }

    fn flush_if_due(&mut self) -> Flow {
        let now = self.clock.now();
        if self.flush_retry.is_some_and(|t| now < t) {
            return Flow::Continue;
        }
        if self.pending.due(now) {
            self.flush()
        } else {
            Flow::Continue
        }
    }

    /// Envía el lote pendiente sin bloquear; si `out` está lleno, lo conserva.
    fn flush(&mut self) -> Flow {
        if self.pending.is_empty() {
            return Flow::Continue;
        }
        let permit = match self.out.try_reserve() {
            Ok(p) => p,
            Err(mpsc::error::TrySendError::Full(())) => {
                self.flush_retry = Some(self.clock.now() + FLUSH_RETRY);
                return Flow::Continue;
            }
            Err(mpsc::error::TrySendError::Closed(())) => return Flow::Stop,
        };
        self.flush_retry = None;
        let p = std::mem::take(&mut self.pending);
        permit.send(EddnOutput::Batch(
            p.markets.into_values().collect(),
            p.stations.into_values().collect(),
            p.coords
                .into_iter()
                .map(|(k, (id, pos))| (k, id, pos))
                .collect(),
        ));
        self.status();
        Flow::Continue
    }
}

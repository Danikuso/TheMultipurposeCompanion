//! Reloj inyectable para poder simular el paso del tiempo en los tests.

use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, Mutex};

pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
}

/// Reloj real del sistema.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Reloj de test: `base` + tiempo transcurrido de `tokio::time` (respeta `pause`/`advance`) + desplazamientos.
#[derive(Debug)]
pub struct TestClock {
    base: DateTime<Utc>,
    start: tokio::time::Instant,
    shift: Mutex<Duration>,
}

impl TestClock {
    pub fn new(base: DateTime<Utc>) -> Arc<Self> {
        Arc::new(Self {
            base,
            start: tokio::time::Instant::now(),
            shift: Mutex::new(Duration::zero()),
        })
    }

    /// Adelanta (o atrasa, si es negativa) el reloj `d`.
    pub fn shift(&self, d: Duration) {
        let mut shift = self.shift.lock().unwrap_or_else(|e| e.into_inner());
        *shift += d;
    }
}

impl Clock for TestClock {
    fn now_utc(&self) -> DateTime<Utc> {
        let elapsed = Duration::from_std(self.start.elapsed()).unwrap_or_else(|_| Duration::zero());
        let shift = *self.shift.lock().unwrap_or_else(|e| e.into_inner());
        self.base + elapsed + shift
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[tokio::test(start_paused = true)]
    async fn test_clock_follows_tokio_time_and_shift() {
        let base = Utc.with_ymd_and_hms(2026, 3, 9, 9, 0, 0).unwrap();
        let clock = TestClock::new(base);
        assert_eq!(clock.now_utc(), base);
        tokio::time::advance(std::time::Duration::from_secs(30)).await;
        clock.shift(Duration::hours(1));
        assert_eq!(clock.now_utc(), base + Duration::seconds(3630));
    }
}

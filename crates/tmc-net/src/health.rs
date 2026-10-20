//! Salud de un servicio: retroceso escalonado y estado «caído».

use std::time::{Duration, Instant};

const BACKOFF: [Duration; 3] = [
    Duration::from_secs(30),
    Duration::from_secs(120),
    Duration::from_secs(600),
];
const DOWN_FOR: Duration = Duration::from_secs(600);
const DOWN_AFTER: u32 = 3;

#[derive(Debug, Clone, Default)]
pub struct ServiceHealth {
    failures: u32,
    retry_at: Option<Instant>,
    down_until: Option<Instant>,
}

impl ServiceHealth {
    pub fn record_success(&mut self) {
        *self = Self::default();
    }

    pub fn record_failure(&mut self, now: Instant) {
        self.failures = self.failures.saturating_add(1);
        let idx = (self.failures as usize - 1).min(BACKOFF.len() - 1);
        self.retry_at = Some(now + BACKOFF[idx]);
        if self.failures >= DOWN_AFTER {
            self.down_until = Some(now + DOWN_FOR);
        }
    }

    /// Fallos consecutivos acumulados (para diagnóstico).
    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// Cuándo reintentar la petición fallida (consumidor: `NetService`).
    pub fn retry_at(&self) -> Option<Instant> {
        self.retry_at
    }

    pub fn is_down(&self, now: Instant) -> bool {
        self.down_until.is_some_and(|until| now < until)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn backoff_sequence_is_30s_2m_10m() {
        let t0 = Instant::now();
        let mut h = ServiceHealth::default();
        assert_eq!(h.retry_at(), None);
        h.record_failure(t0);
        assert_eq!(h.retry_at(), Some(t0 + Duration::from_secs(30)));
        h.record_failure(t0);
        assert_eq!(h.retry_at(), Some(t0 + Duration::from_secs(120)));
        h.record_failure(t0);
        assert_eq!(h.retry_at(), Some(t0 + Duration::from_secs(600)));
    }

    #[test]
    fn three_consecutive_failures_mark_down_for_10_minutes() {
        let t = Instant::now();
        let mut h = ServiceHealth::default();
        h.record_failure(t);
        h.record_failure(t);
        assert!(!h.is_down(t));
        h.record_failure(t);
        assert!(h.is_down(t + Duration::from_secs(9 * 60 + 59)));
        assert!(!h.is_down(t + Duration::from_secs(10 * 60 + 1)));
    }

    #[test]
    fn success_resets_failure_count() {
        let t = Instant::now();
        let mut h = ServiceHealth::default();
        h.record_failure(t);
        h.record_failure(t);
        h.record_success();
        assert_eq!(h.retry_at(), None);
        assert!(!h.is_down(t));
        h.record_failure(t);
        assert_eq!(h.retry_at(), Some(t + Duration::from_secs(30)));
    }
}

//! Limitador de peticiones por servicio (ventana deslizante, reloj inyectado).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Servicio externo con límite propio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Spansh,
    Edsm,
    Canonn,
    Ardent,
    Edcd,
    Eddn,
    Inara,
    Capi,
}

/// Permite como máximo `max` peticiones en cualquier ventana de `window`.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    max: u32,
    window: Duration,
    taken: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn per_window(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            taken: VecDeque::new(),
        }
    }

    /// Spansh 1/1 s, Canonn 1/1 s, Ardent 1/1 s, EDCD 1/1 s, CAPI 1/1 s, EDDN 10/1 s, EDSM 10/10 s, Inara 2/60 s.
    pub fn for_service(service: Service) -> Self {
        match service {
            Service::Spansh | Service::Canonn | Service::Ardent | Service::Edcd | Service::Capi => {
                Self::per_window(1, Duration::from_secs(1))
            }
            Service::Eddn => Self::per_window(10, Duration::from_secs(1)),
            Service::Edsm => Self::per_window(10, Duration::from_secs(10)),
            Service::Inara => Self::per_window(2, Duration::from_secs(60)),
        }
    }

    /// `Ok` registra la petición; `Err(espera)` indica cuánto falta para el siguiente hueco.
    pub fn try_acquire(&mut self, now: Instant) -> Result<(), Duration> {
        while let Some(&first) = self.taken.front() {
            if now.saturating_duration_since(first) >= self.window {
                self.taken.pop_front();
            } else {
                break;
            }
        }
        if (self.taken.len() as u32) < self.max {
            self.taken.push_back(now);
            return Ok(());
        }
        match self.taken.front() {
            Some(&first) => Err(self.window - now.saturating_duration_since(first)),
            None => Err(self.window),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn spansh_limiter_allows_one_per_second() {
        let t0 = Instant::now();
        let mut l = RateLimiter::for_service(Service::Spansh);
        assert_eq!(l.try_acquire(t0), Ok(()));
        let wait = l.try_acquire(t0 + Duration::from_millis(500)).unwrap_err();
        assert_eq!(wait, Duration::from_millis(500));
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(1)), Ok(()));
    }

    #[test]
    fn ardent_limiter_allows_one_per_second() {
        let t0 = Instant::now();
        let mut l = RateLimiter::for_service(Service::Ardent);
        assert_eq!(l.try_acquire(t0), Ok(()));
        assert_eq!(
            l.try_acquire(t0 + Duration::from_millis(250)),
            Err(Duration::from_millis(750))
        );
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(1)), Ok(()));
    }

    #[test]
    fn edcd_limiter_allows_one_per_second() {
        let t0 = Instant::now();
        let mut l = RateLimiter::for_service(Service::Edcd);
        assert_eq!(l.try_acquire(t0), Ok(()));
        assert_eq!(
            l.try_acquire(t0 + Duration::from_millis(500)),
            Err(Duration::from_millis(500))
        );
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(1)), Ok(()));
    }

    #[test]
    fn edsm_limiter_allows_ten_per_ten_seconds() {
        let t0 = Instant::now();
        let mut l = RateLimiter::for_service(Service::Edsm);
        for _ in 0..10 {
            assert_eq!(l.try_acquire(t0), Ok(()));
        }
        assert_eq!(l.try_acquire(t0), Err(Duration::from_secs(10)));
        assert_eq!(
            l.try_acquire(t0 + Duration::from_secs(4)),
            Err(Duration::from_secs(6))
        );
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(10)), Ok(()));
    }

    #[test]
    fn eddn_limiter_allows_ten_per_second() {
        let t0 = Instant::now();
        let mut l = RateLimiter::for_service(Service::Eddn);
        for _ in 0..10 {
            assert_eq!(l.try_acquire(t0), Ok(()));
        }
        assert_eq!(l.try_acquire(t0), Err(Duration::from_secs(1)));
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(1)), Ok(()));
    }

    #[test]
    fn inara_limiter_allows_two_per_minute() {
        let t0 = Instant::now();
        let mut l = RateLimiter::for_service(Service::Inara);
        assert_eq!(l.try_acquire(t0), Ok(()));
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(1)), Ok(()));
        assert_eq!(
            l.try_acquire(t0 + Duration::from_secs(2)),
            Err(Duration::from_secs(58))
        );
        assert_eq!(l.try_acquire(t0 + Duration::from_secs(60)), Ok(()));
    }
}

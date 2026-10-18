//! Control de peticiones de red en vuelo, con caducidad.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Una petición sin respuesta pasado este tiempo se da por perdida y puede repetirse.
pub const INFLIGHT_TTL_S: i64 = 60;

#[derive(Debug, Default)]
pub struct Inflight {
    sent: HashMap<String, DateTime<Utc>>,
}

impl Inflight {
    /// Registra el envío; `false` si ya hay uno en vuelo y no ha caducado.
    pub fn begin(&mut self, key: &str, now: DateTime<Utc>) -> bool {
        if let Some(at) = self.sent.get(key)
            && (now - *at).num_seconds() <= INFLIGHT_TTL_S
        {
            return false;
        }
        self.sent.insert(key.to_owned(), now);
        true
    }

    pub fn finish(&mut self, key: &str) {
        self.sent.remove(key);
    }

    /// Da por terminadas todas las peticiones cuya clave empieza por `prefix`.
    pub fn finish_prefix(&mut self, prefix: &str) {
        self.sent.retain(|k, _| !k.starts_with(prefix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    #[test]
    fn begin_blocks_until_finish_or_ttl() {
        let t0 = Utc.with_ymd_and_hms(2026, 10, 18, 12, 0, 0).unwrap();
        let mut f = Inflight::default();
        assert!(f.begin("a", t0));
        assert!(!f.begin("a", t0 + Duration::seconds(60)));
        assert!(f.begin("a", t0 + Duration::seconds(61)));
        f.finish("a");
        assert!(f.begin("a", t0 + Duration::seconds(62)));
        assert!(f.begin("b", t0));
        assert!(f.begin("sell:1:x", t0));
        f.finish_prefix("sell:");
        assert!(f.begin("sell:1:x", t0));
        assert!(!f.begin("b", t0));
    }
}

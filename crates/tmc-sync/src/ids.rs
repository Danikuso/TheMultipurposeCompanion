//! Identificadores de idempotencia y seudónimos estables.

use chrono::{DateTime, SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use tmc_model::sync::SyncService;

const SEP: char = '\u{1f}';

fn sha256_hex(input: &str) -> String {
    Sha256::digest(input.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Clave de idempotencia de un evento del diario (sha256 hex, 64 caracteres).
pub fn idem_key(service: SyncService, cmdr_fid: &str, file: &str, line: u64, sub: u32) -> String {
    sha256_hex(&format!(
        "{}{SEP}{cmdr_fid}{SEP}{file}{SEP}{line}{SEP}{sub}",
        service.as_str()
    ))
}

/// Clave de idempotencia de una instantánea de activación.
pub fn snapshot_idem_key(
    service: SyncService,
    cmdr_fid: &str,
    activated_at: DateTime<Utc>,
    kind: &str,
) -> String {
    sha256_hex(&format!(
        "{}{SEP}{cmdr_fid}{SEP}{}{SEP}{kind}",
        service.as_str(),
        activated_at.to_rfc3339_opts(SecondsFormat::Millis, true)
    ))
}

/// Seudónimo del comandante: `hex(sha256(sal ‖ FID))[..12]`.
pub fn cmdr_ref(salt: &str, fid: &str) -> String {
    let mut hex = sha256_hex(&format!("{salt}{fid}"));
    hex.truncate(12);
    hex
}

/// Pista visible de una clave API: "••••" + últimos 4 caracteres.
pub fn key_hint(api_key: &str) -> String {
    let chars: Vec<char> = api_key.chars().collect();
    let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    format!("••••{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use tmc_model::sync::SyncService;

    #[test]
    fn idem_key_is_stable_and_distinct() {
        let a = idem_key(SyncService::Inara, "F0000001", "Journal.log", 5, 0);
        assert_eq!(
            a,
            idem_key(SyncService::Inara, "F0000001", "Journal.log", 5, 0)
        );
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(
            a,
            idem_key(SyncService::Inara, "F0000001", "Journal.log", 5, 1)
        );
        assert_ne!(
            a,
            idem_key(SyncService::Edsm, "F0000001", "Journal.log", 5, 0)
        );
        let at = Utc.with_ymd_and_hms(2026, 3, 9, 9, 0, 0).unwrap();
        let s = snapshot_idem_key(SyncService::Inara, "F0000001", at, "ranks");
        assert_eq!(s.len(), 64);
        assert_ne!(
            s,
            snapshot_idem_key(SyncService::Inara, "F0000001", at, "ships")
        );
    }

    #[test]
    fn cmdr_ref_and_hint() {
        let r = cmdr_ref("s", "F0000001");
        assert_eq!(r.len(), 12);
        assert_eq!(r, cmdr_ref("s", "F0000001"));
        assert_ne!(r, cmdr_ref("t", "F0000001"));
        assert_eq!(key_hint("abcdef123456"), "••••3456");
    }
}

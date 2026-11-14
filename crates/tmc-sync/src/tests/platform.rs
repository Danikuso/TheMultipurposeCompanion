//! Política de detección del juego en vivo por plataforma (C3e, ruling T0-1).

use super::support::router_bench::{BenchOptions, F1, TWO_CMDRS, bench, early_activation};
use crate::router::{LiveSignalPolicy, REPLAY_MARKER};
use std::sync::atomic::Ordering;
use tmc_model::sync::{FeedOrigin, SyncFeedEvent, SyncReason, SyncService};

#[test]
fn unavailable_policy_reports_platform_unsupported_and_enqueues_nothing() {
    let mut b = bench(BenchOptions {
        policy: LiveSignalPolicy::Unavailable,
        ..BenchOptions::default()
    });
    b.give_key(F1, early_activation());
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=8, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::PlatformUnsupported)
    );
    assert_eq!(
        b.router.send_gates().blocked_reason(),
        Some(SyncReason::PlatformUnsupported)
    );
    assert!(!b.router.send_gates().send_allowed());
    // Los comandantes se siguen registrando para el selector de claves.
    assert_eq!(b.store.known_cmdrs().unwrap().len(), 2);
}

#[test]
fn journal_activity_policy_applies_same_guards() {
    let mut b = bench(BenchOptions {
        policy: LiveSignalPolicy::JournalActivity,
        ..BenchOptions::default()
    });
    b.give_key(F1, early_activation());
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=3, FeedOrigin::CatchUp);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    // Sin muestra de proceso: la línea Live nueva es la señal; no se retiene.
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 4..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 1);
    assert_eq!(b.router.blocked(SyncService::Inara, Some(F1)), None);

    // La muestra de proceso no influye en esta política.
    b.process(false);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 5..=5, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 1);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(
        b.cursor("F0000002", TWO_CMDRS),
        Some(5),
        "línea 5 (Commander F0000002) procesada sin clave"
    );

    // Resto de guardas: servicios en línea desactivados → nada.
    b.online.store(false, Ordering::SeqCst);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 6..=8, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 1);
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::OnlineOff)
    );

    // Marcador de replay en un archivo nuevo → nada, aunque vuelva a haber conexión.
    b.online.store(true, Ordering::SeqCst);
    std::fs::write(b.dir.path().join(REPLAY_MARKER), b"").unwrap();
    let next = "Journal.2026-03-09T100001.01.log";
    b.fixture("two_cmdrs.01.log", next, 1..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 1);
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::Replay)
    );
    assert_eq!(b.transport.calls(), 0);
}

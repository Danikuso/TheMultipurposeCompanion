//! Guardas del router al encolar (C2), cursor y lotes.

use super::support::fixture_lines;
use super::support::router_bench::{BenchOptions, F1, F2, TWO_CMDRS, bench, early_activation};
use crate::idem_key;
use crate::router::REPLAY_MARKER;
use chrono::{TimeZone, Utc};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tmc_model::sync::{FeedOrigin, OutboxState, RowOrigin, SyncFeedEvent, SyncReason, SyncService};
use tmc_store::{CursorAdvance, NewOutboxRow};

const LEGACY: &str = "Journal.2026-03-07T090001.01.log";
const BETA: &str = "Journal.2026-03-08T090001.01.log";

/// Banco listo para enviar: juego en marcha y clave Inara de F1 activada antes del fixture.
fn ready() -> super::support::router_bench::Bench {
    let mut b = bench(BenchOptions::default());
    b.give_key(F1, early_activation());
    b.process(true);
    b
}

#[test]
fn legacy_blocks() {
    let mut b = ready();
    b.fixture("legacy.01.log", LEGACY, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.cursor(F1, LEGACY), Some(4));
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::Legacy)
    );
}

#[test]
fn beta_blocks() {
    let mut b = ready();
    b.fixture("beta.01.log", BETA, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::Beta)
    );
}

#[test]
fn catchup_non_recoverable_only_feeds_tracker() {
    let mut b = ready();
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=8, FeedOrigin::CatchUp);
    b.assert_nothing_enqueued();
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(b.cursor(F1, TWO_CMDRS), None);
    assert_eq!(b.cursor(F2, TWO_CMDRS), None);
    let snap = b.router.tracker().snapshot();
    assert_eq!(snap.cmdr_fid.as_deref(), Some(F2));
    assert_eq!(snap.system_name.as_deref(), Some("Testf"));
}

#[test]
fn live_with_game_stopped_blocks() {
    let mut b = bench(BenchOptions::default());
    b.give_key(F1, early_activation());
    b.process(false);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::GameNotRunning)
    );
}

#[test]
fn replay_marker_blocks() {
    let mut b = bench(BenchOptions {
        marker_at_start: true,
        ..BenchOptions::default()
    });
    b.give_key(F1, early_activation());
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::Replay)
    );
    assert!(!b.router.send_gates().send_allowed());
}

#[test]
fn e2e_simulation_blocks() {
    let mut b = bench(BenchOptions {
        e2e: true,
        ..BenchOptions::default()
    });
    b.give_key(F1, early_activation());
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::Replay)
    );
    assert!(!b.router.send_gates().send_allowed());
}

#[test]
fn online_off_blocks() {
    let mut b = ready();
    b.online.store(false, Ordering::SeqCst);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::OnlineOff)
    );
    assert_eq!(
        b.router.send_gates().blocked_reason(),
        Some(SyncReason::OnlineOff)
    );
}

#[test]
fn no_lease_blocks() {
    let mut b = ready();
    b.lease.store(false, Ordering::SeqCst);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::OtherInstance)
    );
    assert!(!b.router.send_gates().send_allowed());
}

#[test]
fn no_key_blocks() {
    let mut b = bench(BenchOptions::default());
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::NoKey)
    );
    assert_eq!(
        b.router.blocked(SyncService::Eddn, Some(F1)),
        Some(SyncReason::SwitchOff)
    );
}

#[test]
fn before_activation_blocks() {
    let mut b = bench(BenchOptions::default());
    // Activada a las 10:00; el fixture termina a las 09:40.
    b.give_key(F1, Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap());
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 0);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(b.router.blocked(SyncService::Inara, Some(F1)), None);
}

#[test]
fn no_fid_blocks() {
    let mut b = ready();
    let lines = fixture_lines("two_cmdrs.01.log");
    // Fileheader y FSDJump, sin Commander ni LoadGame.
    b.line(TWO_CMDRS, 1, &lines[0], FeedOrigin::Live);
    b.line(TWO_CMDRS, 4, &lines[3], FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 0);
    assert!(b.store.known_cmdrs().unwrap().is_empty());
    assert_eq!(b.cursor(F1, TWO_CMDRS), None);
}

#[test]
fn cursor_advances_even_when_blocked() {
    let mut b = ready();
    b.online.store(false, Ordering::SeqCst);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=3, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(3));
}

#[test]
fn allowed_live_line_enqueues_with_live_origin() {
    let mut b = ready();
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=3, FeedOrigin::CatchUp);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 4..=4, FeedOrigin::Live);
    let outcome = b.router.flush().unwrap();
    assert_eq!(outcome.inserted, 1);
    assert_eq!(b.pending(), 1);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(b.transport.calls(), 0);

    let rows = b
        .store
        .claim_ready(
            SyncService::Inara,
            None,
            10,
            Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap(),
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].origin, RowOrigin::Live);
    assert_eq!(rows[0].cmdr_fid, F1);
    assert_eq!(rows[0].kind, "FSDJump");
    assert_eq!(
        rows[0].event_ts,
        Utc.with_ymd_and_hms(2026, 3, 9, 9, 5, 0).unwrap()
    );

    // La misma clave de idempotencia se ignora: la fila se insertó con idem_key(.., file, 4, 0).
    let dup = b
        .store
        .enqueue_batch_with_cursor(
            &[NewOutboxRow {
                service: SyncService::Inara,
                cmdr_fid: F1.into(),
                idem_key: idem_key(SyncService::Inara, F1, TWO_CMDRS, 4, 0),
                kind: "FSDJump".into(),
                coalesce_key: None,
                event_ts: Utc.with_ymd_and_hms(2026, 3, 9, 9, 5, 0).unwrap(),
                payload: "{}".into(),
                origin: RowOrigin::Live,
                state: OutboxState::Pending,
                last_class: None,
            }],
            &[],
            Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap(),
        )
        .unwrap();
    assert_eq!(dup.ignored, 1);
    assert_eq!(dup.inserted, 0);
}

#[tokio::test(start_paused = true)]
async fn flush_by_rows_or_timer() {
    let mut b = ready();
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=3, FeedOrigin::CatchUp);
    let jump = &fixture_lines("two_cmdrs.01.log")[3];
    // 200 líneas permitidas → un vaciado sin esperar al temporizador.
    for n in 4..=203u64 {
        b.line(TWO_CMDRS, n, jump, FeedOrigin::Live);
    }
    assert_eq!(b.pending(), 200);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(203));
    assert_eq!(b.router.next_deadline(), None);

    // 1 línea más: queda en el lote hasta que vencen 250 ms.
    b.line(TWO_CMDRS, 204, jump, FeedOrigin::Live);
    assert!(b.router.next_deadline().is_some());
    tokio::time::advance(Duration::from_millis(249)).await;
    b.router.tick().unwrap();
    assert_eq!(b.pending(), 200);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(203));
    tokio::time::advance(Duration::from_millis(1)).await;
    b.router.tick().unwrap();
    assert_eq!(b.pending(), 201);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(204));
    assert_eq!(b.router.next_deadline(), None);
    assert_eq!(b.transport.calls(), 0);
}

#[test]
fn cmdr_switch_mid_file_attributes_rows_to_origin_fid() {
    let mut b = ready();
    b.give_key(F2, early_activation());
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=8, FeedOrigin::Live);
    b.router.flush().unwrap();
    let by_cmdr: Vec<(String, u64)> = b
        .store
        .pending_by_cmdr(SyncService::Inara)
        .unwrap()
        .into_iter()
        .map(|(fid, n, _)| (fid, n))
        .collect();
    // F1: Commander, LoadGame y FSDJump (líneas 2-4); F2: Commander, LoadGame, FSDJump, Shutdown (5-8).
    assert_eq!(by_cmdr, vec![(F1.to_string(), 3), (F2.to_string(), 4)]);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(b.cursor(F2, TWO_CMDRS), Some(8));
    let f1_rows = b
        .store
        .claim_ready(
            SyncService::Inara,
            Some(F1),
            10,
            Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap(),
        )
        .unwrap();
    let kinds: Vec<&str> = f1_rows.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, vec!["Commander", "LoadGame", "FSDJump"]);
    assert!(
        f1_rows
            .iter()
            .all(|r| r.event_ts < Utc.with_ymd_and_hms(2026, 3, 9, 9, 20, 12).unwrap())
    );
    assert_eq!(b.transport.calls(), 0);
}

#[test]
fn live_line_before_first_process_sample_is_held_not_dropped() {
    let mut b = bench(BenchOptions::default());
    b.give_key(F1, early_activation());
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=3, FeedOrigin::CatchUp);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 4..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 0);
    assert_eq!(b.cursor(F1, TWO_CMDRS), None);
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 0);

    b.process(true);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 1);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(b.transport.calls(), 0);
}

#[tokio::test(start_paused = true)]
async fn held_lines_are_evaluated_after_hold_timeout() {
    let mut b = bench(BenchOptions::default());
    b.give_key(F1, early_activation());
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.cursor(F1, TWO_CMDRS), None);
    tokio::time::advance(Duration::from_secs(10)).await;
    b.router.tick().unwrap();
    b.router.flush().unwrap();
    // Sin muestra de proceso tras 10 s: se evalúa como juego parado (no se encola, el cursor avanza).
    assert_eq!(b.pending(), 0);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
}

#[test]
fn replay_marker_created_after_start_blocks_from_next_file() {
    let mut b = ready();
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 3);

    std::fs::write(b.dir.path().join(REPLAY_MARKER), b"").unwrap();
    let next = "Journal.2026-03-09T100001.01.log";
    b.fixture("two_cmdrs.01.log", next, 1..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.pending(), 3);
    assert_eq!(b.cursor(F1, next), Some(4));
    assert_eq!(
        b.router.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::Replay)
    );
    assert_eq!(b.transport.calls(), 0);
}

#[test]
fn truncated_file_resets_cursor_then_processes_new_lines() {
    let mut b = ready();
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=4, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 3);

    // El archivo se trunca y vuelve a empezar: líneas 1-3 nuevas (más tarde).
    b.line(
        TWO_CMDRS,
        1,
        r#"{ "timestamp":"2026-03-09T11:00:01Z", "event":"Fileheader", "part":1, "Odyssey":true, "gameversion":"4.1.0.0", "build":"r999999/r0 " }"#,
        FeedOrigin::Live,
    );
    b.line(
        TWO_CMDRS,
        2,
        r#"{ "timestamp":"2026-03-09T11:00:05Z", "event":"Commander", "FID":"F0000001", "Name":"CMDR TESTER" }"#,
        FeedOrigin::Live,
    );
    b.line(
        TWO_CMDRS,
        3,
        r#"{ "timestamp":"2026-03-09T11:00:12Z", "event":"LoadGame", "FID":"F0000001", "Commander":"CMDR TESTER", "Odyssey":true, "gameversion":"4.1.0.0", "build":"r999999/r0 " }"#,
        FeedOrigin::Live,
    );
    b.router.flush().unwrap();
    // Sin reinicio, el cursor seguiría en 4 y las líneas 1-3 se saltarían.
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(3));
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 5);

    b.line(
        TWO_CMDRS,
        4,
        r#"{ "timestamp":"2026-03-09T11:05:00Z", "event":"FSDJump", "StarSystem":"Testg", "SystemAddress":10477373811, "StarPos":[1.0,2.0,3.0] }"#,
        FeedOrigin::Live,
    );
    b.line(
        TWO_CMDRS,
        5,
        r#"{ "timestamp":"2026-03-09T11:06:00Z", "event":"FSDJump", "StarSystem":"Testh", "SystemAddress":10477373812, "StarPos":[1.0,2.0,4.0] }"#,
        FeedOrigin::Live,
    );
    b.router.flush().unwrap();
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(5));
    assert_eq!(b.mapper_calls.load(Ordering::SeqCst), 7);
    // Las líneas 2-4 comparten clave de idempotencia con las anteriores (ignoradas); la 5 es nueva.
    assert_eq!(b.pending(), 4);
    assert_eq!(
        b.router.tracker().snapshot().system_name.as_deref(),
        Some("Testh")
    );
}

#[test]
fn first_cmdr_seen_takes_waiting_key_and_is_registered() {
    let mut b = bench(BenchOptions::default());
    seed_waiting_key(&b.store);
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=2, FeedOrigin::Live);
    assert_eq!(
        b.store.known_cmdrs().unwrap(),
        vec![(F1.to_string(), "CMDR TESTER".to_string())]
    );
    let cred = b
        .store
        .sync_credential(SyncService::Inara, F1)
        .unwrap()
        .expect("clave asignada al primer comandante");
    assert_eq!(
        cred.assignment,
        tmc_model::sync::KeyAssignment::AutoPendingConfirmation
    );
    assert!(b.store.unassigned_keys().unwrap().is_empty());
}

#[test]
fn replay_marker_never_registers_cmdrs() {
    let mut b = bench(BenchOptions {
        marker_at_start: true,
        ..BenchOptions::default()
    });
    seed_waiting_key(&b.store);
    b.process(true);
    b.fixture("two_cmdrs.01.log", TWO_CMDRS, 1..=3, FeedOrigin::Live);
    assert!(b.store.known_cmdrs().unwrap().is_empty());
    assert_eq!(b.store.unassigned_keys().unwrap().len(), 1);
}

/// Deja una clave Inara heredada en espera del primer comandante (caso b de la migración).
fn seed_waiting_key(store: &tmc_store::Store) {
    store
        .set_account_credential(&tmc_store::AccountCredentialRecord {
            service: "inara".into(),
            api_key: Some("LEGACYKEY01".into()),
            cmdr_name: Some("CMDR TESTER".into()),
            token_json: None,
            salt: None,
            updated_at: "2026-03-01T00:00:00Z".into(),
        })
        .unwrap();
    let report = store
        .migrate_legacy_credentials(SyncService::Inara, Utc::now())
        .unwrap();
    assert_eq!(report.waiting, vec![SyncService::Inara]);
}

#[test]
fn cursor_reset_is_sent_before_new_advances() {
    // Contrato del almacén que usa el router en el truncado: 0 reinicia y después avanza.
    let store = tmc_store::Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap();
    let adv = |n| CursorAdvance {
        cmdr_fid: F1.into(),
        journal_file: TWO_CMDRS.into(),
        last_line: n,
    };
    store
        .enqueue_batch_with_cursor(&[], &[adv(9)], now)
        .unwrap();
    store
        .enqueue_batch_with_cursor(&[], &[adv(0), adv(2)], now)
        .unwrap();
    assert_eq!(store.sync_cursor(F1, TWO_CMDRS).unwrap(), Some(2));
}

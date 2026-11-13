//! Modo recuperable (§5.4, C3b a–e): relectura del vigente y diario anterior de la misma sesión.

use super::support::router_bench::{
    Bench, BenchOptions, F1, F2, TWO_CMDRS, bench, early_activation,
};
use crate::idem_key;
use crate::recovery::{Recovery, same_session};
use crate::router::EDDN_ENABLED_KEY;
use std::collections::BTreeSet;
use tmc_model::sync::{FeedOrigin, SyncFeedEvent, SyncService};

/// Nombres destino de los fixtures (tests/fixtures/sync/core/README.md).
const A1: &str = "Journal.2026-03-05T180001.01.log";
const A2: &str = "Journal.2026-03-05T180001.02.log";
const A3: &str = "Journal.2026-03-05T180001.03.log";
const OTHER: &str = "Journal.2026-03-06T090001.01.log";
const LEGACY: &str = "Journal.2026-03-07T090001.01.log";
const BETA: &str = "Journal.2026-03-08T090001.01.log";

fn ready() -> Bench {
    let mut b = bench(BenchOptions::default());
    b.give_key(F1, early_activation());
    b.process(true);
    b
}

fn header(ts: &str, part: u32, version: &str) -> String {
    format!(
        r#"{{ "timestamp":"{ts}", "event":"Fileheader", "part":{part}, "language":"English/UK", "Odyssey":true, "gameversion":"{version}", "build":"r999999/r0 " }}"#
    )
}

fn commander(ts: &str, fid: &str) -> String {
    format!(r#"{{ "timestamp":"{ts}", "event":"Commander", "FID":"{fid}", "Name":"CMDR TESTER" }}"#)
}

fn load_game(ts: &str, fid: &str) -> String {
    format!(
        r#"{{ "timestamp":"{ts}", "event":"LoadGame", "FID":"{fid}", "Commander":"CMDR TESTER", "Horizons":true, "Odyssey":true, "Ship":"python", "ShipID":7, "GameMode":"Solo", "Credits":1000, "gameversion":"4.1.0.0", "build":"r999999/r0 " }}"#
    )
}

fn jump(ts: &str, n: u64) -> String {
    format!(
        r#"{{ "timestamp":"{ts}", "event":"FSDJump", "StarSystem":"Sys{n}", "SystemAddress":{n}, "StarPos":[1.0,2.0,3.0], "JumpDist":10.5 }}"#
    )
}

/// Claves Inara de F1 para `file` y las líneas dadas.
fn inara_keys(fid: &str, file: &str, lines: impl IntoIterator<Item = u64>) -> BTreeSet<String> {
    lines
        .into_iter()
        .map(|l| idem_key(SyncService::Inara, fid, file, l, 0))
        .collect()
}

fn keys_of(b: &Bench) -> BTreeSet<String> {
    b.rows().into_iter().map(|r| r.idem_key).collect()
}

/// Arranque completo: recuperación del anterior, relectura del vigente y fin de relectura.
fn start(b: &mut Bench, current: &str) {
    b.router.recover_previous_session().unwrap();
    b.catch_up_file(current);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
}

#[test]
fn same_session_rule_on_fixtures() {
    let b = bench(BenchOptions::default());
    b.write_fixture("session_a.01.log", A1);
    b.write_fixture("session_a.02.log", A2);
    b.write_fixture("other_session.01.log", OTHER);
    b.write_fixture("legacy.01.log", LEGACY);
    let p = |n: &str| b.dir.path().join(n);
    assert!(same_session(&p(A1), &p(A2)));
    assert!(!same_session(&p(A2), &p(OTHER)));
    assert!(!same_session(&p(OTHER), &p(LEGACY)));
    // Solo por `Continued`, aunque el nombre no lo diga.
    let renamed = "Journal.2026-03-05T190000.01.log";
    b.write_fixture("session_a.02.log", renamed);
    assert!(same_session(&p(A1), &p(renamed)));
    // Por nombre (`part` consecutivo) aunque el anterior no termine en `Continued`.
    b.write_fixture("session_a.02.log", A3);
    assert!(same_session(&p(A2), &p(A3)));
    // `part` no consecutivo y sin `Continued`: sesión distinta (falla cerrado).
    assert!(!same_session(&p(OTHER), &p(A3)));
    // Nombres inválidos.
    assert!(!same_session(&p("x.log"), &p(A2)));
}

#[test]
fn lines_after_cursor_are_recovered_with_live_idem_key() {
    let mut b = ready();
    let mut lines = vec![
        header("2026-03-05T18:10:01Z", 2, "4.1.0.0"),
        commander("2026-03-05T18:10:02Z", F1),
        load_game("2026-03-05T18:10:03Z", F1),
    ];
    for n in 4..=15u64 {
        lines.push(jump(&format!("2026-03-05T18:{:02}:00Z", 10 + n), n));
    }
    b.write_lines(A2, &lines);
    b.set_cursor(F1, A2, 10);

    start(&mut b, A2);
    let outcome = b.router.flush().unwrap();
    assert_eq!(outcome.inserted, 5);
    assert_eq!(outcome.recovered, 5);
    let rows = b.rows();
    assert_eq!(rows.len(), 5);
    assert!(rows.iter().all(|r| r.origin == "recovered"));
    assert!(rows.iter().all(|r| r.state == "pending"));
    // Misma clave de idempotencia que daría la línea en vivo (C3b a).
    assert_eq!(keys_of(&b), inara_keys(F1, A2, 11..=15));
    assert_eq!(b.cursor(F1, A2), Some(15));
    assert_eq!(b.transport.calls(), 0);
}

#[test]
fn previous_file_only_if_same_session() {
    // Caso 1: session_a.01 + session_a.02 → se recupera .01 (sin cursor: desde el principio).
    let mut b = ready();
    b.write_fixture("session_a.01.log", A1);
    b.write_fixture("session_a.02.log", A2);
    start(&mut b, A2);
    let outcome = b.router.flush().unwrap();
    let keys = keys_of(&b);
    assert!(inara_keys(F1, A1, 2..=6).is_subset(&keys));
    // El Fileheader de .02 hereda el FID de .01 (misma sesión).
    assert!(inara_keys(F1, A2, 1..=5).is_subset(&keys));
    assert_eq!(keys.len(), 10);
    assert_eq!(outcome.recovered, 10);
    assert_eq!(b.cursor(F1, A1), Some(6));
    assert_eq!(b.cursor(F1, A2), Some(5));

    // Caso 2: session_a.02 + other_session.01 → el vigente es other y .02 no es de su sesión.
    let mut b = ready();
    b.write_fixture("session_a.02.log", A2);
    b.write_fixture("other_session.01.log", OTHER);
    let plan = Recovery::new(b.dir.path().to_path_buf())
        .plan()
        .unwrap()
        .unwrap();
    assert_eq!(plan.current, OTHER);
    assert_eq!(plan.previous, None);
    start(&mut b, OTHER);
    b.router.flush().unwrap();
    assert_eq!(b.cursor(F1, A2), None);
    assert!(
        b.rows()
            .iter()
            .all(|r| !inara_keys(F1, A2, 1..=5).contains(&r.idem_key))
    );
    assert_eq!(keys_of(&b), inara_keys(F1, OTHER, 2..=5));
}

#[test]
fn never_older_than_previous() {
    let mut b = ready();
    b.write_fixture("session_a.01.log", A1);
    b.write_fixture("session_a.02.log", A2);
    b.write_lines(
        A3,
        &[
            header("2026-03-05T18:20:01Z", 3, "4.1.0.0"),
            jump("2026-03-05T18:25:00Z", 30),
        ],
    );
    start(&mut b, A3);
    b.router.flush().unwrap();
    let keys = keys_of(&b);
    assert_eq!(b.cursor(F1, A1), None);
    assert!(keys.is_disjoint(&inara_keys(F1, A1, 1..=6)));
    assert!(inara_keys(F1, A2, 2..=5).is_subset(&keys));
    // .03 no tiene Commander: hereda F1 de .02.
    assert!(inara_keys(F1, A3, 1..=2).is_subset(&keys));
    assert_eq!(keys.len(), 6);
}

#[test]
fn before_activation_not_enqueued_and_too_old_is_expired() {
    // EDDN encendido a las 10:00: la línea de las 09:59 no se encola, la de las 10:01 sí.
    let mut b = bench(BenchOptions::default());
    b.store.kv_set(EDDN_ENABLED_KEY, "2026-03-09T10:00:00Z");
    let file = "Journal.2026-03-09T094500.01.log";
    b.write_lines(
        file,
        &[
            header("2026-03-09T09:45:00Z", 1, "4.1.0.0"),
            commander("2026-03-09T09:45:05Z", F1),
            load_game("2026-03-09T09:45:10Z", F1),
            jump("2026-03-09T09:59:00Z", 1),
            jump("2026-03-09T10:01:00Z", 2),
        ],
    );
    start(&mut b, file);
    b.router.flush().unwrap();
    let rows = b.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].service, "eddn");
    assert_eq!(rows[0].state, "pending");
    assert_eq!(rows[0].origin, "recovered");
    assert_eq!(
        rows[0].idem_key,
        idem_key(SyncService::Eddn, F1, file, 5, 0)
    );
    assert_eq!(b.cursor(F1, file), Some(5));

    // Línea EDDN de hace 25 h (máx. 24 h): fila visible en estado expired y contada.
    let mut b = bench(BenchOptions::default());
    b.store.kv_set(EDDN_ENABLED_KEY, "2026-03-01T00:00:00Z");
    let file = "Journal.2026-03-08T105000.01.log";
    b.write_lines(
        file,
        &[
            header("2026-03-08T10:50:00Z", 1, "4.1.0.0"),
            commander("2026-03-08T10:50:05Z", F1),
            load_game("2026-03-08T10:50:10Z", F1),
            jump("2026-03-08T11:00:00Z", 1),
        ],
    );
    b.set_cursor(F1, file, 3);
    start(&mut b, file);
    b.router.flush().unwrap();
    let rows = b.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, "expired");
    assert_eq!(rows[0].origin, "recovered");
    let counts = b.store.sync_counts(SyncService::Eddn, F1).unwrap();
    assert_eq!(counts.expired_total, 1);
    assert_eq!(counts.pending, 0);
}

#[test]
fn replay_marker_or_legacy_or_beta_file_recovers_nothing() {
    // Marcador de replay: ni el anterior ni el vigente.
    let mut b = bench(BenchOptions {
        marker_at_start: true,
        ..BenchOptions::default()
    });
    b.give_key(F1, early_activation());
    b.write_fixture("session_a.01.log", A1);
    b.write_fixture("session_a.02.log", A2);
    start(&mut b, A2);
    b.assert_nothing_enqueued();

    // Galaxia Legacy y beta, determinadas por el propio archivo.
    for (fixture, dest) in [("legacy.01.log", LEGACY), ("beta.01.log", BETA)] {
        let mut b = ready();
        b.write_fixture(fixture, dest);
        start(&mut b, dest);
        b.assert_nothing_enqueued();
        assert_eq!(b.cursor(F1, dest), Some(4));
    }
}

#[test]
fn abrupt_close_before_flush_recovers_unflushed_lines() {
    let mut b = ready();
    b.write_fixture("session_a.02.log", A2);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    b.file_lines(A2, 1..=2, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.cursor(F1, A2), Some(2));
    // Tres líneas procesadas sin vaciar y cierre abrupto.
    b.file_lines(A2, 3..=5, FeedOrigin::Live);
    b.restart();
    assert_eq!(b.rows().len(), 1);

    start(&mut b, A2);
    let outcome = b.router.flush().unwrap();
    assert_eq!(outcome.recovered, 3);
    let rows = b.rows();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].origin, "live");
    assert!(rows[1..].iter().all(|r| r.origin == "recovered"));
    assert_eq!(keys_of(&b), inara_keys(F1, A2, 2..=5));
    assert_eq!(b.cursor(F1, A2), Some(5));
}

#[test]
fn two_cmdrs_same_file_each_line_checked_against_its_owner_cursor() {
    // Cursor F1 = 5 (todas sus líneas ya procesadas), F2 sin cursor.
    let mut b = ready();
    b.give_key(F2, early_activation());
    b.write_fixture("two_cmdrs.01.log", TWO_CMDRS);
    b.set_cursor(F1, TWO_CMDRS, 5);
    start(&mut b, TWO_CMDRS);
    b.router.flush().unwrap();
    assert_eq!(keys_of(&b), inara_keys(F2, TWO_CMDRS, 5..=8));
    assert_eq!(b.cursor(F2, TWO_CMDRS), Some(8));

    // Cursor F1 = 3: su línea 4 sí es posterior a su cursor.
    let mut b = ready();
    b.give_key(F2, early_activation());
    b.write_fixture("two_cmdrs.01.log", TWO_CMDRS);
    b.set_cursor(F1, TWO_CMDRS, 3);
    start(&mut b, TWO_CMDRS);
    b.router.flush().unwrap();
    let mut expected = inara_keys(F2, TWO_CMDRS, 5..=8);
    expected.extend(inara_keys(F1, TWO_CMDRS, [4]));
    assert_eq!(keys_of(&b), expected);
    assert_eq!(b.cursor(F1, TWO_CMDRS), Some(4));
}

#[test]
fn watcher_fallback_old_file_lines_are_tracker_only() {
    let mut b = ready();
    b.write_fixture("other_session.01.log", OTHER);
    let current = "Journal.2026-03-09T110001.01.log";
    b.write_lines(
        current,
        &[
            header("2026-03-09T11:00:01Z", 1, "4.1.0.0"),
            jump("2026-03-09T11:05:00Z", 7),
        ],
    );
    b.router.recover_previous_session().unwrap();
    // El watcher antepone el diario antiguo (otra sesión) como relectura.
    b.catch_up_file(OTHER);
    let snap = b.router.tracker().snapshot();
    assert_eq!(snap.cmdr_fid.as_deref(), Some(F1));
    assert_eq!(snap.system_name.as_deref(), Some("Testd"));
    b.catch_up_file(current);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    b.assert_nothing_enqueued();
    assert_eq!(b.cursor(F1, OTHER), None);
    assert_eq!(b.rows().len(), 0);
}

#[test]
fn new_file_during_recovery_is_processed_after_in_order() {
    let mut b = ready();
    b.write_fixture("session_a.01.log", A1);
    b.write_fixture("session_a.02.log", A2);
    let plan = Recovery::new(b.dir.path().to_path_buf())
        .plan()
        .unwrap()
        .unwrap();
    assert_eq!(plan.current, A2);
    // Aparece .03 mientras se recupera .01: no cambia el plan y llega después como `Live`.
    b.write_lines(
        A3,
        &[
            header("2026-03-05T18:20:01Z", 3, "4.1.0.0"),
            commander("2026-03-05T18:20:02Z", F1),
            jump("2026-03-05T18:25:00Z", 30),
        ],
    );
    b.router.recover_with(&plan).unwrap();
    b.catch_up_file(A2);
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    b.file_lines(A3, 1..=3, FeedOrigin::Live);
    b.router.flush().unwrap();

    let rows = b.rows();
    let ids = |file: &str, lines: std::ops::RangeInclusive<u64>| -> Vec<i64> {
        let keys = inara_keys(F1, file, lines);
        rows.iter()
            .filter(|r| keys.contains(&r.idem_key))
            .map(|r| r.id)
            .collect()
    };
    let a1 = ids(A1, 1..=6);
    let a2 = ids(A2, 1..=5);
    let a3 = ids(A3, 1..=3);
    assert_eq!((a1.len(), a2.len(), a3.len()), (5, 5, 3));
    assert!(a1.iter().max() < a2.iter().min());
    assert!(a2.iter().max() < a3.iter().min());
    assert!(
        rows.iter()
            .filter(|r| a3.contains(&r.id))
            .all(|r| r.origin == "live")
    );
}

#[test]
fn no_catchup_done_means_no_recovery() {
    let mut b = ready();
    b.write_fixture("session_a.01.log", A1);
    b.write_fixture("session_a.02.log", A2);
    b.router.recover_previous_session().unwrap();
    b.file_lines(A2, 1..=3, FeedOrigin::CatchUp);
    // Sin `CatchUpDone`, llegan líneas en vivo: se procesan, pero no hay recuperación.
    b.file_lines(A2, 4..=5, FeedOrigin::Live);
    b.router.flush().unwrap();
    let rows = b.rows();
    assert_eq!(keys_of(&b), inara_keys(F1, A2, 4..=5));
    assert!(rows.iter().all(|r| r.origin == "live"));
    assert_eq!(b.cursor(F1, A1), None);
    assert_eq!(b.cursor(F1, A2), Some(5));
}

#[test]
fn part02_without_commander_inherits_fid() {
    let mut b = ready();
    b.write_fixture("session_a.01.log", A1);
    b.write_lines(
        A2,
        &[
            header("2026-03-05T18:10:01Z", 2, "4.1.0.0"),
            jump("2026-03-05T18:15:00Z", 40),
        ],
    );
    b.send(SyncFeedEvent::CatchUpDone, FeedOrigin::Live);
    b.file_lines(A1, 1..=6, FeedOrigin::Live);
    b.file_lines(A2, 1..=2, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.cursor(F1, A2), Some(2));
    assert!(inara_keys(F1, A2, 1..=2).is_subset(&keys_of(&b)));
    assert_eq!(
        b.router.tracker().snapshot().system_name.as_deref(),
        Some("Sys40")
    );

    // Otra sesión (part 01 sin `Continued` previo) no hereda: sin FID no se encola.
    let other = "Journal.2026-03-06T090001.01.log";
    b.write_lines(
        other,
        &[
            header("2026-03-06T09:00:01Z", 1, "4.1.0.0"),
            jump("2026-03-06T09:05:00Z", 41),
        ],
    );
    b.file_lines(other, 1..=2, FeedOrigin::Live);
    b.router.flush().unwrap();
    assert_eq!(b.router.tracker().snapshot().cmdr_fid, None);
    assert_eq!(b.cursor(F1, other), None);
}

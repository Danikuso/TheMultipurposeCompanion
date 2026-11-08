//! Utilidades compartidas por los tests del crate.

use std::path::PathBuf;
use tmc_model::events::{JournalEvent, parse_line};

/// Ruta a un fixture sintético de `tests/fixtures/sync/core/`.
pub fn core_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/sync/core")
        .join(name)
}

/// Analiza una línea de diario de test.
pub fn ev(line: &str) -> JournalEvent {
    parse_line(line).expect("línea de test válida")
}

/// Líneas no vacías de un fixture del núcleo.
pub fn fixture_lines(name: &str) -> Vec<String> {
    std::fs::read_to_string(core_fixture(name))
        .expect("fixture legible")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// Banco de pruebas del router: carpeta de diario temporal, almacén en memoria y guardas.
pub mod router_bench {
    use super::{ev, fixture_lines};
    use crate::clock::TestClock;
    use crate::router::{
        EnvGates, LiveSignalPolicy, MappedInput, MappedRow, Router, RouterConfig, ServiceMapper,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;
    use tmc_model::state::Galaxy;
    use tmc_model::sync::{
        FeedOrigin, KeyAssignment, OutboxState, SyncFeed, SyncFeedEvent, SyncService,
    };
    use tmc_net::testing::ScriptedTransport;
    use tmc_store::{Store, SyncCredential};

    pub const F1: &str = "F0000001";
    pub const F2: &str = "F0000002";
    /// Nombre destino de `two_cmdrs.01.log`.
    pub const TWO_CMDRS: &str = "Journal.2026-03-09T090001.01.log";

    /// Activación anterior a todos los fixtures del 2026-03-09.
    pub fn early_activation() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap()
    }

    /// Mapeador de test: una fila por línea y contador de llamadas.
    pub struct TestMapper {
        pub service: SyncService,
        pub calls: Arc<AtomicUsize>,
    }

    impl ServiceMapper for TestMapper {
        fn service(&self) -> SyncService {
            self.service
        }

        fn map(&mut self, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
            self.calls.fetch_add(1, Ordering::SeqCst);
            out.push(MappedRow {
                kind: input.ev.name.clone(),
                sub: 0,
                payload: serde_json::json!({ "event": input.ev.name, "line": input.line }),
                coalesce_key: None,
                state: OutboxState::Pending,
                class: None,
            });
        }
    }

    pub struct Bench {
        pub dir: tempfile::TempDir,
        pub store: Store,
        pub online: Arc<AtomicBool>,
        pub lease: Arc<AtomicBool>,
        pub transport: Arc<ScriptedTransport>,
        pub mapper_calls: Arc<AtomicUsize>,
        pub router: Router,
    }

    pub struct BenchOptions {
        pub policy: LiveSignalPolicy,
        pub e2e: bool,
        pub marker_at_start: bool,
    }

    impl Default for BenchOptions {
        fn default() -> Self {
            Self {
                policy: LiveSignalPolicy::ProcessCheck,
                e2e: false,
                marker_at_start: false,
            }
        }
    }

    pub fn config(dir: &std::path::Path, opts: &BenchOptions) -> RouterConfig {
        RouterConfig {
            journal_dir: dir.to_path_buf(),
            env: EnvGates {
                e2e_simulation: opts.e2e,
            },
            live_signal: opts.policy,
            flush_every: Duration::from_millis(250),
            flush_rows: 200,
            hold_before_process: (200, Duration::from_secs(10)),
        }
    }

    pub fn bench(opts: BenchOptions) -> Bench {
        let dir = tempfile::tempdir().unwrap();
        if opts.marker_at_start {
            std::fs::write(dir.path().join(crate::router::REPLAY_MARKER), b"").unwrap();
        }
        let store = Store::open_in_memory().unwrap();
        let online = Arc::new(AtomicBool::new(true));
        let lease = Arc::new(AtomicBool::new(true));
        let calls = Arc::new(AtomicUsize::new(0));
        let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 3, 9, 12, 0, 0).unwrap());
        let router = Router::new(
            store.clone(),
            clock,
            online.clone(),
            lease.clone(),
            config(dir.path(), &opts),
            vec![Box::new(TestMapper {
                service: SyncService::Inara,
                calls: calls.clone(),
            })],
        );
        Bench {
            dir,
            store,
            online,
            lease,
            transport: ScriptedTransport::new(vec![]),
            mapper_calls: calls,
            router,
        }
    }

    impl Bench {
        /// Clave Inara confirmada y activación para un comandante.
        pub fn give_key(&self, fid: &str, activated_at: DateTime<Utc>) {
            self.store
                .put_sync_credential(&SyncCredential {
                    service: SyncService::Inara,
                    cmdr_fid: fid.into(),
                    cmdr_name: "CMDR TESTER".into(),
                    api_key: "TESTKEY0001".into(),
                    created_at: "2026-03-01T00:00:00Z".into(),
                    verified_at: None,
                    assignment: KeyAssignment::Confirmed,
                })
                .unwrap();
            self.store
                .set_activation(SyncService::Inara, fid, activated_at)
                .unwrap();
        }

        pub fn send(&mut self, event: SyncFeedEvent, origin: FeedOrigin) {
            self.router
                .handle(SyncFeed {
                    event,
                    origin,
                    galaxy: Galaxy::Live,
                    game_running: false,
                })
                .unwrap();
        }

        pub fn line(&mut self, file: &str, line: u64, text: &str, origin: FeedOrigin) {
            self.send(
                SyncFeedEvent::Journal {
                    ev: ev(text),
                    file: file.into(),
                    line,
                },
                origin,
            );
        }

        /// Envía las líneas `range` (1-based, inclusivo) de un fixture.
        pub fn fixture(
            &mut self,
            fixture: &str,
            file: &str,
            range: std::ops::RangeInclusive<usize>,
            origin: FeedOrigin,
        ) {
            let lines = fixture_lines(fixture);
            for n in range {
                self.line(file, n as u64, &lines[n - 1], origin);
            }
        }

        pub fn process(&mut self, running: bool) {
            self.send(SyncFeedEvent::GameProcess { running }, FeedOrigin::Live);
        }

        /// Filas pendientes de todos los servicios y comandantes.
        pub fn pending(&self) -> u64 {
            SyncService::ALL
                .into_iter()
                .flat_map(|s| self.store.pending_by_cmdr(s).unwrap())
                .map(|(_, n, _)| n)
                .sum()
        }

        pub fn cursor(&self, fid: &str, file: &str) -> Option<u64> {
            self.store.sync_cursor(fid, file).unwrap()
        }

        /// Vacía el lote y afirma que no se encoló nada ni se tocó la red (C2).
        pub fn assert_nothing_enqueued(&mut self) {
            self.router.flush().unwrap();
            assert_eq!(self.pending(), 0);
            assert_eq!(self.transport.calls(), 0);
        }
    }
}

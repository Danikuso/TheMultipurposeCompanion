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
    use crate::clock::{Clock, TestClock};
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
    use tmc_store::{CursorAdvance, Store, SyncCredential};

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

    /// Fila de la bandeja tal como está en SQLite.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RowView {
        pub id: i64,
        pub service: String,
        pub cmdr_fid: String,
        pub idem_key: String,
        pub kind: String,
        pub origin: String,
        pub state: String,
    }

    pub struct Bench {
        pub dir: tempfile::TempDir,
        pub clock: Arc<TestClock>,
        pub opts: BenchOptions,
        pub store: Store,
        pub online: Arc<AtomicBool>,
        pub lease: Arc<AtomicBool>,
        pub transport: Arc<ScriptedTransport>,
        pub mapper_calls: Arc<AtomicUsize>,
        pub router: Router,
    }

    #[derive(Clone)]
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

    /// Mapeadores de test (Inara y EDDN) que comparten el contador de llamadas.
    fn mappers(calls: &Arc<AtomicUsize>) -> Vec<Box<dyn ServiceMapper>> {
        [SyncService::Inara, SyncService::Eddn]
            .into_iter()
            .map(|service| {
                Box::new(TestMapper {
                    service,
                    calls: calls.clone(),
                }) as Box<dyn ServiceMapper>
            })
            .collect()
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
            clock.clone(),
            online.clone(),
            lease.clone(),
            config(dir.path(), &opts),
            mappers(&calls),
        );
        Bench {
            dir,
            clock,
            opts,
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

        /// Suelta el router sin vaciar el lote (cierre abrupto) y crea otro sobre el mismo almacén.
        pub fn restart(&mut self) {
            self.router = Router::new(
                self.store.clone(),
                self.clock.clone(),
                self.online.clone(),
                self.lease.clone(),
                config(self.dir.path(), &self.opts),
                mappers(&self.mapper_calls),
            );
        }

        /// Copia un fixture del núcleo a la carpeta de diario con su nombre destino.
        pub fn write_fixture(&self, fixture: &str, dest: &str) {
            std::fs::copy(super::core_fixture(fixture), self.dir.path().join(dest)).unwrap();
        }

        /// Escribe un diario sintético (una línea por elemento, terminadas en LF).
        pub fn write_lines(&self, dest: &str, lines: &[String]) {
            let mut text = lines.join("\n");
            text.push('\n');
            std::fs::write(self.dir.path().join(dest), text).unwrap();
        }

        /// Relectura del watcher: todas las líneas del archivo `dest` de la carpeta como `CatchUp`.
        pub fn catch_up_file(&mut self, dest: &str) {
            let text = std::fs::read_to_string(self.dir.path().join(dest)).unwrap();
            let lines: Vec<String> = text.lines().map(str::to_owned).collect();
            for (i, text) in lines.iter().enumerate() {
                self.line(dest, i as u64 + 1, text, FeedOrigin::CatchUp);
            }
        }

        /// Líneas `range` (1-based, inclusivo) del archivo `dest` de la carpeta con el origen dado.
        pub fn file_lines(
            &mut self,
            dest: &str,
            range: std::ops::RangeInclusive<usize>,
            origin: FeedOrigin,
        ) {
            let text = std::fs::read_to_string(self.dir.path().join(dest)).unwrap();
            let lines: Vec<String> = text.lines().map(str::to_owned).collect();
            for n in range {
                self.line(dest, n as u64, &lines[n - 1], origin);
            }
        }

        /// Fija un cursor confirmado como si una ejecución anterior lo hubiera vaciado.
        pub fn set_cursor(&self, fid: &str, file: &str, line: u64) {
            self.store
                .enqueue_batch_with_cursor(
                    &[],
                    &[CursorAdvance {
                        cmdr_fid: fid.into(),
                        journal_file: file.into(),
                        last_line: line,
                    }],
                    self.clock.now_utc(),
                )
                .unwrap();
        }

        /// Todas las filas de la bandeja, por id.
        pub fn rows(&self) -> Vec<RowView> {
            self.store
                .query_json(
                    "SELECT id, service, cmdr_fid, idem_key, kind, origin, state FROM sync_outbox ORDER BY id",
                    &[],
                )
                .unwrap()
                .into_iter()
                .map(|r| {
                    let s = |i: usize| r[i].as_str().unwrap().to_string();
                    RowView {
                        id: r[0].as_i64().unwrap(),
                        service: s(1),
                        cmdr_fid: s(2),
                        idem_key: s(3),
                        kind: s(4),
                        origin: s(5),
                        state: s(6),
                    }
                })
                .collect()
        }

        /// Vacía el lote y afirma que no se encoló nada ni se tocó la red (C2).
        pub fn assert_nothing_enqueued(&mut self) {
            self.router.flush().unwrap();
            assert_eq!(self.pending(), 0);
            assert_eq!(self.transport.calls(), 0);
        }
    }
}

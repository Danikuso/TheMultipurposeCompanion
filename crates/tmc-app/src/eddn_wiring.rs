//! Cableado de EDDN: observa el núcleo y los ajustes, alimenta el filtro y
//! escribe las coordenadas propias en vivo.
//!
//! Solo conecta con el juego en marcha y una señal explícita del núcleo que
//! confirme que el diario es en vivo, además de un centro conocido.

use crate::eddn_service::{eddn_wanted, spawn_with_clock};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tmc_core::{CoreHandle, CoreMessage};
use tmc_eddn::{Clock, EddnFilter, EddnMode, FrameSource, SystemClock, ZmqSource};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::Galaxy;
use tmc_store::Store;
use tokio::sync::{broadcast, broadcast::error::RecvError, mpsc, watch};

/// Módulos cuya actividad mantiene EDDN conectado en modo `OnDemand`.
const EDDN_MODULES: [&str; 2] = ["trade", "mining"];
/// Modo elegido por el spike de F3.
const EDDN_MODE: EddnMode = EddnMode::OnDemand;
const GRACE: chrono::Duration = chrono::Duration::seconds(60);

/// Sistema propio: nombre, `SystemAddress` y `StarPos`.
type OwnSystem = (String, u64, [f64; 3]);
type Topics = BTreeMap<String, serde_json::Value>;
pub(crate) type Snapshot =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Topics> + Send>> + Send + Sync>;
pub(crate) type NowFn = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Lo que se sabe del núcleo para decidir el filtro de EDDN.
struct EddnView {
    settings: ModuleSettings,
    galaxy: Galaxy,
    /// EDDN solo con el juego en marcha y diario confirmado en vivo.
    game_running: bool,
    replaying: bool,
    system: String,
    system_address: u64,
    star_pos: [f64; 3],
    active: bool,
    inactive_since: Option<DateTime<Utc>>,
    /// Se perdieron mensajes del núcleo: apagado hasta ver un `core.game` nuevo.
    stale: bool,
}

impl EddnView {
    fn new(settings: ModuleSettings) -> Self {
        Self {
            settings,
            galaxy: Galaxy::Unknown,
            game_running: false,
            replaying: true,
            system: String::new(),
            system_address: 0,
            star_pos: [0.0; 3],
            active: false,
            inactive_since: None,
            stale: false,
        }
    }

    /// Motivo fijo del estado del filtro (solo diagnóstico, sin datos del jugador).
    #[cfg(debug_assertions)]
    fn reason(&self, filter: &EddnFilter) -> &'static str {
        if filter.enabled {
            "enabled"
        } else if self.stale {
            "stale"
        } else if !self.game_running {
            "game_not_running"
        } else if self.replaying {
            "replay"
        } else if filter.center.is_none() {
            "no_center"
        } else if self.galaxy != Galaxy::Live {
            "legacy"
        } else if !self.settings.online_services {
            "offline"
        } else {
            "inactive"
        }
    }

    fn center(&self) -> Option<[f64; 3]> {
        (!self.system.is_empty() && self.system_address != 0).then_some(self.star_pos)
    }

    fn filter(&self, now: DateTime<Utc>) -> EddnFilter {
        let center = self.center();
        EddnFilter {
            // Sin centro conocido no se filtraría por distancia: no se conecta.
            enabled: !self.stale
                && self.game_running
                && !self.replaying
                && center.is_some()
                && eddn_wanted(
                    &self.settings,
                    self.galaxy,
                    EDDN_MODE,
                    self.active,
                    self.inactive_since,
                    now,
                ),
            center,
            radius_ly: f64::from(self.settings.eddn_radius_ly),
        }
    }

    /// Aplica un `Update` (o una instantánea) del núcleo. Devuelve el sistema propio si cambió.
    fn apply(&mut self, topics: &Topics, now: DateTime<Utc>) -> Option<OwnSystem> {
        if let Some(g) = topics.get("core.game") {
            self.stale = false;
            self.replaying = g.get("replaying").and_then(|v| v.as_bool()) != Some(false);
            if let Some(r) = g.get("running").and_then(|v| v.as_bool()) {
                self.game_running = r;
            }
            self.galaxy = match g.get("galaxy").and_then(|v| v.as_str()) {
                Some("Live") => Galaxy::Live,
                Some("Legacy") => Galaxy::Legacy,
                _ => Galaxy::Unknown,
            };
        }
        if let Some(mods) = topics.get("core.modules").and_then(|v| v.as_array()) {
            let active = mods.iter().any(|m| {
                m.get("id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| EDDN_MODULES.contains(&id))
                    && matches!(
                        m.get("relevance").and_then(|v| v.as_str()),
                        Some("active" | "urgent")
                    )
            });
            if active != self.active {
                self.inactive_since = (!active).then_some(now);
                self.active = active;
            }
        }
        let loc = topics.get("core.location")?;
        let system = loc.get("system")?.as_str()?.to_owned();
        let address = loc.get("system_address").and_then(|v| v.as_u64())?;
        let pos = loc.get("star_pos")?.as_array()?;
        let pos = [
            pos.first()?.as_f64()?,
            pos.get(1)?.as_f64()?,
            pos.get(2)?.as_f64()?,
        ];
        let changed = system != self.system || address != self.system_address;
        self.system = system.clone();
        self.system_address = address;
        self.star_pos = pos;
        (changed && self.center().is_some() && pos.iter().all(|v| v.is_finite()))
            .then_some((system, address, pos))
    }
}

/// Alimenta el filtro de EDDN, escribe las coordenadas propias en vivo y lanza el servicio.
pub(crate) fn spawn_eddn_wiring(
    store: Store,
    core_tx: mpsc::Sender<SourceEvent>,
    core: CoreHandle,
    settings_rx: watch::Receiver<ModuleSettings>,
    url: String,
) {
    let rx = core.subscribe();
    let snapshot: Snapshot = Arc::new(move || {
        let core = core.clone();
        Box::pin(async move { core.snapshot().await })
    });
    match ZmqSource::new() {
        Ok(source) => spawn_wiring_with(
            store,
            core_tx,
            rx,
            settings_rx,
            snapshot,
            url,
            Box::new(source),
            Box::new(SystemClock),
            Arc::new(Utc::now),
        ),
        Err(error) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "eddn.transport.failed",
                serde_json::json!({"failed":true})
            );
            tracing::warn!(%error, "EDDN unavailable: could not create transport");
            // Sin transporte solo se mantienen las coordenadas propias.
            let view = EddnView::new(settings_rx.borrow().clone());
            let (filter_tx, _filter_rx) = watch::channel(view.filter(Utc::now()));
            let own_tx = spawn_coords_writer(store);
            tokio::spawn(run_loop(
                view,
                rx,
                settings_rx,
                snapshot,
                Arc::new(Utc::now),
                filter_tx,
                own_tx,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_wiring_with(
    store: Store,
    core_tx: mpsc::Sender<SourceEvent>,
    rx: broadcast::Receiver<CoreMessage>,
    mut settings_rx: watch::Receiver<ModuleSettings>,
    snapshot: Snapshot,
    url: String,
    source: Box<dyn FrameSource>,
    clock: Box<dyn Clock>,
    now_fn: NowFn,
) {
    let view = EddnView::new(settings_rx.borrow_and_update().clone());
    let (filter_tx, filter_rx) = watch::channel(view.filter(now_fn()));
    spawn_with_clock(
        store.clone(),
        core_tx,
        filter_rx,
        EDDN_MODE,
        source,
        url,
        clock,
    );
    let own_tx = spawn_coords_writer(store);
    tokio::spawn(run_loop(
        view,
        rx,
        settings_rx,
        snapshot,
        now_fn,
        filter_tx,
        own_tx,
    ));
}

/// Escritor de coordenadas propias: un `watch` fusiona cambios rápidos y el bucle
/// principal nunca espera a la base de datos.
fn spawn_coords_writer(store: Store) -> watch::Sender<Option<OwnSystem>> {
    let (tx, mut rx) = watch::channel::<Option<OwnSystem>>(None);
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let latest = rx.borrow_and_update().clone();
            if let Some((name, address, pos)) = latest {
                let st = store.clone();
                // Aunque EDDN esté desactivado: el diario propio alimenta `system_coords`.
                let _ = tokio::task::spawn_blocking(move || {
                    if let Err(error) = st.upsert_coords(&[(name, Some(address), pos)], Utc::now())
                    {
                        tmc_model::diagnostic!(
                            tmc_model::diagnostics::DiagnosticContext::State,
                            "eddn.coords.write_failed",
                            serde_json::json!({"failure_class":error.diagnostic_class()})
                        );
                        tracing::warn!(%error, "own system coords write failed");
                    }
                })
                .await;
            }
        }
    });
    tx
}

fn publish(view: &EddnView, tx: &watch::Sender<EddnFilter>, now: DateTime<Utc>) {
    let filter = view.filter(now);
    tx.send_if_modified(|f| {
        let changed = *f != filter;
        if changed {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::State,
                "eddn.filter.changed",
                serde_json::json!({"enabled":filter.enabled,"radius_ly":filter.radius_ly,"center_known":filter.center.is_some(),"reason":view.reason(&filter)})
            );
            *f = filter;
        }
        changed
    });
}

async fn run_loop(
    mut view: EddnView,
    mut rx: broadcast::Receiver<CoreMessage>,
    mut settings_rx: watch::Receiver<ModuleSettings>,
    snapshot: Snapshot,
    now_fn: NowFn,
    filter_tx: watch::Sender<EddnFilter>,
    own_tx: watch::Sender<Option<OwnSystem>>,
) {
    loop {
        // Despierta solo ante eventos, ajustes o el fin del margen de 60 s.
        let now = now_fn();
        let wake = match (view.active, view.inactive_since) {
            (false, Some(t)) if now - t < GRACE => {
                Some((GRACE - (now - t)).to_std().unwrap_or_default())
            }
            _ => None,
        };
        let mut own: Option<OwnSystem> = None;
        tokio::select! {
            msg = rx.recv() => match msg {
                Ok(CoreMessage::Update { topics, .. }) => own = view.apply(&topics, now_fn()),
                Ok(_) => {}
                Err(RecvError::Lagged(_)) => {
                    // Estado dudoso: se apaga ya y se rehace desde una instantánea.
                    view.stale = true;
                    publish(&view, &filter_tx, now_fn());
                    let topics = snapshot().await;
                    own = view.apply(&topics, now_fn());
                }
                Err(RecvError::Closed) => return,
            },
            changed = settings_rx.changed() => {
                if changed.is_err() {
                    return;
                }
                view.settings = settings_rx.borrow_and_update().clone();
            }
            _ = async {
                match wake {
                    Some(d) => tokio::time::sleep(d).await,
                    None => std::future::pending().await,
                }
            } => {}
        }
        if let Some(o) = own {
            own_tx.send_replace(Some(o));
        }
        publish(&view, &filter_tx, now_fn());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use tmc_eddn::{EddnError, FakeClock};

    fn topics(v: serde_json::Value) -> Topics {
        serde_json::from_value(v).unwrap()
    }

    fn live_active() -> serde_json::Value {
        json!({
            "core.game": {"running": true, "galaxy": "Live", "replaying": false},
            "core.location": {"system": "Lave", "system_address": 5, "star_pos": [1.0, 2.0, 3.0]},
            "core.modules": [{"id": "mining", "relevance": "active"}],
        })
    }

    #[test]
    fn replay_and_missing_signal_keep_eddn_off_even_with_game_running() {
        let now = Utc::now();
        let mut view = EddnView::new(ModuleSettings::default());
        let mut live = live_active();
        live["core.game"]
            .as_object_mut()
            .unwrap()
            .remove("replaying");
        view.apply(&topics(live.clone()), now);
        assert!(
            !view.filter(now).enabled,
            "missing replay signal must fail closed"
        );
        live["core.game"]["replaying"] = json!(true);
        view.apply(&topics(live.clone()), now);
        assert!(!view.filter(now).enabled);
        live["core.game"]["replaying"] = json!(false);
        view.apply(&topics(live.clone()), now);
        assert!(view.filter(now).enabled);
        // Recovery from a lag uses a snapshot and must preserve the replay guard.
        view.stale = true;
        live["core.game"]["replaying"] = json!(true);
        view.apply(&topics(live), now);
        assert!(!view.stale);
        assert!(!view.filter(now).enabled);
    }

    #[test]
    fn filter_follows_game_galaxy_location_and_activity() {
        let now = Utc::now();
        let mut v = EddnView::new(ModuleSettings::default());
        assert!(!v.filter(now).enabled);
        let own = v.apply(&topics(live_active()), now);
        assert_eq!(own, Some(("Lave".into(), 5, [1.0, 2.0, 3.0])));
        let f = v.filter(now);
        assert!(f.enabled);
        assert_eq!(f.center, Some([1.0, 2.0, 3.0]));
        assert_eq!(f.radius_ly, 200.0);
        v.apply(
            &topics(json!({"core.modules": [{"id": "mining", "relevance": "normal"}]})),
            now,
        );
        assert!(v.filter(now + chrono::Duration::seconds(59)).enabled);
        assert!(!v.filter(now + chrono::Duration::seconds(61)).enabled);
        v.apply(
            &topics(json!({"core.modules": [{"id": "mining", "relevance": "active"}]})),
            now,
        );
        assert!(v.filter(now).enabled);
        v.apply(
            &topics(json!({"core.game": {"running": true, "galaxy": "Legacy"}})),
            now,
        );
        assert!(!v.filter(now).enabled);
        v.apply(
            &topics(json!({"core.game": {"running": false, "galaxy": "Live"}})),
            now,
        );
        assert!(!v.filter(now).enabled);
    }

    #[test]
    fn lag_disables_until_a_fresh_core_game() {
        let now = Utc::now();
        let mut v = EddnView::new(ModuleSettings::default());
        v.apply(&topics(live_active()), now);
        assert!(v.filter(now).enabled);
        v.stale = true;
        assert!(!v.filter(now).enabled);
        // Una instantánea sin core.game no rehabilita.
        v.apply(&topics(json!({"core.modules": []})), now);
        assert!(!v.filter(now).enabled);
        // Tras el lag la galaxia pasó a Legacy: sigue apagado.
        v.apply(
            &topics(json!({"core.game": {"running": true, "galaxy": "Legacy"}})),
            now,
        );
        assert!(!v.stale);
        assert!(!v.filter(now).enabled);
    }

    #[derive(Default)]
    struct Counts {
        connects: AtomicU32,
        disconnects: AtomicU32,
    }
    struct Src(Arc<Counts>, FakeClock);
    impl FrameSource for Src {
        fn connect(&mut self, _: &str) -> Result<(), tmc_eddn::EddnError> {
            self.0.connects.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn recv(&mut self, t: Duration) -> Result<Option<Vec<u8>>, EddnError> {
            self.1.advance(t);
            std::thread::sleep(Duration::from_millis(2));
            Ok(None)
        }
        fn disconnect(&mut self) {
            self.0.disconnects.fetch_add(1, Ordering::SeqCst);
        }
    }

    async fn until(what: &str, f: impl Fn() -> bool) {
        let end = std::time::Instant::now() + Duration::from_secs(10);
        while !f() {
            assert!(std::time::Instant::now() < end, "{what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn own_coords_without_import_completion(lag: bool) {
        let store = Store::open_in_memory().unwrap();
        let settings = ModuleSettings {
            online_services: false,
            ..ModuleSettings::default()
        };
        let view = EddnView::new(settings.clone());
        let (btx, brx) = broadcast::channel(2);
        let (_stx, srx) = watch::channel(settings);
        let (ftx, frx) = watch::channel(view.filter(Utc::now()));
        let own_tx = spawn_coords_writer(store.clone());
        let snapshot: Snapshot = Arc::new(|| Box::pin(async { topics(live_active()) }));
        let task = tokio::spawn(run_loop(
            view,
            brx,
            srx,
            snapshot,
            Arc::new(Utc::now),
            ftx,
            own_tx,
        ));
        btx.send(CoreMessage::ImportProgress { done: 0, total: 1 })
            .unwrap();
        until("import progress not consumed", || btx.is_empty()).await;
        // An import error (or a lost final progress message) leaves no completion event.
        // Overflow forces recovery through the snapshot while that import is unfinished.
        for seq in 0..if lag { 8 } else { 1 } {
            btx.send(CoreMessage::Update {
                seq,
                topics: topics(live_active()),
            })
            .unwrap();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let coords = store.coords_by_names(&["lave".into()]).unwrap();
                if let Some(pos) = coords.get("lave") {
                    assert_eq!(*pos, [1.0, 2.0, 3.0]);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        task.abort();
        assert!(!frx.borrow().enabled, "offline must stay disconnected");
        assert!(
            result.is_ok(),
            "unfinished import froze own coordinates (lag={lag})"
        );
    }

    #[tokio::test]
    async fn own_coords_continue_without_import_completion() {
        own_coords_without_import_completion(false).await;
    }

    #[tokio::test]
    async fn own_coords_recover_from_lag_without_import_completion() {
        own_coords_without_import_completion(true).await;
    }

    #[tokio::test]
    async fn disabling_reaches_the_subscriber_as_disconnect() {
        let store = Store::open_in_memory().unwrap();
        let counts = Arc::new(Counts::default());
        let clock = FakeClock::new();
        let (core_tx, _core_rx) = mpsc::channel(8);
        let (btx, brx) = broadcast::channel(16);
        let (_stx, srx) = watch::channel(ModuleSettings::default());
        let snapshot: Snapshot = Arc::new(|| Box::pin(async { Topics::new() }));
        spawn_wiring_with(
            store,
            core_tx,
            brx,
            srx,
            snapshot,
            "tcp://fake".into(),
            Box::new(Src(counts.clone(), clock.clone())),
            Box::new(clock),
            Arc::new(Utc::now),
        );
        let send = |v: serde_json::Value| {
            btx.send(CoreMessage::Update {
                seq: 1,
                topics: topics(v),
            })
            .unwrap();
        };
        send(live_active());
        until("sin conexión", || {
            counts.connects.load(Ordering::SeqCst) >= 1
        })
        .await;
        send(json!({"core.game": {"running": false, "galaxy": "Live"}}));
        until("sin desconexión", || {
            counts.disconnects.load(Ordering::SeqCst) >= 1
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn linger_keeps_filter_on_for_60s_then_turns_off() {
        let start = Utc::now();
        let t0 = tokio::time::Instant::now();
        let now_fn: NowFn = Arc::new(move || {
            start + chrono::Duration::from_std(tokio::time::Instant::now() - t0).unwrap()
        });
        let (btx, brx) = broadcast::channel(16);
        let (_stx, srx) = watch::channel(ModuleSettings::default());
        let view = EddnView::new(ModuleSettings::default());
        let (ftx, mut frx) = watch::channel(view.filter(start));
        let (otx, _orx) = watch::channel(None);
        let snapshot: Snapshot = Arc::new(|| Box::pin(async { Topics::new() }));
        tokio::spawn(run_loop(view, brx, srx, snapshot, now_fn, ftx, otx));
        let send = |v: serde_json::Value| {
            btx.send(CoreMessage::Update {
                seq: 1,
                topics: topics(v),
            })
            .unwrap();
        };
        send(live_active());
        frx.wait_for(|f| f.enabled).await.unwrap();
        send(json!({"core.modules": [{"id": "mining", "relevance": "normal"}]}));
        tokio::time::sleep(Duration::from_secs(59)).await;
        assert!(frx.borrow().enabled);
        tokio::time::timeout(Duration::from_secs(5), frx.wait_for(|f| !f.enabled))
            .await
            .expect("no se apagó tras 60 s")
            .unwrap();
        let _ = VecDeque::<u8>::new();
    }
}

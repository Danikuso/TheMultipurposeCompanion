use crate::{
    Alert, Module, ModuleCtx, ModulePageQuery, ModuleQueryError, Relevance, Severity, TopicSet,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;
use tmc_model::net::NetRequest;
use tmc_model::source::SourceEvent;
use tmc_model::{
    state::{Changes, GameState},
    theme::HudTheme,
};
use tmc_store::StoredEvent;
use tmc_store::{EventWriter, Store};
use tokio::sync::{broadcast, mpsc, oneshot};

#[cfg(debug_assertions)]
mod diag;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CoreMessage {
    Update {
        seq: u64,
        topics: BTreeMap<String, Value>,
    },
    Alert(Alert),
    ImportProgress {
        done: u64,
        total: u64,
    },
}

enum ControlRequest {
    Snapshot(oneshot::Sender<BTreeMap<String, Value>>),
    ModuleQuery {
        module: String,
        resource: String,
        query: ModulePageQuery,
        reply: oneshot::Sender<Result<Value, ModuleQueryError>>,
    },
}

#[derive(Clone)]
pub struct CoreHandle {
    pub events: mpsc::Sender<SourceEvent>,
    updates: broadcast::Sender<CoreMessage>,
    snapshots: mpsc::Sender<ControlRequest>,
}
impl CoreHandle {
    pub fn subscribe(&self) -> broadcast::Receiver<CoreMessage> {
        self.updates.subscribe()
    }
    pub async fn query_module(
        &self,
        module: &str,
        resource: &str,
        query: ModulePageQuery,
    ) -> Result<Value, ModuleQueryError> {
        let (reply, receive) = oneshot::channel();
        self.snapshots
            .send(ControlRequest::ModuleQuery {
                module: module.to_owned(),
                resource: resource.to_owned(),
                query,
                reply,
            })
            .await
            .map_err(|_| ModuleQueryError::Unavailable)?;
        receive.await.unwrap_or(Err(ModuleQueryError::Unavailable))
    }
    pub async fn snapshot(&self) -> BTreeMap<String, Value> {
        let (sender, receiver) = oneshot::channel();
        if self
            .snapshots
            .send(ControlRequest::Snapshot(sender))
            .await
            .is_err()
        {
            return BTreeMap::new();
        }
        receiver.await.unwrap_or_default()
    }
}

pub struct CoreOptions {
    pub writer: Option<EventWriter>,
    pub flush_every: Duration,
    pub store: Option<Store>,
    pub net: Option<mpsc::Sender<NetRequest>>,
}

pub fn spawn_core(
    modules: Vec<Box<dyn Module>>,
    writer: Option<EventWriter>,
    flush_every: Duration,
) -> CoreHandle {
    spawn_core_with(
        modules,
        CoreOptions {
            writer,
            flush_every,
            store: None,
            net: None,
        },
    )
}

pub fn spawn_core_with(modules: Vec<Box<dyn Module>>, opts: CoreOptions) -> CoreHandle {
    let CoreOptions {
        writer,
        flush_every,
        store,
        net,
    } = opts;
    let (events, event_receiver) = mpsc::channel(256);
    let (snapshots, snapshot_receiver) = mpsc::channel(32);
    let (updates, _) = broadcast::channel(256);
    let actor_updates = updates.clone();
    // Nunca se publica más de diez veces por segundo, incluso con configuración inválida.
    let period = flush_every.max(Duration::from_millis(100));
    tokio::spawn(async move {
        let (stored_tx, stored_rx) = mpsc::channel(64);
        let mut actor = Actor::new(modules, writer, actor_updates);
        actor.net = net;
        // Un único worker ejecuta las tareas de SQLite en orden FIFO; termina
        // cuando el actor suelta `task_tx`.
        if let Some(store) = store {
            let (task_tx, task_rx) = std::sync::mpsc::channel::<StoreJob>();
            actor.task_tx = Some(task_tx);
            tokio::task::spawn_blocking(move || store_worker(store, task_rx, stored_tx));
        }
        actor
            .run(event_receiver, snapshot_receiver, stored_rx, period)
            .await;
    });
    CoreHandle {
        events,
        updates,
        snapshots,
    }
}

type StoreJob = (&'static str, String, crate::StoreTask);

fn store_worker(
    store: Store,
    jobs: std::sync::mpsc::Receiver<StoreJob>,
    stored: mpsc::Sender<SourceEvent>,
) {
    while let Ok((module, tag, task)) = jobs.recv() {
        #[cfg(debug_assertions)]
        crate::debug_hooks::record_store_task(&tag);
        #[cfg(debug_assertions)]
        let started = std::time::Instant::now();
        let outcome = catch_unwind(AssertUnwindSafe(|| task(&store)));
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "actor.store_task.finished",
            tmc_model::diagnostics::json!({
                "module": module,
                "ok": matches!(outcome, Ok(Ok(_))),
                "panicked": outcome.is_err(),
                "duration_us": started.elapsed().as_micros() as u64,
            })
        );
        match outcome {
            Ok(Ok(value)) => {
                // Si el actor ya cerró el receptor, `blocking_send` falla y no bloquea.
                let _ = stored.blocking_send(SourceEvent::Stored {
                    module: module.to_owned(),
                    tag,
                    value,
                });
            }
            Ok(Err(error)) => tracing::warn!(module, tag, %error, "store task failed"),
            Err(_) => tracing::error!(module, tag, "store task panicked"),
        }
    }
}

struct ModuleSlot {
    id: &'static str,
    module: Box<dyn Module>,
    topics: TopicSet,
    relevance: Relevance,
}

struct Actor {
    state: GameState,
    replaying: bool,
    theme: HudTheme,
    modules: Vec<ModuleSlot>,
    topics: BTreeMap<String, Value>,
    dirty: TopicSet,
    seq: u64,
    writer: Option<EventWriter>,
    updates: broadcast::Sender<CoreMessage>,
    net: Option<mpsc::Sender<NetRequest>>,
    // Cola (sin límite, nunca bloquea al actor) hacia el worker único de SQLite.
    // Los resultados vuelven por un canal propio, que no mantiene vivo al actor.
    task_tx: Option<std::sync::mpsc::Sender<StoreJob>>,
    next_request_id: u64,
    // Último instante del diario publicado en `core.clock`.
    clock_published: Option<chrono::DateTime<chrono::Utc>>,
    #[cfg(debug_assertions)]
    diag: diag::ActorDiag,
}

impl Actor {
    fn new(
        modules: Vec<Box<dyn Module>>,
        writer: Option<EventWriter>,
        updates: broadcast::Sender<CoreMessage>,
    ) -> Self {
        let mut actor = Self {
            state: GameState::default(),
            replaying: true,
            theme: HudTheme::default(),
            modules: Vec::new(),
            topics: BTreeMap::new(),
            dirty: TopicSet::new(),
            seq: 0,
            writer,
            updates,
            net: None,
            task_tx: None,
            next_request_id: 1,
            clock_published: None,
            #[cfg(debug_assertions)]
            diag: diag::ActorDiag::default(),
        };
        actor.update_core(Changes::all());
        actor.set_topic("core.theme", value(&actor.theme));
        for module in modules {
            let id = match catch_unwind(AssertUnwindSafe(|| module.id())) {
                Ok(id) => id,
                Err(_) => {
                    actor.crashed("unknown", false);
                    continue;
                }
            };
            let registered = catch_unwind(AssertUnwindSafe(|| {
                let topics: TopicSet = module.topics().into_iter().collect();
                let relevance = module.relevance(&actor.state);
                let values = topics
                    .iter()
                    .filter_map(|topic| module.topic_value(topic).map(|v| (topic.clone(), v)))
                    .collect::<Vec<_>>();
                (topics, relevance, values)
            }));
            match registered {
                Ok((topics, relevance, values)) => {
                    for (topic, value) in values {
                        actor.set_topic(&topic, value);
                    }
                    actor.modules.push(ModuleSlot {
                        id,
                        module,
                        topics,
                        relevance,
                    });
                }
                Err(_) => actor.crashed(id, false),
            }
        }
        actor.update_modules();
        actor.dirty.clear();
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "actor.started",
            tmc_model::diagnostics::json!({"modules": actor.modules.len(), "topics": actor.topics.len()})
        );
        actor
    }

    fn set_topic(&mut self, topic: &str, value: Value) {
        if self.topics.get(topic) != Some(&value) {
            #[cfg(debug_assertions)]
            self.diag_topic(topic, &value);
            self.topics.insert(topic.to_owned(), value);
            self.dirty.insert(topic.to_owned());
        }
    }

    fn update_core(&mut self, changes: Changes) {
        for (flag, topic, value) in [
            (
                Changes::CMDR,
                "core.cmdr",
                changes
                    .contains(Changes::CMDR)
                    .then(|| value(&self.state.cmdr)),
            ),
            (
                Changes::SHIP,
                "core.ship",
                changes
                    .contains(Changes::SHIP)
                    .then(|| value(&self.state.ship)),
            ),
            (
                Changes::LOCATION,
                "core.location",
                changes
                    .contains(Changes::LOCATION)
                    .then(|| value(&self.state.location)),
            ),
            (
                Changes::STATUS,
                "core.status",
                changes
                    .contains(Changes::STATUS)
                    .then(|| value(&self.state.status)),
            ),
            (
                Changes::ROUTE,
                "core.route",
                changes
                    .contains(Changes::ROUTE)
                    .then(|| value(&self.state.route)),
            ),
            (
                Changes::SESSION,
                "core.session",
                changes
                    .contains(Changes::SESSION)
                    .then(|| value(&self.state.session)),
            ),
        ] {
            if changes.contains(flag)
                && let Some(value) = value
            {
                self.set_topic(topic, value);
            }
        }
        self.set_topic(
            "core.game",
            serde_json::json!({"running":self.state.game_running,"galaxy":self.state.galaxy,"replaying":self.replaying}),
        );
    }

    /// Publica el reloj del diario como mucho una vez cada 10 s de tiempo del
    /// diario (y en el primer evento o en `LoadGame`), para que un cliente que se
    /// conecta después pueda calcular el desfase con el reloj local.
    fn publish_clock(&mut self, ts: chrono::DateTime<chrono::Utc>, force: bool) {
        let due = force
            || self
                .clock_published
                .is_none_or(|last| ts < last || (ts - last).num_seconds() >= 10);
        if due {
            self.clock_published = Some(ts);
            self.set_topic("core.clock", serde_json::json!({"as_of": ts}));
        }
    }

    fn update_modules(&mut self) {
        self.set_topic(
            "core.modules",
            Value::Array(
                self.modules
                    .iter()
                    .map(|m| serde_json::json!({"id":m.id,"relevance":m.relevance}))
                    .collect(),
            ),
        );
    }

    fn crashed(&self, id: &str, replaying: bool) {
        tracing::error!(module = id, "module disabled after panic");
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "actor.module.crashed",
            tmc_model::diagnostics::json!({"module": id, "replaying": replaying})
        );
        if !replaying {
            let _ = self.updates.send(CoreMessage::Alert(
                Alert::new(Severity::Critical, "alert-module-crashed").arg("module", id),
            ));
        }
    }

    fn dispatch_requests(&self, requests: Vec<NetRequest>) {
        for request in requests {
            let Some(net) = &self.net else {
                tracing::debug!(key = %request.key, "network request dropped: no network channel");
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "actor.request.dropped_no_channel",
                    tmc_model::diagnostics::json!({"id": request.id})
                );
                continue;
            };
            if let Err(error) = net.try_send(request) {
                tracing::warn!(%error, "network request dropped");
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "actor.request.dropped_queue_full",
                    tmc_model::diagnostics::json!({})
                );
            }
        }
    }

    fn spawn_store_tasks(&self, module: &'static str, tasks: Vec<(String, crate::StoreTask)>) {
        for (tag, task) in tasks {
            let Some(queue) = &self.task_tx else {
                tracing::debug!(module, tag, "store task dropped: no store");
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "actor.store_task.dropped_no_store",
                    tmc_model::diagnostics::json!({"module": module})
                );
                continue;
            };
            if queue.send((module, tag, task)).is_err() {
                tracing::warn!(module, "store task dropped: worker stopped");
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "actor.store_task.dropped_worker_stopped",
                    tmc_model::diagnostics::json!({"module": module})
                );
            }
        }
    }

    fn apply(&mut self, event: SourceEvent) {
        #[cfg(debug_assertions)]
        let diag = self.diag_begin(&event);
        let replaying = matches!(
            &event,
            SourceEvent::Journal {
                replaying: true,
                ..
            }
        );
        let changes = match &event {
            SourceEvent::Journal {
                ev,
                file,
                line,
                replaying,
            } => {
                self.replaying = *replaying;
                let changes = self.state.apply_event(ev);
                self.publish_clock(ev.timestamp, ev.name == "LoadGame");
                if let Some(writer) = &self.writer {
                    writer.send(StoredEvent {
                        file_name: file.clone(),
                        line: *line,
                        ts: ev.timestamp,
                        name: ev.name.clone(),
                        cmdr: self.state.cmdr.as_ref().map(|c| c.name.clone()),
                        json: ev.raw.get().to_owned(),
                    });
                }
                changes
            }
            SourceEvent::Status(status) => self.state.apply_status(status),
            SourceEvent::Companion(file) => self.state.apply_companion(file),
            SourceEvent::GameProcess { running } => {
                self.state.game_running = *running;
                Changes::empty()
            }
            SourceEvent::Theme(theme) => {
                self.theme = *theme;
                self.set_topic("core.theme", value(theme));
                Changes::empty()
            }
            SourceEvent::ImportProgress { done, total } => {
                let _ = self.updates.send(CoreMessage::ImportProgress {
                    done: *done,
                    total: *total,
                });
                Changes::empty()
            }
            SourceEvent::Net(_)
            | SourceEvent::Action { .. }
            | SourceEvent::Settings(_)
            | SourceEvent::Eddn(_)
            | SourceEvent::Stored { .. } => Changes::empty(),
        };
        self.update_core(changes);
        let mut index = 0;
        while index < self.modules.len() {
            let slot = &mut self.modules[index];
            let mut context = ModuleCtx::new(slot.id, replaying, self.next_request_id);
            #[cfg(debug_assertions)]
            let module_started = std::time::Instant::now();
            #[cfg(debug_assertions)]
            let previous_relevance = slot.relevance;
            let processed = catch_unwind(AssertUnwindSafe(|| {
                let dirty = slot.module.on_event(&event, &self.state, &mut context);
                let declared: TopicSet = slot.module.topics().into_iter().collect();
                let relevance = slot.module.relevance(&self.state);
                let values = declared
                    .iter()
                    .filter(|t| dirty.contains(*t) || !slot.topics.contains(*t))
                    .filter_map(|t| slot.module.topic_value(t).map(|v| (t.clone(), v)))
                    .collect::<Vec<_>>();
                (declared, relevance, values)
            }));
            match processed {
                Ok((declared, relevance, values)) => {
                    for removed in slot.topics.difference(&declared) {
                        self.topics.remove(removed);
                        self.dirty.remove(removed);
                    }
                    slot.topics = declared;
                    slot.relevance = relevance;
                    #[cfg(debug_assertions)]
                    let refreshed = values.len();
                    for (topic, value) in values {
                        self.set_topic(&topic, value);
                    }
                    let alerts = context.drain_alerts();
                    #[cfg(debug_assertions)]
                    let alert_count = alerts.len();
                    for alert in alerts {
                        let _ = self.updates.send(CoreMessage::Alert(alert));
                    }
                    let (requests, tasks, next_id) = context.take_outputs();
                    self.next_request_id = next_id;
                    let id = self.modules[index].id;
                    #[cfg(debug_assertions)]
                    self.diag_module(
                        id,
                        module_started,
                        previous_relevance,
                        relevance,
                        refreshed,
                        requests.len(),
                        tasks.len(),
                        alert_count,
                    );
                    self.dispatch_requests(requests);
                    self.spawn_store_tasks(id, tasks);
                    index += 1;
                }
                Err(_) => {
                    let removed = self.modules.remove(index);
                    for topic in removed.topics {
                        self.topics.remove(&topic);
                        self.dirty.remove(&topic);
                    }
                    self.crashed(removed.id, replaying);
                }
            }
        }
        self.update_modules();
        #[cfg(debug_assertions)]
        self.diag_end(&event, diag);
    }

    fn flush(&mut self) {
        if self.dirty.is_empty() {
            return;
        }
        let topics: BTreeMap<String, Value> = std::mem::take(&mut self.dirty)
            .into_iter()
            .filter_map(|topic| self.topics.get(&topic).cloned().map(|value| (topic, value)))
            .collect();
        self.seq += 1;
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "actor.flush",
            tmc_model::diagnostics::json!({"seq": self.seq, "topics": topics.len()})
        );
        let _ = self.updates.send(CoreMessage::Update {
            seq: self.seq,
            topics,
        });
    }

    fn control(&self, request: ControlRequest, _forced_turn: bool) {
        match request {
            ControlRequest::Snapshot(reply) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::State,
                    "actor.snapshot.served",
                    tmc_model::diagnostics::json!({"topics": self.topics.len(), "forced_turn": _forced_turn})
                );
                let _ = reply.send(self.topics.clone());
            }
            ControlRequest::ModuleQuery {
                module,
                resource,
                query,
                reply,
            } => {
                let result = self
                    .modules
                    .iter()
                    .find(|slot| slot.id == module)
                    .ok_or(ModuleQueryError::NotFound)
                    .and_then(|slot| {
                        catch_unwind(AssertUnwindSafe(|| slot.module.query(&resource, &query)))
                            .unwrap_or(Err(ModuleQueryError::Unavailable))
                    });
                let _ = reply.send(result);
            }
        }
    }

    async fn run(
        &mut self,
        mut events: mpsc::Receiver<SourceEvent>,
        mut snapshots: mpsc::Receiver<ControlRequest>,
        mut stored: mpsc::Receiver<SourceEvent>,
        period: Duration,
    ) {
        let mut tick = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut events_open = true;
        let mut snapshots_open = true;
        let mut events_since_control = 0;
        while events_open || snapshots_open {
            // Conserva FIFO de eventos, pero garantiza un turno de control ante colas continuas.
            if snapshots_open && events_since_control >= 32 {
                match snapshots.try_recv() {
                    Ok(reply) => {
                        self.control(reply, true);
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => snapshots_open = false,
                    Err(mpsc::error::TryRecvError::Empty) => {}
                }
                events_since_control = 0;
            }
            tokio::select! {
                biased;
                _ = tick.tick() => self.flush(),
                // Antes que `events`: está acotado y lo alimenta trabajo ya terminado.
                Some(event) = stored.recv() => { self.apply(event); if snapshots_open { events_since_control += 1; } },
                event = events.recv(), if events_open => match event {
                    Some(event)=>{self.apply(event); if snapshots_open { events_since_control += 1; }}, None=>events_open=false,
                },
                request = snapshots.recv(), if snapshots_open => match request {
                    Some(reply)=>{
                        self.control(reply, false); events_since_control = 0;
                    }, None=>snapshots_open=false,
                },
            }
        }
        #[cfg(debug_assertions)]
        self.diag_replay_finished();
        // Cerrar ya el receptor y la cola evita que el worker se quede en `blocking_send`
        // durante el volcado final y termina al acabar su tarea en curso.
        drop(stored);
        self.task_tx = None;
        if let Some(writer) = self.writer.take() {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(error) = writer.flush_blocking() {
                    tracing::error!(%error,"failed to flush writer on core shutdown");
                }
                drop(writer);
            })
            .await;
        }
    }
}

fn value(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModuleCtx, Relevance, Severity, TopicSet};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tmc_model::{
        events::parse_line,
        state::GameState,
        status::Status,
        theme::{HudTheme, Rgb},
    };
    use tmc_store::{EventQuery, Store};
    use tokio::time::{advance, timeout};

    fn journal(name: &str, extra: &str, replaying: bool) -> SourceEvent {
        SourceEvent::Journal {
            ev: parse_line(&format!(
                r#"{{"timestamp":"2026-10-17T18:00:00Z","event":"{name}"{extra}}}"#
            ))
            .unwrap(),
            file: "Journal.synthetic.log".into(),
            line: 1,
            replaying,
        }
    }
    fn status(flags: u32) -> SourceEvent {
        SourceEvent::Status(
            Status::parse(&format!(
                r#"{{"timestamp":"2026-10-17T18:00:00Z","Flags":{flags}}}"#
            ))
            .unwrap(),
        )
    }
    async fn next(rx: &mut broadcast::Receiver<CoreMessage>) -> CoreMessage {
        timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap()
    }
    struct QueryProbe;
    impl Module for QueryProbe {
        fn id(&self) -> &'static str {
            "query"
        }
        fn on_event(&mut self, _: &SourceEvent, _: &GameState, _: &mut ModuleCtx) -> TopicSet {
            TopicSet::new()
        }
        fn topics(&self) -> Vec<String> {
            vec![]
        }
        fn topic_value(&self, _: &str) -> Option<Value> {
            None
        }
        fn query(
            &self,
            resource: &str,
            query: &ModulePageQuery,
        ) -> Result<Value, ModuleQueryError> {
            assert_ne!(resource, "panic", "synthetic query panic");
            Ok(serde_json::json!({"cursor":query.cursor,"revision":query.revision}))
        }
    }
    #[tokio::test(start_paused = true)]
    async fn module_queries_are_read_only_isolated_and_survive_panics() {
        let core = spawn_core(
            vec![
                Box::new(QueryProbe),
                Box::new(Probe {
                    count: Arc::default(),
                    panic: false,
                    alerts: false,
                }),
            ],
            None,
            Duration::from_millis(100),
        );
        let mut updates = core.subscribe();
        let before = core.snapshot().await;
        let query = ModulePageQuery {
            revision: 7,
            cursor: 3,
            section: None,
            engineer_id: None,
        };
        assert_eq!(
            core.query_module("query", "rows", query.clone())
                .await
                .unwrap(),
            serde_json::json!({"cursor":3,"revision":7})
        );
        assert_eq!(
            core.query_module("missing", "rows", query.clone()).await,
            Err(ModuleQueryError::NotFound)
        );
        assert_eq!(
            core.query_module("test", "rows", query.clone()).await,
            Err(ModuleQueryError::NotFound)
        );
        assert_eq!(
            core.query_module("query", "panic", query.clone()).await,
            Err(ModuleQueryError::Unavailable)
        );
        assert!(core.query_module("query", "rows", query).await.is_ok());
        assert_eq!(core.snapshot().await, before);
        advance(Duration::from_millis(200)).await;
        assert!(matches!(
            updates.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }
    #[tokio::test]
    async fn module_query_closed_actor_returns_unavailable() {
        let (events, _) = mpsc::channel(1);
        let (snapshots, receiver) = mpsc::channel(1);
        drop(receiver);
        let (updates, _) = broadcast::channel(1);
        let core = CoreHandle {
            events,
            snapshots,
            updates,
        };
        assert_eq!(
            core.query_module(
                "query",
                "rows",
                ModulePageQuery {
                    revision: 0,
                    cursor: 0,
                    section: None,
                    engineer_id: None,
                }
            )
            .await,
            Err(ModuleQueryError::Unavailable)
        );
    }

    fn core() -> CoreHandle {
        spawn_core(vec![], None, Duration::from_millis(100))
    }

    struct Probe {
        count: Arc<AtomicUsize>,
        panic: bool,
        alerts: bool,
    }
    impl Module for Probe {
        fn id(&self) -> &'static str {
            "test"
        }
        fn on_event(&mut self, _: &SourceEvent, _: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
            self.count.fetch_add(1, Ordering::SeqCst);
            assert!(!self.panic, "synthetic module panic");
            if self.alerts {
                ctx.alert(Alert::new(Severity::Warn, "test-alert"));
            }
            ["test.count".into()].into()
        }
        fn query(&self, resource: &str, _: &ModulePageQuery) -> Result<Value, ModuleQueryError> {
            if resource == "count" {
                Ok(Value::from(self.count.load(Ordering::SeqCst)))
            } else {
                Err(ModuleQueryError::NotFound)
            }
        }
        fn topic_value(&self, topic: &str) -> Option<Value> {
            (topic == "test.count").then(|| Value::from(self.count.load(Ordering::SeqCst)))
        }
        fn topics(&self) -> Vec<String> {
            vec!["test.count".into()]
        }
        fn relevance(&self, _: &GameState) -> Relevance {
            if self.count.load(Ordering::SeqCst) > 0 {
                Relevance::Active
            } else {
                Relevance::Normal
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn journal_event_updates_core_topics() {
        let core = core();
        let mut rx = core.subscribe();
        core.events
            .send(journal(
                "FSDJump",
                r#", "StarSystem":"Example","JumpDist":9.5"#,
                false,
            ))
            .await
            .unwrap();
        match next(&mut rx).await {
            CoreMessage::Update { seq, topics } => {
                assert_eq!(seq, 1);
                assert_eq!(topics["core.location"]["system"], "Example");
                assert_eq!(topics["core.session"]["jumps"], 1);
                assert_eq!(topics["core.clock"]["as_of"], "2026-10-17T18:00:00Z");
                assert_eq!(topics["core.game"]["replaying"], false);
                assert_eq!(topics.len(), 4);
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn updates_are_coalesced_to_flush_interval() {
        let core = core();
        let mut rx = core.subscribe();
        for flags in 0..50 {
            core.events.send(status(flags)).await.unwrap();
        }
        core.snapshot().await;
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        advance(Duration::from_millis(100)).await;
        assert!(matches!(
            next(&mut rx).await,
            CoreMessage::Update { seq: 1, .. }
        ));
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn snapshot_returns_all_topics() {
        let core = spawn_core(
            vec![Box::new(Probe {
                count: Arc::default(),
                panic: false,
                alerts: false,
            })],
            None,
            Duration::from_millis(100),
        );
        let snapshot = core.snapshot().await;
        for topic in [
            "core.cmdr",
            "core.ship",
            "core.location",
            "core.status",
            "core.route",
            "core.session",
            "core.game",
            "core.theme",
            "core.modules",
            "test.count",
        ] {
            assert!(snapshot.contains_key(topic), "{topic}");
        }
        assert_eq!(snapshot["core.theme"]["primary"], "#ff7100");
        assert_eq!(snapshot["core.modules"][0]["relevance"], "normal");
    }

    #[tokio::test(start_paused = true)]
    async fn core_clock_is_published_at_most_every_10_journal_seconds() {
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let at = |sec: u32, name: &str| SourceEvent::Journal {
            ev: parse_line(&format!(
                r#"{{"timestamp":"2026-10-17T18:00:{sec:02}Z","event":"{name}"}}"#
            ))
            .unwrap(),
            file: "Journal.synthetic.log".into(),
            line: 1,
            replaying: false,
        };
        assert!(!core.snapshot().await.contains_key("core.clock"));
        core.events.send(at(0, "Music")).await.unwrap();
        assert_eq!(
            core.snapshot().await["core.clock"]["as_of"],
            "2026-10-17T18:00:00Z"
        );
        core.events.send(at(9, "Music")).await.unwrap();
        assert_eq!(
            core.snapshot().await["core.clock"]["as_of"],
            "2026-10-17T18:00:00Z"
        );
        core.events.send(at(10, "Music")).await.unwrap();
        assert_eq!(
            core.snapshot().await["core.clock"]["as_of"],
            "2026-10-17T18:00:10Z"
        );
        core.events.send(at(12, "LoadGame")).await.unwrap();
        assert_eq!(
            core.snapshot().await["core.clock"]["as_of"],
            "2026-10-17T18:00:12Z"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn panicking_module_is_disabled_and_reported() {
        let count = Arc::new(AtomicUsize::new(0));
        let core = spawn_core(
            vec![Box::new(Probe {
                count: count.clone(),
                panic: true,
                alerts: false,
            })],
            None,
            Duration::from_millis(100),
        );
        let mut rx = core.subscribe();
        core.events.send(status(0)).await.unwrap();
        match next(&mut rx).await {
            CoreMessage::Alert(alert) => {
                assert_eq!(alert.key, "alert-module-crashed");
                assert_eq!(serde_json::to_value(alert.severity).unwrap(), "critical");
            }
            other => panic!("{other:?}"),
        }
        let snapshot = core.snapshot().await;
        assert!(!snapshot.contains_key("test.count"));
        assert_eq!(snapshot["core.modules"], serde_json::json!([]));
        core.events.send(status(1)).await.unwrap();
        assert_eq!(core.snapshot().await["core.status"]["Flags"], 1);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn alerts_suppressed_while_replaying() {
        let core = spawn_core(
            vec![Box::new(Probe {
                count: Arc::default(),
                panic: false,
                alerts: true,
            })],
            None,
            Duration::from_millis(100),
        );
        let mut rx = core.subscribe();
        core.events
            .send(journal("UnderAttack", "", true))
            .await
            .unwrap();
        core.snapshot().await;
        advance(Duration::from_millis(100)).await;
        assert!(matches!(next(&mut rx).await, CoreMessage::Update { .. }));
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        core.events
            .send(journal("UnderAttack", "", false))
            .await
            .unwrap();
        assert!(matches!(next(&mut rx).await, CoreMessage::Alert(a) if a.key == "test-alert"));
    }

    #[tokio::test]
    async fn journal_events_are_sent_to_writer() {
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(
            vec![],
            Some(EventWriter::spawn(&store)),
            Duration::from_millis(100),
        );
        core.events
            .send(journal(
                "LoadGame",
                r#", "Commander":"TEST_A","FID":"F1""#,
                false,
            ))
            .await
            .unwrap();
        core.snapshot().await;
        drop(core);
        timeout(Duration::from_secs(3), async {
            loop {
                let rows = store
                    .query_events(&EventQuery {
                        limit: 10,
                        ..Default::default()
                    })
                    .unwrap();
                if !rows.is_empty() {
                    assert_eq!(rows[0].cmdr.as_deref(), Some("TEST_A"));
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn no_update_when_nothing_changed() {
        let core = core();
        let mut rx = core.subscribe();
        core.events
            .send(journal("UnderAttack", "", false))
            .await
            .unwrap();
        core.events
            .send(SourceEvent::GameProcess { running: false })
            .await
            .unwrap();
        core.snapshot().await;
        advance(Duration::from_millis(200)).await;
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn replay_signal_persists_until_an_explicit_live_journal() {
        let core = core();
        assert_eq!(core.snapshot().await["core.game"]["replaying"], true);
        core.events
            .send(journal("LoadGame", r#", "gameversion":"4.1.0.0""#, true))
            .await
            .unwrap();
        let game = core.snapshot().await["core.game"].clone();
        assert_eq!(game["running"], true);
        assert_eq!(game["galaxy"], "Live");
        assert_eq!(game["replaying"], true);
        core.events
            .send(SourceEvent::GameProcess { running: true })
            .await
            .unwrap();
        core.events
            .send(SourceEvent::Settings(Default::default()))
            .await
            .unwrap();
        assert_eq!(core.snapshot().await["core.game"]["replaying"], true);
        core.events
            .send(journal("UnderAttack", "", false))
            .await
            .unwrap();
        assert_eq!(core.snapshot().await["core.game"]["replaying"], false);
        core.events
            .send(journal("UnderAttack", "", true))
            .await
            .unwrap();
        core.events
            .send(SourceEvent::GameProcess { running: true })
            .await
            .unwrap();
        assert_eq!(core.snapshot().await["core.game"]["replaying"], true);
    }

    #[tokio::test(start_paused = true)]
    async fn game_process_loadgame_shutdown_and_theme_update_only_changed_topics() {
        let core = core();
        let mut rx = core.subscribe();
        core.events
            .send(SourceEvent::GameProcess { running: true })
            .await
            .unwrap();
        match next(&mut rx).await {
            CoreMessage::Update { topics, .. } => {
                assert_eq!(topics.len(), 1);
                assert_eq!(topics["core.game"]["running"], true);
            }
            other => panic!("{other:?}"),
        }
        core.events
            .send(journal("LoadGame", r#", "gameversion":"4.1.0.0""#, false))
            .await
            .unwrap();
        assert_eq!(core.snapshot().await["core.game"]["galaxy"], "Live");
        next(&mut rx).await;
        core.events
            .send(journal("Shutdown", "", false))
            .await
            .unwrap();
        match next(&mut rx).await {
            CoreMessage::Update { topics, .. } => {
                assert_eq!(topics.len(), 1);
                assert_eq!(topics["core.game"]["running"], false);
            }
            other => panic!("{other:?}"),
        }
        let theme = HudTheme {
            primary: Rgb(1, 2, 3),
            is_default: false,
            ..Default::default()
        };
        core.events.send(SourceEvent::Theme(theme)).await.unwrap();
        match next(&mut rx).await {
            CoreMessage::Update { topics, .. } => {
                assert_eq!(topics.len(), 1);
                assert_eq!(topics["core.theme"]["primary"], "#010203");
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn import_progress_is_immediate_and_not_a_dirty_update() {
        let core = spawn_core(vec![], None, Duration::from_secs(10));
        let mut rx = core.subscribe();
        core.events
            .send(SourceEvent::ImportProgress { done: 3, total: 9 })
            .await
            .unwrap();
        assert!(matches!(
            next(&mut rx).await,
            CoreMessage::ImportProgress { done: 3, total: 9 }
        ));
        core.snapshot().await;
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn actor_runs_without_broadcast_subscribers_and_stops_after_inputs_close() {
        let core = core();
        core.events.send(status(1)).await.unwrap();
        assert_eq!(core.snapshot().await["core.status"]["Flags"], 1);
        let mut rx = core.subscribe();
        drop(core);
        timeout(Duration::from_secs(2), async {
            while rx.recv().await.is_ok() {}
        })
        .await
        .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn module_relevance_changes_even_without_a_core_modules_dirty_topic() {
        let core = spawn_core(
            vec![Box::new(Probe {
                count: Arc::default(),
                panic: false,
                alerts: false,
            })],
            None,
            Duration::from_millis(100),
        );
        core.events.send(status(0)).await.unwrap();
        assert_eq!(
            core.snapshot().await["core.modules"][0]["relevance"],
            "active"
        );
    }

    #[tokio::test]
    async fn snapshot_responds_while_event_producer_keeps_backlog() {
        use std::sync::atomic::AtomicBool;
        struct FloodModule;
        impl Module for FloodModule {
            fn id(&self) -> &'static str {
                "flood"
            }
            fn on_event(&mut self, _: &SourceEvent, _: &GameState, _: &mut ModuleCtx) -> TopicSet {
                // Hace el consumidor algo más lento que el productor sin ceder su backlog.
                std::thread::sleep(Duration::from_micros(100));
                TopicSet::new()
            }
            fn topics(&self) -> Vec<String> {
                Vec::new()
            }
            fn topic_value(&self, _: &str) -> Option<Value> {
                None
            }
        }
        let core = spawn_core(
            vec![Box::new(FloodModule)],
            None,
            Duration::from_millis(100),
        );
        let keep_sending = Arc::new(AtomicBool::new(true));
        let producer_flag = keep_sending.clone();
        let sender = core.events.clone();
        let producer = std::thread::spawn(move || {
            while producer_flag.load(Ordering::Relaxed) {
                if sender.blocking_send(status(0)).is_err() {
                    break;
                }
            }
        });
        // El productor llena la cola antes de que el actor empiece a consumir.
        std::thread::sleep(Duration::from_millis(20));
        let response = timeout(Duration::from_millis(150), core.snapshot()).await;
        let query_response = timeout(
            Duration::from_millis(150),
            core.query_module(
                "flood",
                "rows",
                ModulePageQuery {
                    revision: 0,
                    cursor: 0,
                    section: None,
                    engineer_id: None,
                },
            ),
        )
        .await;
        assert_eq!(query_response.unwrap(), Err(ModuleQueryError::NotFound));
        let replied_while_producing = response.is_ok();
        keep_sending.store(false, Ordering::Relaxed);
        tokio::task::spawn_blocking(move || producer.join().unwrap())
            .await
            .unwrap();
        assert!(
            replied_while_producing,
            "snapshot starved behind a continuous event backlog"
        );
    }

    #[tokio::test]
    async fn module_queries_get_control_turn_after_32_stored_events() {
        let (event_tx, event_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = mpsc::channel(1);
        let (stored_tx, stored_rx) = mpsc::channel(256);
        let (updates, _) = broadcast::channel(1);
        let mut actor = Actor::new(
            vec![Box::new(Probe {
                count: Arc::default(),
                panic: false,
                alerts: false,
            })],
            None,
            updates,
        );
        for _ in 0..128 {
            stored_tx
                .send(SourceEvent::Stored {
                    module: "test".into(),
                    tag: "test".into(),
                    value: Value::Null,
                })
                .await
                .unwrap();
        }
        let (reply, receive) = oneshot::channel();
        control_tx
            .send(ControlRequest::ModuleQuery {
                module: "test".into(),
                resource: "count".into(),
                query: ModulePageQuery {
                    revision: 0,
                    cursor: 0,
                    section: None,
                    engineer_id: None,
                },
                reply,
            })
            .await
            .unwrap();
        let task = tokio::spawn(async move {
            actor
                .run(event_rx, control_rx, stored_rx, Duration::from_secs(10))
                .await;
        });
        assert_eq!(
            timeout(Duration::from_secs(1), receive)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            Value::from(32)
        );
        drop(event_tx);
        drop(control_tx);
        drop(stored_tx);
        task.await.unwrap();
    }

    type Script = Box<dyn FnMut(&SourceEvent, &mut ModuleCtx) + Send>;

    struct FakeModule {
        script: Script,
        stored: Vec<Value>,
    }
    impl FakeModule {
        fn boxed(script: impl FnMut(&SourceEvent, &mut ModuleCtx) + Send + 'static) -> Box<Self> {
            Box::new(Self {
                script: Box::new(script),
                stored: Vec::new(),
            })
        }
    }
    impl Module for FakeModule {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn on_event(&mut self, ev: &SourceEvent, _: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
            if let SourceEvent::Stored { module, tag, value } = ev {
                self.stored
                    .push(serde_json::json!({"module":module,"tag":tag,"value":value}));
            }
            (self.script)(ev, ctx);
            ["fake.stored".into()].into()
        }
        fn topic_value(&self, _: &str) -> Option<Value> {
            Some(Value::Array(self.stored.clone()))
        }
        fn topics(&self) -> Vec<String> {
            vec!["fake.stored".into()]
        }
    }

    fn sol_query() -> tmc_model::net::NetQuery {
        tmc_model::net::NetQuery::System {
            id64: 1,
            name: "Sol".into(),
        }
    }

    fn options(net: Option<mpsc::Sender<NetRequest>>, store: Option<Store>) -> CoreOptions {
        CoreOptions {
            writer: None,
            flush_every: Duration::from_millis(100),
            store,
            net,
        }
    }

    #[tokio::test]
    async fn module_request_reaches_net_channel_with_module_and_id() {
        let (net, mut net_rx) = mpsc::channel(8);
        let ids = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = ids.clone();
        let core = spawn_core_with(
            vec![FakeModule::boxed(move |ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    seen.lock()
                        .unwrap()
                        .push(ctx.request("system:1", sol_query()));
                }
            })],
            options(Some(net), None),
        );
        core.events.send(status(0)).await.unwrap();
        core.events.send(status(1)).await.unwrap();
        core.snapshot().await;
        let first = net_rx.try_recv().unwrap();
        assert_eq!(
            (first.id, first.module.as_str(), first.key.as_str()),
            (1, "fake", "system:1")
        );
        assert_eq!(first.query, sol_query());
        assert_eq!(net_rx.try_recv().unwrap().id, 2);
        assert_eq!(*ids.lock().unwrap(), vec![Some(1), Some(2)]);
    }

    #[tokio::test]
    async fn requests_during_replay_are_dropped() {
        let (net, mut net_rx) = mpsc::channel(8);
        let result = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = result.clone();
        let core = spawn_core_with(
            vec![FakeModule::boxed(move |ev, ctx| {
                if matches!(ev, SourceEvent::Journal { .. }) {
                    seen.lock()
                        .unwrap()
                        .push(ctx.request("system:1", sol_query()));
                }
            })],
            options(Some(net), None),
        );
        core.events
            .send(journal("UnderAttack", "", true))
            .await
            .unwrap();
        core.snapshot().await;
        assert_eq!(*result.lock().unwrap(), vec![None]);
        assert!(matches!(
            net_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    async fn wait_for_stored(core: &CoreHandle, count: usize) -> Value {
        timeout(Duration::from_secs(2), async {
            loop {
                let snapshot = core.snapshot().await;
                if snapshot["fake.stored"].as_array().unwrap().len() >= count {
                    return snapshot["fake.stored"].clone();
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn store_task_result_returns_as_stored_event() {
        let core = spawn_core_with(
            vec![FakeModule::boxed(|ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    ctx.store_task(
                        "t",
                        Box::new(|store| Ok(store.query_json("SELECT 42", &[])?[0][0].clone())),
                    );
                }
            })],
            options(None, Some(Store::open_in_memory().unwrap())),
        );
        core.events.send(status(0)).await.unwrap();
        assert_eq!(
            wait_for_stored(&core, 1).await,
            serde_json::json!([{"module":"fake","tag":"t","value":42}])
        );
    }

    #[tokio::test]
    async fn failed_store_task_is_not_reinjected_and_tasks_without_store_are_dropped() {
        let core = spawn_core_with(
            vec![FakeModule::boxed(|ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    ctx.store_task(
                        "bad",
                        Box::new(|store| {
                            store.exec_json("THIS IS NOT SQL", &[])?;
                            Ok(Value::Null)
                        }),
                    );
                    ctx.store_task("ok", Box::new(|_| Ok(Value::from(1))));
                }
            })],
            options(None, Some(Store::open_in_memory().unwrap())),
        );
        core.events.send(status(0)).await.unwrap();
        assert_eq!(
            wait_for_stored(&core, 1).await,
            serde_json::json!([{"module":"fake","tag":"ok","value":1}])
        );
        let without_store = spawn_core_with(
            vec![FakeModule::boxed(|ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    ctx.store_task("t", Box::new(|_| Ok(Value::from(1))));
                }
            })],
            options(None, None),
        );
        without_store.events.send(status(0)).await.unwrap();
        assert_eq!(
            without_store.snapshot().await["fake.stored"],
            serde_json::json!([])
        );
    }

    #[tokio::test]
    async fn store_task_runs_during_replay() {
        let core = spawn_core_with(
            vec![FakeModule::boxed(|ev, ctx| {
                if let SourceEvent::Journal { replaying, .. } = ev {
                    assert!(*replaying);
                    ctx.store_task("replay", Box::new(|_| Ok(Value::from(7))));
                }
            })],
            options(None, Some(Store::open_in_memory().unwrap())),
        );
        core.events
            .send(journal("UnderAttack", "", true))
            .await
            .unwrap();
        assert_eq!(
            wait_for_stored(&core, 1).await,
            serde_json::json!([{"module":"fake","tag":"replay","value":7}])
        );
    }

    #[tokio::test]
    async fn store_tasks_run_in_fifo_order_on_one_worker() {
        let core = spawn_core_with(
            vec![FakeModule::boxed(|ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    for n in 0..20 {
                        ctx.store_task("n", Box::new(move |_| Ok(Value::from(n))));
                    }
                }
            })],
            options(None, Some(Store::open_in_memory().unwrap())),
        );
        core.events.send(status(0)).await.unwrap();
        let values: Vec<i64> = wait_for_stored(&core, 20)
            .await
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["value"].as_i64().unwrap())
            .collect();
        assert_eq!(values, (0..20).collect::<Vec<i64>>());
    }

    #[tokio::test]
    async fn stored_results_apply_under_continuous_event_flood() {
        use std::sync::atomic::AtomicBool;
        let mut first = true;
        let core = spawn_core_with(
            vec![FakeModule::boxed(move |ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    std::thread::sleep(Duration::from_micros(100));
                    if std::mem::take(&mut first) {
                        ctx.store_task("flood", Box::new(|_| Ok(Value::from(1))));
                    }
                }
            })],
            options(None, Some(Store::open_in_memory().unwrap())),
        );
        let keep_sending = Arc::new(AtomicBool::new(true));
        let flag = keep_sending.clone();
        let sender = core.events.clone();
        let producer = std::thread::spawn(move || {
            while flag.load(Ordering::Relaxed) {
                if sender.blocking_send(status(0)).is_err() {
                    break;
                }
            }
        });
        let stored = wait_for_stored(&core, 1).await;
        keep_sending.store(false, Ordering::Relaxed);
        tokio::task::spawn_blocking(move || producer.join().unwrap())
            .await
            .unwrap();
        assert_eq!(
            stored,
            serde_json::json!([{"module":"fake","tag":"flood","value":1}])
        );
    }

    #[tokio::test]
    async fn full_net_channel_never_blocks_actor() {
        let (net, _net_rx) = mpsc::channel(1);
        let core = spawn_core_with(
            vec![FakeModule::boxed(|ev, ctx| {
                if matches!(ev, SourceEvent::Status(_)) {
                    for _ in 0..50 {
                        ctx.request("system:1", sol_query());
                    }
                }
            })],
            options(Some(net), None),
        );
        core.events.send(status(0)).await.unwrap();
        let snapshot = timeout(Duration::from_millis(100), core.snapshot())
            .await
            .expect("actor blocked on a full network channel");
        assert!(snapshot.contains_key("core.modules"));
    }

    #[tokio::test]
    async fn actor_still_stops_when_inputs_close_with_store() {
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let (started_tx, started_rx) = std::sync::mpsc::channel::<()>();
        let core = spawn_core_with(
            vec![FakeModule::boxed({
                let mut slot = Some((release_rx, started_tx));
                move |ev, ctx| {
                    if matches!(ev, SourceEvent::Status(_))
                        && let Some((release, started)) = slot.take()
                    {
                        ctx.store_task(
                            "slow",
                            Box::new(move |_| {
                                let _ = started.send(());
                                let _ = release.recv_timeout(Duration::from_secs(5));
                                Ok(Value::Null)
                            }),
                        );
                    }
                }
            })],
            options(None, Some(Store::open_in_memory().unwrap())),
        );
        let mut rx = core.subscribe();
        core.events.send(status(0)).await.unwrap();
        core.snapshot().await;
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(core);
        timeout(Duration::from_secs(1), async {
            while !matches!(rx.recv().await, Err(broadcast::error::RecvError::Closed)) {}
        })
        .await
        .expect("actor did not stop with a store task in flight");
        let _ = release_tx.send(());
    }

    #[test]
    fn core_messages_serialize_the_wire_protocol() {
        let update = serde_json::to_value(CoreMessage::Update {
            seq: 7,
            topics: [("core.game".into(), serde_json::json!({"running":true}))].into(),
        })
        .unwrap();
        assert_eq!(update["type"], "update");
        assert_eq!(update["seq"], 7);
        assert_eq!(update["topics"]["core.game"]["running"], true);
        let alert = serde_json::to_value(CoreMessage::Alert(
            Alert::new(Severity::Critical, "alert-module-crashed").arg("module", "test"),
        ))
        .unwrap();
        assert_eq!(alert["type"], "alert");
        assert_eq!(alert["severity"], "critical");
        assert_eq!(alert["key"], "alert-module-crashed");
        assert_eq!(alert["args"]["module"], "test");
        let progress =
            serde_json::to_value(CoreMessage::ImportProgress { done: 3, total: 9 }).unwrap();
        assert_eq!(
            progress,
            serde_json::json!({"type":"import-progress","done":3,"total":9})
        );
    }
}

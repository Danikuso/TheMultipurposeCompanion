use crate::{Alert, Module, ModuleCtx, Relevance, Severity, TopicSet};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;
use tmc_model::source::SourceEvent;
use tmc_model::{
    state::{Changes, GameState},
    theme::HudTheme,
};
use tmc_store::EventWriter;
use tmc_store::StoredEvent;
use tokio::sync::{broadcast, mpsc, oneshot};

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

#[derive(Clone)]
pub struct CoreHandle {
    pub events: mpsc::Sender<SourceEvent>,
    updates: broadcast::Sender<CoreMessage>,
    snapshots: mpsc::Sender<oneshot::Sender<BTreeMap<String, Value>>>,
}
impl CoreHandle {
    pub fn subscribe(&self) -> broadcast::Receiver<CoreMessage> {
        self.updates.subscribe()
    }
    pub async fn snapshot(&self) -> BTreeMap<String, Value> {
        let (sender, receiver) = oneshot::channel();
        if self.snapshots.send(sender).await.is_err() {
            return BTreeMap::new();
        }
        receiver.await.unwrap_or_default()
    }
}

pub fn spawn_core(
    modules: Vec<Box<dyn Module>>,
    writer: Option<EventWriter>,
    flush_every: Duration,
) -> CoreHandle {
    let (events, event_receiver) = mpsc::channel(256);
    let (snapshots, snapshot_receiver) = mpsc::channel(32);
    let (updates, _) = broadcast::channel(256);
    let actor_updates = updates.clone();
    // Nunca se publica más de diez veces por segundo, incluso con configuración inválida.
    let period = flush_every.max(Duration::from_millis(100));
    tokio::spawn(async move {
        let mut actor = Actor::new(modules, writer, actor_updates);
        actor.run(event_receiver, snapshot_receiver, period).await;
    });
    CoreHandle {
        events,
        updates,
        snapshots,
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
    theme: HudTheme,
    modules: Vec<ModuleSlot>,
    topics: BTreeMap<String, Value>,
    dirty: TopicSet,
    seq: u64,
    writer: Option<EventWriter>,
    updates: broadcast::Sender<CoreMessage>,
}

impl Actor {
    fn new(
        modules: Vec<Box<dyn Module>>,
        writer: Option<EventWriter>,
        updates: broadcast::Sender<CoreMessage>,
    ) -> Self {
        let mut actor = Self {
            state: GameState::default(),
            theme: HudTheme::default(),
            modules: Vec::new(),
            topics: BTreeMap::new(),
            dirty: TopicSet::new(),
            seq: 0,
            writer,
            updates,
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
        actor
    }

    fn set_topic(&mut self, topic: &str, value: Value) {
        if self.topics.get(topic) != Some(&value) {
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
            serde_json::json!({"running":self.state.game_running,"galaxy":self.state.galaxy}),
        );
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
        if !replaying {
            let _ = self.updates.send(CoreMessage::Alert(
                Alert::new(Severity::Critical, "alert-module-crashed").arg("module", id),
            ));
        }
    }

    fn apply(&mut self, event: SourceEvent) {
        let replaying = matches!(
            &event,
            SourceEvent::Journal {
                replaying: true,
                ..
            }
        );
        let changes = match &event {
            SourceEvent::Journal { ev, file, line, .. } => {
                let changes = self.state.apply_event(ev);
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
            | SourceEvent::Stored { .. } => Changes::empty(),
        };
        self.update_core(changes);
        let mut index = 0;
        while index < self.modules.len() {
            let slot = &mut self.modules[index];
            let mut context = ModuleCtx::new(replaying);
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
                    for (topic, value) in values {
                        self.set_topic(&topic, value);
                    }
                    for alert in context.drain_alerts() {
                        let _ = self.updates.send(CoreMessage::Alert(alert));
                    }
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
    }

    fn flush(&mut self) {
        if self.dirty.is_empty() {
            return;
        }
        let topics = std::mem::take(&mut self.dirty)
            .into_iter()
            .filter_map(|topic| self.topics.get(&topic).cloned().map(|value| (topic, value)))
            .collect();
        self.seq += 1;
        let _ = self.updates.send(CoreMessage::Update {
            seq: self.seq,
            topics,
        });
    }

    async fn run(
        &mut self,
        mut events: mpsc::Receiver<SourceEvent>,
        mut snapshots: mpsc::Receiver<oneshot::Sender<BTreeMap<String, Value>>>,
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
                        let _ = reply.send(self.topics.clone());
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => snapshots_open = false,
                    Err(mpsc::error::TryRecvError::Empty) => {}
                }
                events_since_control = 0;
            }
            tokio::select! {
                biased;
                _ = tick.tick() => self.flush(),
                event = events.recv(), if events_open => match event {
                    Some(event)=>{self.apply(event); if snapshots_open { events_since_control += 1; }}, None=>events_open=false,
                },
                request = snapshots.recv(), if snapshots_open => match request {
                    Some(reply)=>{let _ = reply.send(self.topics.clone()); events_since_control = 0;}, None=>snapshots_open=false,
                },
            }
        }
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
                assert_eq!(topics.len(), 2);
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

//! Cliente trabajador de Inara (InAPI v1, E3-5).
//!
//! Cumple §7.1:
//! - Lotes de hasta 100 eventos por comandante (`per_cmdr: true`, `max_rows: 100`).
//! - Límite global compartido `RateLimiter::for_service(Service::Inara)` (2 peticiones / 60 s)
//!   compartido con la verificación de claves (`verify::check_service_key_limited`).
//! - Intervalo mínimo de 35 s entre envíos (`min_interval: 35 s`).
//! - Coalescencia de eventos `set*` previa a cada envío.
//! - Respuesta de cabecera `400` → `Hold("auth")` + `ServiceEffect::CmdrError(Auth)` sin
//!   reintento automático hasta que el comandante cambie la clave.
//! - HTTP `429` → `Retry(RateLimited, Retry-After | 120 s)`.

use super::coalesce::coalesce;
use crate::worker::{
    BatchPolicy, BoxFut, RowVerdict, SendCtx, SendOutcome, ServiceClient, ServiceEffect,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tmc_model::sync::{ErrorClass, SyncService};
use tmc_net::Transport;
use tmc_net::inara::{
    INARA_API_URL, InaraBatch, InaraLocalError, InaraVerdict, MAX_EVENTS_PER_BATCH, build_request,
    classify,
};
use tmc_net::limiter::{RateLimiter, Service, SharedRateLimiter};
use tmc_net::transport::HttpPost;
use tmc_store::OutboxRow;

pub struct InaraClient {
    transport: Arc<dyn Transport>,
    limiter: SharedRateLimiter,
}

impl InaraClient {
    pub fn new(transport: Arc<dyn Transport>, limiter: SharedRateLimiter) -> Self {
        Self { transport, limiter }
    }

    pub fn shared_limiter(&self) -> SharedRateLimiter {
        self.limiter.clone()
    }
}

fn map_verdict(verdict: InaraVerdict) -> RowVerdict {
    match verdict {
        InaraVerdict::Sent { code } => RowVerdict::Sent {
            code: code.to_string(),
        },
        InaraVerdict::Rejected { code } => RowVerdict::Rejected {
            code: code.to_string(),
            class: ErrorClass::InvalidEvent,
            hint: None,
        },
        InaraVerdict::AuthError => RowVerdict::Hold {
            code: "400".into(),
            class: ErrorClass::Auth,
        },
        InaraVerdict::Retry { class, code, after } => RowVerdict::Retry { code, class, after },
    }
}

impl ServiceClient for InaraClient {
    fn service(&self) -> SyncService {
        SyncService::Inara
    }

    fn policy(&self) -> BatchPolicy {
        BatchPolicy {
            max_rows: MAX_EVENTS_PER_BATCH,
            per_cmdr: true,
            min_interval: Duration::from_secs(35),
            limiter: RateLimiter::for_service(Service::Inara),
            flush_after: None,
        }
    }

    fn shared_limiter(&self) -> Option<SharedRateLimiter> {
        Some(self.limiter.clone())
    }

    fn needs_credential(&self) -> bool {
        true
    }

    fn filter_claimed(&self, rows: Vec<OutboxRow>) -> (Vec<OutboxRow>, Vec<(i64, RowVerdict)>) {
        let (kept, superseded_ids) = coalesce(rows);
        let verdicts = superseded_ids
            .into_iter()
            .map(|id| (id, RowVerdict::Superseded))
            .collect();
        (kept, verdicts)
    }

    fn send<'a>(&'a self, ctx: SendCtx<'a>, rows: &'a [OutboxRow]) -> BoxFut<'a, SendOutcome> {
        Box::pin(async move {
            let Some(cred) = ctx.credential else {
                return SendOutcome {
                    verdicts: rows
                        .iter()
                        .map(|r| {
                            (
                                r.id,
                                RowVerdict::Hold {
                                    code: "400".into(),
                                    class: ErrorClass::Auth,
                                },
                            )
                        })
                        .collect(),
                    effect: ServiceEffect::CmdrError(ErrorClass::Auth),
                };
            };

            let mut verdicts = Vec::with_capacity(rows.len());
            let mut events: Vec<(i64, Value)> = Vec::with_capacity(rows.len());
            for row in rows {
                match serde_json::from_str::<Value>(&row.payload) {
                    Ok(v) => events.push((row.id, v)),
                    Err(_) => verdicts.push((
                        row.id,
                        RowVerdict::Rejected {
                            code: "encode".into(),
                            class: ErrorClass::InvalidEvent,
                            hint: None,
                        },
                    )),
                }
            }

            if events.is_empty() {
                return SendOutcome {
                    verdicts,
                    effect: ServiceEffect::None,
                };
            }

            let sent_ids: Vec<i64> = events.iter().map(|(id, _)| *id).collect();
            let req_json = match build_request(&InaraBatch {
                api_key: &cred.api_key,
                commander_name: &cred.cmdr_name,
                commander_fid: &cred.cmdr_fid,
                events: &events,
            }) {
                Ok(v) => v,
                Err(InaraLocalError::MissingApiKey) => {
                    for id in sent_ids {
                        verdicts.push((
                            id,
                            RowVerdict::Hold {
                                code: "400".into(),
                                class: ErrorClass::Auth,
                            },
                        ));
                    }
                    return SendOutcome {
                        verdicts,
                        effect: ServiceEffect::CmdrError(ErrorClass::Auth),
                    };
                }
                Err(_) => {
                    for id in sent_ids {
                        verdicts.push((
                            id,
                            RowVerdict::Rejected {
                                code: "encode".into(),
                                class: ErrorClass::InvalidEvent,
                                hint: None,
                            },
                        ));
                    }
                    return SendOutcome {
                        verdicts,
                        effect: ServiceEffect::None,
                    };
                }
            };

            let Ok(body) = serde_json::to_vec(&req_json) else {
                for id in sent_ids {
                    verdicts.push((
                        id,
                        RowVerdict::Rejected {
                            code: "encode".into(),
                            class: ErrorClass::InvalidEvent,
                            hint: None,
                        },
                    ));
                }
                return SendOutcome {
                    verdicts,
                    effect: ServiceEffect::None,
                };
            };

            let reply = self
                .transport
                .post(HttpPost {
                    url: INARA_API_URL,
                    body: &body,
                    content_type: "application/json",
                    headers: &[],
                    http1_only: false,
                })
                .await;

            let classified = classify(&reply, &sent_ids);

            if let Some(header) = classified.header {
                let effect = match &header {
                    InaraVerdict::AuthError => ServiceEffect::CmdrError(ErrorClass::Auth),
                    InaraVerdict::Retry { class, after, .. } => {
                        let until = after
                            .and_then(|d| chrono::Duration::from_std(d).ok())
                            .map(|cd| ctx.now + cd);
                        ServiceEffect::ServiceError {
                            class: *class,
                            until,
                        }
                    }
                    InaraVerdict::Sent { .. } | InaraVerdict::Rejected { .. } => {
                        ServiceEffect::Connected
                    }
                };
                for id in sent_ids {
                    verdicts.push((id, map_verdict(header.clone())));
                }
                return SendOutcome { verdicts, effect };
            }

            let mut any_auth = false;
            let mut any_ok_or_rejected = false;
            let mut first_retry: Option<(ErrorClass, Option<Duration>)> = None;

            for (id, v) in classified.per_event {
                match &v {
                    InaraVerdict::AuthError => any_auth = true,
                    InaraVerdict::Sent { .. } | InaraVerdict::Rejected { .. } => {
                        any_ok_or_rejected = true;
                    }
                    InaraVerdict::Retry { class, after, .. } if first_retry.is_none() => {
                        first_retry = Some((*class, *after));
                    }
                    InaraVerdict::Retry { .. } => {}
                }
                verdicts.push((id, map_verdict(v)));
            }

            let effect = if any_auth {
                ServiceEffect::CmdrError(ErrorClass::Auth)
            } else if any_ok_or_rejected {
                ServiceEffect::Connected
            } else if let Some((class, after)) = first_retry {
                let until = after
                    .and_then(|d| chrono::Duration::from_std(d).ok())
                    .map(|cd| ctx.now + cd);
                ServiceEffect::ServiceError { class, until }
            } else {
                ServiceEffect::Connected
            };

            SendOutcome { verdicts, effect }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::TestClock;
    use crate::router::SendGates;
    use crate::verify::check_service_key_limited;
    use crate::worker::{Worker, WorkerEvent};
    use chrono::{DateTime, TimeZone, Utc};
    use serde_json::{Value, json};
    use std::collections::BTreeSet;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;
    use tmc_model::sync::{ErrorClass, KeyAssignment, OutboxState, RowOrigin};
    use tmc_net::transport::{BoxFut, HttpPost, HttpReply, HttpReplyEx, TransportError};
    use tmc_store::{NewOutboxRow, Store, SyncCredential};
    use tokio::sync::{Notify, mpsc, watch};

    const F1: &str = "F0000001";
    const F2: &str = "F0000002";

    #[derive(Clone)]
    struct RecordedSend {
        at: tokio::time::Instant,
        body: Value,
    }

    struct InaraTestTransport {
        posts: Mutex<Vec<RecordedSend>>,
        force_header_400: AtomicBool,
    }

    impl InaraTestTransport {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                posts: Mutex::new(Vec::new()),
                force_header_400: AtomicBool::new(false),
            })
        }

        fn posts(&self) -> Vec<RecordedSend> {
            self.posts.lock().unwrap().clone()
        }
    }

    impl Transport for InaraTestTransport {
        fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("unused".into())) })
        }

        fn post_form(
            &self,
            _url: &str,
            _form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("unused".into())) })
        }

        fn post<'a>(
            &'a self,
            req: HttpPost<'a>,
        ) -> BoxFut<'a, Result<HttpReplyEx, TransportError>> {
            let at = tokio::time::Instant::now();
            let parsed: Value = serde_json::from_slice(req.body).unwrap_or(Value::Null);
            self.posts.lock().unwrap().push(RecordedSend {
                at,
                body: parsed.clone(),
            });
            let force_400 = self
                .force_header_400
                .load(std::sync::atomic::Ordering::Acquire);
            let reply = if force_400 {
                json!({
                    "header": { "eventStatus": 400, "eventStatusText": "Invalid APIkey" },
                    "events": []
                })
            } else {
                let evs = parsed
                    .get("events")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let resp_events: Vec<Value> = evs
                    .into_iter()
                    .map(|ev| {
                        let cid = ev.get("eventCustomID").and_then(Value::as_i64);
                        json!({
                            "eventStatus": 200,
                            "eventCustomID": cid,
                            "eventData": {}
                        })
                    })
                    .collect();
                json!({
                    "header": { "eventStatus": 200 },
                    "events": resp_events
                })
            };
            Box::pin(async move {
                Ok(HttpReplyEx {
                    status: 200,
                    body: reply.to_string(),
                    retry_after_s: None,
                    rate_limit_remaining: None,
                    rate_limit_reset_s: None,
                })
            })
        }
    }

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap()
    }

    fn open_gates() -> Arc<SendGates> {
        Arc::new(SendGates::new(
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(true)),
        ))
    }

    fn add_key(store: &Store, fid: &str, name: &str, key: &str) {
        store
            .put_sync_credential(&SyncCredential {
                service: SyncService::Inara,
                cmdr_fid: fid.into(),
                cmdr_name: name.into(),
                api_key: key.into(),
                created_at: t0().to_rfc3339(),
                verified_at: Some(t0().to_rfc3339()),
                assignment: KeyAssignment::Confirmed,
            })
            .unwrap();
    }

    fn enqueue_events(store: &Store, fid: &str, prefix: &str, count: usize, start: DateTime<Utc>) {
        let rows: Vec<NewOutboxRow> = (0..count)
            .map(|i| {
                let ts = start + chrono::Duration::milliseconds(i as i64);
                NewOutboxRow {
                    service: SyncService::Inara,
                    cmdr_fid: fid.into(),
                    origin: RowOrigin::Live,
                    idem_key: format!("{prefix}-{i}"),
                    kind: "addCommanderTravelFSDJump".into(),
                    coalesce_key: None,
                    payload: json!({
                        "eventName": "addCommanderTravelFSDJump",
                        "eventTimestamp": ts.to_rfc3339(),
                        "eventData": {
                            "starsystemName": format!("System-{i}"),
                            "jumpDistance": 12.5
                        }
                    })
                    .to_string(),
                    event_ts: ts,
                    state: OutboxState::Pending,
                    last_class: None,
                }
            })
            .collect();
        store.enqueue_batch_with_cursor(&rows, &[], start).unwrap();
    }

    struct TestHarness {
        _clock: Arc<TestClock>,
        wake: Arc<Notify>,
        rx: mpsc::Receiver<WorkerEvent>,
        sd_tx: watch::Sender<bool>,
        handle: tokio::task::JoinHandle<()>,
    }

    fn spawn_test_worker(
        store: &Store,
        transport: Arc<dyn Transport>,
        limiter: SharedRateLimiter,
    ) -> TestHarness {
        let clock = TestClock::new(t0());
        let client: Arc<dyn ServiceClient> = Arc::new(InaraClient::new(transport, limiter));
        let wake = Arc::new(Notify::new());
        let (tx, rx) = mpsc::channel(512);
        let (sd_tx, sd_rx) = watch::channel(false);
        let worker = Worker::new(
            client,
            store.clone(),
            clock.clone(),
            open_gates(),
            wake.clone(),
            tx,
        );
        let handle = tokio::spawn(worker.run(sd_rx));
        TestHarness {
            _clock: clock,
            wake,
            rx,
            sd_tx,
            handle,
        }
    }

    async fn advance_secs(secs: u64) {
        for _ in 0..secs {
            tokio::time::advance(Duration::from_secs(1)).await;
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test]
    async fn five_hundred_events_ten_minutes_never_more_than_two_per_minute() {
        tokio::time::pause();
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "k-f1");
        enqueue_events(&store, F1, "ev", 500, t0());

        let transport = InaraTestTransport::new();
        let limiter = SharedRateLimiter::for_service(Service::Inara);
        let h = spawn_test_worker(&store, transport.clone(), limiter);

        advance_secs(600).await;
        let _ = h.sd_tx.send(true);
        let _ = h.handle.await;

        let posts = transport.posts();
        assert!(!posts.is_empty());
        let mut total_events = 0usize;
        for (idx, p) in posts.iter().enumerate() {
            let len = p.body["events"].as_array().unwrap().len();
            assert!(len <= 100, "lote {idx} excede 100 eventos: {len}");
            total_events += len;
        }
        assert_eq!(total_events, 500, "ningun evento debe perderse");
        assert!(
            store
                .pending_by_cmdr(SyncService::Inara)
                .unwrap()
                .is_empty()
        );

        for i in 0..posts.len() {
            let count_in_60s = posts
                .iter()
                .filter(|p| {
                    p.at >= posts[i].at
                        && p.at.duration_since(posts[i].at) < Duration::from_secs(60)
                })
                .count();
            assert!(
                count_in_60s <= 2,
                "mas de 2 POST en ventana de 60 s desde #{i}: {count_in_60s}"
            );
        }
    }

    #[tokio::test]
    async fn min_35s_between_sends() {
        tokio::time::pause();
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "k-f1");
        enqueue_events(&store, F1, "ev", 150, t0());

        let transport = InaraTestTransport::new();
        let limiter = SharedRateLimiter::for_service(Service::Inara);
        let h = spawn_test_worker(&store, transport.clone(), limiter);

        advance_secs(90).await;
        let _ = h.sd_tx.send(true);
        let _ = h.handle.await;

        let posts = transport.posts();
        assert_eq!(posts.len(), 2);
        let delta = posts[1].at.duration_since(posts[0].at);
        assert!(delta >= Duration::from_secs(35), "delta fue {delta:?}");
    }

    #[tokio::test]
    async fn batch_never_mixes_commanders() {
        tokio::time::pause();
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "k-f1");
        add_key(&store, F2, "CmdrTwo", "k-f2");
        enqueue_events(&store, F1, "f1", 2, t0());
        enqueue_events(&store, F2, "f2", 2, t0() + chrono::Duration::seconds(1));

        let transport = InaraTestTransport::new();
        let limiter = SharedRateLimiter::for_service(Service::Inara);
        let h = spawn_test_worker(&store, transport.clone(), limiter);

        advance_secs(80).await;
        let _ = h.sd_tx.send(true);
        let _ = h.handle.await;

        let posts = transport.posts();
        assert_eq!(posts.len(), 2);
        let fids: BTreeSet<String> = posts
            .iter()
            .map(|p| {
                p.body["header"]["commanderFrontierID"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(fids.len(), 2);
        for p in &posts {
            assert_eq!(p.body["events"].as_array().unwrap().len(), 2);
        }
    }

    #[tokio::test]
    async fn oldest_pending_commander_goes_first() {
        tokio::time::pause();
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "k-f1");
        add_key(&store, F2, "CmdrTwo", "k-f2");
        // F2 tiene el pendiente mas antiguo aunque F1 vaya antes alfabeticamente.
        enqueue_events(&store, F1, "f1", 1, t0() + chrono::Duration::seconds(20));
        enqueue_events(&store, F2, "f2", 1, t0());

        let transport = InaraTestTransport::new();
        let limiter = SharedRateLimiter::for_service(Service::Inara);
        let h = spawn_test_worker(&store, transport.clone(), limiter);

        advance_secs(5).await;
        let _ = h.sd_tx.send(true);
        let _ = h.handle.await;

        let posts = transport.posts();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].body["header"]["commanderFrontierID"], F2);
    }

    #[tokio::test]
    async fn header_400_holds_until_key_change() {
        tokio::time::pause();
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "bad-key");
        enqueue_events(&store, F1, "h400", 1, t0());

        let transport = InaraTestTransport::new();
        transport
            .force_header_400
            .store(true, std::sync::atomic::Ordering::Release);
        let limiter = SharedRateLimiter::for_service(Service::Inara);
        let mut h = spawn_test_worker(&store, transport.clone(), limiter);

        advance_secs(300).await;
        assert_eq!(
            transport.posts().len(),
            1,
            "header 400 no debe reintentar automaticamente"
        );

        let mut saw_auth = false;
        while let Ok(ev) = h.rx.try_recv() {
            if matches!(
                ev,
                WorkerEvent::Outcome {
                    effect: ServiceEffect::CmdrError(ErrorClass::Auth),
                    ..
                }
            ) {
                saw_auth = true;
            }
        }
        assert!(saw_auth);

        transport
            .force_header_400
            .store(false, std::sync::atomic::Ordering::Release);
        add_key(&store, F1, "CmdrOne", "good-key");
        h.wake.notify_one();
        advance_secs(5).await;

        let _ = h.sd_tx.send(true);
        let _ = h.handle.await;

        let posts = transport.posts();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[1].body["header"]["APIkey"], "good-key");
        assert!(
            store
                .pending_by_cmdr(SyncService::Inara)
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn key_verification_consumes_limiter_slot() {
        tokio::time::pause();
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "k-f1");
        enqueue_events(&store, F1, "lim", 150, t0());

        let transport = InaraTestTransport::new();
        let limiter = SharedRateLimiter::for_service(Service::Inara);

        // Una verificacion de clave Inara consume 1 hueco de los 2/60 s al arrancar.
        let check = check_service_key_limited(
            transport.as_ref(),
            SyncService::Inara,
            "CmdrOne",
            Some(F1),
            "k-f1",
            Some(&limiter),
        )
        .await;
        assert_eq!(check, tmc_net::KeyCheck::Valid);

        let h = spawn_test_worker(&store, transport.clone(), limiter);

        // Antes de los 60 s solo cabe 1 POST de trabajo (el 2.o hueco de la ventana).
        advance_secs(50).await;
        let posts_before_60 = transport.posts();
        let worker_posts_before_60 = posts_before_60
            .iter()
            .filter(|p| p.body["events"][0]["eventName"] != "getCommanderProfile")
            .count();
        assert_eq!(worker_posts_before_60, 1);

        // Pasados los 60 s vuelve a liberarse el hueco consumido por la verificacion.
        advance_secs(20).await;
        let _ = h.sd_tx.send(true);
        let _ = h.handle.await;

        let worker_posts_after: Vec<_> = transport
            .posts()
            .into_iter()
            .filter(|p| p.body["events"][0]["eventName"] != "getCommanderProfile")
            .collect();
        assert_eq!(worker_posts_after.len(), 2);
        assert!(
            worker_posts_after[1]
                .at
                .duration_since(worker_posts_after[0].at)
                >= Duration::from_secs(60)
        );
    }

    #[tokio::test]
    async fn http_429_retries_with_retry_after_or_120s_and_coalesces_before_send() {
        let store = Store::open_in_memory().unwrap();
        add_key(&store, F1, "CmdrOne", "k-f1");
        let ts1 = t0();
        let ts2 = ts1 + chrono::Duration::seconds(5);
        let mk_set = |idem: &str, credits: u64, ts: DateTime<Utc>| NewOutboxRow {
            service: SyncService::Inara,
            cmdr_fid: F1.into(),
            origin: RowOrigin::Live,
            idem_key: idem.into(),
            kind: "setCommanderCredits".into(),
            coalesce_key: Some(format!("{F1}|setCommanderCredits|")),
            payload: json!({
                "eventName": "setCommanderCredits",
                "eventTimestamp": ts.to_rfc3339(),
                "eventData": { "commanderCredits": credits }
            })
            .to_string(),
            event_ts: ts,
            state: OutboxState::Pending,
            last_class: None,
        };
        store
            .enqueue_batch_with_cursor(&[mk_set("s1", 100, ts1), mk_set("s2", 200, ts2)], &[], ts2)
            .unwrap();

        let limiter = SharedRateLimiter::for_service(Service::Inara);
        let transport = tmc_net::testing::ScriptedTransport::new(vec![
            Ok(HttpReplyEx {
                status: 429,
                body: String::new(),
                retry_after_s: Some(90),
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            }),
            Ok(HttpReplyEx {
                status: 429,
                body: String::new(),
                retry_after_s: None,
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            }),
        ]);
        let client = InaraClient::new(transport, limiter);

        // Simulamos un lote con 2 filas reclamadas para probar coalescencia en filter_claimed + 429.
        let claimed = vec![
            OutboxRow {
                id: 1,
                service: SyncService::Inara,
                cmdr_fid: F1.into(),
                kind: "setCommanderCredits".into(),
                coalesce_key: Some(format!("{F1}|setCommanderCredits|")),
                event_ts: ts1,
                payload: mk_set("c1", 100, ts1).payload,
                origin: RowOrigin::Live,
                attempts: 0,
            },
            OutboxRow {
                id: 2,
                service: SyncService::Inara,
                cmdr_fid: F1.into(),
                kind: "setCommanderCredits".into(),
                coalesce_key: Some(format!("{F1}|setCommanderCredits|")),
                event_ts: ts2,
                payload: mk_set("c2", 200, ts2).payload,
                origin: RowOrigin::Live,
                attempts: 0,
            },
        ];

        let (kept, mut pre_verdicts) = client.filter_claimed(claimed);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, 2);
        assert_eq!(pre_verdicts.pop(), Some((1, RowVerdict::Superseded)));

        let cred = store
            .sync_credential(SyncService::Inara, F1)
            .unwrap()
            .unwrap();
        let out1 = client
            .send(
                SendCtx {
                    cmdr_fid: F1,
                    credential: Some(&cred),
                    now: ts2,
                },
                &kept,
            )
            .await;
        assert_eq!(
            out1.verdicts,
            vec![(
                2,
                RowVerdict::Retry {
                    code: "inara_429".into(),
                    class: ErrorClass::RateLimited,
                    after: Some(Duration::from_secs(90)),
                }
            )]
        );
        assert_eq!(
            out1.effect,
            ServiceEffect::ServiceError {
                class: ErrorClass::RateLimited,
                until: Some(ts2 + chrono::Duration::seconds(90)),
            }
        );

        let out2 = client
            .send(
                SendCtx {
                    cmdr_fid: F1,
                    credential: Some(&cred),
                    now: ts2,
                },
                &kept,
            )
            .await;
        assert_eq!(
            out2.verdicts,
            vec![(
                2,
                RowVerdict::Retry {
                    code: "inara_429".into(),
                    class: ErrorClass::RateLimited,
                    after: Some(Duration::from_secs(120)),
                }
            )]
        );
    }
}

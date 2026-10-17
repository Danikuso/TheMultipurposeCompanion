//! `NetService`: compone el cliente de red con la caché SQLite, el opt-out,
//! la unificación de peticiones idénticas y los reintentos con retroceso.
//!
//! Decisiones:
//! - Cada petición se atiende en su propia tarea; el bucle del servicio solo
//!   recibe y despacha, así que nunca se bloquea ni bloquea al actor.
//! - La frescura se calcula con `fetched_at` y `ttl_for(kind)` (`api_cache`
//!   guarda `ttl_s = 0`). La respuesta `System` se parte en ficha (24 h) y
//!   política (1 h) y se recompone al servirla desde caché.
//! - Peticiones idénticas (misma consulta) en curso comparten una sola llamada
//!   y cada una recibe su respuesta con su propio `id`/`key`. Durante el
//!   retroceso de un reintento el grupo ya no está «en curso»: una petición
//!   nueva idéntica llama por su cuenta (acotado por la salud del servicio).
//! - Caché caducada pero dentro de `max_stale`: se responde con ella y se
//!   refresca una sola vez en segundo plano; si el refresco funciona, se envía
//!   además la respuesta nueva con el mismo `id`. Si falla, no se reintenta.
//! - Reintentos: tras `Failed(..)` se consulta `NetClient::retry_at` y se
//!   duerme hasta ese instante en la tarea de la petición. Si el servicio cae,
//!   se envía una última `Failed(Down)` y se deja de reprogramar.
//! - `Disabled` (opt-out) se responde con `source: Cache`, sin tocar el
//!   transporte ni la caché.

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tmc_core::Store;
use tmc_model::net::{
    NetFailure, NetOutcome, NetQuery, NetRequest, NetResponse, NetSource, RemoteSystem,
    SystemSuggestion,
};
use tmc_model::source::SourceEvent;
use tmc_net::{CacheKind, NetClient, cache_keys, ttl_for};
use tokio::sync::mpsc;

/// Mínimo de caracteres para consultar el autocompletado.
const AUTOCOMPLETE_MIN_CHARS: usize = 3;
const CARD_FIELDS: [&str; 3] = ["discovered", "body_count", "primary_star"];

#[derive(Clone)]
pub struct NetServiceHandle {
    pub tx: mpsc::Sender<NetRequest>,
    client: Arc<NetClient>,
    online: Arc<AtomicBool>,
}

impl NetServiceHandle {
    /// Autocompletado para REST (sin pasar por el actor).
    pub async fn autocomplete(&self, prefix: &str) -> Result<Vec<SystemSuggestion>, NetFailure> {
        if prefix.chars().count() < AUTOCOMPLETE_MIN_CHARS || !self.online.load(Ordering::Relaxed) {
            return Ok(Vec::new());
        }
        let query = NetQuery::Autocomplete {
            prefix: prefix.to_owned(),
        };
        match self.client.execute(&query).await.1 {
            NetOutcome::Autocomplete(items) => Ok(items),
            NetOutcome::Failed(failure) => Err(failure),
            _ => Ok(Vec::new()),
        }
    }
}

struct Waiter {
    id: u64,
    module: String,
    key: String,
}

#[derive(Default)]
struct Shared {
    inflight: HashMap<String, Vec<Waiter>>,
    refreshing: HashSet<String>,
}

struct Ctx {
    client: Arc<NetClient>,
    store: Store,
    events: mpsc::Sender<SourceEvent>,
    online: Arc<AtomicBool>,
    shared: Mutex<Shared>,
}

pub fn spawn_net_service(
    client: Arc<NetClient>,
    store: Store,
    events: mpsc::Sender<SourceEvent>,
    online: Arc<AtomicBool>,
) -> NetServiceHandle {
    let (tx, mut rx) = mpsc::channel::<NetRequest>(64);
    let ctx = Arc::new(Ctx {
        client: client.clone(),
        store,
        events,
        online: online.clone(),
        shared: Mutex::new(Shared::default()),
    });
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            tokio::spawn(handle(ctx.clone(), request));
        }
    });
    NetServiceHandle { tx, client, online }
}

fn response(w: &Waiter, source: NetSource, at: DateTime<Utc>, outcome: NetOutcome) -> NetResponse {
    NetResponse {
        id: w.id,
        module: w.module.clone(),
        key: w.key.clone(),
        source,
        fetched_at: at,
        outcome,
    }
}

async fn send_all(
    ctx: &Ctx,
    waiters: &[Waiter],
    source: NetSource,
    at: DateTime<Utc>,
    outcome: &NetOutcome,
) {
    for w in waiters {
        let ev = SourceEvent::Net(response(w, source, at, outcome.clone()));
        if ctx.events.send(ev).await.is_err() {
            return;
        }
    }
}

async fn handle(ctx: Arc<Ctx>, req: NetRequest) {
    let waiter = Waiter {
        id: req.id,
        module: req.module,
        key: req.key,
    };
    if !ctx.online.load(Ordering::Relaxed) {
        send_all(
            &ctx,
            &[waiter],
            NetSource::Cache,
            Utc::now(),
            &NetOutcome::Disabled,
        )
        .await;
        return;
    }
    let qkey = serde_json::to_string(&req.query).unwrap_or_default();
    {
        let mut shared = ctx.shared.lock().unwrap();
        if let Some(group) = shared.inflight.get_mut(&qkey) {
            group.push(waiter);
            return;
        }
        shared.inflight.insert(qkey.clone(), vec![waiter]);
    }
    lead(ctx, qkey, req.query).await;
}

fn take_group(ctx: &Ctx, qkey: &str) -> Vec<Waiter> {
    ctx.shared
        .lock()
        .unwrap()
        .inflight
        .remove(qkey)
        .unwrap_or_default()
}

async fn lead(ctx: Arc<Ctx>, qkey: String, query: NetQuery) {
    let cached = read_cache(&ctx.store, &query).await;
    if let Some(hit) = cached {
        let age_ok = |kind: CacheKind, at: DateTime<Utc>| age(at) < ttl_for(kind).fresh;
        let fresh = age_ok(CacheKind::SystemCard, hit.card_at)
            && age_ok(CacheKind::SystemPolitics, hit.politics_at);
        let usable = age(hit.card_at) < ttl_for(CacheKind::SystemCard).max_stale
            && age(hit.politics_at) < ttl_for(CacheKind::SystemPolitics).max_stale;
        if fresh || usable {
            let waiters = take_group(&ctx, &qkey);
            let at = hit.card_at.min(hit.politics_at);
            send_all(
                &ctx,
                &waiters,
                NetSource::Cache,
                at,
                &NetOutcome::System(hit.system),
            )
            .await;
            if fresh {
                return;
            }
            // Una sola actualización en segundo plano por consulta.
            if !ctx.shared.lock().unwrap().refreshing.insert(qkey.clone()) {
                return;
            }
            refresh(ctx, qkey, query, waiters).await;
            return;
        }
    }
    let (source, outcome) = ctx.client.execute(&query).await;
    let waiters = take_group(&ctx, &qkey);
    finish(&ctx, &query, &waiters, source, &outcome).await;
    if matches!(outcome, NetOutcome::Failed(_)) {
        retry(ctx, query, waiters, outcome).await;
    }
}

/// Refresco en segundo plano tras servir una caché caducada.
async fn refresh(ctx: Arc<Ctx>, qkey: String, query: NetQuery, waiters: Vec<Waiter>) {
    let (source, outcome) = ctx.client.execute(&query).await;
    ctx.shared.lock().unwrap().refreshing.remove(&qkey);
    if matches!(outcome, NetOutcome::System(_)) {
        finish(&ctx, &query, &waiters, source, &outcome).await;
    }
}

/// Guarda en caché (si procede) y responde a todo el grupo.
async fn finish(
    ctx: &Ctx,
    query: &NetQuery,
    waiters: &[Waiter],
    source: NetSource,
    outcome: &NetOutcome,
) {
    let now = Utc::now();
    if let NetOutcome::System(system) = outcome {
        write_cache(&ctx.store, query, system, now).await;
    }
    send_all(ctx, waiters, source, now, outcome).await;
}

async fn retry(ctx: Arc<Ctx>, query: NetQuery, waiters: Vec<Waiter>, first: NetOutcome) {
    let mut last = first;
    loop {
        let Some(at) = ctx.client.retry_at(&query) else {
            if ctx.client.is_down_for(&query) && last != NetOutcome::Failed(NetFailure::Down) {
                let down = NetOutcome::Failed(NetFailure::Down);
                send_all(&ctx, &waiters, NetSource::Cache, Utc::now(), &down).await;
            }
            return;
        };
        tokio::time::sleep_until(at).await;
        if !ctx.online.load(Ordering::Relaxed) {
            send_all(
                &ctx,
                &waiters,
                NetSource::Cache,
                Utc::now(),
                &NetOutcome::Disabled,
            )
            .await;
            return;
        }
        let (source, outcome) = ctx.client.execute(&query).await;
        finish(&ctx, &query, &waiters, source, &outcome).await;
        if !matches!(outcome, NetOutcome::Failed(_)) {
            return;
        }
        last = outcome;
    }
}

fn age(at: DateTime<Utc>) -> std::time::Duration {
    (Utc::now() - at).to_std().unwrap_or_default()
}

struct CacheHit {
    system: RemoteSystem,
    card_at: DateTime<Utc>,
    politics_at: DateTime<Utc>,
}

/// Lee ficha y política de un `System`; cualquier fallo cuenta como «sin caché».
async fn read_cache(store: &Store, query: &NetQuery) -> Option<CacheHit> {
    let keys = cache_keys(query);
    let card_key = keys
        .iter()
        .find(|(k, _)| *k == CacheKind::SystemCard)?
        .1
        .clone();
    let politics_key = keys
        .iter()
        .find(|(k, _)| *k == CacheKind::SystemPolitics)?
        .1
        .clone();
    let store = store.clone();
    let result = tokio::task::spawn_blocking(move || {
        Ok::<_, tmc_core::StoreError>((
            store.api_cache_get(&card_key)?,
            store.api_cache_get(&politics_key)?,
        ))
    })
    .await;
    let (card, politics) = match result {
        Ok(Ok((Some(card), Some(politics)))) => (card, politics),
        Ok(Err(error)) => {
            tracing::warn!(%error, "api_cache read failed");
            return None;
        }
        _ => return None,
    };
    let mut merged: Map<String, Value> = serde_json::from_str(&politics.value).ok()?;
    let card_map: Map<String, Value> = serde_json::from_str(&card.value).ok()?;
    merged.extend(card_map);
    let system = serde_json::from_value(Value::Object(merged)).ok()?;
    Some(CacheHit {
        system,
        card_at: card.fetched_at,
        politics_at: politics.fetched_at,
    })
}

async fn write_cache(store: &Store, query: &NetQuery, system: &RemoteSystem, at: DateTime<Utc>) {
    let keys = cache_keys(query);
    let (Some(card_key), Some(politics_key)) = (
        keys.iter().find(|(k, _)| *k == CacheKind::SystemCard),
        keys.iter().find(|(k, _)| *k == CacheKind::SystemPolitics),
    ) else {
        return;
    };
    let Ok(Value::Object(full)) = serde_json::to_value(system) else {
        return;
    };
    let (mut card, mut politics) = (Map::new(), Map::new());
    for (name, value) in full {
        if CARD_FIELDS.contains(&name.as_str()) {
            card.insert(name, value);
        } else {
            politics.insert(name, value);
        }
    }
    let card = Value::Object(card).to_string();
    let politics = Value::Object(politics).to_string();
    let (card_key, politics_key) = (card_key.1.clone(), politics_key.1.clone());
    let store = store.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.api_cache_put(&card_key, &card, at)?;
        store.api_cache_put(&politics_key, &politics, at)
    })
    .await;
    if let Ok(Err(error)) = result {
        tracing::warn!(%error, "api_cache write failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;
    use tmc_net::{BoxFut, HttpReply, Transport, TransportError};

    const SYSTEM: &str = include_str!("../../../tests/fixtures/net/spansh_system_ok.json");
    const ID64: u64 = 10477373803;

    type Reply = Result<HttpReply, TransportError>;

    fn ok(status: u16, body: &str) -> Reply {
        Ok(HttpReply {
            status,
            body: body.to_owned(),
        })
    }

    struct Fake {
        replies: StdMutex<VecDeque<Reply>>,
        calls: StdMutex<u32>,
    }

    impl Fake {
        fn new(replies: Vec<Reply>) -> Arc<Self> {
            Arc::new(Self {
                replies: StdMutex::new(replies.into()),
                calls: StdMutex::new(0),
            })
        }
        fn calls(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
        fn next(&self) -> Reply {
            *self.calls.lock().unwrap() += 1;
            let mut q = self.replies.lock().unwrap();
            if q.len() > 1 {
                q.pop_front().unwrap()
            } else {
                q.front().cloned().expect("sin respuestas")
            }
        }
    }

    impl Transport for Fake {
        fn get(&self, _url: &str) -> BoxFut<'_, Reply> {
            let r = self.next();
            Box::pin(async move { r })
        }
        fn post_form(&self, _url: &str, _form: &[(String, String)]) -> BoxFut<'_, Reply> {
            let r = self.next();
            Box::pin(async move { r })
        }
    }

    struct Rig {
        handle: NetServiceHandle,
        events: mpsc::Receiver<SourceEvent>,
        store: Store,
        online: Arc<AtomicBool>,
        fake: Arc<Fake>,
    }

    fn rig(replies: Vec<Reply>) -> Rig {
        let fake = Fake::new(replies);
        let store = Store::open_in_memory().unwrap();
        let (tx, events) = mpsc::channel(64);
        let online = Arc::new(AtomicBool::new(true));
        let handle = spawn_net_service(
            Arc::new(NetClient::new(fake.clone())),
            store.clone(),
            tx,
            online.clone(),
        );
        Rig {
            handle,
            events,
            store,
            online,
            fake,
        }
    }

    fn sol() -> NetQuery {
        NetQuery::System {
            id64: ID64,
            name: "Sol".into(),
        }
    }

    fn request(id: u64) -> NetRequest {
        NetRequest {
            id,
            module: "navigation".into(),
            key: format!("system:{ID64}"),
            query: sol(),
        }
    }

    async fn next_response(rig: &mut Rig) -> NetResponse {
        match rig.events.recv().await.expect("canal cerrado") {
            SourceEvent::Net(r) => r,
            other => panic!("{other:?}"),
        }
    }

    fn cached_sol() -> RemoteSystem {
        RemoteSystem {
            id64: ID64,
            name: "Sol".into(),
            discovered: Some(true),
            body_count: Some(7),
            primary_star: Some("K".into()),
            allegiance: Some("Federation".into()),
            government: None,
            controlling_faction: None,
            faction_state: None,
            economy: Some("Refinery".into()),
            second_economy: None,
            security: None,
            population: Some(1234),
            powers: vec!["Aisling Duval".into()],
            powerplay_state: None,
        }
    }

    /// Siembra la caché con `card_age`/`politics_age` de antigüedad.
    fn seed(store: &Store, card_age: Duration, politics_age: Duration) {
        let Value::Object(full) = serde_json::to_value(cached_sol()).unwrap() else {
            unreachable!()
        };
        let (mut card, mut politics) = (Map::new(), Map::new());
        for (k, v) in full {
            if CARD_FIELDS.contains(&k.as_str()) {
                card.insert(k, v);
            } else {
                politics.insert(k, v);
            }
        }
        let ago = |d: Duration| Utc::now() - chrono::Duration::from_std(d).unwrap();
        store
            .api_cache_put(
                &format!("spansh:system:{ID64}:card"),
                &Value::Object(card).to_string(),
                ago(card_age),
            )
            .unwrap();
        store
            .api_cache_put(
                &format!("spansh:system:{ID64}:politics"),
                &Value::Object(politics).to_string(),
                ago(politics_age),
            )
            .unwrap();
    }

    const MIN: u64 = 60;
    const HOUR: u64 = 3600;

    #[tokio::test]
    async fn disabled_online_services_never_reach_transport() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        rig.online.store(false, Ordering::Relaxed);
        rig.handle.tx.send(request(1)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.id, 1);
        assert_eq!(r.outcome, NetOutcome::Disabled);
        assert_eq!(rig.fake.calls(), 0);
    }

    #[tokio::test]
    async fn fresh_cache_hit_skips_transport() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        seed(
            &rig.store,
            Duration::from_secs(10 * MIN),
            Duration::from_secs(10 * MIN),
        );
        rig.handle.tx.send(request(4)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.id, 4);
        assert_eq!(r.source, NetSource::Cache);
        assert_eq!(r.outcome, NetOutcome::System(cached_sol()));
        assert_eq!(rig.fake.calls(), 0);
    }

    #[tokio::test]
    async fn cache_beyond_max_stale_is_ignored() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        // Ficha y política de hace 25 h: fuera de max_stale (24 h), se llama a la red.
        seed(
            &rig.store,
            Duration::from_secs(25 * HOUR),
            Duration::from_secs(25 * HOUR),
        );
        rig.handle.tx.send(request(1)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.source, NetSource::Spansh);
        assert_eq!(rig.fake.calls(), 1);
    }

    #[tokio::test]
    async fn stale_cache_is_served_and_refreshed_once() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        // 23 h: la política (fresca 1 h) está caducada pero dentro de max_stale.
        seed(
            &rig.store,
            Duration::from_secs(23 * HOUR),
            Duration::from_secs(23 * HOUR),
        );
        rig.handle.tx.send(request(1)).await.unwrap();
        let first = next_response(&mut rig).await;
        assert_eq!(first.source, NetSource::Cache);
        assert_eq!(first.id, 1);
        assert_eq!(first.outcome, NetOutcome::System(cached_sol()));
        let refreshed = next_response(&mut rig).await;
        assert_eq!(refreshed.id, 1);
        assert_eq!(refreshed.source, NetSource::Spansh);
        assert_eq!(rig.fake.calls(), 1);
        // Tras el refresco la caché vuelve a estar fresca: 0 llamadas más.
        rig.handle.tx.send(request(2)).await.unwrap();
        let second = next_response(&mut rig).await;
        assert_eq!(second.id, 2);
        assert_eq!(second.source, NetSource::Cache);
        assert_eq!(rig.fake.calls(), 1);
    }

    #[tokio::test]
    async fn politics_expires_after_1h_while_card_lasts_24h() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        seed(
            &rig.store,
            Duration::from_secs(2 * HOUR),
            Duration::from_secs(2 * HOUR),
        );
        rig.handle.tx.send(request(1)).await.unwrap();
        let first = next_response(&mut rig).await;
        assert_eq!(first.source, NetSource::Cache);
        let refreshed = next_response(&mut rig).await;
        assert_eq!(refreshed.source, NetSource::Spansh);
        assert_eq!(rig.fake.calls(), 1);

        let mut rig = self::rig(vec![ok(200, SYSTEM)]);
        seed(
            &rig.store,
            Duration::from_secs(2 * HOUR),
            Duration::from_secs(10 * MIN),
        );
        rig.handle.tx.send(request(1)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.source, NetSource::Cache);
        // `fetched_at` es el de la ficha más antigua (2 h).
        assert_eq!((Utc::now() - r.fetched_at).num_minutes(), 120);
        assert_eq!(rig.fake.calls(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn failed_request_is_retried_after_backoff() {
        let mut rig = rig(vec![ok(503, ""), ok(200, SYSTEM)]);
        rig.handle.tx.send(request(9)).await.unwrap();
        let first = next_response(&mut rig).await;
        assert_eq!(first.id, 9);
        assert_eq!(first.outcome, NetOutcome::Failed(NetFailure::Http(503)));
        assert_eq!(rig.fake.calls(), 1);
        tokio::time::sleep(Duration::from_secs(29)).await;
        assert_eq!(rig.fake.calls(), 1);
        tokio::time::sleep(Duration::from_secs(2)).await;
        let second = next_response(&mut rig).await;
        assert_eq!(second.id, 9);
        assert_eq!(second.source, NetSource::Spansh);
        let NetOutcome::System(s) = second.outcome else {
            panic!("{:?}", second.outcome)
        };
        assert_eq!(s.name, "Sol");
        assert_eq!(rig.fake.calls(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn retries_stop_when_service_goes_down() {
        let mut rig = rig(vec![ok(503, "")]);
        rig.handle.tx.send(request(1)).await.unwrap();
        let mut outcomes = Vec::new();
        // 3 fallos de Spansh, luego EDSM (sin guion: mismo 503) hasta caer también.
        tokio::time::timeout(Duration::from_secs(3600), async {
            while let Some(SourceEvent::Net(r)) = rig.events.recv().await {
                outcomes.push(r.outcome.clone());
                if r.outcome == NetOutcome::Failed(NetFailure::Down) {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(outcomes.last(), Some(&NetOutcome::Failed(NetFailure::Down)));
        let calls = rig.fake.calls();
        tokio::time::sleep(Duration::from_secs(3600)).await;
        assert_eq!(rig.fake.calls(), calls);
        assert!(rig.events.try_recv().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn identical_inflight_requests_are_coalesced() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        for id in 1..=3 {
            rig.handle.tx.send(request(id)).await.unwrap();
        }
        let mut ids = Vec::new();
        for _ in 0..3 {
            let r = next_response(&mut rig).await;
            assert!(matches!(r.outcome, NetOutcome::System(_)));
            ids.push(r.id);
        }
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(rig.fake.calls(), 1);
    }

    #[tokio::test]
    async fn autocomplete_respects_minimum_length_and_opt_out() {
        let rig = rig(vec![ok(200, r#"{"min_max":[],"values":["Sol"]}"#)]);
        assert_eq!(rig.handle.autocomplete("So").await, Ok(vec![]));
        assert_eq!(rig.fake.calls(), 0);
        let got = rig.handle.autocomplete("Sol").await.unwrap();
        assert_eq!(got[0].name, "Sol");
        assert_eq!(rig.fake.calls(), 1);
        rig.online.store(false, Ordering::Relaxed);
        assert_eq!(rig.handle.autocomplete("Sol").await, Ok(vec![]));
        assert_eq!(rig.fake.calls(), 1);
    }
}

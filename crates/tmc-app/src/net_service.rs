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
//! - Como máximo `MAX_CONCURRENT` llamadas simultáneas al cliente (semáforo);
//!   las demás esperan turno en su propia tarea, sin bloquear el bucle.
//! - El grupo sigue «en curso» durante el retroceso de un reintento: una
//!   petición idéntica nueva se une a él y recibe la respuesta del reintento
//!   con su propio `id`, sin llamar al transporte.
//! - Las claves de caché llevan el prefijo `spansh:` también cuando el
//!   `System` proviene de EDSM (aceptado). Cada intento fallido envía su
//!   `Failed(..)`, así que los consumidores deben ser idempotentes por `id`.
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
use tokio::sync::{Semaphore, mpsc};

/// Evento de diagnóstico del servicio de red (solo debug; sin URLs ni texto de errores).
macro_rules! svc_event {
    ($code:literal, { $($payload:tt)* }) => {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::NetOutcome,
            $code,
            tmc_model::diagnostics::json!({ $($payload)* })
        )
    };
}

/// Huella estable de la consulta (la misma que usa `NetClient`).
#[cfg(debug_assertions)]
fn fp(query: &NetQuery) -> u64 {
    tmc_net::client::query_fingerprint(query)
}

/// Mínimo de caracteres para consultar el autocompletado.
const AUTOCOMPLETE_MIN_CHARS: usize = 3;
/// Llamadas simultáneas máximas al cliente de red.
const MAX_CONCURRENT: usize = 4;
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
            svc_event!("net.service.autocomplete_skipped", {
                "too_short": prefix.chars().count() < AUTOCOMPLETE_MIN_CHARS,
                "online": self.online.load(Ordering::Relaxed)});
            return Ok(Vec::new());
        }
        let query = NetQuery::Autocomplete {
            prefix: prefix.to_owned(),
        };
        match self
            .client
            .execute_with_online(&query, &self.online)
            .await
            .1
        {
            NetOutcome::Autocomplete(items) => Ok(items),
            NetOutcome::Failed(failure) => Err(failure),
            _ => Ok(Vec::new()),
        }
    }
}

#[derive(Clone)]
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
    permits: Semaphore,
    /// Canonn es lento y limitado a 1/s: no acapara los permisos globales.
    canonn_permits: Semaphore,
}

/// Contrato: la defensa de la galaxia Legacy es de los módulos (no emiten
/// peticiones en Legacy); el servicio no la comprueba.
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
        permits: Semaphore::new(MAX_CONCURRENT),
        canonn_permits: Semaphore::new(1),
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
    svc_event!("net.service.responded", {"waiters": waiters.len(),
        "ids": waiters.iter().map(|w| w.id).collect::<Vec<_>>(),
        "source": source, "outcome": outcome.diag_summary().0,
        "items": outcome.diag_summary().1});
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
    svc_event!("net.service.received", {"id": waiter.id, "query_fp": fp(&req.query)});
    if !ctx.online.load(Ordering::Relaxed) {
        svc_event!("net.optout.blocked", {"stage": 3, "id": waiter.id});
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
            svc_event!("net.coalesced.joined", {"id": waiter.id, "query_fp": fp(&req.query),
                "waiters": group.len() + 1});
            group.push(waiter);
            return;
        }
        svc_event!("net.coalesced.leader", {"id": waiter.id, "query_fp": fp(&req.query)});
        shared.inflight.insert(qkey.clone(), vec![waiter]);
    }
    lead(ctx, qkey, req.query).await;
}

fn group(ctx: &Ctx, qkey: &str) -> Vec<Waiter> {
    ctx.shared
        .lock()
        .unwrap()
        .inflight
        .get(qkey)
        .cloned()
        .unwrap_or_default()
}

/// Llamada al cliente con turno de concurrencia; `None` si el opt-out está
/// activo justo antes de llamar.
async fn run(ctx: &Ctx, query: &NetQuery) -> Option<(NetSource, NetOutcome)> {
    let permits = match query {
        NetQuery::BodyBio { .. } => &ctx.canonn_permits,
        _ => &ctx.permits,
    };
    #[cfg(debug_assertions)]
    let waited_since = std::time::Instant::now();
    svc_event!("net.service.permit_wait", {"canonn_pool": matches!(query, NetQuery::BodyBio { .. }),
        "available": permits.available_permits(), "query_fp": fp(query)});
    let _permit = permits.acquire().await.ok()?;
    svc_event!("net.service.permit_acquired", {"canonn_pool": matches!(query, NetQuery::BodyBio { .. }),
        "waited_ms": waited_since.elapsed().as_millis() as u64, "query_fp": fp(query)});
    if !ctx.online.load(Ordering::Relaxed) {
        svc_event!("net.optout.blocked", {"stage": 4, "query_fp": fp(query)});
        return None;
    }
    let result = ctx.client.execute_with_online(query, &ctx.online).await;
    if result.1 == NetOutcome::Disabled && !ctx.online.load(Ordering::Acquire) {
        None
    } else {
        Some(result)
    }
}

async fn disabled(ctx: &Ctx, qkey: &str) {
    let waiters = take_group(ctx, qkey);
    send_all(
        ctx,
        &waiters,
        NetSource::Cache,
        Utc::now(),
        &NetOutcome::Disabled,
    )
    .await;
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
    if let Some((outcome, at)) = read_api_cache(&ctx.store, &query).await {
        svc_event!("net.cache.api.hit", {"query_fp": fp(&query), "age_s": age(at).as_secs()});
        let waiters = take_group(&ctx, &qkey);
        send_all(&ctx, &waiters, NetSource::Cache, at, &outcome).await;
        return;
    }
    let cached = read_cache(&ctx.store, &query).await;
    if let Some(hit) = cached {
        let age_ok = |kind: CacheKind, at: DateTime<Utc>| age(at) < ttl_for(kind).fresh;
        let fresh = age_ok(CacheKind::SystemCard, hit.card_at)
            && age_ok(CacheKind::SystemPolitics, hit.politics_at);
        let usable = age(hit.card_at) < ttl_for(CacheKind::SystemCard).max_stale
            && age(hit.politics_at) < ttl_for(CacheKind::SystemPolitics).max_stale;
        svc_event!("net.cache.system.entry", {"query_fp": fp(&query), "fresh": fresh,
            "usable": usable, "card_age_s": age(hit.card_at).as_secs(),
            "politics_age_s": age(hit.politics_at).as_secs()});
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
                svc_event!("net.cache.refresh.already_running", {"query_fp": fp(&query)});
                return;
            }
            svc_event!("net.cache.refresh.started", {"query_fp": fp(&query)});
            refresh(ctx, qkey, query, waiters).await;
            return;
        }
    }
    svc_event!("net.cache.miss", {"query_fp": fp(&query)});
    let Some((source, outcome)) = run(&ctx, &query).await else {
        disabled(&ctx, &qkey).await;
        return;
    };
    settle(&ctx, &qkey, &query, source, &outcome).await;
    if matches!(outcome, NetOutcome::Failed(_)) {
        retry(ctx, qkey, query, outcome).await;
    }
}

/// Responde al grupo: un fallo deja el grupo en curso (esperará su reintento);
/// cualquier otro resultado lo cierra.
async fn settle(ctx: &Ctx, qkey: &str, query: &NetQuery, source: NetSource, outcome: &NetOutcome) {
    let waiters = if matches!(outcome, NetOutcome::Failed(_)) {
        group(ctx, qkey)
    } else {
        take_group(ctx, qkey)
    };
    finish(ctx, query, &waiters, source, outcome).await;
}

/// Refresco en segundo plano tras servir una caché caducada.
async fn refresh(ctx: Arc<Ctx>, qkey: String, query: NetQuery, waiters: Vec<Waiter>) {
    let result = run(&ctx, &query).await;
    ctx.shared.lock().unwrap().refreshing.remove(&qkey);
    let Some((source, outcome)) = result else {
        svc_event!("net.cache.refresh.aborted", {"query_fp": fp(&query)});
        return;
    };
    svc_event!("net.cache.refresh.finished", {"query_fp": fp(&query),
        "replaced": matches!(outcome, NetOutcome::System(_))});
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
    write_api_cache(&ctx.store, query, outcome, now).await;
    send_all(ctx, waiters, source, now, outcome).await;
}

async fn retry(ctx: Arc<Ctx>, qkey: String, query: NetQuery, first: NetOutcome) {
    let mut last = first;
    loop {
        let Some(at) = ctx.client.retry_at(&query) else {
            svc_event!("net.retry.stopped", {"query_fp": fp(&query),
                "down": ctx.client.is_down_for(&query)});
            let waiters = take_group(&ctx, &qkey);
            if ctx.client.is_down_for(&query) && last != NetOutcome::Failed(NetFailure::Down) {
                let down = NetOutcome::Failed(NetFailure::Down);
                send_all(&ctx, &waiters, NetSource::Cache, Utc::now(), &down).await;
            }
            return;
        };
        svc_event!("net.retry.scheduled", {"query_fp": fp(&query),
            "delay_ms": at.saturating_duration_since(tokio::time::Instant::now()).as_millis() as u64});
        tokio::time::sleep_until(at).await;
        let Some((source, outcome)) = run(&ctx, &query).await else {
            disabled(&ctx, &qkey).await;
            return;
        };
        settle(&ctx, &qkey, &query, source, &outcome).await;
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
            svc_event!("net.cache.read_failed", {"table_kind": 0});
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
        svc_event!("net.cache.write_failed", {"table_kind": 0});
        tracing::warn!(%error, "api_cache write failed");
    }
}

/// Clave única de las consultas de Ardent (listas de mercados/estaciones).
fn api_cache_key(query: &NetQuery) -> Option<(CacheKind, String)> {
    match query {
        NetQuery::CommodityImportsNearby { .. }
        | NetQuery::NearestService { .. }
        | NetQuery::TradeRoute(_)
        | NetQuery::CommodityExportsNearby { .. }
        | NetQuery::SystemCoordsByName { .. }
        | NetQuery::BgsTick
        | NetQuery::OutfittingSearch(_)
        | NetQuery::BodyBio { .. } => cache_keys(query).into_iter().next(),
        _ => None,
    }
}

/// Respuesta de Ardent fresca en caché; sin `max_stale` útil (caducada = sin caché).
async fn read_api_cache(store: &Store, query: &NetQuery) -> Option<(NetOutcome, DateTime<Utc>)> {
    let (kind, key) = api_cache_key(query)?;
    let store = store.clone();
    let entry = tokio::task::spawn_blocking(move || store.api_cache_get(&key))
        .await
        .ok()?
        .map_err(|error| tracing::warn!(%error, "api_cache read failed"))
        .ok()??;
    if age(entry.fetched_at) >= ttl_for(kind).fresh {
        return None;
    }
    let outcome = match kind {
        CacheKind::ArdentImports => NetOutcome::Markets(serde_json::from_str(&entry.value).ok()?),
        CacheKind::BodyBio => NetOutcome::BodyBio(serde_json::from_str(&entry.value).ok()?),
        CacheKind::SpanshTrade => NetOutcome::TradeRoute(serde_json::from_str(&entry.value).ok()?),
        CacheKind::ArdentExports => NetOutcome::Exports(serde_json::from_str(&entry.value).ok()?),
        CacheKind::SystemCoords => NetOutcome::Coords(serde_json::from_str(&entry.value).ok()?),
        CacheKind::BgsTick => NetOutcome::BgsTick(serde_json::from_str(&entry.value).ok()?),
        CacheKind::SpanshOutfitting => {
            NetOutcome::OutfittingStations(serde_json::from_str(&entry.value).ok()?)
        }
        _ => NetOutcome::Stations(serde_json::from_str(&entry.value).ok()?),
    };
    Some((outcome, entry.fetched_at))
}

async fn write_api_cache(store: &Store, query: &NetQuery, outcome: &NetOutcome, at: DateTime<Utc>) {
    let Some((_, key)) = api_cache_key(query) else {
        return;
    };
    let json = match outcome {
        NetOutcome::Markets(items) => serde_json::to_string(items),
        NetOutcome::Stations(items) => serde_json::to_string(items),
        NetOutcome::BodyBio(bio) => serde_json::to_string(bio),
        NetOutcome::TradeRoute(plan) => serde_json::to_string(plan),
        NetOutcome::Exports(items) => serde_json::to_string(items),
        NetOutcome::Coords(coords) => serde_json::to_string(coords),
        NetOutcome::BgsTick(tick) => serde_json::to_string(tick),
        NetOutcome::OutfittingStations(res) => serde_json::to_string(res),
        _ => return,
    };
    let Ok(json) = json else {
        return;
    };
    let store = store.clone();
    let result = tokio::task::spawn_blocking(move || store.api_cache_put(&key, &json, at)).await;
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

    #[tokio::test(start_paused = true)]
    async fn opt_out_while_client_waits_for_rate_limit_returns_disabled_without_retry() {
        let mut rig = rig(vec![ok(200, SYSTEM)]);
        rig.handle.client.execute(&sol()).await;
        rig.handle.tx.send(request(1)).await.unwrap();
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert_eq!(rig.fake.calls(), 1);
        rig.online.store(false, Ordering::Release);
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(next_response(&mut rig).await.outcome, NetOutcome::Disabled);
        assert_eq!(rig.fake.calls(), 1);
        tokio::time::advance(Duration::from_secs(120)).await;
        assert_eq!(rig.fake.calls(), 1);
        assert!(rig.events.try_recv().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn opt_out_after_run_enters_client_prevents_http() {
        let rig = rig(vec![ok(200, SYSTEM)]);
        rig.handle.client.execute(&sol()).await;
        let (events, _responses) = mpsc::channel(1);
        let ctx = Arc::new(Ctx {
            client: rig.handle.client.clone(),
            store: rig.store.clone(),
            events,
            online: rig.online.clone(),
            shared: Mutex::new(Shared::default()),
            permits: Semaphore::new(MAX_CONCURRENT),
            canonn_permits: Semaphore::new(1),
        });
        let pending = tokio::spawn(async move { run(&ctx, &sol()).await });
        tokio::task::yield_now().await;
        assert_eq!(rig.fake.calls(), 1);
        rig.online.store(false, Ordering::Release);
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(pending.await.unwrap().is_none());
        assert_eq!(rig.fake.calls(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn opt_out_while_autocomplete_waits_prevents_direct_http() {
        let rig = rig(vec![ok(200, r#"{"min_max":[],"values":["Sol"]}"#)]);
        assert_eq!(rig.handle.autocomplete("Sol").await.unwrap().len(), 1);
        let pending = tokio::spawn({
            let handle = rig.handle.clone();
            async move { handle.autocomplete("Col").await }
        });
        tokio::task::yield_now().await;
        rig.online.store(false, Ordering::Release);
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(pending.await.unwrap(), Ok(vec![]));
        assert_eq!(rig.fake.calls(), 1);
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
    async fn request_during_backoff_joins_pending_retry() {
        let mut rig = rig(vec![ok(503, ""), ok(200, SYSTEM)]);
        rig.handle.tx.send(request(1)).await.unwrap();
        let first = next_response(&mut rig).await;
        assert_eq!(first.outcome, NetOutcome::Failed(NetFailure::Http(503)));
        tokio::time::sleep(Duration::from_secs(10)).await;
        rig.handle.tx.send(request(2)).await.unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(rig.fake.calls(), 1);
        tokio::time::sleep(Duration::from_secs(11)).await;
        let mut ids = Vec::new();
        for _ in 0..2 {
            let r = next_response(&mut rig).await;
            assert!(matches!(r.outcome, NetOutcome::System(_)));
            ids.push(r.id);
        }
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(rig.fake.calls(), 2);
    }

    struct Slow {
        now: StdMutex<u32>,
        max: StdMutex<u32>,
    }

    impl Transport for Slow {
        fn get(&self, _url: &str) -> BoxFut<'_, Reply> {
            Box::pin(async move {
                {
                    let mut n = self.now.lock().unwrap();
                    *n += 1;
                    let mut m = self.max.lock().unwrap();
                    *m = (*m).max(*n);
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
                *self.now.lock().unwrap() -= 1;
                ok(200, SYSTEM)
            })
        }
        fn post_form(&self, _url: &str, _form: &[(String, String)]) -> BoxFut<'_, Reply> {
            self.get("")
        }
    }

    #[tokio::test(start_paused = true)]
    async fn transport_concurrency_is_bounded_to_four() {
        let slow = Arc::new(Slow {
            now: StdMutex::new(0),
            max: StdMutex::new(0),
        });
        let (tx, mut events) = mpsc::channel(64);
        let handle = spawn_net_service(
            Arc::new(NetClient::new(slow.clone())),
            Store::open_in_memory().unwrap(),
            tx,
            Arc::new(AtomicBool::new(true)),
        );
        for id in 1..=10u64 {
            handle
                .tx
                .send(NetRequest {
                    id,
                    module: "navigation".into(),
                    key: format!("system:{id}"),
                    query: NetQuery::System {
                        id64: id,
                        name: format!("S{id}"),
                    },
                })
                .await
                .unwrap();
        }
        let mut done = 0;
        while done < 10 {
            if let Some(SourceEvent::Net(_)) = events.recv().await {
                done += 1;
            }
        }
        assert_eq!(*slow.max.lock().unwrap(), 4);
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

    const ARDENT_PLATINUM: &str =
        include_str!("../../../tests/fixtures/net/ardent_nearby_imports_platinum.json");

    fn imports() -> NetQuery {
        NetQuery::CommodityImportsNearby {
            system_address: ID64,
            commodity: "Platinum".into(),
            max_distance_ly: 100,
            min_pad: tmc_model::net::PadSize::Large,
        }
    }

    fn imports_request(id: u64) -> NetRequest {
        NetRequest {
            id,
            module: "mining".into(),
            key: "sell:platinum".into(),
            query: imports(),
        }
    }

    #[tokio::test]
    async fn fresh_ardent_imports_cache_hit_skips_transport() {
        let mut rig = rig(vec![ok(200, ARDENT_PLATINUM)]);
        let markets = vec![tmc_model::net::RemoteMarket {
            station_name: "Cache Hub".into(),
            system_name: "Sol".into(),
            market_id: Some(5),
            distance_ly: 12.5,
            distance_to_arrival_ls: None,
            sell_price: 299000,
            demand: Some(100),
            max_pad: Some(tmc_model::net::PadSize::Large),
            station_type: None,
            updated_at: None,
            origin: None,
        }];
        rig.store
            .api_cache_put(
                &format!("ardent:imports:{ID64}:platinum:100:l"),
                &serde_json::to_string(&markets).unwrap(),
                Utc::now() - chrono::Duration::minutes(10),
            )
            .unwrap();
        rig.handle.tx.send(imports_request(7)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.id, 7);
        assert_eq!(r.source, NetSource::Cache);
        assert_eq!(r.outcome, NetOutcome::Markets(markets));
        assert_eq!(rig.fake.calls(), 0);
    }

    #[tokio::test]
    async fn expired_ardent_cache_calls_network_and_stores_result() {
        let mut rig = rig(vec![ok(200, ARDENT_PLATINUM)]);
        rig.store
            .api_cache_put(
                &format!("ardent:imports:{ID64}:platinum:100:l"),
                "[]",
                Utc::now() - chrono::Duration::minutes(16),
            )
            .unwrap();
        rig.handle.tx.send(imports_request(1)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.source, NetSource::Ardent);
        assert!(matches!(&r.outcome, NetOutcome::Markets(_)));
        assert_eq!(rig.fake.calls(), 1);
        rig.handle.tx.send(imports_request(2)).await.unwrap();
        let again = next_response(&mut rig).await;
        assert_eq!(again.source, NetSource::Cache);
        assert_eq!(rig.fake.calls(), 1);
    }

    #[tokio::test]
    async fn coords_cached_for_30_days() {
        let coords = r#"{"systemAddress":9000000000001,"systemName":"Origin","systemX":0.5,"systemY":-1.25,"systemZ":3.75}"#;
        let mut rig = rig(vec![ok(200, coords)]);
        let request = |id| NetRequest {
            id,
            module: "trade".into(),
            key: "coords:origin".into(),
            query: NetQuery::SystemCoordsByName {
                name: "Origin".into(),
            },
        };
        rig.handle.tx.send(request(1)).await.unwrap();
        let first = next_response(&mut rig).await;
        assert_eq!(first.source, NetSource::Ardent);
        assert!(matches!(first.outcome, NetOutcome::Coords(_)));
        rig.handle.tx.send(request(2)).await.unwrap();
        let again = next_response(&mut rig).await;
        assert_eq!(again.source, NetSource::Cache);
        assert_eq!(again.outcome, first.outcome);
        assert_eq!(rig.fake.calls(), 1);
        // A los 29 días sigue vigente; a los 31 se vuelve a pedir.
        rig.store
            .api_cache_put(
                "coords:origin",
                &serde_json::to_string(&match &first.outcome {
                    NetOutcome::Coords(c) => c.clone(),
                    _ => unreachable!(),
                })
                .unwrap(),
                Utc::now() - chrono::Duration::days(31),
            )
            .unwrap();
        rig.handle.tx.send(request(3)).await.unwrap();
        assert_eq!(next_response(&mut rig).await.source, NetSource::Ardent);
        assert_eq!(rig.fake.calls(), 2);
    }

    #[tokio::test]
    async fn fresh_body_bio_cache_hit_skips_transport() {
        let mut rig = rig(vec![ok(500, "")]);
        let bio = tmc_model::net::RemoteBodyBio {
            hints: vec![tmc_model::net::RemoteSpeciesHint {
                genus: "GeneroA".into(),
                species: "EspecieAlfa".into(),
                weight: 1.0,
            }],
        };
        rig.store
            .api_cache_put(
                "canonn:body:1:2",
                &serde_json::to_string(&bio).unwrap(),
                Utc::now() - chrono::Duration::days(1),
            )
            .unwrap();
        let query = NetQuery::BodyBio {
            system_address: 1,
            star_system: "Sistema".into(),
            body_id: 2,
            body_name: "Sistema A 1".into(),
            profile: tmc_model::net::BodyProfile {
                planet_class: None,
                atmosphere_type: None,
                gravity_g: None,
                temperature_k: None,
                pressure_atm: None,
                volcanism: None,
                star_class: None,
            },
        };
        let request = NetRequest {
            id: 9,
            module: "exobiology".into(),
            key: "bio:1:2".into(),
            query,
        };
        rig.handle.tx.send(request).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.source, NetSource::Cache);
        assert_eq!(r.outcome, NetOutcome::BodyBio(bio));
        assert_eq!(rig.fake.calls(), 0);
    }

    const CANONN_BODY: &str = include_str!("../../../tests/fixtures/net/canonn_body_ok.json");
    const CANONN_REF: &str = r#"{
      "9900001": {"entryid": 9900001, "english_name": "EspecieAlfa - Roja", "sub_class": "GeneroA"},
      "9900002": {"entryid": 9900002, "english_name": "EspecieBeta - Azul", "sub_class": "GeneroB"}
    }"#;

    fn bio_query(id: u32, body: &str) -> NetQuery {
        NetQuery::BodyBio {
            system_address: 1,
            star_system: "Sistema Sintetico de Prueba".into(),
            body_id: id,
            body_name: format!("Sistema Sintetico de Prueba {body}"),
            profile: tmc_model::net::BodyProfile {
                planet_class: None,
                atmosphere_type: None,
                gravity_g: None,
                temperature_k: None,
                pressure_atm: None,
                volcanism: None,
                star_class: None,
            },
        }
    }

    #[tokio::test(start_paused = true)]
    async fn body_bio_network_result_is_cached_and_served_next_time() {
        let mut rig = rig(vec![ok(200, CANONN_REF), ok(200, CANONN_BODY)]);
        let send = |id: u64, q: NetQuery| NetRequest {
            id,
            module: "exobiology".into(),
            key: format!("bio:{id}"),
            query: q,
        };
        rig.handle
            .tx
            .send(send(1, bio_query(2, "A 1 a")))
            .await
            .unwrap();
        let first = next_response(&mut rig).await;
        assert_eq!(first.source, NetSource::Canonn);
        assert!(matches!(first.outcome, NetOutcome::BodyBio(_)));
        assert_eq!(rig.fake.calls(), 2);
        let stored = rig.store.api_cache_get("canonn:body:1:2").unwrap();
        assert!(stored.is_some());
        rig.handle
            .tx
            .send(send(2, bio_query(2, "A 1 a")))
            .await
            .unwrap();
        let again = next_response(&mut rig).await;
        assert_eq!(again.source, NetSource::Cache);
        assert_eq!(again.outcome, first.outcome);
        assert_eq!(rig.fake.calls(), 2);
    }

    struct CanonnSlow;

    impl Transport for CanonnSlow {
        fn get(&self, url: &str) -> BoxFut<'_, Reply> {
            let slow = url.contains("canonn");
            Box::pin(async move {
                if slow {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    return ok(200, "{}");
                }
                ok(200, SYSTEM)
            })
        }
        fn post_form(&self, _url: &str, _form: &[(String, String)]) -> BoxFut<'_, Reply> {
            self.get("")
        }
    }

    #[tokio::test(start_paused = true)]
    async fn slow_canonn_requests_do_not_block_other_services() {
        let (tx, mut events) = mpsc::channel(64);
        let handle = spawn_net_service(
            Arc::new(NetClient::new(Arc::new(CanonnSlow))),
            Store::open_in_memory().unwrap(),
            tx,
            Arc::new(AtomicBool::new(true)),
        );
        for id in 1..=4u32 {
            handle
                .tx
                .send(NetRequest {
                    id: u64::from(id),
                    module: "exobiology".into(),
                    key: format!("bio:{id}"),
                    query: bio_query(id, &format!("A {id}")),
                })
                .await
                .unwrap();
        }
        handle.tx.send(request(99)).await.unwrap();
        let first = tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("la consulta de Spansh quedó bloqueada")
            .unwrap();
        let SourceEvent::Net(r) = first else {
            panic!("evento inesperado")
        };
        assert_eq!(r.id, 99);
    }

    #[tokio::test(start_paused = true)]
    async fn identical_ardent_requests_are_coalesced() {
        let mut rig = rig(vec![ok(200, ARDENT_PLATINUM)]);
        for id in 1..=3 {
            rig.handle.tx.send(imports_request(id)).await.unwrap();
        }
        let mut ids = Vec::new();
        for _ in 0..3 {
            let r = next_response(&mut rig).await;
            assert!(matches!(&r.outcome, NetOutcome::Markets(_)));
            ids.push(r.id);
        }
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(rig.fake.calls(), 1);
    }

    #[tokio::test]
    async fn ardent_request_with_online_off_is_disabled_without_transport() {
        let mut rig = rig(vec![ok(200, ARDENT_PLATINUM)]);
        rig.online.store(false, Ordering::Relaxed);
        rig.handle.tx.send(imports_request(3)).await.unwrap();
        let r = next_response(&mut rig).await;
        assert_eq!(r.id, 3);
        assert_eq!(r.outcome, NetOutcome::Disabled);
        assert_eq!(rig.fake.calls(), 0);
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

    #[cfg(debug_assertions)]
    #[test]
    fn diagnostics_cover_service_decisions_without_error_text() {
        use tmc_model::diagnostics::capture::capture;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()
            .unwrap();
        let (_, records) = capture(|| {
            rt.block_on(async {
                let mut rig = rig(vec![Err(TransportError::Connect(
                    "https://spansh.co.uk/api?token=CANARYERR".into(),
                ))]);
                rig.handle.tx.send(request(7)).await.unwrap();
                let first = next_response(&mut rig).await;
                assert_eq!(first.id, 7);
            })
        });
        let all: String = records
            .iter()
            .map(|r| {
                format!(
                    "{} {}
",
                    r.code, r.payload
                )
            })
            .collect();
        for canary in ["CANARYERR", "https://", "spansh.co.uk", "token="] {
            assert!(!all.contains(canary), "{canary} leaked: {all}");
        }
        let codes: Vec<&str> = records.iter().map(|r| r.code.as_str()).collect();
        for code in [
            "net.service.received",
            "net.coalesced.leader",
            "net.cache.miss",
            "net.service.permit_acquired",
            "net.service.responded",
        ] {
            assert!(codes.contains(&code), "falta {code}: {codes:?}");
        }
    }
}

//! `NetClient`: limitador, salud por servicio, alternativa Spansh -> EDSM,
//! sondeo de rutas y traducción de errores HTTP.
//!
//! Decisiones:
//! - El tiempo se lee de `tokio::time::Instant`, así los tests con reloj
//!   pausado son deterministas y el limitador/salud usan el mismo reloj.
//! - La alternativa a EDSM solo se usa cuando Spansh está marcado como caído
//!   (3 fallos seguidos, 10 min); un fallo suelto se devuelve tal cual. Un 404
//!   de Spansh es `NoData` y no activa la alternativa.
//! - `ServiceHealth::retry_at` no se aplica aquí (lo consume el servicio que
//!   reintenta); el cliente solo respeta `is_down`. Tras caducar `down_until`
//!   los fallos siguen en 3, así que un único fallo más vuelve a marcar el
//!   servicio como caído (aceptado).
//! - Un fallo de conexión (DNS, TLS, rechazo) se informa como `Failed(Down)`,
//!   la variante más cercana de `NetFailure`.
//! - Un trabajo de ruta que Spansh declara fallido se informa como `NoData`
//!   (no hay ruta) y el motivo va al registro.
//! - Ardent (mercados cercanos e Interstellar Factors) usa el mismo `fetch`; su
//!   URL base la fija `ardent::BASE` y en e2e se simula con el `Transport`
//!   inyectado (igual que Spansh). El «ahora» para filtrar `updatedAt` se
//!   inyecta con `with_clock`. 404 = `NoData`, `[]` = lista vacía.
//! - La capa 3 (Canonn, exobiología) está activa con permiso de Canonn:
//!   `BodyBio` descarga `codex/ref` (una vez por sesión y 7 días, en memoria) y
//!   `getSystemPoi` por nombre de sistema, ambos por el limitador de Canonn
//!   (1/s), la salud del servicio y el opt-out. Solo salen el nombre del
//!   sistema y la categoría `Biology`; ningún dato del jugador. Una consulta
//!   de sistema se reutiliza 5 min para los demás cuerpos del mismo sistema.
//!   Replay y Legacy no emiten la consulta (lo garantizan `ModuleCtx` y el
//!   módulo).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::ardent;
use crate::canonn;
use crate::edsm;
use crate::health::ServiceHealth;
use crate::limiter::{RateLimiter, Service};
use crate::spansh::{self, RoutePoll, TradePoll};
use crate::transport::{HttpReply, Transport, TransportError};
use chrono::{DateTime, Utc};
use tmc_model::net::{NearbyService, NetFailure, NetOutcome, NetQuery, NetSource};

const POLL_EVERY: Duration = Duration::from_secs(2);
const MAX_TRANSIENT_POLL_FAILURES: u32 = 2;
const POLL_LIMIT: Duration = Duration::from_secs(120);
/// Las rutas comerciales tardan 15-20 s: se espera como máximo 60 s.
const TRADE_POLL_LIMIT: Duration = Duration::from_secs(60);
const CODEX_REF_TTL: Duration = Duration::from_secs(7 * 24 * 3600);
const POI_TTL: Duration = Duration::from_secs(5 * 60);
/// Canonn tarda 2-4 s por respuesta: tiempo máximo propio (el resto, el del transporte).
pub const CANONN_TIMEOUT: Duration = Duration::from_secs(15);

#[cfg(debug_assertions)]
tokio::task_local! {
    /// Identificador de correlación de la consulta en curso (solo diagnóstico).
    static REQUEST_ID: u64;
}

#[cfg(debug_assertions)]
static NEXT_REQUEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[cfg(debug_assertions)]
fn request_id() -> u64 {
    REQUEST_ID.try_with(|id| *id).unwrap_or(0)
}

/// Huella de la consulta para correlacionar con `NetService` sin exponer su contenido.
#[cfg(debug_assertions)]
pub fn query_fingerprint(q: &NetQuery) -> u64 {
    tmc_model::diagnostics::sql_fingerprint(&serde_json::to_string(q).unwrap_or_default())
}

/// Plantilla de endpoint (nunca URL ni query string) para los registros de diagnóstico.
#[cfg(debug_assertions)]
fn endpoint_of(req: &Req) -> &'static str {
    let (Req::Get(url) | Req::Post(url, _) | Req::PostJson(url, _)) = req;
    let path = url.split('?').next().unwrap_or("");
    if let Some(rest) = path.strip_prefix(spansh::BASE) {
        return match rest {
            r if r.starts_with("/system/") => "spansh.system",
            "/route" | "/generic/route" => "spansh.route.job",
            "/trade/route" => "spansh.trade.job",
            r if r.starts_with("/results/") => "spansh.route.results",
            r if r.starts_with("/systems/") => "spansh.autocomplete",
            "/stations/search" => {
                let body = match req {
                    Req::PostJson(_, body) => serde_json::from_str::<serde_json::Value>(body).ok(),
                    _ => None,
                };
                let has = |k: &str| body.as_ref().and_then(|b| b.pointer(k)).is_some();
                if has("/filters/technology_broker") {
                    "spansh.stations.technology_broker"
                } else if has("/filters/material_trader") {
                    "spansh.stations.material_trader"
                } else {
                    "unknown"
                }
            }
            _ => "unknown",
        };
    }
    if path.starts_with(edsm::BASE) {
        return if url.contains("showCoordinates=1") {
            "edsm.coords"
        } else {
            "edsm.system"
        };
    }
    if path.starts_with(canonn::BASE) {
        return if path.ends_with("/codex/ref") {
            "canonn.codex_ref"
        } else {
            "canonn.system_poi"
        };
    }
    if path.starts_with(ardent::BASE) {
        return if path.contains("/nearby/imports") {
            "ardent.nearby.imports"
        } else if path.contains("/nearby/exports") {
            "ardent.nearby.exports"
        } else if path.contains("/system/name/") {
            "ardent.system.coords"
        } else if path.contains("/nearest/material-trader") {
            "ardent.nearest.material_trader"
        } else if path.contains("/nearest/technology-broker") {
            "ardent.nearest.technology_broker"
        } else {
            "ardent.nearest.service"
        };
    }
    "unknown"
}

/// Evento de red redactado; añade siempre el identificador de correlación.
macro_rules! net_event {
    ($code:literal, { $($payload:tt)* }) => {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::NetOutcome,
            $code,
            tmc_model::diagnostics::json!({ "request_id": request_id(), $($payload)* })
        )
    };
}

/// Almacén persistente del `codex/ref` compacto (lo implementa la app sobre `api_cache`).
pub trait RefPersist: Send + Sync {
    fn load(&self) -> Option<(DateTime<Utc>, String)>;
    fn save(&self, value: &str, at: DateTime<Utc>);
}

type CodexRef = HashMap<u64, (String, String)>;

/// Resultado no exitoso de una llamada (más pequeño que `NetOutcome`).
enum Miss {
    Disabled,
    NoData,
    Fail(NetFailure),
}

impl From<Miss> for NetOutcome {
    fn from(m: Miss) -> Self {
        match m {
            Miss::Disabled => NetOutcome::Disabled,
            Miss::NoData => NetOutcome::NoData,
            Miss::Fail(f) => NetOutcome::Failed(f),
        }
    }
}

/// Estado de un trabajo sondeado, ya traducido al resultado final.
enum Step {
    Pending,
    Done(Box<NetOutcome>),
    Failed(String),
}

enum Req {
    Get(String),
    Post(String, Vec<(String, String)>),
    /// `POST` con cuerpo JSON ya serializado.
    PostJson(String, String),
}

struct State {
    limiters: HashMap<Service, RateLimiter>,
    health: HashMap<Service, ServiceHealth>,
}

pub struct NetClient {
    transport: Arc<dyn Transport>,
    state: Mutex<State>,
    /// Una sola ruta a la vez (alta + sondeos).
    route_lock: tokio::sync::Mutex<()>,
    /// Reloj de pared (inyectable) para la frescura de los datos de Ardent.
    utc_now: fn() -> DateTime<Utc>,
    /// `codex/ref` de Canonn (entryid -> género, especie), en memoria.
    codex_ref: Mutex<Option<(std::time::Instant, Arc<CodexRef>)>>,
    /// Última respuesta de `getSystemPoi`: (sistema en minúsculas, cuándo, cuerpo).
    poi: Mutex<Option<(String, std::time::Instant, Arc<String>)>>,
    persist: Option<Arc<dyn RefPersist>>,
    /// Una sola descarga a la vez de `codex/ref` y de `getSystemPoi`.
    ref_lock: tokio::sync::Mutex<()>,
    poi_lock: tokio::sync::Mutex<()>,
}

fn now() -> std::time::Instant {
    tokio::time::Instant::now().into_std()
}

fn online_allowed(online: Option<&AtomicBool>) -> bool {
    online.is_none_or(|online| online.load(Ordering::Acquire))
}

/// Consulta de comerciante/bróker con tipo concreto: la atiende Spansh.
fn is_typed_station_search(q: &NetQuery) -> bool {
    matches!(
        q,
        NetQuery::NearestService {
            service: NearbyService::MaterialTrader(Some(_))
                | NearbyService::TechnologyBroker(Some(_)),
            ..
        }
    )
}

fn source_of(service: Service) -> NetSource {
    match service {
        Service::Spansh => NetSource::Spansh,
        Service::Edsm => NetSource::Edsm,
        Service::Canonn => NetSource::Canonn,
        Service::Ardent => NetSource::Ardent,
        Service::Edcd => NetSource::Edcd,
        Service::Eddn => NetSource::Eddn,
        Service::Inara => NetSource::Inara,
        Service::Capi => NetSource::Capi,
    }
}

impl NetClient {
    /// Revalida el opt-out después de las esperas y antes de cada envío HTTP.
    /// Las peticiones ya enviadas no se pueden retirar.
    pub async fn execute_with_online(
        &self,
        q: &NetQuery,
        online: &AtomicBool,
    ) -> (NetSource, NetOutcome) {
        self.execute_inner(q, Some(online)).await
    }
    pub fn new(transport: Arc<dyn Transport>) -> NetClient {
        Self::with_clock(transport, Utc::now)
    }

    /// Como [`NetClient::new`] con el reloj de pared inyectado (tests).
    pub fn with_clock(transport: Arc<dyn Transport>, utc_now: fn() -> DateTime<Utc>) -> NetClient {
        let services = [
            Service::Spansh,
            Service::Edsm,
            Service::Canonn,
            Service::Ardent,
            Service::Edcd,
            Service::Eddn,
            Service::Inara,
            Service::Capi,
        ];
        NetClient {
            transport,
            state: Mutex::new(State {
                limiters: services
                    .iter()
                    .map(|s| (*s, RateLimiter::for_service(*s)))
                    .collect(),
                health: services
                    .iter()
                    .map(|s| (*s, ServiceHealth::default()))
                    .collect(),
            }),
            route_lock: tokio::sync::Mutex::new(()),
            utc_now,
            codex_ref: Mutex::new(None),
            poi: Mutex::new(None),
            persist: None,
            ref_lock: tokio::sync::Mutex::new(()),
            poi_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Guarda y recupera el `codex/ref` compacto (7 días) para no bajarlo en cada arranque.
    pub fn with_codex_persist(mut self, persist: Arc<dyn RefPersist>) -> NetClient {
        self.persist = Some(persist);
        self
    }

    pub async fn execute(&self, q: &NetQuery) -> (NetSource, NetOutcome) {
        self.execute_inner(q, None).await
    }

    async fn execute_inner(
        &self,
        q: &NetQuery,
        online: Option<&AtomicBool>,
    ) -> (NetSource, NetOutcome) {
        #[cfg(debug_assertions)]
        {
            let id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
            REQUEST_ID
                .scope(id, async {
                    let started = std::time::Instant::now();
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::NetQuery,
                        "net.query.started",
                        tmc_model::diagnostics::json!({
                            "request_id": id,
                            "query_fp": query_fingerprint(q),
                            "query": q,
                            "online_guard": online.is_some(),
                        })
                    );
                    let result = self.execute_query(q, online).await;
                    tmc_model::diagnostic!(
                        tmc_model::diagnostics::DiagnosticContext::NetOutcome,
                        "net.query.finished",
                        tmc_model::diagnostics::json!({
                            "request_id": id,
                            "query_fp": query_fingerprint(q),
                            "source": result.0,
                            "outcome": result.1.diag_summary().0,
                            "items": result.1.diag_summary().1,
                            "duration_ms": started.elapsed().as_millis() as u64,
                        })
                    );
                    result
                })
                .await
        }
        #[cfg(not(debug_assertions))]
        {
            self.execute_query(q, online).await
        }
    }

    async fn execute_query(
        &self,
        q: &NetQuery,
        online: Option<&AtomicBool>,
    ) -> (NetSource, NetOutcome) {
        if !online_allowed(online) {
            net_event!("net.optout.blocked", {"stage": 0});
            return (NetSource::Cache, NetOutcome::Disabled);
        }
        match q {
            NetQuery::System { id64, name } => self.system(*id64, name, online).await,
            NetQuery::NeutronRoute(p) => {
                let req = Req::Post(spansh::neutron_url(), spansh::neutron_form(p));
                (NetSource::Spansh, self.route(req, online).await)
            }
            NetQuery::GalaxyRoute(p) => {
                let req = Req::Post(spansh::galaxy_url(), spansh::galaxy_form(p));
                (NetSource::Spansh, self.route(req, online).await)
            }
            NetQuery::Autocomplete { prefix } => {
                let req = Req::Get(spansh::autocomplete_url(prefix));
                let out = self
                    .fetch(Service::Spansh, req, online, |b| {
                        spansh::parse_autocomplete(b).map(|v| Some(NetOutcome::Autocomplete(v)))
                    })
                    .await;
                (NetSource::Spansh, out)
            }
            NetQuery::BodyBio {
                star_system,
                body_name,
                ..
            } => {
                let out = self.body_bio(star_system, body_name, online).await;
                (NetSource::Canonn, out)
            }
            NetQuery::CommodityImportsNearby {
                system_address,
                commodity,
                max_distance_ly,
                min_pad,
            } => {
                let url = ardent::imports_nearby_url(
                    *system_address,
                    &ardent::ardent_commodity(commodity),
                    *max_distance_ly,
                    *min_pad,
                );
                let (pad, at) = (*min_pad, (self.utc_now)());
                let out = self
                    .fetch(Service::Ardent, Req::Get(url), online, |b| {
                        ardent::parse_markets(b, at, pad).map(|m| Some(NetOutcome::Markets(m)))
                    })
                    .await;
                (NetSource::Ardent, out)
            }
            NetQuery::NearestService {
                service:
                    service @ (NearbyService::MaterialTrader(Some(_))
                    | NearbyService::TechnologyBroker(Some(_))),
                system_name,
                min_pad,
                ..
            } => {
                // Con tipo solo Spansh lo conoce; si falla, «sin datos» (nunca se infiere
                // el tipo por la economía ni se cae a Ardent sin tipo).
                let body = spansh::station_search_body(system_name, *service).to_string();
                let req = Req::PostJson(spansh::station_search_url(), body);
                let pad = *min_pad;
                let out = self
                    .fetch(Service::Spansh, req, online, |b| {
                        spansh::parse_station_search(b, pad).map(|s| Some(NetOutcome::Stations(s)))
                    })
                    .await;
                let out = match out {
                    NetOutcome::Failed(_) => NetOutcome::NoData,
                    other => other,
                };
                (NetSource::Spansh, out)
            }
            NetQuery::NearestService {
                system_address,
                service,
                min_pad,
                ..
            } => {
                let url = ardent::nearest_url(*system_address, *service, *min_pad);
                let pad = *min_pad;
                let out = self
                    .fetch(Service::Ardent, Req::Get(url), online, |b| {
                        ardent::parse_stations(b, pad).map(|s| Some(NetOutcome::Stations(s)))
                    })
                    .await;
                (NetSource::Ardent, out)
            }
            NetQuery::TradeRoute(p) => {
                let req = Req::Post(spansh::trade_url(), spansh::trade_form(p));
                let out = self
                    .run_job(req, online, TRADE_POLL_LIMIT, |b| {
                        Ok(match spansh::parse_trade_result(b)? {
                            TradePoll::Pending => Step::Pending,
                            TradePoll::Done(plan) => {
                                Step::Done(Box::new(NetOutcome::TradeRoute(plan)))
                            }
                            TradePoll::Failed(msg) => Step::Failed(msg),
                        })
                    })
                    .await;
                (NetSource::Spansh, out)
            }
            NetQuery::CommodityExportsNearby {
                system_address,
                commodity,
                max_distance_ly,
                min_pad,
            } => {
                let url = ardent::exports_nearby_url(
                    *system_address,
                    &ardent::ardent_commodity(commodity),
                    *max_distance_ly,
                );
                let pad = *min_pad;
                let out = self
                    .fetch(Service::Ardent, Req::Get(url), online, |b| {
                        ardent::parse_exports_for(b, pad).map(|e| Some(NetOutcome::Exports(e)))
                    })
                    .await;
                (NetSource::Ardent, out)
            }
            NetQuery::SystemCoordsByName { name } => self.coords(name, online).await,
            NetQuery::BgsTick => {
                let req = Req::Get(crate::bgs_tick::BGS_TICK_URL.to_string());
                let out = self
                    .fetch(Service::Edcd, req, online, |b| {
                        crate::bgs_tick::parse_ticks(b, Utc::now())
                            .map(|opt| opt.map(NetOutcome::BgsTick))
                    })
                    .await;
                (NetSource::Edcd, out)
            }
        }
    }

    /// Coordenadas por nombre: Ardent y, solo si está caído, EDSM.
    async fn coords(&self, name: &str, online: Option<&AtomicBool>) -> (NetSource, NetOutcome) {
        let mut first = None;
        if !self.is_down(Service::Ardent) {
            let req = Req::Get(ardent::system_name_url(name));
            let out = self
                .fetch(Service::Ardent, req, online, |b| {
                    ardent::parse_coords(b).map(|c| c.map(NetOutcome::Coords))
                })
                .await;
            match out {
                NetOutcome::Coords(_) | NetOutcome::Disabled => {
                    return (NetSource::Ardent, out);
                }
                // 404, fallo o cuerpo corrupto: se prueba EDSM.
                other => first = Some(other),
            }
        }
        if self.is_down(Service::Edsm) {
            return match first {
                Some(out) => (NetSource::Ardent, out),
                None => (NetSource::Edsm, NetOutcome::Failed(NetFailure::Down)),
            };
        }
        let req = Req::Get(edsm::coords_url(name));
        let out = self
            .fetch(Service::Edsm, req, online, |b| {
                edsm::parse_coords(b).map(|c| c.map(NetOutcome::Coords))
            })
            .await;
        match (out, first) {
            // EDSM sin datos: si Ardent falló, se informa su fallo.
            (NetOutcome::NoData, Some(f @ NetOutcome::Failed(_))) => (NetSource::Ardent, f),
            (out, _) => (NetSource::Edsm, out),
        }
    }

    async fn system(
        &self,
        id64: u64,
        name: &str,
        online: Option<&AtomicBool>,
    ) -> (NetSource, NetOutcome) {
        if !self.is_down(Service::Spansh) {
            let req = Req::Get(spansh::system_url(id64));
            let out = self
                .fetch(Service::Spansh, req, online, |b| {
                    spansh::parse_system(b).map(|s| s.map(NetOutcome::System))
                })
                .await;
            return (NetSource::Spansh, out);
        }
        if self.is_down(Service::Edsm) {
            net_event!("net.health.skipped_down", {"source": NetSource::Edsm});
            return (NetSource::Edsm, NetOutcome::Failed(NetFailure::Down));
        }
        net_event!("net.fallback.edsm", {"spansh_down": true});
        let req = Req::Get(edsm::system_url(name));
        let out = self
            .fetch(Service::Edsm, req, online, |b| {
                edsm::parse_system(b).map(|s| s.map(NetOutcome::System))
            })
            .await;
        (NetSource::Edsm, out)
    }

    /// Capa 3 (Canonn): `codex/ref` (cacheado) + `getSystemPoi` del sistema.
    async fn body_bio(
        &self,
        star_system: &str,
        body_name: &str,
        online: Option<&AtomicBool>,
    ) -> NetOutcome {
        let service = Service::Canonn;
        if !online_allowed(online) {
            return NetOutcome::Disabled;
        }
        if self.is_down(service) {
            net_event!("net.health.skipped_down", {"source": source_of(service)});
            return NetOutcome::Failed(NetFailure::Down);
        }
        let reference = match self.codex_ref(online).await {
            Ok(r) => r,
            Err(miss) => return miss.into(),
        };
        let system_key = star_system.trim().to_lowercase();
        let cached = |key: &str| {
            let poi = self.poi.lock().unwrap();
            poi.as_ref()
                .filter(|(k, at, _)| k == key && at.elapsed() < POI_TTL)
                .map(|(_, _, body)| body.clone())
        };
        let body = match cached(&system_key) {
            Some(body) => {
                net_event!("net.cache.poi.hit", {"bytes": body.len()});
                body
            }
            None => {
                let _one_download = self.poi_lock.lock().await;
                match cached(&system_key) {
                    Some(body) => {
                        net_event!("net.coalesced.poi", {"bytes": body.len()});
                        body
                    }
                    None => {
                        net_event!("net.cache.poi.miss", {});
                        let req = Req::Get(canonn::body_url(star_system));
                        match self.call(service, req, online).await {
                            Ok(body) => {
                                let body = Arc::new(body);
                                *self.poi.lock().unwrap() = Some((
                                    system_key.clone(),
                                    std::time::Instant::now(),
                                    body.clone(),
                                ));
                                body
                            }
                            Err(miss) => return miss.into(),
                        }
                    }
                }
            }
        };
        let resolve = |id: u64| reference.get(&id).cloned();
        match canonn::parse_body(&body, star_system, body_name, &resolve) {
            Ok(bio) => {
                self.succeed(service);
                if bio.hints.is_empty() {
                    NetOutcome::NoData
                } else {
                    NetOutcome::BodyBio(bio)
                }
            }
            Err(e) => {
                // Una respuesta corrupta no se reutiliza.
                *self.poi.lock().unwrap() = None;
                self.fail(service);
                NetOutcome::Failed(e)
            }
        }
    }

    fn memory_ref(&self) -> Option<Arc<CodexRef>> {
        let cache = self.codex_ref.lock().unwrap();
        cache
            .as_ref()
            .filter(|(at, _)| at.elapsed() < CODEX_REF_TTL)
            .map(|(_, r)| r.clone())
    }

    async fn codex_ref(&self, online: Option<&AtomicBool>) -> Result<Arc<CodexRef>, Miss> {
        if let Some(r) = self.memory_ref() {
            net_event!("net.cache.codex_ref.memory_hit", {"entries": r.len()});
            return Ok(r);
        }
        let _one_download = self.ref_lock.lock().await;
        if let Some(r) = self.memory_ref() {
            net_event!("net.coalesced.codex_ref", {"entries": r.len()});
            return Ok(r);
        }
        if let Some(persist) = self.persist.clone() {
            let stored = tokio::task::spawn_blocking(move || persist.load())
                .await
                .ok()
                .flatten();
            if let Some((at, text)) = stored {
                let age = (self.utc_now)() - at;
                if let (Ok(age), Some(map)) = (age.to_std(), canonn::deserialize_codex_ref(&text))
                    && age < CODEX_REF_TTL
                    && !map.is_empty()
                {
                    let r = Arc::new(map);
                    let seen = std::time::Instant::now()
                        .checked_sub(age)
                        .unwrap_or_else(std::time::Instant::now);
                    *self.codex_ref.lock().unwrap() = Some((seen, r.clone()));
                    net_event!("net.cache.codex_ref.persisted_hit", {"entries": r.len(),
                        "age_s": age.as_secs()});
                    return Ok(r);
                }
            }
        }
        net_event!("net.cache.codex_ref.miss", {"persisted": self.persist.is_some()});
        let req = Req::Get(canonn::codex_ref_url());
        let body = self.call(Service::Canonn, req, online).await?;
        match canonn::parse_codex_ref(&body) {
            Ok(map) => {
                self.succeed(Service::Canonn);
                if let Some(persist) = self.persist.clone() {
                    let text = canonn::serialize_codex_ref(&map);
                    let at = (self.utc_now)();
                    // Fallos de escritura los registra quien implementa `save`.
                    let _ = tokio::task::spawn_blocking(move || persist.save(&text, at)).await;
                }
                let r = Arc::new(map);
                *self.codex_ref.lock().unwrap() = Some((std::time::Instant::now(), r.clone()));
                Ok(r)
            }
            Err(e) => {
                self.fail(Service::Canonn);
                Err(Miss::Fail(e))
            }
        }
    }

    /// Petición simple: `parse` devuelve `Ok(None)` para «sin datos».
    async fn fetch(
        &self,
        service: Service,
        req: Req,
        online: Option<&AtomicBool>,
        parse: impl FnOnce(&str) -> Result<Option<NetOutcome>, NetFailure>,
    ) -> NetOutcome {
        if !online_allowed(online) {
            return NetOutcome::Disabled;
        }
        if self.is_down(service) {
            net_event!("net.health.skipped_down", {"source": source_of(service)});
            return NetOutcome::Failed(NetFailure::Down);
        }
        let body = match self.call(service, req, online).await {
            Ok(body) => body,
            Err(miss) => return miss.into(),
        };
        match parse(&body) {
            Ok(Some(out)) => {
                self.succeed(service);
                out
            }
            Ok(None) => {
                self.succeed(service);
                NetOutcome::NoData
            }
            Err(e) => {
                self.fail(service);
                NetOutcome::Failed(e)
            }
        }
    }

    /// Alta de un trabajo de ruta en Spansh y sondeo hasta terminar (o 120 s).
    ///
    /// Los 120 s se miden desde que se obtiene el cerrojo de rutas y tras el
    /// alta: no son un tope absoluto de reloj (peor caso aproximado: 120 s +
    /// intervalo de sondeo + tiempo máximo de una petición, sin contar la espera
    /// del cerrojo). Se toleran hasta 2 fallos transitorios consecutivos de
    /// sondeo (5xx, tiempo agotado, 429); el 3.º se devuelve.
    async fn route(&self, post: Req, online: Option<&AtomicBool>) -> NetOutcome {
        self.run_job(post, online, POLL_LIMIT, |b| {
            Ok(match spansh::parse_route_result(b)? {
                RoutePoll::Pending => Step::Pending,
                RoutePoll::Done(route) => Step::Done(Box::new(NetOutcome::Route(route))),
                RoutePoll::Failed(msg) => Step::Failed(msg),
            })
        })
        .await
    }

    /// Alta + sondeo de un trabajo de Spansh (rutas y comercio comparten
    /// cerrojo: un solo trabajo a la vez).
    async fn run_job(
        &self,
        post: Req,
        online: Option<&AtomicBool>,
        limit: Duration,
        parse: impl Fn(&str) -> Result<Step, NetFailure>,
    ) -> NetOutcome {
        let service = Service::Spansh;
        if self.is_down(service) {
            net_event!("net.health.skipped_down", {"source": source_of(service)});
            return NetOutcome::Failed(NetFailure::Down);
        }
        let _one_at_a_time = self.route_lock.lock().await;
        net_event!("net.route.lock_acquired", {});
        let job = match self.call(service, post, online).await {
            Ok(body) => match spansh::parse_job(&body) {
                Ok(job) => job,
                Err(e) => {
                    self.fail(service);
                    return NetOutcome::Failed(e);
                }
            },
            Err(miss) => return miss.into(),
        };
        // El alta fue correcta: cuenta como éxito para la salud (solo cuentan fallos consecutivos).
        self.succeed(service);
        let mut transient = 0u32;
        let deadline = tokio::time::Instant::now() + limit;
        loop {
            let body = match self
                .call(service, Req::Get(spansh::results_url(&job)), online)
                .await
            {
                Ok(body) => {
                    transient = 0;
                    body
                }
                Err(Miss::Fail(
                    f @ (NetFailure::Timeout | NetFailure::RateLimited | NetFailure::Http(500..)),
                )) => {
                    transient += 1;
                    net_event!("net.route.poll.transient", {"transient": transient,
                        "max": MAX_TRANSIENT_POLL_FAILURES});
                    if transient > MAX_TRANSIENT_POLL_FAILURES
                        || tokio::time::Instant::now() >= deadline
                    {
                        return NetOutcome::Failed(f);
                    }
                    tokio::time::sleep(POLL_EVERY).await;
                    continue;
                }
                Err(miss) => return miss.into(),
            };
            match parse(&body) {
                Ok(Step::Done(out)) => {
                    self.succeed(service);
                    return *out;
                }
                Ok(Step::Failed(msg)) => {
                    net_event!("net.route.job_failed", {"message_bytes": msg.len()});
                    tracing::warn!(%msg, "Spansh no pudo calcular la ruta");
                    self.succeed(service);
                    return NetOutcome::NoData;
                }
                Ok(Step::Pending) => {
                    net_event!("net.route.poll.pending", {});
                    self.succeed(service)
                }
                Err(e) => {
                    self.fail(service);
                    return NetOutcome::Failed(e);
                }
            }
            if tokio::time::Instant::now() >= deadline {
                net_event!("net.route.deadline", {"limit_s": POLL_LIMIT.as_secs()});
                return NetOutcome::Failed(NetFailure::Timeout);
            }
            tokio::time::sleep(POLL_EVERY).await;
        }
    }

    /// Limitador + transporte + traducción de estado HTTP. `Ok(cuerpo)` solo
    /// para 2xx; el éxito de salud lo registra quien interpreta el cuerpo.
    async fn call(
        &self,
        service: Service,
        req: Req,
        online: Option<&AtomicBool>,
    ) -> Result<String, Miss> {
        if !online_allowed(online) {
            net_event!("net.optout.blocked", {"stage": 1, "source": source_of(service)});
            return Err(Miss::Disabled);
        }
        self.wait_turn(service).await;
        if !online_allowed(online) {
            net_event!("net.optout.blocked", {"stage": 2, "source": source_of(service)});
            return Err(Miss::Disabled);
        }
        #[cfg(debug_assertions)]
        let (endpoint, sent_at) = (endpoint_of(&req), std::time::Instant::now());
        net_event!("net.http.request", {"endpoint": endpoint, "source": source_of(service),
            "method": if matches!(req, Req::Get(_)) { "GET" } else { "POST" }});
        let reply = match &req {
            Req::Get(url) if service == Service::Canonn => {
                self.transport.get_with_timeout(url, CANONN_TIMEOUT).await
            }
            Req::Get(url) => self.transport.get(url).await,
            Req::Post(url, form) => self.transport.post_form(url, form).await,
            Req::PostJson(url, body) => self.transport.post_json(url, body).await,
        };
        net_event!("net.http.response", {"endpoint": endpoint, "source": source_of(service),
            "duration_ms": sent_at.elapsed().as_millis() as u64,
            "class": match &reply {
                Err(TransportError::Timeout) => 1,
                Err(TransportError::Connect(_)) => 2,
                Ok(r) => i64::from(r.status / 100) * 100,
            },
            "status": reply.as_ref().map_or(0, |r| r.status),
            "bytes": reply.as_ref().map_or(0, |r| r.body.len())});
        match reply {
            Err(TransportError::Timeout) => {
                self.fail(service);
                Err(Miss::Fail(NetFailure::Timeout))
            }
            Err(TransportError::Connect(msg)) => {
                tracing::debug!(%msg, ?service, "fallo de conexión");
                self.fail(service);
                Err(Miss::Fail(NetFailure::Down))
            }
            Ok(HttpReply { status, body }) => match status {
                200..=299 => Ok(body),
                404 => {
                    self.succeed(service);
                    Err(Miss::NoData)
                }
                429 => {
                    self.fail(service);
                    Err(Miss::Fail(NetFailure::RateLimited))
                }
                500..=599 => {
                    self.fail(service);
                    Err(Miss::Fail(NetFailure::Http(status)))
                }
                // Otros 4xx: petición rechazada, no es un fallo del servicio.
                _ => {
                    tracing::warn!(status, ?service, "petición rechazada por el servicio");
                    Err(Miss::Fail(NetFailure::Http(status)))
                }
            },
        }
    }

    async fn wait_turn(&self, service: Service) {
        #[cfg(debug_assertions)]
        let (started, mut waits) = (std::time::Instant::now(), 0u32);
        loop {
            let wait = {
                let mut st = self.state.lock().unwrap();
                let limiter = st.limiters.get_mut(&service).expect("limitador");
                limiter.try_acquire(now())
            };
            match wait {
                Ok(()) => {
                    net_event!("net.limiter.acquired", {"source": source_of(service),
                        "waits": waits, "waited_ms": started.elapsed().as_millis() as u64});
                    return;
                }
                Err(d) => {
                    #[cfg(debug_assertions)]
                    {
                        waits += 1;
                    }
                    net_event!("net.limiter.wait", {"source": source_of(service),
                        "delay_ms": d.as_millis() as u64});
                    tokio::time::sleep(d).await
                }
            }
        }
    }

    /// Cuándo reintentar la consulta tras un fallo (consumidor: `NetService`).
    ///
    /// `None` si no hay nada que reintentar: el servicio que la atendería está
    /// caído (o, en `System`, lo están Spansh y EDSM), no hay retroceso
    /// pendiente o la consulta no usa red. Si Spansh acaba de caer, `System`
    /// se reintenta ya mismo contra EDSM.
    pub fn retry_at(&self, q: &NetQuery) -> Option<tokio::time::Instant> {
        let t = now();
        let st = self.state.lock().unwrap();
        let down = |s: Service| st.health.get(&s).is_none_or(|h| h.is_down(t));
        let (service, fallback) = match q {
            NetQuery::BodyBio { .. } => (Service::Canonn, false),
            q if is_typed_station_search(q) => (Service::Spansh, false),
            NetQuery::CommodityImportsNearby { .. }
            | NetQuery::NearestService { .. }
            | NetQuery::CommodityExportsNearby { .. } => (Service::Ardent, false),
            NetQuery::System { .. } if down(Service::Spansh) => (Service::Edsm, true),
            NetQuery::SystemCoordsByName { .. } if down(Service::Ardent) => (Service::Edsm, true),
            NetQuery::SystemCoordsByName { .. } => (Service::Ardent, false),
            _ => (Service::Spansh, false),
        };
        if down(service) {
            return None;
        }
        match st.health.get(&service).and_then(|h| h.retry_at()) {
            Some(at) => Some(tokio::time::Instant::from_std(at.max(t))),
            None if fallback => Some(tokio::time::Instant::from_std(t)),
            None => None,
        }
    }

    /// `true` si ningún servicio puede atender la consulta ahora mismo.
    pub fn is_down_for(&self, q: &NetQuery) -> bool {
        match q {
            NetQuery::BodyBio { .. } => self.is_down(Service::Canonn),
            q if is_typed_station_search(q) => self.is_down(Service::Spansh),
            NetQuery::System { .. } => self.is_down(Service::Spansh) && self.is_down(Service::Edsm),
            NetQuery::SystemCoordsByName { .. } => {
                self.is_down(Service::Ardent) && self.is_down(Service::Edsm)
            }
            NetQuery::CommodityImportsNearby { .. }
            | NetQuery::NearestService { .. }
            | NetQuery::CommodityExportsNearby { .. } => self.is_down(Service::Ardent),
            _ => self.is_down(Service::Spansh),
        }
    }

    fn is_down(&self, service: Service) -> bool {
        let st = self.state.lock().unwrap();
        st.health.get(&service).is_some_and(|h| h.is_down(now()))
    }

    fn fail(&self, service: Service) {
        let mut st = self.state.lock().unwrap();
        if let Some(h) = st.health.get_mut(&service) {
            h.record_failure(now());
            net_event!("net.health.failure", {"source": source_of(service),
                "failures": h.failures(), "down": h.is_down(now()),
                "backoff_ms": h.retry_at().map_or(0, |at|
                    at.saturating_duration_since(now()).as_millis() as u64)});
        }
        tracing::debug!(source = ?source_of(service), "fallo registrado");
    }

    fn succeed(&self, service: Service) {
        let mut st = self.state.lock().unwrap();
        if let Some(h) = st.health.get_mut(&service) {
            net_event!("net.health.success", {"source": source_of(service),
                "recovered_from": h.failures()});
            h.record_success();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{BoxFut, HttpReply, TransportError};
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;
    use tmc_model::net::{
        BodyProfile, GalaxyRouteParams, NearbyService, NetFailure, NeutronRouteParams, PadSize,
        ShipParams,
    };

    const SYSTEM: &str = include_str!("../../../tests/fixtures/net/spansh_system_ok.json");
    const SYSTEM_404: &str = include_str!("../../../tests/fixtures/net/spansh_system_404.json");
    const JOB: &str = include_str!("../../../tests/fixtures/net/spansh_route_job.json");
    const NEUTRON: &str =
        include_str!("../../../tests/fixtures/net/spansh_route_neutron_result.json");
    const GALAXY: &str =
        include_str!("../../../tests/fixtures/net/spansh_route_galaxy_result.json");
    const AUTOCOMPLETE: &str = include_str!("../../../tests/fixtures/net/spansh_autocomplete.json");
    const EDSM_SOL: &str = r#"{"name":"Sol","id":27,"id64":10477373803,"information":{"allegiance":"Federation","government":"Democracy","population":22780919531},"primaryStar":{"type":"G (White-Yellow) Star"}}"#;
    const PENDING: &str = r#"{"job":"J","status":"queued","state":"started"}"#;

    type Reply = Result<HttpReply, TransportError>;

    fn ok(status: u16, body: &str) -> Reply {
        Ok(HttpReply {
            status,
            body: body.to_string(),
        })
    }

    /// Transporte simulado: respuestas guiadas por subcadena de la URL. Cada
    /// ruta consume su cola y repite la última respuesta cuando queda una.
    #[derive(Default)]
    struct FakeTransport {
        routes: Mutex<Vec<(String, VecDeque<Reply>)>>,
        calls: Mutex<Vec<String>>,
        timeouts: Mutex<Vec<(String, Duration)>>,
        forms: Mutex<Vec<Vec<(String, String)>>>,
        json_bodies: Mutex<Vec<String>>,
    }

    impl FakeTransport {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }
        fn script(self: &Arc<Self>, pattern: &str, replies: Vec<Reply>) {
            self.routes
                .lock()
                .unwrap()
                .push((pattern.to_string(), replies.into()));
        }
        fn count(&self, pattern: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|c| c.contains(pattern))
                .count()
        }
        fn log(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
        fn total(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
        fn serve(&self, label: String, url: &str) -> Reply {
            self.calls.lock().unwrap().push(label);
            let mut routes = self.routes.lock().unwrap();
            for (pat, q) in routes.iter_mut() {
                if url.contains(pat.as_str()) {
                    return if q.len() > 1 {
                        q.pop_front().unwrap()
                    } else {
                        q.front().cloned().expect("ruta sin respuestas")
                    };
                }
            }
            Err(TransportError::Connect(format!("sin guion para {url}")))
        }
    }

    impl Transport for FakeTransport {
        fn get(&self, url: &str) -> BoxFut<'_, Reply> {
            let r = self.serve(format!("GET {url}"), url);
            Box::pin(async move { r })
        }
        fn get_with_timeout(&self, url: &str, timeout: Duration) -> BoxFut<'_, Reply> {
            self.timeouts
                .lock()
                .unwrap()
                .push((url.to_string(), timeout));
            self.get(url)
        }
        fn post_form(&self, url: &str, form: &[(String, String)]) -> BoxFut<'_, Reply> {
            self.forms.lock().unwrap().push(form.to_vec());
            let r = self.serve(format!("POST {url}"), url);
            Box::pin(async move { r })
        }
        fn post_json(&self, url: &str, body: &str) -> BoxFut<'_, Reply> {
            self.json_bodies.lock().unwrap().push(body.to_string());
            let r = self.serve(format!("POST {url}"), url);
            Box::pin(async move { r })
        }
    }

    fn client(t: &Arc<FakeTransport>) -> NetClient {
        NetClient::new(t.clone())
    }

    fn sol_query() -> NetQuery {
        NetQuery::System {
            id64: 10477373803,
            name: "Sol".into(),
        }
    }

    fn neutron_query() -> NetQuery {
        NetQuery::NeutronRoute(NeutronRouteParams {
            from: "Sol".into(),
            to: "Colonia".into(),
            range_ly: 50.0,
            efficiency: 60,
            supercharge_multiplier: 4.0,
        })
    }

    #[tokio::test(start_paused = true)]
    async fn opt_out_after_rate_wait_prevents_http_without_health_change() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let transport = FakeTransport::new();
        transport.script("/api/system/", vec![ok(200, SYSTEM)]);
        let client = Arc::new(client(&transport));
        let online = Arc::new(AtomicBool::new(true));
        client.execute(&sol_query()).await;
        let pending = tokio::spawn({
            let client = client.clone();
            let online = online.clone();
            async move { client.execute_with_online(&sol_query(), &online).await }
        });
        tokio::task::yield_now().await;
        assert_eq!(transport.total(), 1);
        online.store(false, Ordering::Release);
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(pending.await.unwrap().1, NetOutcome::Disabled);
        assert_eq!(transport.total(), 1);
        assert!(client.retry_at(&sol_query()).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn opt_out_while_waiting_for_route_lock_prevents_submission() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let transport = FakeTransport::new();
        transport.script("/api/route", vec![ok(202, JOB)]);
        transport.script("/api/results/", vec![ok(200, NEUTRON)]);
        let client = Arc::new(client(&transport));
        let online = Arc::new(AtomicBool::new(true));
        let lock = client.route_lock.lock().await;
        let pending = tokio::spawn({
            let client = client.clone();
            let online = online.clone();
            async move { client.execute_with_online(&neutron_query(), &online).await }
        });
        tokio::task::yield_now().await;
        online.store(false, Ordering::Release);
        drop(lock);
        assert_eq!(pending.await.unwrap().1, NetOutcome::Disabled);
        assert_eq!(transport.total(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn opt_out_between_route_polls_prevents_next_http() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let transport = FakeTransport::new();
        transport.script("/api/route", vec![ok(202, JOB)]);
        transport.script("/api/results/", vec![ok(202, PENDING)]);
        let client = Arc::new(client(&transport));
        let online = Arc::new(AtomicBool::new(true));
        let pending = tokio::spawn({
            let client = client.clone();
            let online = online.clone();
            async move { client.execute_with_online(&neutron_query(), &online).await }
        });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(transport.total(), 2);
        online.store(false, Ordering::Release);
        tokio::time::advance(Duration::from_secs(2)).await;
        assert_eq!(pending.await.unwrap().1, NetOutcome::Disabled);
        assert_eq!(transport.total(), 2);
        assert!(client.retry_at(&neutron_query()).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn opt_out_after_fallback_rate_wait_prevents_edsm_http() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let transport = FakeTransport::new();
        transport.script("edsm.net", vec![ok(200, EDSM_SOL)]);
        let client = Arc::new(client(&transport));
        for _ in 0..3 {
            client.fail(Service::Spansh);
        }
        {
            let mut state = client.state.lock().unwrap();
            for _ in 0..10 {
                state
                    .limiters
                    .get_mut(&Service::Edsm)
                    .unwrap()
                    .try_acquire(now())
                    .unwrap();
            }
        }
        let online = Arc::new(AtomicBool::new(true));
        let pending = tokio::spawn({
            let client = client.clone();
            let online = online.clone();
            async move { client.execute_with_online(&sol_query(), &online).await }
        });
        tokio::task::yield_now().await;
        online.store(false, Ordering::Release);
        tokio::time::advance(Duration::from_secs(10)).await;
        assert_eq!(
            pending.await.unwrap(),
            (NetSource::Edsm, NetOutcome::Disabled)
        );
        assert_eq!(transport.total(), 0);
        assert!(
            client.state.lock().unwrap().health[&Service::Edsm]
                .retry_at()
                .is_none()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn system_from_spansh() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/10477373803", vec![ok(200, SYSTEM)]);
        let (src, out) = client(&t).execute(&sol_query()).await;
        assert_eq!(src, NetSource::Spansh);
        let NetOutcome::System(s) = out else {
            panic!("{out:?}")
        };
        assert_eq!(s.name, "Sol");
        assert_eq!(s.body_count, Some(40));
    }

    #[tokio::test(start_paused = true)]
    async fn spansh_404_is_no_data() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(404, SYSTEM_404)]);
        let c = client(&t);
        assert_eq!(
            c.execute(&sol_query()).await,
            (NetSource::Spansh, NetOutcome::NoData)
        );
        // La salud de Spansh no cuenta fallos: cinco 404 seguidos y sigue consultándose a Spansh.
        for _ in 0..5 {
            assert_eq!(
                c.execute(&sol_query()).await,
                (NetSource::Spansh, NetOutcome::NoData)
            );
        }
        assert_eq!(t.count("spansh.co.uk"), 6);
        assert_eq!(t.count("edsm.net"), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn three_failures_mark_spansh_down_and_use_edsm() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(503, "")]);
        t.script("edsm.net/api-v1/system", vec![ok(200, EDSM_SOL)]);
        let c = client(&t);
        for _ in 0..3 {
            assert_eq!(
                c.execute(&sol_query()).await,
                (NetSource::Spansh, NetOutcome::Failed(NetFailure::Http(503)))
            );
        }
        let (src, out) = c.execute(&sol_query()).await;
        assert_eq!(src, NetSource::Edsm);
        let NetOutcome::System(s) = out else {
            panic!("{out:?}")
        };
        assert_eq!(s.id64, 10477373803);
        assert_eq!(s.primary_star.as_deref(), Some("G (White-Yellow) Star"));
        assert_eq!(t.count("spansh.co.uk"), 3);
        assert_eq!(t.count("edsm.net"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn edsm_unknown_system_is_no_data_and_edsm_failure_is_reported() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(500, "")]);
        t.script("edsm.net", vec![ok(200, "[]"), ok(502, "")]);
        let c = client(&t);
        for _ in 0..3 {
            c.execute(&sol_query()).await;
        }
        assert_eq!(
            c.execute(&sol_query()).await,
            (NetSource::Edsm, NetOutcome::NoData)
        );
        assert_eq!(
            c.execute(&sol_query()).await,
            (NetSource::Edsm, NetOutcome::Failed(NetFailure::Http(502)))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn both_services_down_reports_down() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk", vec![ok(503, "")]);
        t.script("edsm.net", vec![ok(503, "")]);
        let c = client(&t);
        for _ in 0..6 {
            c.execute(&sol_query()).await;
        }
        assert_eq!(
            c.execute(&sol_query()).await,
            (NetSource::Edsm, NetOutcome::Failed(NetFailure::Down))
        );
        assert_eq!(t.total(), 6);
    }

    #[tokio::test(start_paused = true)]
    async fn spansh_recovers_after_ten_minutes_down() {
        let t = FakeTransport::new();
        t.script(
            "spansh.co.uk/api/system/",
            vec![ok(503, ""), ok(503, ""), ok(503, ""), ok(200, SYSTEM)],
        );
        t.script("edsm.net", vec![ok(200, EDSM_SOL)]);
        let c = client(&t);
        for _ in 0..3 {
            c.execute(&sol_query()).await;
        }
        assert_eq!(c.execute(&sol_query()).await.0, NetSource::Edsm);
        tokio::time::sleep(Duration::from_secs(601)).await;
        assert_eq!(c.execute(&sol_query()).await.0, NetSource::Spansh);
    }

    #[tokio::test(start_paused = true)]
    async fn rate_429_maps_to_rate_limited_and_counts_as_failure() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(429, "")]);
        t.script("edsm.net", vec![ok(200, EDSM_SOL)]);
        let c = client(&t);
        for _ in 0..3 {
            assert_eq!(
                c.execute(&sol_query()).await,
                (
                    NetSource::Spansh,
                    NetOutcome::Failed(NetFailure::RateLimited)
                )
            );
        }
        assert_eq!(c.execute(&sol_query()).await.0, NetSource::Edsm);
    }

    #[tokio::test(start_paused = true)]
    async fn invalid_json_is_malformed() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(200, "<html>")]);
        assert_eq!(
            client(&t).execute(&sol_query()).await,
            (NetSource::Spansh, NetOutcome::Failed(NetFailure::Malformed))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_maps_to_failed_timeout() {
        let t = FakeTransport::new();
        t.script(
            "spansh.co.uk/api/system/",
            vec![Err(TransportError::Timeout)],
        );
        assert_eq!(
            client(&t).execute(&sol_query()).await,
            (NetSource::Spansh, NetOutcome::Failed(NetFailure::Timeout))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn connect_error_maps_to_down() {
        let t = FakeTransport::new();
        t.script(
            "spansh.co.uk/api/system/",
            vec![Err(TransportError::Connect("dns".into()))],
        );
        assert_eq!(
            client(&t).execute(&sol_query()).await,
            (NetSource::Spansh, NetOutcome::Failed(NetFailure::Down))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn limiter_spaces_spansh_requests_by_one_second() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(200, SYSTEM)]);
        let c = client(&t);
        let start = tokio::time::Instant::now();
        c.execute(&sol_query()).await;
        assert_eq!(start.elapsed(), Duration::ZERO);
        c.execute(&sol_query()).await;
        assert_eq!(start.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn route_job_is_polled_until_done() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script(
            "/api/results/1F2B690E-CA6C-11F1-B0E5-AD2C2EDDEC53",
            vec![ok(202, PENDING), ok(202, PENDING), ok(200, NEUTRON)],
        );
        let start = tokio::time::Instant::now();
        let (src, out) = client(&t).execute(&neutron_query()).await;
        assert_eq!(src, NetSource::Spansh);
        let NetOutcome::Route(r) = out else {
            panic!("{out:?}")
        };
        assert_eq!(r.hops.len(), 4);
        assert_eq!(r.hops[0].name, "PSR J1752-2806");
        assert_eq!(t.count("/api/results/"), 3);
        assert_eq!(t.count("POST https://spansh.co.uk/api/route"), 1);
        // 1 s del limitador antes del primer sondeo + dos esperas de 2 s entre tres sondeos.
        assert_eq!(start.elapsed(), Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn galaxy_route_posts_to_generic_route() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/generic/route", vec![ok(202, JOB)]);
        t.script("/api/results/", vec![ok(200, GALAXY)]);
        let q = NetQuery::GalaxyRoute(GalaxyRouteParams {
            from: "10477373803".into(),
            to: "3238296097059".into(),
            ship: ShipParams {
                unladen_mass: 300.0,
                fuel_main_capacity: 32.0,
                fuel_reserve: 2.0,
                cargo: 0.0,
                optimal_mass: 1052.0,
                max_fuel_per_jump: 5.2,
                fuel_mul: 0.012,
                fuel_power: 2.45,
                guardian_bonus_ly: 0.0,
                supercharge_multiplier: 4.0,
            },
            refuel_scoopable: true,
            use_injections: false,
        });
        let (src, out) = client(&t).execute(&q).await;
        assert_eq!(src, NetSource::Spansh);
        let NetOutcome::Route(r) = out else {
            panic!("{out:?}")
        };
        assert_eq!(r.hops.len(), 3);
        assert_eq!(t.count("POST https://spansh.co.uk/api/generic/route"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn route_that_never_finishes_times_out_after_120_seconds() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script("/api/results/", vec![ok(202, PENDING)]);
        let start = tokio::time::Instant::now();
        let (src, out) = client(&t).execute(&neutron_query()).await;
        assert_eq!(
            (src, out),
            (NetSource::Spansh, NetOutcome::Failed(NetFailure::Timeout))
        );
        assert!(start.elapsed() >= Duration::from_secs(120));
        assert!(start.elapsed() < Duration::from_secs(125));
        assert_eq!(t.count("/api/results/"), 61);
    }

    #[tokio::test(start_paused = true)]
    async fn failed_job_is_no_data_and_bad_submit_is_http_error() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script(
            "/api/results/",
            vec![
                ok(202, PENDING),
                ok(200, r#"{"status":"failed","error":"x"}"#),
            ],
        );
        assert_eq!(
            client(&t).execute(&neutron_query()).await,
            (NetSource::Spansh, NetOutcome::NoData)
        );

        let t = FakeTransport::new();
        t.script(
            "spansh.co.uk/api/route",
            vec![ok(400, r#"{"error":"Could not find finishing system"}"#)],
        );
        assert_eq!(
            client(&t).execute(&neutron_query()).await,
            (NetSource::Spansh, NetOutcome::Failed(NetFailure::Http(400)))
        );
        assert_eq!(t.count("/api/results/"), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn route_poll_timeout_from_transport_fails_cleanly() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script("/api/results/", vec![Err(TransportError::Timeout)]);
        assert_eq!(
            client(&t).execute(&neutron_query()).await,
            (NetSource::Spansh, NetOutcome::Failed(NetFailure::Timeout))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn routes_run_one_at_a_time() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script(
            "/api/results/",
            vec![ok(202, PENDING), ok(202, PENDING), ok(200, NEUTRON)],
        );
        let c = Arc::new(client(&t));
        let (a, b) = {
            let (c1, c2) = (c.clone(), c.clone());
            let h1 = tokio::spawn(async move { c1.execute(&neutron_query()).await });
            let h2 = tokio::spawn(async move { c2.execute(&neutron_query()).await });
            (h1.await.unwrap(), h2.await.unwrap())
        };
        assert!(matches!(a.1, NetOutcome::Route(_)));
        // La segunda ruta empieza cuando termina la primera: sus sondeos ya
        // devuelven la última respuesta (Done) y no se mezclan con los de la primera.
        assert!(matches!(b.1, NetOutcome::Route(_)));
        let kinds: Vec<String> = t
            .log()
            .iter()
            .map(|c| c.split(' ').next().unwrap().to_string())
            .collect();
        assert_eq!(kinds, ["POST", "GET", "GET", "GET", "POST", "GET"]);
    }

    #[tokio::test(start_paused = true)]
    async fn autocomplete_uses_spansh() {
        let t = FakeTransport::new();
        t.script(
            "field_values/system_names?q=Sol",
            vec![ok(200, AUTOCOMPLETE)],
        );
        let (src, out) = client(&t)
            .execute(&NetQuery::Autocomplete {
                prefix: "Sol".into(),
            })
            .await;
        assert_eq!(src, NetSource::Spansh);
        let NetOutcome::Autocomplete(v) = out else {
            panic!("{out:?}")
        };
        assert_eq!(v.len(), 5);
        assert_eq!(v[1].name, "Solitude");
        assert_eq!(v[1].id64, Some(5059379007779));
    }

    const CANONN_BODY: &str = include_str!("../../../tests/fixtures/net/canonn_body_ok.json");
    const CANONN_EMPTY: &str = include_str!("../../../tests/fixtures/net/canonn_body_empty.json");
    const CANONN_SYS: &str = "Sistema Sintetico de Prueba";
    const CANONN_REF: &str = r#"{
      "9900001": {"entryid": 9900001, "english_name": "EspecieAlfa - Roja", "sub_class": "GeneroA"},
      "9900002": {"entryid": 9900002, "english_name": "EspecieBeta - Azul", "sub_class": "GeneroB"},
      "9900004": {"entryid": 9900004, "english_name": "EspecieGamma - Verde", "sub_class": "GeneroC"}
    }"#;

    fn bio_query(body: &str) -> NetQuery {
        NetQuery::BodyBio {
            system_address: 1,
            star_system: CANONN_SYS.into(),
            body_id: 2,
            body_name: format!("{CANONN_SYS} {body}"),
            profile: BodyProfile {
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

    fn canonn_transport(poi: Vec<Reply>) -> Arc<FakeTransport> {
        let t = FakeTransport::new();
        t.script("/codex/ref", vec![ok(200, CANONN_REF)]);
        t.script("/getSystemPoi", poi);
        t
    }

    #[tokio::test(start_paused = true)]
    async fn body_bio_builds_requests_and_parses_hints() {
        let t = canonn_transport(vec![ok(200, CANONN_BODY)]);
        let c = client(&t);
        let (src, out) = c.execute(&bio_query("A 1 a")).await;
        assert_eq!(src, NetSource::Canonn);
        let NetOutcome::BodyBio(bio) = out else {
            panic!("{out:?}")
        };
        assert_eq!(bio.hints.len(), 2);
        assert_eq!(bio.hints[0].genus, "GeneroA");
        assert_eq!(bio.hints[0].species, "EspecieAlfa");
        assert!((bio.hints[0].weight - 0.75).abs() < 1e-12);
        // Solo sistema y categoria salen a la red: nada del jugador.
        assert_eq!(
            t.log(),
            vec![
                "GET https://us-central1-canonn-api-236217.cloudfunctions.net/query/codex/ref?category=Biology".to_string(),
                "GET https://us-central1-canonn-api-236217.cloudfunctions.net/query/getSystemPoi?system=Sistema%20Sintetico%20de%20Prueba&odyssey=Y".to_string(),
            ]
        );
        assert!(t.forms.lock().unwrap().is_empty());
        // Canonn usa su propio tiempo máximo; Spansh no.
        let timeouts = t.timeouts.lock().unwrap().clone();
        assert_eq!(timeouts.len(), 2);
        assert!(timeouts.iter().all(|(_, d)| *d == Duration::from_secs(15)));
        // Otro cuerpo del mismo sistema reutiliza la referencia y la consulta.
        let (_, out) = c.execute(&bio_query("A 1")).await;
        assert!(matches!(out, NetOutcome::BodyBio(_)));
        assert_eq!(t.total(), 2);
    }

    #[derive(Default)]
    struct MemPersist(Mutex<Option<(DateTime<Utc>, String)>>);

    impl RefPersist for MemPersist {
        fn load(&self) -> Option<(DateTime<Utc>, String)> {
            self.0.lock().unwrap().clone()
        }
        fn save(&self, value: &str, at: DateTime<Utc>) {
            *self.0.lock().unwrap() = Some((at, value.to_string()));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn persisted_codex_ref_avoids_download_until_it_expires() {
        let persist = Arc::new(MemPersist::default());
        let t = canonn_transport(vec![ok(200, CANONN_BODY)]);
        let first = NetClient::new(t.clone()).with_codex_persist(persist.clone());
        assert!(matches!(
            first.execute(&bio_query("A 1 a")).await.1,
            NetOutcome::BodyBio(_)
        ));
        assert_eq!(t.count("/codex/ref"), 1);
        let saved = persist.load().unwrap().1;
        assert!(saved.contains("GeneroA") && !saved.contains("english_name"));
        // Nuevo cliente (otro arranque): la referencia sale de la caché persistente.
        let t2 = canonn_transport(vec![ok(200, CANONN_BODY)]);
        let second = NetClient::new(t2.clone()).with_codex_persist(persist.clone());
        assert!(matches!(
            second.execute(&bio_query("A 1 a")).await.1,
            NetOutcome::BodyBio(_)
        ));
        assert_eq!(t2.count("/codex/ref"), 0);
        // Caducada (8 días): se vuelve a bajar.
        let old = Utc::now() - chrono::Duration::days(8);
        persist.save(&saved, old);
        let t3 = canonn_transport(vec![ok(200, CANONN_BODY)]);
        let third = NetClient::new(t3.clone()).with_codex_persist(persist);
        third.execute(&bio_query("A 1 a")).await;
        assert_eq!(t3.count("/codex/ref"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn parallel_body_bio_downloads_once() {
        let t = canonn_transport(vec![ok(200, CANONN_BODY)]);
        let c = Arc::new(client(&t));
        let (q1, q2, q3, q4) = (
            bio_query("A 1 a"),
            bio_query("A 1"),
            bio_query("A 1 a"),
            bio_query("A 2"),
        );
        let (a, b, d, e) = tokio::join!(
            c.execute(&q1),
            c.execute(&q2),
            c.execute(&q3),
            c.execute(&q4)
        );
        assert!(matches!(a.1, NetOutcome::BodyBio(_)));
        assert!(matches!(b.1, NetOutcome::BodyBio(_)));
        assert!(matches!(d.1, NetOutcome::BodyBio(_)));
        assert_eq!(e.1, NetOutcome::NoData);
        assert_eq!(t.count("/codex/ref"), 1);
        assert_eq!(t.count("/getSystemPoi"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn empty_codex_ref_fails_and_is_not_persisted() {
        let persist = Arc::new(MemPersist::default());
        let t = FakeTransport::new();
        t.script("/codex/ref", vec![ok(200, "{}")]);
        t.script("/getSystemPoi", vec![ok(200, CANONN_BODY)]);
        let c = NetClient::new(t.clone()).with_codex_persist(persist.clone());
        assert_eq!(
            c.execute(&bio_query("A 1")).await.1,
            NetOutcome::Failed(NetFailure::Malformed)
        );
        assert!(persist.load().is_none());
        assert_eq!(t.count("/getSystemPoi"), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn body_bio_without_observations_is_no_data() {
        let t = canonn_transport(vec![ok(200, CANONN_EMPTY)]);
        let (_, out) = client(&t).execute(&bio_query("A 1")).await;
        assert_eq!(out, NetOutcome::NoData);
    }

    #[tokio::test(start_paused = true)]
    async fn body_bio_errors_fail_with_backoff_and_then_down() {
        let t = canonn_transport(vec![
            ok(503, ""),
            Err(TransportError::Timeout),
            ok(200, "no es json"),
        ]);
        let c = client(&t);
        let q = bio_query("A 1");
        assert_eq!(c.retry_at(&q), None);
        assert_eq!(
            c.execute(&q).await,
            (NetSource::Canonn, NetOutcome::Failed(NetFailure::Http(503)))
        );
        assert!(c.retry_at(&q).is_some());
        assert!(!c.is_down_for(&q));
        assert_eq!(
            c.execute(&q).await.1,
            NetOutcome::Failed(NetFailure::Timeout)
        );
        assert_eq!(
            c.execute(&q).await.1,
            NetOutcome::Failed(NetFailure::Malformed)
        );
        // Tres fallos seguidos: servicio caido, sin mas trafico.
        assert!(c.is_down_for(&q));
        let sent = t.total();
        assert_eq!(c.execute(&q).await.1, NetOutcome::Failed(NetFailure::Down));
        assert_eq!(t.total(), sent);
    }

    #[tokio::test(start_paused = true)]
    async fn body_bio_opt_out_is_disabled_without_network() {
        use std::sync::atomic::AtomicBool;
        let t = canonn_transport(vec![ok(200, CANONN_BODY)]);
        let online = AtomicBool::new(false);
        assert_eq!(
            client(&t)
                .execute_with_online(&bio_query("A 1"), &online)
                .await,
            (NetSource::Cache, NetOutcome::Disabled)
        );
        assert_eq!(t.total(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn successful_route_resets_health_so_failures_are_consecutive() {
        let t = FakeTransport::new();
        t.script(
            "spansh.co.uk/api/system/",
            vec![ok(503, ""), ok(503, ""), ok(200, SYSTEM)],
        );
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script("/api/results/", vec![ok(200, NEUTRON)]);
        t.script("edsm.net", vec![ok(200, EDSM_SOL)]);
        let c = client(&t);
        c.execute(&sol_query()).await;
        c.execute(&sol_query()).await;
        assert!(matches!(
            c.execute(&neutron_query()).await.1,
            NetOutcome::Route(_)
        ));
        c.execute(&sol_query()).await; // sigue contando 1 fallo, no 3
        let (src, _) = c.execute(&sol_query()).await;
        assert_eq!(src, NetSource::Spansh);
        assert_eq!(t.count("edsm.net"), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn poll_tolerates_two_transient_failures_but_not_three() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script("/api/results/", vec![ok(503, ""), ok(200, NEUTRON)]);
        let (_, out) = client(&t).execute(&neutron_query()).await;
        assert!(matches!(out, NetOutcome::Route(_)));

        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(202, JOB)]);
        t.script(
            "/api/results/",
            vec![
                ok(503, ""),
                Err(TransportError::Timeout),
                ok(429, ""),
                ok(200, NEUTRON),
            ],
        );
        let (_, out) = client(&t).execute(&neutron_query()).await;
        assert_eq!(out, NetOutcome::Failed(NetFailure::RateLimited));
        assert_eq!(t.count("/api/results/"), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn forms_and_urls_carry_special_characters_intact() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/route", vec![ok(400, "{}")]);
        let mut q = neutron_query();
        if let NetQuery::NeutronRoute(p) = &mut q {
            p.from = "Ñandú A+B".into();
        }
        client(&t).execute(&q).await;
        let forms = t.forms.lock().unwrap();
        let from = forms[0].iter().find(|(k, _)| k == "from").unwrap();
        // El transporte codifica el formulario; el adaptador entrega el texto sin tocar.
        assert_eq!(from.1, "Ñandú A+B");
        assert_eq!(
            crate::edsm::system_url("Ñandú A+B"),
            "https://www.edsm.net/api-v1/system?systemName=%C3%91and%C3%BA%20A%2BB&showId=1&showInformation=1&showPrimaryStar=1"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn retry_at_follows_backoff_then_falls_back_to_edsm_then_stops() {
        let t = FakeTransport::new();
        t.script("spansh.co.uk/api/system/", vec![ok(503, "")]);
        t.script("edsm.net", vec![ok(503, "")]);
        let c = client(&t);
        let q = sol_query();
        assert_eq!(c.retry_at(&q), None);
        c.execute(&q).await;
        assert_eq!(
            c.retry_at(&q),
            Some(tokio::time::Instant::now() + Duration::from_secs(30))
        );
        c.execute(&q).await;
        assert_eq!(
            c.retry_at(&q),
            Some(tokio::time::Instant::now() + Duration::from_secs(120))
        );
        c.execute(&q).await;
        // Spansh caído: se reintenta ya contra EDSM, que aún no ha fallado.
        assert!(!c.is_down_for(&q));
        assert_eq!(c.retry_at(&q), Some(tokio::time::Instant::now()));
        for _ in 0..3 {
            c.execute(&q).await;
        }
        assert!(c.is_down_for(&q));
        assert_eq!(c.retry_at(&q), None);
    }

    const ARDENT_PLATINUM: &str =
        include_str!("../../../tests/fixtures/net/ardent_nearby_imports_platinum.json");
    const ARDENT_NEAREST: &str = include_str!("../../../tests/fixtures/net/ardent_nearest_if.json");
    const ARDENT_404: &str = include_str!("../../../tests/fixtures/net/ardent_system_404.json");

    fn ardent_client(t: &Arc<FakeTransport>) -> NetClient {
        NetClient::with_clock(t.clone(), || "2026-10-18T12:00:00Z".parse().unwrap())
    }

    fn imports_query(commodity: &str) -> NetQuery {
        NetQuery::CommodityImportsNearby {
            system_address: 10477373803,
            commodity: commodity.into(),
            max_distance_ly: 100,
            min_pad: PadSize::Medium,
        }
    }

    fn nearest_query() -> NetQuery {
        NetQuery::NearestService {
            system_address: 10477373803,
            system_name: "Sol".into(),
            service: NearbyService::InterstellarFactors,
            min_pad: PadSize::Large,
        }
    }

    const ARDENT_TRADER: &str =
        include_str!("../../../tests/fixtures/net/ardent_nearest_material_trader.json");
    const SPANSH_STATIONS: &str =
        include_str!("../../../tests/fixtures/net/spansh_stations_material_trader.json");

    fn service_query(service: NearbyService) -> NetQuery {
        NetQuery::NearestService {
            system_address: 10477373803,
            system_name: "Sol".into(),
            service,
            min_pad: PadSize::Medium,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn untyped_trader_uses_ardent_material_trader_segment() {
        let t = FakeTransport::new();
        t.script("/nearest/material-trader", vec![ok(200, ARDENT_TRADER)]);
        let (src, out) = ardent_client(&t)
            .execute(&service_query(NearbyService::MaterialTrader(None)))
            .await;
        assert_eq!(src, NetSource::Ardent);
        let NetOutcome::Stations(s) = out else {
            panic!("{out:?}")
        };
        assert!(!s.is_empty() && s.len() <= 5);
        assert_eq!(
            t.log(),
            [
                "GET https://api.ardent-insight.com/v2/system/address/10477373803/nearest/material-trader?minLandingPadSize=2"
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn typed_trader_posts_json_to_spansh_and_reports_spansh() {
        let t = FakeTransport::new();
        t.script("/api/stations/search", vec![ok(200, SPANSH_STATIONS)]);
        let (src, out) = ardent_client(&t)
            .execute(&service_query(NearbyService::MaterialTrader(Some(
                tmc_model::net::TraderKind::Raw,
            ))))
            .await;
        assert_eq!(src, NetSource::Spansh);
        let NetOutcome::Stations(s) = out else {
            panic!("{out:?}")
        };
        assert_eq!(s[0].station_name, "Mercader Ficticio");
        assert_eq!(t.log(), ["POST https://spansh.co.uk/api/stations/search"]);
        let body: serde_json::Value =
            serde_json::from_str(&t.json_bodies.lock().unwrap()[0]).unwrap();
        assert_eq!(body["reference_system"], "Sol");
        assert_eq!(body["filters"]["material_trader"]["value"][0], "Raw");
    }

    #[tokio::test(start_paused = true)]
    async fn spansh_failure_for_typed_service_is_no_data() {
        let t = FakeTransport::new();
        t.script("/api/stations/search", vec![ok(500, "boom")]);
        let q = service_query(NearbyService::TechnologyBroker(Some(
            tmc_model::net::BrokerKind::Guardian,
        )));
        let (src, out) = ardent_client(&t).execute(&q).await;
        assert_eq!((src, out), (NetSource::Spansh, NetOutcome::NoData));
        // Nunca se cae a Ardent sin tipo.
        assert_eq!(t.count("ardent"), 0);
        // Cuerpo corrupto: también «sin datos».
        let t = FakeTransport::new();
        t.script("/api/stations/search", vec![ok(200, "{nope")]);
        let (_, out) = ardent_client(&t).execute(&q).await;
        assert_eq!(out, NetOutcome::NoData);
    }

    #[tokio::test(start_paused = true)]
    async fn typed_service_search_respects_opt_out() {
        use std::sync::atomic::AtomicBool;
        let t = FakeTransport::new();
        let off = AtomicBool::new(false);
        let (_, out) = ardent_client(&t)
            .execute_with_online(
                &service_query(NearbyService::MaterialTrader(Some(
                    tmc_model::net::TraderKind::Encoded,
                ))),
                &off,
            )
            .await;
        assert_eq!(out, NetOutcome::Disabled);
        assert_eq!(t.total(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn ardent_imports_parse_and_filter_with_injected_clock() {
        let t = FakeTransport::new();
        t.script(
            "api.ardent-insight.com/v2/system/address/10477373803/commodity/name/platinum/nearby/imports",
            vec![ok(200, ARDENT_PLATINUM)],
        );
        let (src, out) = ardent_client(&t).execute(&imports_query("Platinum")).await;
        assert_eq!(src, NetSource::Ardent);
        let NetOutcome::Markets(m) = out else {
            panic!("{out:?}")
        };
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].sell_price, 299000);
        assert_eq!(t.total(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn ardent_nearest_returns_stations_filtered_by_pad() {
        let t = FakeTransport::new();
        t.script(
            "nearest/interstellar-factors?minLandingPadSize=3",
            vec![ok(200, ARDENT_NEAREST)],
        );
        let (src, out) = ardent_client(&t).execute(&nearest_query()).await;
        assert_eq!(src, NetSource::Ardent);
        let NetOutcome::Stations(s) = out else {
            panic!("{out:?}")
        };
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].station_name, "Factores Ficticios");
    }

    #[tokio::test(start_paused = true)]
    async fn ardent_404_is_no_data_empty_is_markets_and_garbage_is_malformed() {
        let t = FakeTransport::new();
        t.script(
            "ardent-insight",
            vec![ok(404, ARDENT_404), ok(200, "[]"), ok(200, "<html>")],
        );
        let c = ardent_client(&t);
        let q = imports_query("platinum");
        assert_eq!(c.execute(&q).await, (NetSource::Ardent, NetOutcome::NoData));
        assert_eq!(
            c.execute(&q).await,
            (NetSource::Ardent, NetOutcome::Markets(vec![]))
        );
        assert_eq!(
            c.execute(&q).await,
            (NetSource::Ardent, NetOutcome::Failed(NetFailure::Malformed))
        );
        // El 404 no cuenta como fallo del servicio.
        assert!(!c.is_down_for(&q));
    }

    #[tokio::test(start_paused = true)]
    async fn disabled_online_services_never_reach_ardent() {
        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(200, "[]")]);
        let c = ardent_client(&t);
        let online = AtomicBool::new(false);
        for q in [imports_query("platinum"), nearest_query()] {
            assert_eq!(
                c.execute_with_online(&q, &online).await,
                (NetSource::Cache, NetOutcome::Disabled)
            );
        }
        assert_eq!(t.total(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn ardent_respects_one_request_per_second() {
        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(200, "[]")]);
        let c = ardent_client(&t);
        let start = tokio::time::Instant::now();
        c.execute(&imports_query("platinum")).await;
        assert_eq!(start.elapsed(), Duration::ZERO);
        c.execute(&imports_query("painite")).await;
        assert_eq!(start.elapsed(), Duration::from_secs(1));
        c.execute(&nearest_query()).await;
        assert_eq!(start.elapsed(), Duration::from_secs(2));
        assert_eq!(t.count("ardent-insight"), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn three_ardent_failures_mark_down() {
        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(503, "")]);
        let c = ardent_client(&t);
        let q = imports_query("platinum");
        for _ in 0..3 {
            assert_eq!(
                c.execute(&q).await,
                (NetSource::Ardent, NetOutcome::Failed(NetFailure::Http(503)))
            );
        }
        assert!(c.is_down_for(&q));
        assert!(c.is_down_for(&nearest_query()));
        assert_eq!(c.retry_at(&q), None);
        assert_eq!(
            c.execute(&q).await,
            (NetSource::Ardent, NetOutcome::Failed(NetFailure::Down))
        );
        assert_eq!(t.total(), 3);
        // Spansh no se ve afectado.
        assert!(!c.is_down_for(&sol_query()));
    }

    #[tokio::test(start_paused = true)]
    async fn ardent_failure_sets_backoff_retry() {
        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(503, "")]);
        let c = ardent_client(&t);
        let q = nearest_query();
        assert_eq!(c.retry_at(&q), None);
        c.execute(&q).await;
        assert_eq!(
            c.retry_at(&q),
            Some(tokio::time::Instant::now() + Duration::from_secs(30))
        );
    }

    const TRADE_JOB: &str = include_str!("../../../tests/fixtures/net/spansh_trade_job.json");
    const TRADE_RESULT: &str = include_str!("../../../tests/fixtures/net/spansh_trade_result.json");
    const EXPORTS: &str =
        include_str!("../../../tests/fixtures/net/ardent_nearby_exports_gold.json");
    const COORDS: &str = include_str!("../../../tests/fixtures/net/coords_by_name.json");

    fn coords_part(k: &str) -> String {
        let v: serde_json::Value = serde_json::from_str(COORDS).unwrap();
        v[k].to_string()
    }

    fn trade_query() -> NetQuery {
        NetQuery::TradeRoute(tmc_model::net::TradeRouteParams {
            system: "Sol".into(),
            station: "Abraham Lincoln".into(),
            starting_capital: 5_000_000,
            max_cargo: 256,
            max_hops: 3,
            max_hop_distance_ly: 30.0,
            min_pad: PadSize::Large,
            max_price_age_days: 1,
            allow_planetary: false,
        })
    }

    fn exports_query() -> NetQuery {
        NetQuery::CommodityExportsNearby {
            system_address: 10477373803,
            commodity: "Gold".into(),
            max_distance_ly: 100,
            min_pad: PadSize::Large,
        }
    }

    fn coords_query(name: &str) -> NetQuery {
        NetQuery::SystemCoordsByName { name: name.into() }
    }

    fn bgs_tick_query() -> NetQuery {
        NetQuery::BgsTick
    }

    #[tokio::test]
    async fn bgs_tick_query_returns_tick_info() {
        let t = FakeTransport::new();
        let payload = r#"[{"time": "2026-10-23T04:15:00Z", "updated_at": "2026-10-23T04:15:30Z"}]"#;
        t.script("tick.edcd.io", vec![ok(200, payload)]);
        let c = client(&t);
        let (src, out) = c.execute(&bgs_tick_query()).await;
        assert_eq!(src, NetSource::Edcd);
        match out {
            NetOutcome::BgsTick(info) => {
                assert_eq!(info.latest_tick.to_rfc3339(), "2026-10-23T04:15:00+00:00");
                assert_eq!(info.updated_at.to_rfc3339(), "2026-10-23T04:15:30+00:00");
            }
            other => panic!("expected BgsTick, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn bgs_tick_offline_optout_disabled() {
        let t = FakeTransport::new();
        let c = client(&t);
        let guard = AtomicBool::new(false);
        let (src, out) = c.execute_with_online(&bgs_tick_query(), &guard).await;
        assert_eq!(src, NetSource::Cache);
        assert_eq!(out, NetOutcome::Disabled);
    }

    #[tokio::test(start_paused = true)]
    async fn trade_route_polls_and_returns_plan() {
        let t = FakeTransport::new();
        t.script("/api/trade/route", vec![ok(202, TRADE_JOB)]);
        t.script(
            "/api/results/",
            vec![ok(202, r#"{"status":"queued"}"#), ok(200, TRADE_RESULT)],
        );
        let c = client(&t);
        let (src, out) = c.execute(&trade_query()).await;
        assert_eq!(src, NetSource::Spansh);
        let NetOutcome::TradeRoute(plan) = out else {
            panic!("se esperaba TradeRoute");
        };
        assert_eq!(plan.stops.len(), 2);
        let forms = t.forms.lock().unwrap();
        assert!(
            forms[0]
                .iter()
                .any(|(k, v)| k == "requires_large_pad" && v == "1")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn trade_route_times_out_after_60_seconds() {
        let t = FakeTransport::new();
        t.script("/api/trade/route", vec![ok(202, TRADE_JOB)]);
        t.script("/api/results/", vec![ok(202, r#"{"status":"queued"}"#)]);
        let c = client(&t);
        let start = tokio::time::Instant::now();
        let (_, out) = c.execute(&trade_query()).await;
        assert_eq!(out, NetOutcome::Failed(NetFailure::Timeout));
        assert!(start.elapsed() >= Duration::from_secs(60));
        assert!(start.elapsed() < Duration::from_secs(70));
    }

    #[tokio::test(start_paused = true)]
    async fn trade_route_disabled_offline_never_reaches_transport() {
        let t = FakeTransport::new();
        t.script("spansh", vec![ok(202, TRADE_JOB)]);
        let c = client(&t);
        let online = AtomicBool::new(false);
        for q in [trade_query(), exports_query(), coords_query("Sol")] {
            assert_eq!(
                c.execute_with_online(&q, &online).await.1,
                NetOutcome::Disabled
            );
        }
        assert_eq!(t.total(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn exports_share_ardent_one_per_second_limit() {
        let t = FakeTransport::new();
        t.script("nearby/imports", vec![ok(200, "[]")]);
        t.script("nearby/exports", vec![ok(200, EXPORTS)]);
        let c = ardent_client(&t);
        let start = tokio::time::Instant::now();
        c.execute(&imports_query("gold")).await;
        let (src, out) = c.execute(&exports_query()).await;
        assert_eq!(start.elapsed(), Duration::from_secs(1));
        assert_eq!(src, NetSource::Ardent);
        let NetOutcome::Exports(e) = out else {
            panic!("se esperaba Exports");
        };
        assert_eq!(e[0].buy_price, 41000);
        assert!(e.iter().all(|x| x.max_pad == Some(PadSize::Large)));
        assert!(t.log().iter().any(|l| l.contains("/commodity/name/gold/")));
    }

    #[tokio::test(start_paused = true)]
    async fn coords_use_ardent_then_edsm_when_down() {
        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(200, &coords_part("ardent"))]);
        let c = client(&t);
        let (src, out) = c.execute(&coords_query("Hip 1+2")).await;
        assert_eq!(src, NetSource::Ardent);
        assert!(matches!(out, NetOutcome::Coords(ref s) if s.star_pos == [0.5, -1.25, 3.75]));
        assert!(t.log()[0].ends_with("/system/name/Hip%201%2B2"));

        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(500, "")]);
        t.script("edsm.net", vec![ok(200, &coords_part("edsm"))]);
        let c = client(&t);
        for _ in 0..3 {
            c.execute(&coords_query("Origin")).await;
        }
        assert!(!c.is_down_for(&coords_query("Origin")));
        let (src, out) = c.execute(&coords_query("Origin")).await;
        assert_eq!(src, NetSource::Edsm);
        assert!(matches!(out, NetOutcome::Coords(_)));
    }

    #[tokio::test(start_paused = true)]
    async fn coords_unknown_is_no_data() {
        let t = FakeTransport::new();
        t.script("ardent-insight", vec![ok(404, "{}")]);
        t.script("edsm.net", vec![ok(200, "{}")]);
        let c = client(&t);
        assert_eq!(c.execute(&coords_query("Zzz")).await.1, NetOutcome::NoData);
        assert_eq!(t.count("edsm.net"), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn coords_ardent_404_or_failure_falls_back_to_edsm() {
        for ardent_reply in [ok(404, "{}"), ok(500, ""), ok(200, "not json")] {
            let t = FakeTransport::new();
            t.script("ardent-insight", vec![ardent_reply]);
            t.script("edsm.net", vec![ok(200, &coords_part("edsm"))]);
            let c = client(&t);
            let (src, out) = c.execute(&coords_query("Origin")).await;
            assert_eq!(src, NetSource::Edsm);
            assert!(matches!(out, NetOutcome::Coords(_)));
        }
    }

    #[cfg(debug_assertions)]
    mod diagnostics {
        use super::*;
        use tmc_model::diagnostics::capture::capture;

        fn run(
            t: &Arc<FakeTransport>,
            q: NetQuery,
        ) -> Vec<tmc_model::diagnostics::capture::Record> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .start_paused(true)
                .build()
                .unwrap();
            let c = client(t);
            capture(|| rt.block_on(c.execute(&q))).1
        }

        #[test]
        fn station_endpoints_have_closed_labels_and_hide_system_name_in_requests() {
            use tmc_model::net::{BrokerKind, TraderKind};
            let cases = [
                (
                    NearbyService::MaterialTrader(None),
                    "ardent.nearest.material_trader",
                ),
                (
                    NearbyService::TechnologyBroker(None),
                    "ardent.nearest.technology_broker",
                ),
                (
                    NearbyService::MaterialTrader(Some(TraderKind::Raw)),
                    "spansh.stations.material_trader",
                ),
                (
                    NearbyService::TechnologyBroker(Some(BrokerKind::Human)),
                    "spansh.stations.technology_broker",
                ),
            ];
            for (service, label) in cases {
                let t = FakeTransport::new();
                t.script("/nearest/", vec![ok(500, "x")]);
                t.script("/stations/search", vec![ok(500, "x")]);
                let records = run(
                    &t,
                    NetQuery::NearestService {
                        system_address: 1,
                        system_name: "Sol".into(),
                        service,
                        min_pad: PadSize::Small,
                    },
                );
                let all: String = records.iter().map(|r| r.payload.clone()).collect();
                assert!(all.contains(label), "{label} no emitido en {all}");
                assert!(!all.contains("https://"), "{all}");
            }
        }

        #[test]
        fn query_and_outcome_use_templates_never_urls_or_raw_errors() {
            let t = FakeTransport::new();
            t.script(
                "field_values",
                vec![Err(TransportError::Connect(
                    "https://spansh.co.uk/x?token=CANARYERR".into(),
                ))],
            );
            let q = NetQuery::Autocomplete {
                prefix: "Sol?token=CANARYQ".into(),
            };
            let records = run(&t, q);
            let all: String = records
                .iter()
                .map(|r| {
                    format!(
                        "{} {} {}
",
                        r.code, r.context, r.payload
                    )
                })
                .collect();
            for canary in ["CANARYQ", "CANARYERR", "https://", "spansh.co.uk", "?token"] {
                assert!(!all.contains(canary), "{canary} leaked in {all}");
            }
            let codes: Vec<&str> = records.iter().map(|r| r.code.as_str()).collect();
            for code in [
                "net.query.started",
                "net.limiter.acquired",
                "net.http.request",
                "net.http.response",
                "net.health.failure",
                "net.query.finished",
            ] {
                assert!(codes.contains(&code), "falta {code}: {codes:?}");
            }
            assert!(all.contains("spansh.autocomplete"));
            let ids: std::collections::HashSet<_> = records
                .iter()
                .map(|r| {
                    serde_json::from_str::<serde_json::Value>(&r.payload).unwrap()["request_id"]
                        .clone()
                })
                .collect();
            assert_eq!(ids.len(), 1, "todos comparten el mismo request_id");
        }

        #[test]
        fn f3_queries_use_new_endpoint_templates_without_leaks() {
            let t = FakeTransport::new();
            t.script("/api/trade/route", vec![ok(503, "CANARYBODY")]);
            let records = run(&t, trade_query());
            let all: String = records.iter().map(|r| r.payload.clone()).collect();
            assert!(all.contains("spansh.trade.job"), "{all}");
            assert!(!all.contains("CANARYBODY"));

            let t = FakeTransport::new();
            t.script("nearby/exports", vec![ok(200, EXPORTS)]);
            let records = run(&t, exports_query());
            let all: String = records.iter().map(|r| r.payload.clone()).collect();
            assert!(all.contains("ardent.nearby.exports"), "{all}");
            let finished = records
                .iter()
                .find(|r| r.code == "net.query.finished")
                .unwrap();
            let v: serde_json::Value = serde_json::from_str(&finished.payload).unwrap();
            assert_eq!(v["outcome"], "exports");

            let t = FakeTransport::new();
            t.script("ardent-insight", vec![ok(404, "{}")]);
            t.script("edsm.net", vec![ok(200, "{}")]);
            let records = run(&t, coords_query("CANARYCOORD?token=CANARYQ"));
            let all: String = records.iter().map(|r| r.payload.clone()).collect();
            for canary in ["CANARYCOORD", "CANARYQ", "https://", "?token"] {
                assert!(!all.contains(canary), "{canary} leaked in {all}");
            }
            assert!(all.contains("ardent.system.coords"), "{all}");
            assert!(all.contains("edsm.coords"), "{all}");
        }

        #[test]
        fn http_status_is_numeric_class_with_route_template() {
            let t = FakeTransport::new();
            t.script("/api/route", vec![ok(503, "CANARYBODY")]);
            let records = run(&t, neutron_query());
            let all: String = records.iter().map(|r| r.payload.clone()).collect();
            assert!(!all.contains("CANARYBODY"));
            assert!(all.contains("spansh.route.job"));
            let resp = records
                .iter()
                .find(|r| r.code == "net.http.response")
                .unwrap();
            let v: serde_json::Value = serde_json::from_str(&resp.payload).unwrap();
            assert_eq!(v["status"], 503);
            assert_eq!(v["class"], 500);
            assert_eq!(v["bytes"], 10);
        }
    }
}

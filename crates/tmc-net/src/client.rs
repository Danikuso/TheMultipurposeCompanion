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
//! - La capa 3 (Canonn) está desactivada en F1: `BodyBio` devuelve `Disabled`
//!   sin tocar el transporte.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::ardent;
use crate::edsm;
use crate::health::ServiceHealth;
use crate::limiter::{RateLimiter, Service};
use crate::spansh::{self, RoutePoll};
use crate::transport::{HttpReply, Transport, TransportError};
use chrono::{DateTime, Utc};
use tmc_model::net::{NetFailure, NetOutcome, NetQuery, NetSource};

const POLL_EVERY: Duration = Duration::from_secs(2);
const MAX_TRANSIENT_POLL_FAILURES: u32 = 2;
const POLL_LIMIT: Duration = Duration::from_secs(120);

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

enum Req {
    Get(String),
    Post(String, Vec<(String, String)>),
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
}

fn now() -> std::time::Instant {
    tokio::time::Instant::now().into_std()
}

fn online_allowed(online: Option<&AtomicBool>) -> bool {
    online.is_none_or(|online| online.load(Ordering::Acquire))
}

fn source_of(service: Service) -> NetSource {
    match service {
        Service::Spansh => NetSource::Spansh,
        Service::Edsm => NetSource::Edsm,
        Service::Canonn => NetSource::Canonn,
        Service::Ardent => NetSource::Ardent,
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
        }
    }

    pub async fn execute(&self, q: &NetQuery) -> (NetSource, NetOutcome) {
        self.execute_inner(q, None).await
    }

    async fn execute_inner(
        &self,
        q: &NetQuery,
        online: Option<&AtomicBool>,
    ) -> (NetSource, NetOutcome) {
        if !online_allowed(online) {
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
            // Capa 3 desactivada en F1 (a la espera del permiso de Canonn).
            NetQuery::BodyBio { .. } => (NetSource::Canonn, NetOutcome::Disabled),
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
                system_address,
                service,
                min_pad,
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
            return (NetSource::Edsm, NetOutcome::Failed(NetFailure::Down));
        }
        let req = Req::Get(edsm::system_url(name));
        let out = self
            .fetch(Service::Edsm, req, online, |b| {
                edsm::parse_system(b).map(|s| s.map(NetOutcome::System))
            })
            .await;
        (NetSource::Edsm, out)
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
        let service = Service::Spansh;
        if self.is_down(service) {
            return NetOutcome::Failed(NetFailure::Down);
        }
        let _one_at_a_time = self.route_lock.lock().await;
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
        let deadline = tokio::time::Instant::now() + POLL_LIMIT;
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
            match spansh::parse_route_result(&body) {
                Ok(RoutePoll::Done(route)) => {
                    self.succeed(service);
                    return NetOutcome::Route(route);
                }
                Ok(RoutePoll::Failed(msg)) => {
                    tracing::warn!(%msg, "Spansh no pudo calcular la ruta");
                    self.succeed(service);
                    return NetOutcome::NoData;
                }
                Ok(RoutePoll::Pending) => self.succeed(service),
                Err(e) => {
                    self.fail(service);
                    return NetOutcome::Failed(e);
                }
            }
            if tokio::time::Instant::now() >= deadline {
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
            return Err(Miss::Disabled);
        }
        self.wait_turn(service).await;
        if !online_allowed(online) {
            return Err(Miss::Disabled);
        }
        let reply = match &req {
            Req::Get(url) => self.transport.get(url).await,
            Req::Post(url, form) => self.transport.post_form(url, form).await,
        };
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
        loop {
            let wait = {
                let mut st = self.state.lock().unwrap();
                let limiter = st.limiters.get_mut(&service).expect("limitador");
                limiter.try_acquire(now())
            };
            match wait {
                Ok(()) => return,
                Err(d) => tokio::time::sleep(d).await,
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
            NetQuery::BodyBio { .. } => return None,
            NetQuery::CommodityImportsNearby { .. } | NetQuery::NearestService { .. } => {
                (Service::Ardent, false)
            }
            NetQuery::System { .. } if down(Service::Spansh) => (Service::Edsm, true),
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
            NetQuery::BodyBio { .. } => false,
            NetQuery::System { .. } => self.is_down(Service::Spansh) && self.is_down(Service::Edsm),
            NetQuery::CommodityImportsNearby { .. } | NetQuery::NearestService { .. } => {
                self.is_down(Service::Ardent)
            }
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
        }
        tracing::debug!(source = ?source_of(service), "fallo registrado");
    }

    fn succeed(&self, service: Service) {
        let mut st = self.state.lock().unwrap();
        if let Some(h) = st.health.get_mut(&service) {
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
        forms: Mutex<Vec<Vec<(String, String)>>>,
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
        fn post_form(&self, url: &str, form: &[(String, String)]) -> BoxFut<'_, Reply> {
            self.forms.lock().unwrap().push(form.to_vec());
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

    #[tokio::test(start_paused = true)]
    async fn body_bio_is_disabled_and_never_touches_the_transport() {
        let t = FakeTransport::new();
        let q = NetQuery::BodyBio {
            system_address: 1,
            body_id: 2,
            body_name: "Sol A 1".into(),
            profile: BodyProfile {
                planet_class: None,
                atmosphere_type: None,
                gravity_g: None,
                temperature_k: None,
                pressure_atm: None,
                volcanism: None,
                star_class: None,
            },
        };
        assert_eq!(
            client(&t).execute(&q).await,
            (NetSource::Canonn, NetOutcome::Disabled)
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
            service: NearbyService::InterstellarFactors,
            min_pad: PadSize::Large,
        }
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
}

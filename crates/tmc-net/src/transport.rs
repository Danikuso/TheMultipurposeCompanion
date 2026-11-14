//! Transporte HTTP abstracto (para poder simularlo en tests) y su
//! implementación con `reqwest` + rustls.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

/// Petición POST con cuerpo binario y cabeceras propias (sincronización).
#[derive(Debug, Clone, Copy)]
pub struct HttpPost<'a> {
    pub url: &'a str,
    pub body: &'a [u8],
    pub content_type: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    /// Fuerza HTTP/1.1 (algunos servicios fallan con HTTP/2).
    pub http1_only: bool,
}

/// Respuesta con las cabeceras de limitación de ritmo ya interpretadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReplyEx {
    pub status: u16,
    pub body: String,
    pub retry_after_s: Option<u64>,
    pub rate_limit_remaining: Option<u32>,
    pub rate_limit_reset_s: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Timeout,
    Connect(String),
}

pub trait Transport: Send + Sync {
    fn get(&self, url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>>;
    /// GET con tiempo máximo propio (servicios lentos como Canonn). Por defecto
    /// ignora el tiempo y usa el del transporte.
    fn get_with_timeout(
        &self,
        url: &str,
        _timeout: Duration,
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.get(url)
    }
    fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> BoxFut<'_, Result<HttpReply, TransportError>>;
    /// POST con cuerpo JSON (`Content-Type: application/json`). Por defecto no
    /// está soportado y responde como un fallo de conexión.
    fn post_json(&self, _url: &str, _body: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        Box::pin(async { Err(TransportError::Connect("post_json no soportado".into())) })
    }
    /// GET con cabecera `Authorization: Bearer <token>`.
    fn get_with_auth(
        &self,
        _url: &str,
        _token: &str,
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        Box::pin(async { Err(TransportError::Connect("get_with_auth no soportado".into())) })
    }
    /// POST con cuerpo binario, cabeceras propias y cabeceras de respuesta.
    /// Por defecto no está soportado y responde como un fallo de conexión.
    fn post<'a>(&'a self, _req: HttpPost<'a>) -> BoxFut<'a, Result<HttpReplyEx, TransportError>> {
        Box::pin(async { Err(TransportError::Connect("post no soportado".into())) })
    }
}

const TIMEOUT: Duration = Duration::from_secs(15);

/// Transporte real: tiempo máximo de 15 s y `User-Agent: TMC/<versión>`.
pub struct ReqwestTransport {
    client: reqwest::Client,
    client_h1: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, TransportError> {
        let base = || {
            reqwest::Client::builder()
                .timeout(TIMEOUT)
                .user_agent(concat!("TMC/", env!("CARGO_PKG_VERSION")))
        };
        let client = base().build().map_err(map_err)?;
        let client_h1 = base().http1_only().build().map_err(map_err)?;
        Ok(Self { client, client_h1 })
    }
}

/// Interpreta `Retry-After` (solo segundos), `X-Rate-Limit-Remaining` y
/// `X-Rate-Limit-Reset`. Valores no numéricos (p. ej. fecha HTTP) → `None`.
pub(crate) fn reply_headers(
    h: &reqwest::header::HeaderMap,
) -> (Option<u64>, Option<u32>, Option<u64>) {
    fn num<T: std::str::FromStr>(h: &reqwest::header::HeaderMap, name: &str) -> Option<T> {
        h.get(name)?.to_str().ok()?.trim().parse().ok()
    }
    (
        num(h, "retry-after"),
        num(h, "x-rate-limit-remaining"),
        num(h, "x-rate-limit-reset"),
    )
}

fn map_err(e: reqwest::Error) -> TransportError {
    if e.is_timeout() {
        TransportError::Timeout
    } else {
        TransportError::Connect(e.without_url().to_string())
    }
}

async fn finish(req: reqwest::RequestBuilder) -> Result<HttpReply, TransportError> {
    let resp = req.send().await.map_err(map_err)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(map_err)?;
    Ok(HttpReply { status, body })
}

impl Transport for ReqwestTransport {
    fn get(&self, url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        let req = self.client.get(url);
        Box::pin(finish(req))
    }

    fn get_with_timeout(
        &self,
        url: &str,
        timeout: Duration,
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        let req = self.client.get(url).timeout(timeout);
        Box::pin(finish(req))
    }

    fn post_form(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        let req = self.client.post(url).form(form);
        Box::pin(finish(req))
    }

    fn post_json(&self, url: &str, body: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        let req = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_owned());
        Box::pin(finish(req))
    }

    fn get_with_auth(
        &self,
        url: &str,
        token: &str,
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        let req = self
            .client
            .get(url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
        Box::pin(finish(req))
    }

    fn post<'a>(&'a self, req: HttpPost<'a>) -> BoxFut<'a, Result<HttpReplyEx, TransportError>> {
        let client = if req.http1_only {
            &self.client_h1
        } else {
            &self.client
        };
        let mut rb = client
            .post(req.url)
            .header(reqwest::header::CONTENT_TYPE, req.content_type);
        for (k, v) in req.headers {
            rb = rb.header(*k, *v);
        }
        let rb = rb.body(req.body.to_vec());
        Box::pin(async move {
            let resp = rb.send().await.map_err(map_err)?;
            let status = resp.status().as_u16();
            let (retry_after_s, rate_limit_remaining, rate_limit_reset_s) =
                reply_headers(resp.headers());
            let body = resp.text().await.map_err(map_err)?;
            Ok(HttpReplyEx {
                status,
                body,
                retry_after_s,
                rate_limit_remaining,
                rate_limit_reset_s,
            })
        })
    }
}

/// Codifica un componente de URL (todo salvo `A-Za-z0-9-_.~`).
pub(crate) fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reqwest_transport_builds_without_network() {
        assert!(ReqwestTransport::new().is_ok());
    }

    #[test]
    fn url_encode_escapes_spaces_and_unicode() {
        assert_eq!(url_encode("HIP 1727"), "HIP%201727");
        assert_eq!(
            url_encode("Col 359 Sector CG-O d6-71"),
            "Col%20359%20Sector%20CG-O%20d6-71"
        );
        assert_eq!(url_encode("Ñ&"), "%C3%91%26");
    }

    struct OnlyGet;
    impl Transport for OnlyGet {
        fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async {
                Ok(HttpReply {
                    status: 200,
                    body: String::new(),
                })
            })
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async {
                Ok(HttpReply {
                    status: 200,
                    body: String::new(),
                })
            })
        }
    }

    fn hm(pairs: &[(&'static str, &str)]) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, v.parse().unwrap());
        }
        h
    }

    #[test]
    fn reply_headers_parse_retry_after_and_rate_limit() {
        let h = hm(&[
            ("retry-after", "120"),
            ("x-rate-limit-remaining", "7"),
            ("x-rate-limit-reset", "30"),
        ]);
        assert_eq!(reply_headers(&h), (Some(120), Some(7), Some(30)));
    }

    #[test]
    fn reply_headers_ignore_http_date_and_garbage() {
        let h = hm(&[("retry-after", "Wed, 21 Oct 2026 07:28:00 GMT")]);
        assert_eq!(reply_headers(&h), (None, None, None));
        let h = hm(&[("retry-after", "abc"), ("x-rate-limit-remaining", "x")]);
        assert_eq!(reply_headers(&h), (None, None, None));
    }

    #[tokio::test]
    async fn default_post_is_unsupported() {
        let req = HttpPost {
            url: "http://x",
            body: b"",
            content_type: "application/json",
            headers: &[],
            http1_only: false,
        };
        assert_eq!(
            OnlyGet.post(req).await,
            Err(TransportError::Connect("post no soportado".into()))
        );
    }

    #[tokio::test]
    async fn scripted_transport_records_posts() {
        use crate::testing::{RecordedPost, ScriptedTransport};
        let ok = HttpReplyEx {
            status: 200,
            body: "ok".into(),
            retry_after_s: None,
            rate_limit_remaining: None,
            rate_limit_reset_s: None,
        };
        let t = ScriptedTransport::new(vec![Ok(ok.clone())]);
        let r = t
            .post(HttpPost {
                url: "http://x/api",
                body: b"abc",
                content_type: "application/json",
                headers: &[("Content-Encoding", "gzip")],
                http1_only: true,
            })
            .await;
        assert_eq!(r, Ok(ok));
        assert_eq!(t.calls(), 1);
        assert_eq!(
            t.posts()[0],
            RecordedPost {
                url: "http://x/api".into(),
                content_type: "application/json".into(),
                headers: vec![("Content-Encoding".into(), "gzip".into())],
                body: b"abc".to_vec(),
                http1_only: true,
            }
        );
    }
}

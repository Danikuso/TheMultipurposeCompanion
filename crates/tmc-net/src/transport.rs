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
}

const TIMEOUT: Duration = Duration::from_secs(5);

/// Transporte real: tiempo máximo de 5 s y `User-Agent: TMC/<versión>`.
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, TransportError> {
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .user_agent(concat!("TMC/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(map_err)?;
        Ok(Self { client })
    }
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
}

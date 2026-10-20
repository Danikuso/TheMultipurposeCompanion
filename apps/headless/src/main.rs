//! Binario headless de TMC.
#![forbid(unsafe_code)]

mod eval;

use std::path::PathBuf;
use tmc_app::{Config, data_dir, init_logging, start};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut raw = std::env::args().skip(1).peekable();
    if raw.peek().map(String::as_str) == Some("eval") {
        raw.next();
        return eval::run(raw);
    }

    let mut config_file: Option<PathBuf> = None;
    let mut journal_dir: Option<PathBuf> = None;
    let mut ui_dir: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_file = args.next().map(PathBuf::from),
            "--journal-dir" => journal_dir = args.next().map(PathBuf::from),
            "--ui-dir" => ui_dir = args.next().map(PathBuf::from),
            _ => {}
        }
    }

    let mut config = match &config_file {
        Some(path) => Config::load_from_or_default(path),
        None => Config::load_or_default(),
    };
    if journal_dir.is_some() {
        config.journal_dir = journal_dir;
    }
    if ui_dir.is_some() {
        config.ui_dir = ui_dir;
    }

    let log_dir = config.data_dir.clone().unwrap_or_else(data_dir);
    let _guard = init_logging(&log_dir);
    _guard.application_started();

    #[cfg(debug_assertions)]
    let running = {
        let transport = e2e_transport()?;
        let relay = e2e_eddn_relay()?;
        if transport.is_none() && relay.is_none() {
            start(config).await?
        } else {
            let transport = match transport {
                Some(t) => t,
                None => std::sync::Arc::new(
                    tmc_net::ReqwestTransport::new()
                        .map_err(|e| anyhow::anyhow!("no se pudo crear el transporte: {e:?}"))?,
                ),
            };
            tmc_app::start_full(config, transport, None, relay).await?
        }
    };
    #[cfg(not(debug_assertions))]
    let running = start(config).await?;
    println!("local_url: {}", running.server.local_url);
    if let Some(lan_url) = &running.server.lan_url {
        println!("lan_url: {lan_url}");
    }

    tokio::signal::ctrl_c().await?;
    running.server.shutdown();
    Ok(())
}

/// Solo en compilaciones de depuración (pruebas e2e de la UI): con
/// `TMC_E2E_ARDENT_BASE=http://127.0.0.1:<puerto>` las peticiones a Ardent van a ese
/// servidor falso local y cualquier otra petición de red se rechaza.
#[cfg(debug_assertions)]
fn e2e_transport() -> anyhow::Result<Option<std::sync::Arc<dyn tmc_net::Transport>>> {
    use tmc_net::{BoxFut, HttpReply, ReqwestTransport, Transport, TransportError};
    // Solo estos servicios se redirigen al servidor falso local (Ardent, Spansh y EDSM).
    const HOSTS: [&str; 3] = [
        "https://api.ardent-insight.com",
        "https://spansh.co.uk",
        "https://www.edsm.net",
    ];
    const SPANSH: &str = "https://spansh.co.uk";
    struct Redirect {
        base: String,
        inner: ReqwestTransport,
    }
    impl Redirect {
        fn map(&self, url: &str) -> Option<String> {
            HOSTS
                .iter()
                .find_map(|h| url.strip_prefix(h))
                .map(|rest| format!("{}{rest}", self.base))
        }
    }
    impl Transport for Redirect {
        fn get(&self, url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            match self.map(url) {
                Some(target) => self.inner.get(&target),
                None => Box::pin(async { Err(TransportError::Connect("blocked in e2e".into())) }),
            }
        }
        fn post_form(
            &self,
            url: &str,
            form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            match url.strip_prefix(SPANSH) {
                Some(rest) => self.inner.post_form(&format!("{}{rest}", self.base), form),
                None => Box::pin(async { Err(TransportError::Connect("blocked in e2e".into())) }),
            }
        }
    }
    let base = match std::env::var("TMC_E2E_ARDENT_BASE") {
        Ok(b) => b,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(e) => anyhow::bail!("TMC_E2E_ARDENT_BASE inválida: {e}"),
    };
    // Estricto: http://127.0.0.1:<puerto> y nada más (sin usuario, ruta, consulta ni barra).
    let port = base
        .strip_prefix("http://127.0.0.1:")
        .filter(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|p| p.parse::<u16>().ok())
        .filter(|p| *p > 0);
    let Some(port) = port else {
        anyhow::bail!("TMC_E2E_ARDENT_BASE debe ser http://127.0.0.1:<puerto>, no {base:?}");
    };
    let inner = ReqwestTransport::new()
        .map_err(|e| anyhow::anyhow!("no se pudo crear el transporte e2e: {e:?}"))?;
    Ok(Some(std::sync::Arc::new(Redirect {
        base: format!("http://127.0.0.1:{port}"),
        inner,
    })))
}

/// Solo en depuración: `TMC_E2E_EDDN_RELAY=tcp://127.0.0.1:<puerto>` sustituye el relay de EDDN.
/// Si está definida pero no es válida, falla cerrado con un error.
#[cfg(debug_assertions)]
fn e2e_eddn_relay() -> anyhow::Result<Option<String>> {
    match std::env::var("TMC_E2E_EDDN_RELAY") {
        Ok(url) => parse_e2e_relay(&url).map(Some),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(e) => anyhow::bail!("TMC_E2E_EDDN_RELAY inválida: {e}"),
    }
}

/// Estricto: `tcp://127.0.0.1:<puerto>` y nada más.
#[cfg(debug_assertions)]
fn parse_e2e_relay(url: &str) -> anyhow::Result<String> {
    let port = url
        .strip_prefix("tcp://127.0.0.1:")
        .filter(|p| !p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|p| p.parse::<u16>().ok())
        .filter(|p| *p > 0);
    match port {
        Some(port) => Ok(format!("tcp://127.0.0.1:{port}")),
        None => anyhow::bail!("TMC_E2E_EDDN_RELAY debe ser tcp://127.0.0.1:<puerto>, no {url:?}"),
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::parse_e2e_relay;

    #[test]
    fn e2e_relay_override_only_loopback() {
        for bad in [
            "tcp://10.0.0.1:1",
            "tcp://127.0.0.1:",
            "tcp://127.0.0.1:0",
            "tcp://127.0.0.1:5555/x",
            "tcp://127.0.0.1:70000",
            "tcp://127.0.0.1.evil.com:5555",
            "tcp://eddn.edcd.io:9500",
            "",
        ] {
            assert!(parse_e2e_relay(bad).is_err(), "{bad}");
        }
        assert_eq!(
            parse_e2e_relay("tcp://127.0.0.1:5555").unwrap(),
            "tcp://127.0.0.1:5555"
        );
    }
}

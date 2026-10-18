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

    #[cfg(debug_assertions)]
    let running = match e2e_transport() {
        Some(t) => tmc_app::start_with_transport(config, t).await?,
        None => start(config).await?,
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
fn e2e_transport() -> Option<std::sync::Arc<dyn tmc_net::Transport>> {
    use tmc_net::{BoxFut, HttpReply, ReqwestTransport, Transport, TransportError};
    const ARDENT: &str = "https://api.ardent-insight.com";
    struct Redirect {
        base: String,
        inner: ReqwestTransport,
    }
    impl Transport for Redirect {
        fn get(&self, url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            match url.strip_prefix(ARDENT) {
                Some(rest) => self.inner.get(&format!("{}{rest}", self.base)),
                None => Box::pin(async { Err(TransportError::Connect("blocked in e2e".into())) }),
            }
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("blocked in e2e".into())) })
        }
    }
    let base = std::env::var("TMC_E2E_ARDENT_BASE").ok()?;
    if !base.starts_with("http://127.0.0.1:") {
        return None;
    }
    let inner = ReqwestTransport::new().ok()?;
    Some(std::sync::Arc::new(Redirect { base, inner }))
}

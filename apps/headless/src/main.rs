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

    let running = start(config).await?;
    println!("local_url: {}", running.server.local_url);
    if let Some(lan_url) = &running.server.lan_url {
        println!("lan_url: {lan_url}");
    }

    tokio::signal::ctrl_c().await?;
    running.server.shutdown();
    Ok(())
}

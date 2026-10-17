//! Inicialización de logs con rotación diaria.

use std::path::Path;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{Builder, Rotation},
};
use tracing_subscriber::EnvFilter;

pub fn init_logging(data_dir: &Path) -> WorkerGuard {
    let _ = std::fs::create_dir_all(data_dir);
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("tmc")
        .filename_suffix("log")
        .build(data_dir)
        .unwrap_or_else(|_| tracing_appender::rolling::daily(data_dir, "tmc.log"));
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .try_init();
    guard
}

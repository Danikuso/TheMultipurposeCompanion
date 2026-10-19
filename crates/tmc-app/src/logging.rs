//! General application logs and debug-only structured diagnostics.
use std::path::Path;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{Builder, Rotation},
};
use tracing_subscriber::filter::FilterExt;
use tracing_subscriber::{EnvFilter, prelude::*};

#[cfg(debug_assertions)]
mod diagnostics;

/// Keep alive until the application has stopped. Drop drains both writers.
pub struct LoggingGuards {
    #[cfg(debug_assertions)]
    diagnostics: Option<diagnostics::DiagnosticGuard>,
    general: Option<WorkerGuard>,
}

impl LoggingGuards {
    /// Record application startup without including configuration values.
    pub fn application_started(&self) {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "app.start",
            serde_json::json!({"started":true})
        );
    }
}

impl Drop for LoggingGuards {
    fn drop(&mut self) {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            "app.stop",
            serde_json::json!({"stopping":true})
        );
        #[cfg(debug_assertions)]
        drop(self.diagnostics.take());
        drop(self.general.take());
    }
}

pub fn init_logging(data_dir: &Path) -> LoggingGuards {
    let general = Builder::new()
        .rotation(Rotation::DAILY)
        .max_log_files(7)
        .filename_prefix("tmc")
        .filename_suffix("log")
        .build(data_dir);
    let (writer, general) = match general {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                tracing_subscriber::fmt::writer::BoxMakeWriter::new(writer),
                Some(guard),
            )
        }
        Err(_) => {
            eprintln!("TMC logging: general_directory_unavailable");
            (
                tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stderr),
                None,
            )
        }
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // A per-layer filter prevents RUST_LOG=trace from copying diagnostics into AppData.
    let general_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_filter(filter.and(tracing_subscriber::filter::filter_fn(|m| {
            m.target() != "tmc::diagnostics"
        })));
    let registry = tracing_subscriber::registry().with(general_layer);
    #[cfg(debug_assertions)]
    let (registry, diagnostics) = {
        let result = std::env::current_exe()
            .and_then(|exe| diagnostics::directory_for_executable(&exe))
            .and_then(|dir| diagnostics::start_writer(dir, diagnostics::Limits::default()));
        let (layer, guard) = match result {
            Ok((layer, guard)) => (Some(layer), Some(guard)),
            Err(_) => {
                eprintln!("TMC diagnostics: directory_or_worker_unavailable");
                (None, None)
            }
        };
        (
            registry.with(layer.map(|layer| {
                layer.with_filter(tracing_subscriber::filter::filter_fn(|m| {
                    m.target() == "tmc::diagnostics"
                }))
            })),
            guard,
        )
    };
    if registry.try_init().is_err() {
        eprintln!("TMC logging: subscriber_already_initialized");
    }
    LoggingGuards {
        general,
        #[cfg(debug_assertions)]
        diagnostics,
    }
}

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::Rotation,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct ShutdownLogs(Arc<Mutex<Option<WorkerGuard>>>);

impl ShutdownLogs {
    pub fn new(guard: WorkerGuard) -> Self {
        Self(Arc::new(Mutex::new(Some(guard))))
    }

    pub fn flush(&self) {
        let worker = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        // Release the mutex before waiting for the logging worker to finish.
        drop(worker);
    }
}

pub fn file_writer(directory: &Path) -> Result<(NonBlocking, WorkerGuard)> {
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("Sonvail")
        .filename_suffix("log")
        .build(directory)
        .context("Cannot initialize log files")?;
    // Apply backpressure rather than silently discarding diagnostics.
    Ok(
        tracing_appender::non_blocking::NonBlockingBuilder::default()
            .lossy(false)
            .finish(appender),
    )
}

pub fn init(directory: &Path) -> Result<ShutdownLogs> {
    let filter = EnvFilter::builder()
        .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
        .from_env()
        .context("Invalid RUST_LOG filter")?;
    let (writer, guard) = file_writer(directory)?;
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .try_init()
        .context("Cannot initialize tracing")?;
    Ok(ShutdownLogs::new(guard))
}

#[cfg(test)]
mod tests;

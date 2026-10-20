#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod config;
mod logging;
mod ui;

fn main() -> anyhow::Result<()> {
    let paths = config::AppPaths::resolve()?;
    let logs = logging::init(&paths.logs_path)?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Starting Soundora");
    let result = config::ConfigStore::load(paths).map(app::run);
    if let Err(error) = &result {
        tracing::error!(error = %format!("{error:#}"), "Soundora startup failed");
    }
    // GPUI may emit diagnostics while tearing down its windows after on_app_quit.
    // Keep the file worker alive until the entire application loop has returned.
    logs.flush();
    result
}

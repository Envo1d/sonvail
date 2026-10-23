#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod config;
mod library;
mod logging;
mod playback;
#[cfg(target_os = "windows")]
mod tray;
mod ui;
mod updater;

fn main() -> anyhow::Result<()> {
    // Installer hooks must run before GPUI, configuration, logs or SQLite.
    // Only explicitly verified packages may be applied, including after a restart.
    velopack::VelopackApp::build()
        .set_auto_apply_on_startup(false)
        .run();

    let paths = config::AppPaths::resolve()?;
    let logs = logging::init(&paths.logs_path)?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Starting Sonvail");
    let result = config::ConfigStore::load(paths).map(app::run);
    if let Err(error) = &result {
        tracing::error!(error = %format!("{error:#}"), "Sonvail startup failed");
    }
    // GPUI may emit diagnostics while tearing down its windows after on_app_quit.
    // Keep the file worker alive until the entire application loop has returned.
    logs.flush();
    result
}

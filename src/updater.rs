//! Signed Windows package updates. All functions here run on a background executor.
use std::sync::mpsc;

use anyhow::{Context, Result, bail};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions};

mod source;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct PendingUpdate {
    manager: UpdateManager,
    update: UpdateInfo,
}

impl PendingUpdate {
    pub(crate) fn version(&self) -> &str {
        &self.update.TargetFullRelease.Version
    }

    pub(crate) fn notes(&self) -> &str {
        &self.update.TargetFullRelease.NotesMarkdown
    }
}

pub(crate) fn check() -> Result<Option<PendingUpdate>> {
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        bail!("Package updates are supported on Windows x64 only.");
    }
    let manager = UpdateManager::new(
        source::SignedSource,
        Some(UpdateOptions {
            ExplicitChannel: Some("win-x64".into()),
            MaximumDeltasBeforeFallback: -1,
            ..Default::default()
        }),
        None,
    )
    .context("Install Sonvail Setup from GitHub Releases once to enable updates. Development builds cannot use package updates.")?;
    source::require_public_key()?;
    tracing::info!("Checking signed Sonvail update feed");
    match manager
        .check_for_updates()
        .context("Could not check or verify updates. Check your connection and try again.")?
    {
        UpdateCheck::UpdateAvailable(update) => Ok(Some(PendingUpdate {
            manager,
            update: *update,
        })),
        UpdateCheck::NoUpdateAvailable => Ok(None),
        UpdateCheck::RemoteIsEmpty => {
            bail!("No Windows update packages have been published yet. Try again later.")
        }
    }
}

pub(crate) fn install(pending: PendingUpdate, progress: async_channel::Sender<u8>) -> Result<()> {
    let (sender, receiver) = mpsc::channel::<i16>();
    std::thread::scope(|scope| -> Result<()> {
        scope.spawn(move || {
            for percent in receiver {
                // UI notifications must never block the download worker.
                let _ = progress.try_send(percent.clamp(0, 100) as u8);
            }
        });
        pending
            .manager
            .download_updates(&pending.update, Some(sender))
            .context("Could not download the update. You can retry.")?;
        // Velopack trusts an existing cached package: always recheck its signed hash.
        source::verify_cached_package(&pending.update.TargetFullRelease).context(
            "Could not verify the update. Installation was not started; retry the download.",
        )?;
        Ok(())
    })?;
    // Launch first: on failure the player stays open. The independent Velopack
    // process waits for our PID while GPUI shuts down playback and flushes logs.
    pending
        .manager
        .wait_exit_then_apply_updates(&pending.update, false, true, Vec::<String>::new())
        .context("Could not start the updater. Sonvail remains open; you can retry.")?;
    tracing::info!(
        version = pending.version(),
        "Updater launched; preparing graceful shutdown"
    );
    Ok(())
}

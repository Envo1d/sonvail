use std::{fs, io::Write, path::PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct AppConfig {
    pub theme: ThemeMode,
    pub library_folders: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_path: PathBuf,
    pub logs_path: PathBuf,
    pub library_path: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "Envo1d", "Soundora")
            .context("Unable to locate application system folders")?;
        Ok(Self {
            config_path: dirs.config_dir().join("settings.json"),
            logs_path: dirs.data_local_dir().join("logs"),
            library_path: dirs.data_local_dir().join("library.sqlite3"),
        })
    }
}

// GPUI owns this store on its UI thread; mutation requires exclusive access.
// Disk replacement completes before the in-memory snapshot is updated.
pub struct ConfigStore {
    paths: AppPaths,
    settings: AppConfig,
}

impl ConfigStore {
    #[tracing::instrument(skip_all, fields(config_path = %paths.config_path.display()), err)]
    pub fn load(paths: AppPaths) -> Result<Self> {
        let settings = match fs::read(&paths.config_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).with_context(|| {
                format!("Invalid configuration {}", paths.config_path.display())
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let defaults = AppConfig::default();
                Self::write(&paths, &defaults, false)?;
                tracing::info!("Default configuration created");
                defaults
            }
            Err(error) => return Err(error).context("Cannot read configuration"),
        };
        tracing::info!("Configuration loaded");
        Ok(Self { paths, settings })
    }

    pub fn snapshot(&self) -> &AppConfig {
        &self.settings
    }

    pub fn library_path(&self) -> &std::path::Path {
        &self.paths.library_path
    }

    #[tracing::instrument(skip_all, err)]
    pub fn save(&mut self, draft: AppConfig) -> Result<()> {
        if draft == self.settings {
            return Ok(());
        }
        Self::write(&self.paths, &draft, true)?;
        if self.settings.theme != draft.theme {
            tracing::info!(theme = ?draft.theme, "Interface theme changed");
        }
        if self.settings.library_folders != draft.library_folders {
            tracing::info!(
                folder_count = draft.library_folders.len(),
                "Library folders changed"
            );
        }
        self.settings = draft;
        tracing::info!("Configuration saved");
        Ok(())
    }

    fn write(paths: &AppPaths, settings: &AppConfig, replace: bool) -> Result<()> {
        let parent = paths
            .config_path
            .parent()
            .context("Invalid configuration path")?;
        fs::create_dir_all(parent).context("Cannot create config directory")?;
        let mut file = tempfile::NamedTempFile::new_in(parent)
            .context("Cannot create temporary configuration")?;
        let json = serde_json::to_vec_pretty(settings).context("Cannot serialize configuration")?;
        file.write_all(&json)
            .context("Cannot write configuration")?;
        file.as_file()
            .sync_all()
            .context("Cannot flush configuration")?;
        if replace {
            file.persist(&paths.config_path)
        } else {
            file.persist_noclobber(&paths.config_path)
        }
        .map_err(|error| error.error)
        .context("Cannot commit configuration")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

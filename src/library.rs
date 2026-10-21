use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

pub mod metadata;
pub mod service;
mod store;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub metadata: metadata::Metadata,
}

impl Track {
    pub fn from_path(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Self {
            path,
            title,
            metadata: metadata::Metadata::default(),
        }
    }

    pub fn artist(&self) -> String {
        if self.metadata.artists.is_empty() {
            "Unknown artist".into()
        } else {
            self.metadata.artists.join(", ")
        }
    }

    pub fn album(&self) -> &str {
        self.metadata.album.as_deref().unwrap_or("Unknown album")
    }
}

#[derive(Default)]
pub struct ScanResult {
    pub tracks: Vec<Track>,
    pub errors: usize,
    pub failed_paths: Vec<PathBuf>,
}

/// A standalone scan, also used by filesystem tests without a persistent cache.
#[cfg(test)]
pub fn scan(folders: &[PathBuf]) -> ScanResult {
    scan_cached(folders, &HashMap::new(), None)
}

pub(super) fn scan_cached(
    folders: &[PathBuf],
    cached: &HashMap<PathBuf, Track>,
    artwork: Option<&Path>,
) -> ScanResult {
    let mut result = ScanResult::default();
    let mut pending = folders.to_vec();
    let mut directories = HashSet::new();
    let mut files = HashSet::new();
    while let Some(directory) = pending.pop() {
        let canonical = match directory.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                record_error(&mut result, &directory, error);
                continue;
            }
        };
        if !directories.insert(canonical.clone()) {
            continue;
        }
        let entries = match fs::read_dir(&canonical) {
            Ok(entries) => entries,
            Err(error) => {
                record_error(&mut result, &directory, error);
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    record_error(&mut result, &directory, error);
                    continue;
                }
            };
            let path = entry.path();
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(error) => {
                    record_error(&mut result, &path, error);
                    continue;
                }
            };
            // Do not follow links inside sources, including links forming cycles.
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() && supported(&path) && files.insert(path.clone()) {
                match metadata::read_track(&path, cached.get(&path), artwork) {
                    Ok(track) => result.tracks.push(track),
                    Err(error) => record_error(&mut result, &path, error),
                }
            }
        }
    }
    result
        .tracks
        .sort_by_cached_key(|track| (track.title.to_lowercase(), track.path.clone()));
    tracing::info!(
        tracks = result.tracks.len(),
        errors = result.errors,
        "Library scan finished"
    );
    result
}

fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["mp3", "wav", "flac", "ogg"]
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

fn record_error(result: &mut ScanResult, path: &Path, error: std::io::Error) {
    result.errors += 1;
    result.failed_paths.push(path.to_owned());
    tracing::warn!(path = %path.display(), %error, "Could not scan library entry");
}

#[cfg(test)]
mod tests;

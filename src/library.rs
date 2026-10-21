use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub mod service;
mod store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
}

#[derive(Default)]
pub struct ScanResult {
    pub tracks: Vec<Track>,
    pub errors: usize,
    pub failed_paths: Vec<PathBuf>,
}

/// Discovers files by extension; decoding and tags belong to later stages.
pub fn scan(folders: &[PathBuf]) -> ScanResult {
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
                let title = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                result.tracks.push(Track { path, title });
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

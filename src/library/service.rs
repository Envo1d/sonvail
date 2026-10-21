use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::{
    ScanResult, Track, scan,
    store::{Sources, Store, Tracks},
};

const DEBOUNCE: Duration = Duration::from_millis(350);
const MAX_DELAY: Duration = Duration::from_secs(2);
const RETRY: Duration = Duration::from_secs(3);

pub struct Snapshot {
    pub revision: u64,
    pub tracks: Vec<Track>,
    pub errors: usize,
    pub warning: Option<String>,
    pub scanning: bool,
}

pub enum Update {
    Snapshot(Snapshot),
    Failed { revision: u64, message: String },
}

enum Command {
    Sources {
        revision: u64,
        folders: Vec<PathBuf>,
    },
    Event(notify::Result<Event>),
    Shutdown,
}

pub struct LibraryService {
    commands: mpsc::Sender<Command>,
    stop: Arc<AtomicBool>,
}

impl LibraryService {
    pub fn start(
        database: PathBuf,
        folders: Vec<PathBuf>,
    ) -> Result<(Self, async_channel::Receiver<Update>)> {
        let (commands, receiver) = mpsc::channel();
        let (updates, snapshots) = async_channel::unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let events = commands.clone();
        thread::Builder::new()
            .name("soundora-library".into())
            .spawn(move || {
                let mut revision = 0;
                if let Err(error) = run(
                    &database,
                    folders,
                    events,
                    receiver,
                    &updates,
                    &worker_stop,
                    &mut revision,
                ) {
                    tracing::error!(error = %format!("{error:#}"), "Library worker failed");
                    let _ = updates.send_blocking(Update::Failed {
                        revision,
                        message: format!(
                            "Library unavailable: {error:#}. Check the logs and restart Soundora."
                        ),
                    });
                }
            })
            .context("Cannot start library worker")?;
        Ok((Self { commands, stop }, snapshots))
    }

    pub fn set_sources(&self, revision: u64, folders: Vec<PathBuf>) -> Result<()> {
        self.commands
            .send(Command::Sources { revision, folders })
            .context("Library worker has stopped")
    }
}

impl Drop for LibraryService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.commands.send(Command::Shutdown);
    }
}

fn resolve_sources(folders: &[PathBuf], previous: &Sources) -> Sources {
    folders
        .iter()
        .map(|folder| {
            let canonical = folder.canonicalize().unwrap_or_else(|_| {
                previous
                    .get(folder)
                    .cloned()
                    .unwrap_or_else(|| folder.clone())
            });
            (folder.clone(), canonical)
        })
        .collect()
}

fn retain_sources(tracks: &mut Tracks, sources: &Sources) {
    tracks.retain(|path, _| sources.values().any(|root| path.starts_with(root)));
}

/// Replace only successfully read parts of a scope. An offline/unreadable
/// subtree keeps its last known records until it can be reconciled again.
fn reconcile(tracks: &mut Tracks, scope: &Path, result: ScanResult) -> usize {
    tracks.retain(|path, _| {
        !path.starts_with(scope)
            || result
                .failed_paths
                .iter()
                .any(|failed| path.starts_with(failed))
    });
    for track in result.tracks {
        tracks.insert(track.path.clone(), track);
    }
    result.errors
}

fn publish(
    updates: &async_channel::Sender<Update>,
    tracks: &Tracks,
    revision: u64,
    errors: usize,
    warning: Option<String>,
    scanning: bool,
) {
    let mut tracks: Vec<_> = tracks.values().cloned().collect();
    tracks.sort_by_cached_key(|track| (track.title.to_lowercase(), track.path.clone()));
    let _ = updates.send_blocking(Update::Snapshot(Snapshot {
        revision,
        tracks,
        errors,
        warning,
        scanning,
    }));
}

struct Watches {
    watcher: Option<RecommendedWatcher>,
    paths: HashMap<PathBuf, RecursiveMode>,
}

impl Watches {
    fn new() -> Self {
        Self {
            watcher: None,
            paths: HashMap::new(),
        }
    }

    fn refresh(
        &mut self,
        sources: &Sources,
        events: &mpsc::Sender<Command>,
    ) -> (bool, Option<String>) {
        if self.watcher.is_none() {
            let events = events.clone();
            match RecommendedWatcher::new(
                move |event| {
                    let _ = events.send(Command::Event(event));
                },
                Config::default().with_follow_symlinks(false),
            ) {
                Ok(watcher) => self.watcher = Some(watcher),
                Err(error) => {
                    tracing::warn!(%error, "Cannot initialize library watcher");
                    return (false, Some("File watching is unavailable; Soundora will retry automatically. Check the logs.".into()));
                }
            }
        }
        let watcher = self.watcher.as_mut().unwrap();
        let mut desired = HashMap::new();
        for root in sources.values() {
            // Watching the parent also detects deletion/recreation of a source.
            // For an offline source, watch the nearest existing ancestor and
            // advance the watch when missing intermediate directories appear.
            if let Some(parent) = root.ancestors().skip(1).find(|parent| parent.is_dir()) {
                desired
                    .entry(parent.to_owned())
                    .or_insert(RecursiveMode::NonRecursive);
            }
            if root.is_dir() {
                desired.insert(root.clone(), RecursiveMode::Recursive);
            }
        }
        let obsolete: Vec<_> = self
            .paths
            .iter()
            .filter(|(path, mode)| desired.get(*path) != Some(*mode) || !path.is_dir())
            .map(|(path, _)| path.clone())
            .collect();
        for path in obsolete {
            let _ = watcher.unwatch(&path);
            self.paths.remove(&path);
        }
        let mut recovered = false;
        let mut errors = 0;
        for (path, mode) in desired {
            if self.paths.contains_key(&path) {
                continue;
            }
            match watcher.watch(&path, mode) {
                Ok(()) => {
                    self.paths.insert(path, mode);
                    recovered = true;
                }
                Err(error) => {
                    errors += 1;
                    tracing::warn!(path = %path.display(), %error, "Cannot watch library folder");
                }
            }
        }
        (recovered, (errors > 0).then(|| "Some folders cannot be watched; Soundora will retry automatically. Check the logs.".into()))
    }
}

// notify may use ordinary Windows paths while the index uses verbatim paths.
// Resolve the nearest existing ancestor so removed paths normalize as well.
fn normalize(path: &Path) -> PathBuf {
    let mut suffix = Vec::new();
    let mut current = path;
    loop {
        if let Ok(mut canonical) = current.canonicalize() {
            for part in suffix.iter().rev() {
                canonical.push(part);
            }
            return canonical;
        }
        match (current.file_name(), current.parent()) {
            (Some(name), Some(parent)) => {
                suffix.push(name.to_owned());
                current = parent;
            }
            _ => return path.to_owned(),
        }
    }
}

fn event_scopes(event: &Event, sources: &Sources) -> Vec<PathBuf> {
    if matches!(event.kind, EventKind::Access(_)) {
        return Vec::new();
    }
    let mut scopes = Vec::new();
    for path in &event.paths {
        let path = normalize(path);
        for root in sources.values() {
            if root.starts_with(&path) {
                scopes.push(root.clone());
            } else if path.starts_with(root) {
                // Refresh the parent for file and removed-directory events.
                let scope = if path.is_dir() {
                    path.clone()
                } else {
                    path.parent().unwrap_or(root).to_owned()
                };
                scopes.push(scope);
            }
        }
    }
    scopes
}

fn apply_confirmed_removals(tracks: &mut Tracks, event: &Event) {
    use notify::event::ModifyKind;
    if !matches!(
        event.kind,
        EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
    ) {
        return;
    }
    for path in &event.paths {
        // A missing file with an accessible parent is a confirmed removal.
        // An unavailable drive/parent is not evidence that its files were deleted.
        if path.try_exists().is_ok_and(|exists| !exists)
            && path.parent().is_some_and(|parent| parent.is_dir())
        {
            let path = normalize(path);
            tracks.retain(|existing, _| !existing.starts_with(&path));
        }
    }
}

fn compact_scopes(mut scopes: Vec<PathBuf>) -> Vec<PathBuf> {
    scopes.sort();
    let mut compact: Vec<PathBuf> = Vec::new();
    for scope in scopes {
        if !compact.iter().any(|parent| scope.starts_with(parent)) {
            compact.push(scope);
        }
    }
    compact
}

fn run(
    database: &Path,
    mut folders: Vec<PathBuf>,
    events: mpsc::Sender<Command>,
    commands: mpsc::Receiver<Command>,
    updates: &async_channel::Sender<Update>,
    stop: &AtomicBool,
    revision: &mut u64,
) -> Result<()> {
    let mut store = Store::open(database)?;
    let (previous, mut tracks) = store.load()?;
    let mut sources = resolve_sources(&folders, &previous);
    retain_sources(&mut tracks, &sources);
    publish(updates, &tracks, *revision, 0, None, true);
    let mut watches = Watches::new();
    let (_, mut warning) = watches.refresh(&sources, &events);
    // Install watches before scanning so changes during the initial walk queue.
    let mut pending: Vec<_> = sources.values().cloned().collect();
    let mut deadline = Instant::now();
    let mut first_event = Instant::now();
    let mut retry_at = Instant::now() + RETRY;
    let mut needs_save = true;
    let mut scan_errors: HashMap<PathBuf, usize> = HashMap::new();
    let mut database_dirty = false;
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if needs_save && Instant::now() >= deadline {
            for scope in compact_scopes(std::mem::take(&mut pending)) {
                if stop.load(Ordering::Acquire) {
                    return Ok(());
                }
                let errors = reconcile(&mut tracks, &scope, scan(std::slice::from_ref(&scope)));
                scan_errors.retain(|path, _| !path.starts_with(&scope));
                if errors > 0 {
                    scan_errors.insert(scope, errors);
                }
            }
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            retain_sources(&mut tracks, &sources);
            scan_errors.retain(|path, _| sources.values().any(|root| path.starts_with(root)));
            let message = match store.save(&sources, &tracks) {
                Ok(()) => {
                    database_dirty = false;
                    warning.clone()
                }
                Err(error) => {
                    tracing::error!(error = %format!("{error:#}"), "Cannot persist library");
                    retry_at = Instant::now() + RETRY;
                    database_dirty = true;
                    Some("Library changes could not be saved. Soundora will retry automatically. Check the logs.".into())
                }
            };
            publish(
                updates,
                &tracks,
                *revision,
                scan_errors.values().sum(),
                message,
                false,
            );
            needs_save = false;
        }
        match commands.recv_timeout(Duration::from_millis(100)) {
            Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Ok(Command::Sources {
                revision: next,
                folders: next_folders,
            }) => {
                *revision = next;
                folders = next_folders;
                sources = resolve_sources(&folders, &sources);
                retain_sources(&mut tracks, &sources);
                let (_, next_warning) = watches.refresh(&sources, &events);
                warning = next_warning;
                pending = sources.values().cloned().collect();
                deadline = Instant::now();
                needs_save = true;
                scan_errors.clear();
                publish(updates, &tracks, *revision, 0, warning.clone(), true);
            }
            Ok(Command::Event(event)) => {
                let scopes = match event {
                    Ok(event) if event.need_rescan() => sources.values().cloned().collect(),
                    Ok(event) => {
                        apply_confirmed_removals(&mut tracks, &event);
                        // A watch belongs to the old directory object after a
                        // source is replaced, even if its path already exists again.
                        if matches!(
                            event.kind,
                            EventKind::Create(_)
                                | EventKind::Remove(_)
                                | EventKind::Modify(notify::event::ModifyKind::Name(_))
                        ) && event
                            .paths
                            .iter()
                            .any(|path| sources.values().any(|root| *root == normalize(path)))
                        {
                            watches = Watches::new();
                        }
                        event_scopes(&event, &sources)
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Library watcher reported an error");
                        // Reattach on the next retry, then reconcile any missed events.
                        watches = Watches::new();
                        warning = Some(
                            "File watching was interrupted; Soundora will reconnect automatically."
                                .into(),
                        );
                        sources.values().cloned().collect()
                    }
                };
                if !scopes.is_empty() {
                    if !needs_save {
                        first_event = Instant::now();
                    }
                    pending.extend(scopes);
                    // A bound on debounce delay avoids starvation while copying.
                    deadline = (Instant::now() + DEBOUNCE).min(first_event + MAX_DELAY);
                    needs_save = true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if Instant::now() >= retry_at {
            let next_sources = resolve_sources(&folders, &sources);
            let changed = next_sources != sources;
            sources = next_sources;
            let (recovered, next_warning) = watches.refresh(&sources, &events);
            let status_changed = warning != next_warning;
            warning = next_warning;
            if recovered || changed {
                pending.extend(sources.values().cloned());
                deadline = Instant::now();
                needs_save = true;
            } else if status_changed && !database_dirty {
                publish(
                    updates,
                    &tracks,
                    *revision,
                    scan_errors.values().sum(),
                    warning.clone(),
                    false,
                );
            }
            // Retry persistence even when no further filesystem events arrive.
            if !needs_save && database_dirty {
                match store.save(&sources, &tracks) {
                    Ok(()) => {
                        database_dirty = false;
                        publish(
                            updates,
                            &tracks,
                            *revision,
                            scan_errors.values().sum(),
                            warning.clone(),
                            false,
                        );
                    }
                    Err(error) => tracing::warn!(%error, "Library database retry failed"),
                }
            }
            retry_at = Instant::now() + RETRY;
        }
    }
}

#[cfg(test)]
mod tests;

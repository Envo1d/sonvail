mod output;
mod queue;
mod source;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::library::Track;
use output::{LoadError, Output, RodioOutput};
use queue::Queue;
pub use queue::Repeat;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
    Recovering,
}

#[derive(Clone)]
pub struct Snapshot {
    pub tracks: Arc<Vec<Track>>,
    pub order: Arc<Vec<usize>>,
    pub current: Option<usize>,
    pub status: Status,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: f32,
    pub repeat: Repeat,
    pub shuffle: bool,
    pub recovery_paused: bool,
    pub error: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            tracks: Arc::default(),
            order: Arc::default(),
            current: None,
            status: Status::Stopped,
            position: Duration::ZERO,
            duration: None,
            volume: 0.7,
            repeat: Repeat::Off,
            shuffle: false,
            recovery_paused: false,
            error: None,
        }
    }
}

impl Snapshot {
    pub fn track(&self) -> Option<&Track> {
        self.current.and_then(|index| self.tracks.get(index))
    }
}

pub enum Command {
    Play {
        tracks: Arc<Vec<Track>>,
        index: usize,
    },
    Select(usize),
    Toggle,
    Stop,
    Next,
    Previous,
    Seek(Duration),
    Volume(f32),
    Repeat(Repeat),
    Shuffle(bool),
    RefreshLibrary(Arc<Vec<Track>>),
}

pub struct PlaybackService {
    commands: mpsc::SyncSender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    stop: Arc<AtomicBool>,
    finished: async_channel::Receiver<()>,
}

impl PlaybackService {
    pub fn start() -> Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(64);
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let shared = snapshot.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (done, finished) = async_channel::bounded(1);
        thread::Builder::new()
            .name("sonvail-playback".into())
            .spawn(move || {
                let mut engine = Engine::new(RodioOutput::default());
                while !stopped.load(Ordering::Acquire) {
                    match receiver.recv_timeout(Duration::from_millis(50)) {
                        Ok(command) => {
                            engine.handle(command);
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    engine.tick();
                    // Snapshot contains Arc queues; locks cover only a small clone.
                    *shared.lock().unwrap_or_else(|error| error.into_inner()) = engine.snapshot();
                }
                engine.output.stop();
                drop(engine);
                let _ = done.send_blocking(());
            })
            .context("Cannot start playback worker")?;
        Ok(Self {
            commands,
            snapshot,
            stop,
            finished,
        })
    }

    pub fn send(&self, command: Command) -> Result<()> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => anyhow::anyhow!("Playback is busy; try again"),
                mpsc::TrySendError::Disconnected(_) => {
                    anyhow::anyhow!("Playback worker has stopped; restart Sonvail")
                }
            })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn shutdown(&self) -> async_channel::Receiver<()> {
        self.stop.store(true, Ordering::Release);
        self.finished.clone()
    }
}

impl Drop for PlaybackService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

struct Engine<B: Output> {
    queue: Queue,
    state: Snapshot,
    output: B,
    recovery_paused: bool,
    retry_at: Instant,
}

impl<B: Output> Engine<B> {
    fn new(output: B) -> Self {
        Self {
            queue: Queue::default(),
            state: Snapshot::default(),
            output,
            recovery_paused: false,
            retry_at: Instant::now(),
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            tracks: self.queue.tracks.clone(),
            order: self.queue.order.clone(),
            current: self.queue.current(),
            repeat: self.queue.repeat,
            shuffle: self.queue.shuffle,
            recovery_paused: self.recovery_paused,
            ..self.state.clone()
        }
    }

    fn load_current(&mut self, paused: bool) {
        self.output.stop();
        self.state.position = Duration::ZERO;
        self.state.duration = None;
        self.state.status = Status::Loading;
        self.state.error = None;
        // A corrupt playlist must not spin indefinitely, even with repeat enabled.
        for _ in 0..self.queue.tracks.len() {
            let Some(index) = self.queue.current() else {
                break;
            };
            let track = &self.queue.tracks[index];
            self.state.duration = track.metadata.duration_ms.map(Duration::from_millis);
            match self
                .output
                .load(track, Duration::ZERO, paused, self.state.volume)
            {
                Ok(duration) => {
                    self.state.duration = duration;
                    self.state.status = if paused {
                        Status::Paused
                    } else {
                        Status::Playing
                    };
                    return;
                }
                Err(LoadError::Device(error)) => {
                    self.recover(format!("Audio output unavailable: {error:#}"), paused);
                    return;
                }
                Err(LoadError::Track(error)) => {
                    tracing::warn!(path = %track.path.display(), error = %format!("{error:#}"), "Skipping unreadable track");
                    self.state.error = Some(format!("Skipped {}: {error:#}", track.title));
                    if self.queue.advance(false).is_none() {
                        break;
                    }
                }
            }
        }
        self.state.status = Status::Stopped;
    }

    fn recover(&mut self, error: String, paused: bool) {
        tracing::warn!(%error, "Playback output interrupted");
        self.state.status = Status::Recovering;
        self.state.error = Some(error);
        self.recovery_paused = paused;
        self.retry_at = Instant::now() + Duration::from_secs(2);
    }

    fn seek(&mut self, target: Duration) {
        let Some(index) = self.queue.current() else {
            return;
        };
        if self.state.status == Status::Stopped {
            return;
        }
        let target = self.state.duration.map_or(target, |duration| {
            target.min(duration.saturating_sub(Duration::from_millis(1)))
        });
        let paused = self.state.status == Status::Paused
            || (self.state.status == Status::Recovering && self.recovery_paused);
        match self
            .output
            .load(&self.queue.tracks[index], target, paused, self.state.volume)
        {
            Ok(duration) => {
                self.state.position = target;
                self.state.duration = duration;
                self.state.status = if paused {
                    Status::Paused
                } else {
                    Status::Playing
                };
                self.state.error = None;
            }
            Err(LoadError::Track(error)) => {
                self.state.error = Some(format!("Cannot seek: {error:#}"))
            }
            Err(LoadError::Device(error)) => {
                self.recover(format!("Audio output unavailable: {error:#}"), paused)
            }
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Play { tracks, index } => {
                self.output.stop();
                if self.queue.replace(tracks, index) {
                    self.load_current(false);
                } else {
                    self.state = Snapshot {
                        volume: self.state.volume,
                        ..Snapshot::default()
                    };
                }
            }
            Command::Select(index) => {
                if let Some(cursor) = self.queue.order.iter().position(|item| *item == index) {
                    self.queue.cursor = Some(cursor);
                    self.load_current(false);
                }
            }
            Command::Toggle => match self.state.status {
                Status::Playing => {
                    self.output.pause(true);
                    self.state.status = Status::Paused;
                }
                Status::Paused => {
                    self.output.pause(false);
                    self.state.status = Status::Playing;
                }
                Status::Stopped if self.queue.current().is_some() => self.load_current(false),
                Status::Recovering => {
                    self.recovery_paused = !self.recovery_paused;
                }
                _ => {}
            },
            Command::Stop => {
                self.output.stop();
                self.state.status = Status::Stopped;
                self.state.position = Duration::ZERO;
                self.state.error = None;
            }
            Command::Next => {
                let paused = self.state.status == Status::Paused
                    || (self.state.status == Status::Recovering && self.recovery_paused);
                if self.queue.advance(false).is_some() {
                    self.load_current(paused);
                } else {
                    self.output.stop();
                    self.state.status = Status::Stopped;
                    self.state.position = Duration::ZERO;
                }
            }
            Command::Previous => {
                let paused = self.state.status == Status::Paused
                    || (self.state.status == Status::Recovering && self.recovery_paused);
                if self.state.position > Duration::from_secs(3) {
                    self.seek(Duration::ZERO);
                } else if self.queue.previous().is_some() {
                    self.load_current(paused);
                }
            }
            Command::Seek(target) => self.seek(target),
            Command::Volume(volume) if volume.is_finite() => {
                self.state.volume = volume.clamp(0.0, 1.0);
                self.output.volume(self.state.volume);
            }
            Command::Volume(_) => {}
            Command::Repeat(repeat) => self.queue.repeat = repeat,
            Command::Shuffle(shuffle) => self.queue.set_shuffle(shuffle),
            Command::RefreshLibrary(tracks) => {
                let indexed: std::collections::HashMap<_, _> =
                    tracks.iter().map(|track| (&track.path, track)).collect();
                let changed = self.queue.tracks.iter().any(|track| {
                    indexed
                        .get(&track.path)
                        .is_some_and(|updated| *updated != track)
                });
                if changed {
                    let queue = Arc::make_mut(&mut self.queue.tracks);
                    for track in queue {
                        if let Some(updated) = indexed.get(&track.path) {
                            *track = (*updated).clone();
                        }
                    }
                }
            }
        }
    }

    fn tick(&mut self) {
        if matches!(self.state.status, Status::Playing | Status::Paused) {
            self.state.position = self.output.position();
            if let Some(duration) = self.state.duration {
                self.state.position = self.state.position.min(duration);
            }
            if let Some(error) = self.output.error() {
                self.recover(error, self.state.status == Status::Paused);
            } else if self.state.status == Status::Playing && self.output.finished() {
                let truncated = self.state.duration.is_some_and(|duration| {
                    self.state.position + Duration::from_secs(2) < duration
                });
                if truncated {
                    tracing::warn!("Decoder ended before the expected duration; skipping track");
                }
                if self.queue.advance(!truncated).is_some() {
                    self.load_current(false);
                } else {
                    self.output.stop();
                    self.state.status = Status::Stopped;
                }
            }
        }
        if self.state.status == Status::Recovering && Instant::now() >= self.retry_at {
            let Some(index) = self.queue.current() else {
                self.state.status = Status::Stopped;
                return;
            };
            match self.output.load(
                &self.queue.tracks[index],
                self.state.position,
                self.recovery_paused,
                self.state.volume,
            ) {
                Ok(duration) => {
                    self.state.duration = duration;
                    self.state.status = if self.recovery_paused {
                        Status::Paused
                    } else {
                        Status::Playing
                    };
                    self.state.error = None;
                }
                Err(LoadError::Device(_)) => {
                    self.retry_at = Instant::now() + Duration::from_secs(2)
                }
                Err(LoadError::Track(error)) => {
                    self.state.error = Some(format!("Cannot resume track: {error:#}"));
                    if self.queue.advance(false).is_some() {
                        self.load_current(self.recovery_paused);
                    } else {
                        self.state.status = Status::Stopped;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

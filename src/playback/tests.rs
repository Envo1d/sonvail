use super::*;
use std::collections::HashSet;

#[derive(Default)]
struct FakeOutput {
    attempts: Vec<String>,
    broken: HashSet<String>,
    device_down: bool,
    position: Duration,
    paused: bool,
    ended: bool,
    volume: f32,
    equalizer: crate::equalizer::EqualizerSettings,
    stream_error: Option<String>,
}

impl Output for FakeOutput {
    fn load(
        &mut self,
        track: &Track,
        offset: Duration,
        paused: bool,
        volume: f32,
    ) -> Result<Option<Duration>, LoadError> {
        self.attempts.push(track.title.clone());
        if self.broken.contains(&track.title) {
            return Err(LoadError::Track(anyhow::anyhow!("bad file")));
        }
        if self.device_down {
            return Err(LoadError::Device(anyhow::anyhow!("device disconnected")));
        }
        self.position = offset;
        self.paused = paused;
        self.volume = volume;
        self.ended = false;
        Ok(Some(Duration::from_secs(60)))
    }
    fn equalizer(&mut self, settings: crate::equalizer::EqualizerSettings) {
        self.equalizer = settings;
    }
    fn pause(&self, _: bool) {}
    fn volume(&self, _: f32) {}
    fn stop(&mut self) {
        self.ended = false;
    }
    fn position(&self) -> Duration {
        self.position
    }
    fn finished(&self) -> bool {
        self.ended
    }
    fn error(&mut self) -> Option<String> {
        self.stream_error.take()
    }
}

fn playlist() -> Arc<Vec<Track>> {
    Arc::new(
        ["first", "second", "third"]
            .map(|name| Track::from_path(format!("{name}.wav").into()))
            .to_vec(),
    )
}

#[test]
fn equalizer_updates_do_not_restart_or_change_playback_and_survive_transitions() {
    let mut engine = Engine::new(FakeOutput::default());
    let settings = crate::equalizer::EqualizerSettings {
        enabled: true,
        gains: [-12, 0, 0, 0, 6, 0, 0, 0, 12],
    };
    engine.handle(Command::Equalizer(settings));
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    engine.handle(Command::Toggle);
    let attempts = engine.output.attempts.len();
    engine.handle(Command::Equalizer(settings));
    assert_eq!(engine.output.attempts.len(), attempts);
    assert_eq!(engine.state.status, Status::Paused);
    engine.handle(Command::Seek(Duration::from_secs(10)));
    engine.handle(Command::Next);
    engine.handle(Command::Stop);
    engine.handle(Command::Play {
        tracks: Arc::new(vec![]),
        index: 0,
    });
    assert_eq!(engine.output.equalizer, settings);
}

#[test]
fn pause_resume_seek_skip_previous_stop_and_empty_queue() {
    let mut engine = Engine::new(FakeOutput::default());
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    assert_eq!(engine.state.status, Status::Playing);
    engine.handle(Command::Toggle);
    assert_eq!(engine.state.status, Status::Paused);
    engine.handle(Command::Seek(Duration::from_secs(20)));
    assert!(engine.output.paused);
    assert_eq!(engine.state.position, Duration::from_secs(20));
    engine.handle(Command::Toggle);
    engine.handle(Command::Next);
    assert_eq!(engine.queue.current(), Some(1));
    engine.handle(Command::Previous);
    assert_eq!(engine.queue.current(), Some(0));
    engine.handle(Command::Stop);
    assert_eq!(engine.state.status, Status::Stopped);
    assert_eq!(engine.state.position, Duration::ZERO);
    engine.handle(Command::Play {
        tracks: Arc::new(vec![]),
        index: 0,
    });
    assert_eq!(engine.snapshot().current, None);
    engine.handle(Command::Next);
    engine.handle(Command::Previous);
    engine.handle(Command::Toggle);
    assert_eq!(engine.state.status, Status::Stopped);
}

#[test]
fn skips_bad_files_and_bounds_retries_when_every_file_is_bad() {
    let mut engine = Engine::new(FakeOutput::default());
    engine.output.broken.insert("first".into());
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    assert_eq!(engine.queue.current(), Some(1));
    assert_eq!(engine.state.status, Status::Playing);
    engine.handle(Command::Repeat(Repeat::All));
    engine
        .output
        .broken
        .extend(["second".into(), "third".into()]);
    let before = engine.output.attempts.len();
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    assert_eq!(engine.state.status, Status::Stopped);
    assert_eq!(engine.output.attempts.len() - before, 3);
    assert!(engine.state.error.is_some());
}

#[test]
fn natural_end_obeys_repeat_but_manual_skip_advances() {
    let mut engine = Engine::new(FakeOutput::default());
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    engine.handle(Command::Repeat(Repeat::One));
    engine.output.position = Duration::from_secs(60);
    engine.output.ended = true;
    engine.tick();
    assert_eq!(engine.queue.current(), Some(0));
    engine.handle(Command::Next);
    assert_eq!(engine.queue.current(), Some(1));
    engine.handle(Command::Volume(4.0));
    assert_eq!(engine.state.volume, 1.0);
    engine.handle(Command::Volume(f32::NAN));
    assert_eq!(engine.state.volume, 1.0);
}

#[test]
fn recovers_device_at_saved_position_and_preserves_pause() {
    let mut engine = Engine::new(FakeOutput::default());
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    engine.handle(Command::Toggle);
    engine.output.position = Duration::from_secs(12);
    engine.output.stream_error = Some("device gone".into());
    engine.output.device_down = true;
    engine.tick();
    assert_eq!(engine.state.status, Status::Recovering);
    assert_eq!(engine.state.position, Duration::from_secs(12));
    engine.output.device_down = false;
    engine.retry_at = Instant::now();
    engine.tick();
    assert_eq!(engine.state.status, Status::Paused);
    assert_eq!(engine.output.position, Duration::from_secs(12));
    assert!(engine.output.paused);
}

#[test]
fn metadata_refresh_preserves_queue_and_playback_and_paused_skip_stays_paused() {
    let mut engine = Engine::new(FakeOutput::default());
    engine.handle(Command::Play {
        tracks: playlist(),
        index: 0,
    });
    engine.handle(Command::Toggle);
    engine.handle(Command::Next);
    assert_eq!(engine.state.status, Status::Paused);
    let mut tracks = (*playlist()).clone();
    tracks[1].title = "Updated from tags".into();
    engine.handle(Command::RefreshLibrary(Arc::new(tracks)));
    assert_eq!(engine.queue.current(), Some(1));
    assert_eq!(
        engine.snapshot().track().unwrap().title,
        "Updated from tags"
    );
    assert_eq!(engine.state.status, Status::Paused);
    assert_eq!(engine.output.attempts.len(), 2);
}

#[test]
fn idle_service_shuts_down_without_initializing_an_audio_device() {
    let service = PlaybackService::start().unwrap();
    let completion = service.shutdown();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if completion.try_recv().is_ok() {
            break;
        }
        assert!(Instant::now() < deadline, "playback worker did not stop");
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(service.snapshot().status, Status::Stopped);
    assert!(service.send(Command::Stop).is_err());
}

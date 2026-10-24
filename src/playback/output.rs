use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};

use crate::{equalizer::EqualizerSettings, library::Track};

use super::source::BufferedAudio;

#[derive(Debug)]
pub(super) enum LoadError {
    Track(anyhow::Error),
    Device(anyhow::Error),
}

pub(super) trait Output {
    fn load(
        &mut self,
        track: &Track,
        offset: Duration,
        paused: bool,
        volume: f32,
    ) -> Result<Option<Duration>, LoadError>;
    fn equalizer(&mut self, settings: EqualizerSettings);
    fn pause(&self, paused: bool);
    fn volume(&self, volume: f32);
    fn stop(&mut self);
    fn position(&self) -> Duration;
    fn finished(&self) -> bool;
    fn error(&mut self) -> Option<String>;
}

#[derive(Default)]
pub(super) struct RodioOutput {
    equalizer: Arc<Mutex<EqualizerSettings>>,
    player: Option<Player>,
    stream: Option<MixerDeviceSink>,
    errors: Option<mpsc::Receiver<String>>,
    consumed: Option<Arc<AtomicU64>>,
    cancel: Option<Arc<AtomicBool>>,
    samples_per_second: f64,
    offset: Duration,
}

impl RodioOutput {
    fn connect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let stream = DeviceSinkBuilder::from_default_device()
            .context("No usable audio output device")?
            .with_error_callback(move |error| {
                let _ = sender.try_send(format!("{error}"));
            })
            .open_sink_or_fallback()
            .context("Cannot open audio output")?;
        self.stream = Some(stream);
        self.errors = Some(receiver);
        Ok(())
    }
}

impl Output for RodioOutput {
    fn load(
        &mut self,
        track: &Track,
        offset: Duration,
        paused: bool,
        volume: f32,
    ) -> Result<Option<Duration>, LoadError> {
        let (source, duration) =
            BufferedAudio::open(track, offset, self.equalizer.clone()).map_err(LoadError::Track)?;
        self.connect().map_err(LoadError::Device)?;
        self.stop();
        self.samples_per_second =
            f64::from(source.sample_rate().get()) * f64::from(source.channels().get());
        self.consumed = Some(source.consumed.clone());
        self.cancel = Some(source.cancel.clone());
        self.offset = offset;
        let player = Player::connect_new(self.stream.as_ref().unwrap().mixer());
        player.set_volume(volume);
        if paused {
            player.pause();
        }
        player.append(source);
        self.player = Some(player);
        Ok(duration)
    }
    fn equalizer(&mut self, settings: EqualizerSettings) {
        *self
            .equalizer
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = settings.normalized();
    }
    fn pause(&self, paused: bool) {
        if let Some(player) = &self.player {
            if paused {
                player.pause();
            } else {
                player.play();
            }
        }
    }
    fn volume(&self, volume: f32) {
        if let Some(player) = &self.player {
            player.set_volume(volume);
        }
    }
    fn stop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(player) = self.player.take() {
            player.stop();
        }
        self.consumed = None;
    }
    fn position(&self) -> Duration {
        self.offset
            + self.consumed.as_ref().map_or(Duration::ZERO, |samples| {
                Duration::from_secs_f64(
                    samples.load(Ordering::Relaxed) as f64 / self.samples_per_second,
                )
            })
    }
    fn finished(&self) -> bool {
        self.player.as_ref().is_some_and(Player::empty)
    }
    fn error(&mut self) -> Option<String> {
        let error = self.errors.as_ref()?.try_recv().ok()?;
        self.stop();
        self.stream = None;
        self.errors = None;
        Some(error)
    }
}

impl Drop for RodioOutput {
    fn drop(&mut self) {
        self.stop();
    }
}

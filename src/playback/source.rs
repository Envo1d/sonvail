//! Decode and read files on a producer thread, never on the device callback.
//! Underruns produce a short block of silence; progress counts real samples only.
use std::{
    fs::File,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use rodio::{Decoder, Source};

use crate::{
    equalizer::{EqualizerSettings, Processor},
    library::Track,
};

pub(super) struct BufferedAudio {
    receiver: mpsc::Receiver<Vec<f32>>,
    buffer: Vec<f32>,
    cursor: usize,
    silence: usize,
    channels: rodio::ChannelCount,
    rate: rodio::SampleRate,
    duration: Option<Duration>,
    pub consumed: Arc<AtomicU64>,
    pub cancel: Arc<AtomicBool>,
}

impl BufferedAudio {
    pub fn open(
        track: &Track,
        offset: Duration,
        equalizer: Arc<Mutex<EqualizerSettings>>,
    ) -> Result<(Self, Option<Duration>)> {
        let file = File::open(&track.path)
            .with_context(|| format!("Cannot open {}", track.path.display()))?;
        let mut decoder = Decoder::try_from(file).context("Cannot decode audio")?;
        let channels = decoder.channels();
        let rate = decoder.sample_rate();
        anyhow::ensure!(
            channels.get() <= 32 && rate.get() <= 768_000,
            "Unsupported audio channel count or sample rate"
        );
        let duration = decoder
            .total_duration()
            .or(track.metadata.duration_ms.map(Duration::from_millis));
        if !offset.is_zero() {
            decoder.try_seek(offset).context("Cannot seek this file")?;
        }
        let mut initial: Vec<_> = decoder
            .by_ref()
            .take(1024 * usize::from(channels.get()))
            .collect();
        anyhow::ensure!(
            !initial.is_empty(),
            "The file contains no decodable samples at this position"
        );
        let mut processor = Processor::new(rate.get(), usize::from(channels.get()));
        let settings = *equalizer.lock().unwrap_or_else(|error| error.into_inner());
        processor.process(&mut initial, settings);
        let (sender, receiver) = mpsc::sync_channel(8);
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled = cancel.clone();
        thread::Builder::new()
            .name("sonvail-decoder".into())
            .spawn(move || {
                while !cancelled.load(Ordering::Acquire) {
                    let mut block: Vec<_> = decoder
                        .by_ref()
                        .take(1024 * usize::from(channels.get()))
                        .collect();
                    if block.is_empty() {
                        break;
                    }
                    let settings = *equalizer.lock().unwrap_or_else(|error| error.into_inner());
                    processor.process(&mut block, settings);
                    loop {
                        if cancelled.load(Ordering::Acquire) {
                            return;
                        }
                        match sender.try_send(block) {
                            Ok(()) => break,
                            Err(mpsc::TrySendError::Full(returned)) => {
                                block = returned;
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(mpsc::TrySendError::Disconnected(_)) => return,
                        }
                    }
                }
            })
            .context("Cannot start audio decoder")?;
        Ok((
            Self {
                receiver,
                buffer: initial,
                cursor: 0,
                silence: 0,
                channels,
                rate,
                duration: duration.map(|duration| duration.saturating_sub(offset)),
                consumed: Arc::new(AtomicU64::new(0)),
                cancel,
            },
            duration,
        ))
    }
}

impl Iterator for BufferedAudio {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.cancel.load(Ordering::Relaxed) {
            return None;
        }
        if self.silence > 0 {
            self.silence -= 1;
            return Some(0.0);
        }
        if self.cursor == self.buffer.len() {
            match self.receiver.try_recv() {
                Ok(block) => {
                    self.buffer = block;
                    self.cursor = 0;
                }
                Err(mpsc::TryRecvError::Disconnected) => return None,
                Err(mpsc::TryRecvError::Empty) => {
                    self.silence = 128 * usize::from(self.channels.get()) - 1;
                    return Some(0.0);
                }
            }
        }
        let sample = self.buffer[self.cursor];
        self.cursor += 1;
        self.consumed.fetch_add(1, Ordering::Relaxed);
        Some(sample)
    }
}

impl Source for BufferedAudio {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }
}

impl Drop for BufferedAudio {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_audio_returns_error_without_opening_a_device() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("broken.wav");
        std::fs::write(&path, b"broken").unwrap();
        assert!(
            BufferedAudio::open(&Track::from_path(path), Duration::ZERO, Arc::default()).is_err()
        );
    }

    #[test]
    fn buffered_decoder_emits_exact_pcm_length_and_supports_seek_and_cancel() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("silent.wav");
        crate::library::metadata::tests::wav(&path);
        let track = Track::from_path(path);
        let (mut source, duration) =
            BufferedAudio::open(&track, Duration::ZERO, Arc::default()).unwrap();
        assert_eq!(duration, Some(Duration::from_secs(1)));
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while source.next().is_some() {
            assert!(
                std::time::Instant::now() < deadline,
                "decoder did not finish"
            );
        }
        assert_eq!(source.consumed.load(Ordering::Relaxed), 8000);
        let (mut source, _) =
            BufferedAudio::open(&track, Duration::from_millis(250), Arc::default()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while source.next().is_some() {
            assert!(std::time::Instant::now() < deadline);
        }
        assert_eq!(source.consumed.load(Ordering::Relaxed), 6000);
        let (mut source, _) = BufferedAudio::open(&track, Duration::ZERO, Arc::default()).unwrap();
        source.cancel.store(true, Ordering::Release);
        assert_eq!(source.next(), None);
    }
}

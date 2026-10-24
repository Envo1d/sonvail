use super::root::RootView;
use crate::playback::{Command, PlaybackService, Snapshot, Status};
use gpui::{Context, ScrollStrategy, Task, UniformListScrollHandle};
use std::{sync::Arc, time::Duration};

#[derive(Default)]
pub(super) struct Playback {
    pub snapshot: Snapshot,
    pub show_track_details: bool,
    pub track_scroll: UniformListScrollHandle,
    service: Option<PlaybackService>,
    subscription: Option<Task<()>>,
}

impl RootView {
    pub(super) fn start_playback(&mut self, cx: &mut Context<Self>) {
        match PlaybackService::start() {
            Ok(service) => {
                if let Err(error) = service.send(Command::Equalizer(self.settings.equalizer)) {
                    self.playback.snapshot.error = Some(error.to_string());
                }
                self.playback.service = Some(service);
            }
            Err(error) => self.playback.snapshot.error = Some(format!("{error:#}")),
        }
        cx.on_app_quit(|view, _| {
            let completion = view
                .playback
                .service
                .as_ref()
                .map(PlaybackService::shutdown);
            async move {
                if let Some(completion) = completion {
                    let _ = completion.recv().await;
                }
            }
        })
        .detach();
        self.playback.subscription = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if view
                    .update(cx, |view, cx| {
                        let Some(service) = &view.playback.service else {
                            return;
                        };
                        let next = service.snapshot();
                        let old = &view.playback.snapshot;
                        let changed = !Arc::ptr_eq(&next.tracks, &old.tracks)
                            || !Arc::ptr_eq(&next.order, &old.order)
                            || next.current != old.current
                            || next.status != old.status
                            || next.error != old.error
                            || next.volume != old.volume
                            || next.repeat != old.repeat
                            || next.shuffle != old.shuffle
                            || next.recovery_paused != old.recovery_paused
                            || next.duration != old.duration
                            || next.position != old.position;
                        if next.current != old.current
                            && let Some(index) = next.current
                        {
                            view.playback
                                .track_scroll
                                .scroll_to_item(index, ScrollStrategy::Center);
                        }
                        view.playback.snapshot = next;
                        if changed {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    pub(super) fn playback_command(&mut self, command: Command, cx: &mut Context<Self>) {
        let result = self
            .playback
            .service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Playback is unavailable"))
            .and_then(|service| service.send(command));
        if let Err(error) = result {
            tracing::warn!(%error, "Playback command failed");
            self.playback.snapshot.error = Some(error.to_string());
            cx.notify();
        }
    }

    pub(super) fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if self.playback.snapshot.current.is_none() && !self.library.tracks.is_empty() {
            self.playback_command(
                Command::Play {
                    tracks: self.library.tracks.clone(),
                    index: 0,
                },
                cx,
            );
        } else {
            self.playback_command(Command::Toggle, cx);
        }
    }

    pub(super) fn is_playing(&self) -> bool {
        let state = &self.playback.snapshot;
        state.status == Status::Playing
            || (state.status == Status::Recovering && !state.recovery_paused)
    }
}

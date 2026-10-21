use gpui::{Context, Div, FontWeight, Styled, div, prelude::*, px, rgb};

use super::{root::RootView, theme::Palette};
use crate::updater::{self, PendingUpdate};

pub(super) struct Updater {
    busy: bool,
    pending: Option<PendingUpdate>,
    message: String,
    progress: Option<u8>,
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            busy: false,
            pending: None,
            message: "Check for a new version of Sonvail.".into(),
            progress: None,
        }
    }
}

impl RootView {
    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if self.updater.busy {
            return;
        }
        self.updater.busy = true;
        self.updater.pending = None;
        self.updater.progress = None;
        self.updater.message = "Checking for updates…".into();
        cx.notify();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async { updater::check() })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.updater.busy = false;
                match result {
                    Ok(pending) => {
                        view.updater.message = pending.as_ref().map_or_else(
                            || "You're up to date.".into(),
                            |update| format!("Sonvail {} is available.", update.version()),
                        );
                        view.updater.pending = pending;
                    }
                    Err(error) => {
                        tracing::warn!(error = %format!("{error:#}"), "Update check failed");
                        view.updater.message = error.to_string();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn install_update(&mut self, cx: &mut Context<Self>) {
        if self.updater.busy {
            return;
        }
        let Some(pending) = self.updater.pending.clone() else {
            return;
        };
        self.updater.busy = true;
        self.updater.progress = Some(0);
        self.updater.message = "Downloading and verifying the update…".into();
        cx.notify();
        let (sender, receiver) = async_channel::bounded(32);
        cx.spawn(async move |view, cx| {
            while let Ok(percent) = receiver.recv().await {
                if view
                    .update(cx, |view, cx| {
                        if view.updater.busy && view.updater.progress.is_some() {
                            view.updater.progress = Some(percent);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { updater::install(pending, sender) })
                .await;
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.updater.message = "Restarting to finish the update…".into();
                    cx.quit();
                }
                Err(error) => {
                    tracing::warn!(error = %format!("{error:#}"), "Update installation failed");
                    view.updater.busy = false;
                    view.updater.progress = None;
                    view.updater.message = error.to_string();
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

pub(super) fn update_settings(view: &RootView, theme: Palette, cx: &Context<RootView>) -> Div {
    let update = &view.updater;
    div()
        .child(div().text_size(px(18.0)).font_weight(FontWeight::SEMIBOLD).child("Updates"))
        .child(div().mt_2().text_sm().text_color(rgb(theme.muted))
            .child(format!("Installed version: {}", env!("CARGO_PKG_VERSION"))))
        .child(div().mt_2().text_sm().child(update.message.clone()))
        .when_some(update.progress, |this, percent| this.child(div().mt_2().text_sm().child(format!("{percent}%"))))
        .when_some(update.pending.as_ref(), |this, pending| {
            this.when(!pending.notes().is_empty(), |this| this.child(
                div().mt_3().text_sm().text_color(rgb(theme.muted)).child(pending.notes().to_owned())
            ))
        })
        .child(div().mt_4().flex().gap_3()
            .child(div().id("check-for-updates").px_4().py_2().rounded_md()
                .border_1().border_color(rgb(theme.border))
                .when(update.busy, |this| this.opacity(0.6))
                .when(!update.busy, |this| this.cursor_pointer())
                .on_click(cx.listener(|view, _, _, cx| view.check_for_updates(cx)))
                .child("Check for updates"))
            .when(update.pending.is_some(), |this| this.child(
                div().id("install-update").px_4().py_2().rounded_md()
                    .bg(rgb(theme.accent)).text_color(rgb(0xffffff))
                    .when(update.busy, |this| this.opacity(0.6))
                    .when(!update.busy, |this| this.cursor_pointer())
                    .on_click(cx.listener(|view, _, _, cx| view.install_update(cx)))
                    .child("Install and restart")
            )))
        .child(div().mt_3().text_sm().text_color(rgb(theme.muted))
            .child("Installing an update stops playback and restarts Sonvail. Your music and settings are preserved."))
}

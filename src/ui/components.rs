use super::{icons::icon, theme};
use crate::library::Track;
use gpui::{Div, ObjectFit, Styled, div, img, prelude::*, px, rgb};
use std::path::Path;

pub fn label(theme: theme::Palette, text: &str) -> Div {
    div()
        .text_size(px(13.0))
        .text_color(rgb(theme.muted))
        .child(text.to_owned())
}

pub fn icon_button(theme: theme::Palette, name: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(42.0))
        .h(px(36.0))
        .rounded_md()
        .child(icon(theme, name))
}

pub fn track_artwork(
    theme: theme::Palette,
    track: Option<&Track>,
    directory: &Path,
    edge: f32,
) -> Div {
    let path = track.and_then(|track| {
        if edge <= 128.0 {
            track.metadata.thumbnail_path(directory)
        } else {
            track.metadata.artwork_path(directory)
        }
    });
    let missing = path.is_none();
    div()
        .size(px(edge))
        .flex_shrink_0()
        .rounded_lg()
        .overflow_hidden()
        .bg(rgb(theme.surface))
        .flex()
        .items_center()
        .justify_center()
        .when_some(path, |this, path| {
            this.child(
                img(path)
                    .size(px(edge))
                    .object_fit(ObjectFit::Cover)
                    .rounded_lg(),
            )
        })
        .when(missing, |this| {
            this.child(icon(theme, "album").size(px(edge * 0.4)))
        })
}

pub fn track_info(theme: theme::Palette, title: &str, artist: &str) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(14.0))
        .child(div().w_full().truncate().child(title.to_owned()))
        .child(
            div()
                .truncate()
                .w_full()
                .text_size(px(13.0))
                .text_color(rgb(theme.muted))
                .child(artist.to_owned()),
        )
}

pub fn duration_label(milliseconds: Option<u64>) -> String {
    let Some(milliseconds) = milliseconds else {
        return "--:--".into();
    };
    let seconds = milliseconds / 1000;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

use super::{
    components::{duration_label, track_artwork, track_info},
    root::RootView,
    theme::Palette,
};
use crate::playback::Command;
use gpui::{Context, Styled, div, prelude::*, px, rgb, rgba};

pub fn queue(
    view: &RootView,
    theme: Palette,
    width: f32,
    cx: &Context<RootView>,
) -> impl IntoElement {
    let snapshot = &view.playback.snapshot;
    let start = snapshot
        .current
        .and_then(|current| snapshot.order.iter().position(|index| *index == current))
        .map_or(0, |cursor| cursor + 1);
    let upcoming = &snapshot.order[start..];
    let directory = view.config.library_path().with_file_name("artwork");
    let row =
        |index: usize, section: &'static str| {
            let track = &snapshot.tracks[index];
            div()
                .id((section, index))
                .h(px(70.0))
                .w_full()
                .min_w_0()
                .overflow_hidden()
                .flex()
                .items_center()
                .gap_3()
                .rounded_md()
                .cursor_pointer()
                .hover(move |style| style.bg(rgba((theme.text << 8) | 0x0c)))
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.playback_command(Command::Select(index), cx)
                }))
                .child(track_artwork(theme, Some(track), &directory, 50.0))
                .child(track_info(theme, &track.title, &track.artist()))
                .child(
                    div()
                        .w(px(34.0))
                        .flex_shrink_0()
                        .text_right()
                        .text_xs()
                        .text_color(rgb(theme.muted))
                        .child(duration_label(track.metadata.duration_ms)),
                )
        };
    div()
        .id("queue-panel")
        .w(px(width))
        .min_w_0()
        .h_full()
        .flex_shrink_0()
        .overflow_y_scroll()
        .overflow_x_hidden()
        .border_l_1()
        .border_color(rgba(theme.hairline))
        .px(px(26.0))
        .pt(px(48.0))
        .pb_6()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mb_4()
                .text_sm()
                .text_color(rgb(theme.muted))
                .child("QUEUE")
                .child(
                    div()
                        .text_xs()
                        .child(format!("{} remaining", upcoming.len())),
                ),
        )
        .when(upcoming.is_empty(), |this| {
            this.child(
                div()
                    .py_4()
                    .text_sm()
                    .text_color(rgb(theme.muted))
                    .child("No upcoming tracks."),
            )
        })
        .children(
            upcoming
                .iter()
                .take(5)
                .map(|index| row(*index, "queue-preview")),
        )
        .child(
            div()
                .mt_5()
                .pt_5()
                .border_t_1()
                .border_color(rgba(theme.hairline))
                .child(
                    div()
                        .mb_4()
                        .text_sm()
                        .text_color(rgb(theme.muted))
                        .child("UP NEXT"),
                )
                .children(
                    upcoming
                        .iter()
                        .skip(5)
                        .take(3)
                        .map(|index| row(*index, "up-next-preview")),
                ),
        )
}

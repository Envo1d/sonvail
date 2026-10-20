use gpui::{
    Div, FontWeight, Styled, div, linear_color_stop, linear_gradient, prelude::*, px, rgb, rgba,
};

use super::{
    components::{artwork, icon_button, label},
    demo,
    icons::icon,
    theme,
};

pub fn now_playing(theme: theme::Palette, artwork_size: f32, padding: f32) -> impl IntoElement {
    div()
        .id("now-playing")
        .flex_1()
        .min_w_0()
        .h_full()
        .overflow_y_scroll()
        .px(px(padding))
        .pb_8()
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(theme.accent))
                .mb(px(24.0))
                .child("NOW PLAYING"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(32.0))
                .child(artwork(artwork_size, artwork_size + 20.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .pt(px(if artwork_size >= 240.0 { 40.0 } else { 0.0 }))
                        .gap(px(10.0))
                        .child(
                            div()
                                .text_size(px(if artwork_size < 240.0 { 27.0 } else { 32.0 }))
                                .line_height(px(38.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(demo::ALBUM),
                        )
                        .child(
                            div()
                                .text_size(px(20.0))
                                .text_color(rgb(theme.accent))
                                .child(demo::ARTIST),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(theme.muted))
                                .child(demo::ALBUM),
                        )
                        .child(div().flex().gap_2().children([
                            badge(theme, "2017"),
                            badge(theme, "Electronic"),
                            badge(theme, "11 Tracks"),
                        ]))
                        .child(waveform(theme))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap_2()
                                        .h(px(39.0))
                                        .px(px(26.0))
                                        .rounded_md()
                                        .bg(linear_gradient(
                                            130.0,
                                            linear_color_stop(rgb(0x3476ff), 0.0),
                                            linear_color_stop(rgb(0x2050df), 1.0),
                                        ))
                                        .text_color(rgb(0xffffff))
                                        .child(
                                            icon(theme, "pause")
                                                .size(px(15.0))
                                                .text_color(rgb(0xffffff)),
                                        )
                                        .child(div().text_sm().child("Pause")),
                                )
                                .child(
                                    div()
                                        .rounded_md()
                                        .bg(rgb(theme.surface))
                                        .child(icon_button(theme, "heart")),
                                )
                                .child(
                                    div()
                                        .rounded_md()
                                        .bg(rgb(theme.surface))
                                        .child(icon_button(theme, "more")),
                                ),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .mt(px(24.0))
                .px_3()
                .py_3()
                .border_b_1()
                .border_color(rgba(theme.hairline))
                .child(
                    div()
                        .w(px(26.0))
                        .text_xs()
                        .text_color(rgb(theme.muted))
                        .child("#"),
                )
                .child(div().flex_1().child(label(theme, "TITLE")))
                .child(icon(theme, "clock").size(px(15.0)))
                .child(div().w(px(20.0))),
        )
        .children(
            demo::TRACKS
                .iter()
                .enumerate()
                .map(|(index, track)| track_row(theme, index, track)),
        )
}

fn badge(theme: theme::Palette, text: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(rgb(theme.surface))
        .child(label(theme, text))
}

fn waveform(theme: theme::Palette) -> Div {
    // A static visual placeholder, not audio analysis or playback progress.
    div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .h(px(26.0))
        .overflow_hidden()
        .children((0..68).map(|index| {
            let envelope = 1.0 - index as f32 / 90.0;
            let height = 3.0 + ((index * 7 + index * index * 3) % 17) as f32 * envelope;
            div()
                .w(px(2.0))
                .flex_1()
                .min_w(px(2.0))
                .max_w(px(4.0))
                .h(px(height))
                .flex_shrink_0()
                .rounded_full()
                .bg(rgb(if index < 30 { theme.accent } else { 0x4b4395 }))
        }))
}

fn track_row(theme: theme::Palette, index: usize, track: &demo::Track) -> Div {
    let selected = index == demo::CURRENT_TRACK;
    div()
        .flex()
        .items_center()
        .gap_3()
        .h(px(55.0))
        .px_3()
        .rounded_md()
        .when(selected, |this| this.bg(rgba(0x9baaca12)))
        .when(!selected, |this| {
            this.border_b_1().border_color(rgba(theme.hairline))
        })
        .child(
            div()
                .w(px(20.0))
                .flex_shrink_0()
                .text_xs()
                .text_color(rgb(theme.muted))
                .when(selected, |this| {
                    this.child(
                        icon(theme, "playing")
                            .size(px(16.0))
                            .text_color(rgb(theme.accent)),
                    )
                })
                .when(!selected, |this| this.child((index + 1).to_string())),
        )
        .child(super::components::track_info(
            theme,
            track.title,
            demo::ARTIST,
        ))
        .child(div().w(px(20.0)).when(selected, |this| {
            this.child(
                icon(theme, "heart-filled")
                    .size(px(16.0))
                    .text_color(rgb(theme.accent)),
            )
        }))
        .child(
            div()
                .w(px(36.0))
                .text_right()
                .text_xs()
                .text_color(rgb(if selected { theme.accent } else { theme.muted }))
                .child(track.duration),
        )
        .child(div().w(px(16.0)).when(selected, |this| {
            this.child(icon(theme, "more").size(px(16.0)))
        }))
}

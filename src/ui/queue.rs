use gpui::{Div, Styled, div, prelude::*, px, rgba};

use super::{
    components::{cover, label, toggle, track_info},
    demo,
    icons::icon,
    theme,
};

pub fn queue(theme: theme::Palette, width: f32) -> impl IntoElement {
    div()
        .id("queue-panel")
        .w(px(width))
        .h_full()
        .flex_shrink_0()
        .overflow_y_scroll()
        .border_l_1()
        .border_color(rgba(theme.hairline))
        .pt(px(48.0))
        .pb_6()
        .px(px(26.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mb_6()
                .child(label(theme, "QUEUE"))
                .child(label(theme, "Clear")),
        )
        .children(
            demo::QUEUE
                .iter()
                .map(|track| queue_row(theme, track, true)),
        )
        .child(
            div()
                .mt_5()
                .pt_5()
                .border_t_1()
                .border_color(rgba(theme.hairline))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .mb_4()
                        .child(label(theme, "UP NEXT"))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(label(theme, "Autoplay"))
                                .child(toggle(theme, true)),
                        ),
                )
                .children(
                    demo::UP_NEXT
                        .iter()
                        .map(|track| queue_row(theme, track, false)),
                ),
        )
}

fn queue_row(theme: theme::Palette, track: &demo::Track, reorder: bool) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .py(px(9.0))
        .child(cover(50.0, track.color))
        .child(track_info(theme, track.title, demo::ARTIST))
        .when(reorder, |this| {
            this.child(icon(theme, "grip").size(px(15.0)))
        })
}

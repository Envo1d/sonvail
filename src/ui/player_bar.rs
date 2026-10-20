use gpui::{Div, Styled, div, prelude::*, px, relative, rgb, rgba};

use super::{
    components::{cover, icon_button, label, track_info},
    demo,
    icons::icon,
    theme,
};

pub fn player_bar(theme: theme::Palette) -> Div {
    div()
        .h(px(122.0))
        .w_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_6()
        .px_6()
        .bg(rgba(theme.player_tint))
        .border_t_1()
        .border_color(rgba(theme.hairline))
        .child(
            div()
                .w(relative(0.28))
                .min_w_0()
                .flex()
                .items_center()
                .gap_4()
                .child(cover(70.0, 0x986689))
                .child(track_info(theme, demo::ALBUM, demo::ARTIST).text_size(px(16.0)))
                .child(icon(theme, "heart-filled").text_color(rgb(theme.accent))),
        )
        .child(
            div()
                .flex_1()
                .max_w(px(480.0))
                .min_w_0()
                .flex()
                .flex_col()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(icon_button(theme, "shuffle"))
                        .child(icon_button(theme, "previous"))
                        .child(
                            div()
                                .size(px(50.0))
                                .rounded_full()
                                .bg(rgb(theme.text))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    icon(theme, "pause")
                                        .size(px(24.0))
                                        .text_color(rgb(theme.background)),
                                ),
                        )
                        .child(icon_button(theme, "next"))
                        .child(icon_button(theme, "repeat")),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(label(theme, "1:45"))
                        .child(
                            div()
                                .flex_1()
                                .h(px(4.0))
                                .rounded_full()
                                .bg(rgb(theme.border))
                                .child(
                                    div()
                                        .relative()
                                        .w(relative(0.45))
                                        .h_full()
                                        .rounded_full()
                                        .bg(rgb(theme.accent))
                                        .child(
                                            div()
                                                .absolute()
                                                .right(px(-3.0))
                                                .top(px(-1.0))
                                                .size(px(6.0))
                                                .rounded_full()
                                                .bg(rgb(theme.accent)),
                                        ),
                                ),
                        )
                        .child(label(theme, "3:53")),
                ),
        )
        .child(
            div()
                .w(relative(0.28))
                .flex()
                .items_center()
                .justify_end()
                .gap_3()
                .child(icon(theme, "volume"))
                .child(
                    div()
                        .w(px(132.0))
                        .flex_shrink()
                        .h(px(4.0))
                        .rounded_full()
                        .bg(rgb(theme.border))
                        .child(
                            div()
                                .relative()
                                .w(relative(0.45))
                                .h_full()
                                .rounded_full()
                                .bg(rgb(theme.accent))
                                .child(
                                    div()
                                        .absolute()
                                        .right(px(-5.0))
                                        .top(px(-3.0))
                                        .size(px(10.0))
                                        .rounded_full()
                                        .bg(rgb(0x7297ff)),
                                ),
                        ),
                )
                .child(icon_button(theme, "device"))
                .child(icon_button(theme, "expand")),
        )
}

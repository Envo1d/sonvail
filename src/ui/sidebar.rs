use gpui::{Div, Styled, WindowControlArea, div, img, prelude::*, px, rgb, rgba};

use super::{
    components::{icon_button, label, toggle},
    icons::{APP_ICON_SIZE, app_icon_path, icon},
    theme,
};

pub fn sidebar(scale_factor: f32, width: f32) -> Div {
    div()
        .w(px(width))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(rgba(theme::SIDEBAR_TINT))
        .px(px(15.0))
        .py(px(28.0))
        .gap_5()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .px_2()
                .window_control_area(WindowControlArea::Drag)
                .child(
                    img(app_icon_path(scale_factor))
                        .size(px(APP_ICON_SIZE))
                        .flex_shrink_0(),
                )
                .child(div().text_size(px(18.0)).child("Soundora")),
        )
        .child(
            div()
                .h(px(38.0))
                .mx_2()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(theme::BORDER))
                .child(icon("search").size(px(15.0)))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(rgb(theme::MUTED))
                        .child("Search"),
                )
                .child(label("Ctrl + K")),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(nav_item("playing", "Now Playing", true))
                .child(nav_item("library", "Library", false))
                .child(nav_item("playlist", "Playlists", false))
                .child(nav_item("album", "Albums", false))
                .child(nav_item("artist", "Artists", false)),
        )
        .child(
            div()
                .id("sidebar-playlists")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .pl_2()
                        .mb_2()
                        .child(label("PLAYLISTS"))
                        .child(icon_button("plus")),
                )
                .child(nav_item("heart", "My Favorites", false))
                .child(nav_item("playlist", "Chill Vibes", false))
                .child(nav_item("playing", "Workout", false))
                .child(nav_item("album", "Focus", false))
                .child(nav_item("playlist", "Road Trip", false)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(nav_item("settings", "Settings", false))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .child(icon("sun"))
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .text_color(rgb(theme::MUTED))
                                .child("Light Mode"),
                        )
                        .child(toggle(false)),
                ),
        )
}

fn nav_item(symbol: &'static str, title: &'static str, selected: bool) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .h(px(41.0))
        .px_3()
        .rounded_md()
        .text_sm()
        .text_color(rgb(if selected { theme::TEXT } else { theme::MUTED }))
        .when(selected, |this| this.bg(rgb(theme::SELECTED)))
        .child(icon(symbol).text_color(rgb(if selected { theme::TEXT } else { theme::MUTED })))
        .child(title)
}

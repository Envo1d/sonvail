use gpui::{Div, Img, ObjectFit, Styled, div, img, prelude::*, px, rgb};

use super::{
    icons::{ALBUM_ART, icon},
    theme,
};

pub fn label(text: &'static str) -> Div {
    div()
        .text_size(px(13.0))
        .text_color(rgb(theme::MUTED))
        .child(text)
}

pub fn icon_button(name: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(50.0))
        .h(px(39.0))
        .rounded_md()
        .child(icon(name))
}

pub fn cover(edge: f32, color: u32) -> Div {
    div()
        .size(px(edge))
        .flex_shrink_0()
        .rounded_md()
        .bg(rgb(color))
        .child(artwork(edge, edge).rounded_md().opacity(0.94))
}

pub fn artwork(width: f32, height: f32) -> Img {
    img(ALBUM_ART)
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .object_fit(ObjectFit::Cover)
        .rounded_lg()
}

pub fn toggle(on: bool) -> Div {
    div()
        .flex()
        .items_center()
        .w(px(37.0))
        .h(px(21.0))
        .px(px(3.0))
        .rounded_full()
        .bg(rgb(if on { theme::ACCENT } else { theme::SURFACE }))
        .when(on, |this| this.justify_end())
        .child(div().size(px(15.0)).rounded_full().bg(rgb(if on {
            theme::TEXT
        } else {
            theme::MUTED
        })))
}

pub fn track_info(title: &'static str, artist: &'static str) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(14.0))
        .child(div().w_full().truncate().child(title))
        .child(
            div()
                .truncate()
                .w_full()
                .text_size(px(13.0))
                .text_color(rgb(theme::MUTED))
                .child(artist),
        )
}

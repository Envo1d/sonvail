use gpui::{Context, IntoElement, Render, Styled, Window, div, prelude::*, px, rgb, rgba};

use super::{
    layout::Layout, now_playing::now_playing, player_bar::player_bar, queue::queue,
    sidebar::sidebar, theme, titlebar::titlebar,
};

pub struct RootView;

impl Render for RootView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let layout = Layout::for_width(window.viewport_size().width.into());
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .font_family("Segoe UI")
            .text_size(px(14.0))
            .bg(rgba(theme::WINDOW_TINT))
            .text_color(rgb(theme::TEXT))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(sidebar(window.scale_factor(), layout.sidebar))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(titlebar(window))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_h_0()
                                    .w_full()
                                    .overflow_hidden()
                                    .child(now_playing(layout.artwork, layout.padding))
                                    .child(queue(layout.queue)),
                            ),
                    ),
            )
            .child(player_bar())
    }
}

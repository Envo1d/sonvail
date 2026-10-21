use crate::config::ConfigStore;
use gpui::{Context, IntoElement, Render, Styled, Window, div, prelude::*, px, rgb, rgba};

use super::{
    layout::Layout,
    library::{Library, library_view},
    now_playing::now_playing,
    player_bar::player_bar,
    queue::queue,
    settings::{Settings, settings_view},
    sidebar::sidebar,
    titlebar::titlebar,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    NowPlaying,
    Library,
    Settings,
}

pub struct RootView {
    pub(super) page: Page,
    pub(super) settings: Settings,
    pub(super) config: ConfigStore,
    pub(super) library: Library,
}

impl RootView {
    pub fn new(config: ConfigStore, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_appearance(window, |_, _, cx| cx.notify())
            .detach();
        let mut view = Self {
            page: Page::default(),
            settings: Settings::from_config(config.snapshot()),
            config,
            library: Library::default(),
        };
        view.start_library(cx);
        view
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = Layout::for_width(window.viewport_size().width.into());
        let theme = self.settings.theme.palette(window.appearance());
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .font_family("Segoe UI")
            .text_size(px(14.0))
            .bg(rgba(theme.window_tint))
            .text_color(rgb(theme.text))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .child(sidebar(
                        theme,
                        window.scale_factor(),
                        layout.sidebar,
                        self.page,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(titlebar(theme, window))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_h_0()
                                    .w_full()
                                    .overflow_hidden()
                                    .when(self.page == Page::NowPlaying, |this| {
                                        this.child(now_playing(
                                            theme,
                                            layout.artwork,
                                            layout.padding,
                                        ))
                                        .child(queue(theme, layout.queue))
                                    })
                                    .when(self.page == Page::Settings, |this| {
                                        this.child(settings_view(self, theme, layout, cx))
                                    })
                                    .when(self.page == Page::Library, |this| {
                                        this.child(library_view(self, theme, layout, cx))
                                    }),
                            ),
                    ),
            )
            .child(player_bar(theme))
    }
}

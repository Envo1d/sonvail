use gpui::{
    Context, Div, Stateful, Styled, WindowControlArea, div, img, prelude::*, px, rgb, rgba,
};

use super::{
    components::{icon_button, label},
    icons::{APP_ICON_SIZE, app_icon_path, icon},
    root::{Page, RootView},
    theme,
};

pub fn sidebar(
    theme: theme::Palette,
    scale_factor: f32,
    width: f32,
    page: Page,
    cx: &Context<RootView>,
) -> Div {
    div()
        .w(px(width))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(rgba(theme.sidebar_tint))
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
                .border_color(rgb(theme.border))
                .child(icon(theme, "search").size(px(15.0)))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(rgb(theme.muted))
                        .child("Search"),
                )
                .child(label(theme, "Ctrl + K")),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    nav_item(theme, "playing", "Now Playing", page == Page::NowPlaying).on_click(
                        cx.listener(|view, _, _, cx| {
                            view.page = Page::NowPlaying;
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    nav_item(theme, "library", "Library", page == Page::Library).on_click(
                        cx.listener(|view, _, _, cx| {
                            view.page = Page::Library;
                            cx.notify();
                        }),
                    ),
                )
                .child(nav_item(theme, "playlist", "Playlists", false))
                .child(nav_item(theme, "album", "Albums", false))
                .child(nav_item(theme, "artist", "Artists", false)),
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
                        .child(label(theme, "PLAYLISTS"))
                        .child(icon_button(theme, "plus")),
                )
                .child(nav_item(theme, "heart", "My Favorites", false))
                .child(nav_item(theme, "playlist", "Chill Vibes", false))
                .child(nav_item(theme, "playing", "Workout", false))
                .child(nav_item(theme, "album", "Focus", false))
                .child(nav_item(theme, "playlist", "Road Trip", false)),
        )
        .child(div().flex().flex_col().gap_1().child(
            nav_item(theme, "settings", "Settings", page == Page::Settings).on_click(cx.listener(
                |view, _, _, cx| {
                    view.page = Page::Settings;
                    cx.notify();
                },
            )),
        ))
}

fn nav_item(
    theme: theme::Palette,
    symbol: &'static str,
    title: &'static str,
    selected: bool,
) -> Stateful<Div> {
    div()
        .id(title)
        .flex()
        .items_center()
        .gap_3()
        .h(px(41.0))
        .px_3()
        .rounded_md()
        .text_sm()
        .text_color(rgb(if selected { theme.text } else { theme.muted }))
        .when(selected, |this| this.bg(rgb(theme.selected)))
        .cursor_pointer()
        .hover(move |style| {
            style.bg(rgba(if selected {
                (theme.accent << 8) | 0x48
            } else {
                (theme.text << 8) | 0x0c
            }))
        })
        .child(icon(theme, symbol).text_color(rgb(if selected { theme.text } else { theme.muted })))
        .child(title)
}

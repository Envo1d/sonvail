use gpui::{Div, Styled, Window, WindowControlArea, div, prelude::*, px, rgb};

use super::{icons::icon, theme};

pub fn titlebar(theme: theme::Palette, window: &Window) -> Div {
    div()
        .flex()
        .w_full()
        .h(px(44.0))
        .flex_shrink_0()
        // Leave the top edge and upper-right corner available to native resize hit testing.
        .pt(px(5.0))
        .pr(px(5.0))
        .child(
            div()
                .h_full()
                .flex_1()
                .window_control_area(WindowControlArea::Drag),
        )
        .child(window_button(
            theme,
            "window-minimize",
            "minimize",
            WindowControlArea::Min,
        ))
        .child(window_button(
            theme,
            "window-maximize",
            if window.is_maximized() {
                "restore"
            } else {
                "maximize"
            },
            WindowControlArea::Max,
        ))
        .child(window_button(
            theme,
            "window-close",
            "close",
            WindowControlArea::Close,
        ))
}

fn window_button(
    theme: theme::Palette,
    id: &'static str,
    symbol: &'static str,
    area: WindowControlArea,
) -> impl IntoElement {
    // GPUI's Windows backend dispatches native caption actions for these hitboxes.
    // Adding click handlers here would perform the same action twice.
    div()
        .id(id)
        .w(px(60.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .window_control_area(area)
        .hover(move |style| {
            style.bg(rgb(if area == WindowControlArea::Close {
                0xc42b3c
            } else {
                theme.surface
            }))
        })
        .active(move |style| {
            style.bg(rgb(if area == WindowControlArea::Close {
                0xa92333
            } else {
                theme.selected
            }))
        })
        .child(
            icon(theme, symbol)
                .size(px(15.0))
                .text_color(rgb(theme.text)),
        )
}

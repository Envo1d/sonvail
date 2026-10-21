use super::{
    components::{duration_label, icon_button, track_artwork, track_info},
    icons::icon,
    root::RootView,
    theme::Palette,
};
use crate::playback::{Command, Repeat, Status};
use gpui::{
    Bounds, Context, Div, Pixels, Styled, canvas, div, prelude::*, px, relative, rgb, rgba,
};
use std::{cell::Cell, rc::Rc, time::Duration};

pub fn player_bar(view: &RootView, theme: Palette, cx: &Context<RootView>) -> Div {
    let state = &view.playback.snapshot;
    let track = state.track();
    let directory = view.config.library_path().with_file_name("artwork");
    let total = state.duration.map(|duration| duration.as_secs_f32());
    let progress = total
        .filter(|duration| *duration > 0.0)
        .map_or(0.0, |duration| state.position.as_secs_f32() / duration);
    div()
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(rgba(theme.player_tint))
        .border_t_1()
        .border_color(rgba(theme.hairline))
        .when_some(state.error.as_ref(), |this, error| {
            this.child(
                div()
                    .px_6()
                    .pt_2()
                    .text_sm()
                    .text_color(rgb(theme.accent))
                    .child(error.clone()),
            )
        })
        .child(
            div()
                .h(px(122.0))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_5()
                .px_6()
                .child(
                    div()
                        .w(relative(0.27))
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(track_artwork(theme, track, &directory, 70.0))
                        .child(track_info(
                            theme,
                            track.map_or("No track selected", |track| track.title.as_str()),
                            &track.map_or_else(String::new, |track| track.artist()),
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .max_w(px(480.0))
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(control(
                                    theme,
                                    "shuffle",
                                    "shuffle",
                                    state.shuffle,
                                    cx,
                                    |view, cx| {
                                        view.playback_command(
                                            Command::Shuffle(!view.playback.snapshot.shuffle),
                                            cx,
                                        )
                                    },
                                ))
                                .child(control(
                                    theme,
                                    "previous",
                                    "previous",
                                    false,
                                    cx,
                                    |view, cx| view.playback_command(Command::Previous, cx),
                                ))
                                .child(control(
                                    theme,
                                    "play-pause",
                                    if view.is_playing() { "pause" } else { "play" },
                                    true,
                                    cx,
                                    |view, cx| view.toggle_playback(cx),
                                ))
                                .child(control(theme, "next", "next", false, cx, |view, cx| {
                                    view.playback_command(Command::Next, cx)
                                }))
                                .child(control(
                                    theme,
                                    "repeat",
                                    "repeat",
                                    state.repeat != Repeat::Off,
                                    cx,
                                    |view, cx| {
                                        view.playback_command(
                                            Command::Repeat(view.playback.snapshot.repeat.next()),
                                            cx,
                                        )
                                    },
                                ))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(theme.accent))
                                        .child(if state.repeat == Repeat::One { "1" } else { "" }),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div().text_xs().text_color(rgb(theme.muted)).child(
                                        duration_label(Some(state.position.as_millis() as u64)),
                                    ),
                                )
                                .child(
                                    slider(
                                        theme,
                                        "playback-position",
                                        progress,
                                        total.is_some(),
                                        cx,
                                        move |fraction| {
                                            Command::Seek(Duration::from_secs_f32(
                                                total.unwrap_or_default() * fraction,
                                            ))
                                        },
                                    )
                                    .flex_1(),
                                )
                                .child(div().text_xs().text_color(rgb(theme.muted)).child(
                                    duration_label(
                                        state.duration.map(|duration| duration.as_millis() as u64),
                                    ),
                                )),
                        )
                        .when(state.status == Status::Recovering, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme.muted))
                                    .child("Reconnecting audio output…"),
                            )
                        }),
                )
                .child(
                    div()
                        .w(relative(0.22))
                        .min_w_0()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_2()
                        .child(control(
                            theme,
                            "mute",
                            "volume",
                            state.volume == 0.0,
                            cx,
                            |view, cx| {
                                view.playback_command(
                                    Command::Volume(if view.playback.snapshot.volume > 0.0 {
                                        0.0
                                    } else {
                                        0.7
                                    }),
                                    cx,
                                )
                            },
                        ))
                        .child(
                            slider(
                                theme,
                                "playback-volume",
                                state.volume,
                                true,
                                cx,
                                Command::Volume,
                            )
                            .w(px(112.0)),
                        )
                        .child(control(theme, "stop", "stop", false, cx, |view, cx| {
                            view.playback_command(Command::Stop, cx)
                        })),
                ),
        )
}

fn control(
    theme: Palette,
    id: &'static str,
    symbol: &'static str,
    active: bool,
    cx: &Context<RootView>,
    action: impl Fn(&mut RootView, &mut Context<RootView>) + 'static,
) -> impl IntoElement {
    let primary = id == "play-pause";
    let button = if primary {
        div()
            .size(px(50.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(rgb(theme.text))
            .child(
                icon(theme, symbol)
                    .size(px(24.0))
                    .text_color(rgb(theme.surface)),
            )
    } else {
        icon_button(theme, symbol)
    };
    button
        .id(id)
        .cursor_pointer()
        .when(active && !primary, |this| {
            this.bg(rgba((theme.accent << 8) | 0x20))
        })
        .when(!primary, |this| {
            this.hover(move |style| style.bg(rgba((theme.text << 8) | 0x10)))
        })
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}

fn slider(
    theme: Palette,
    id: &'static str,
    fraction: f32,
    enabled: bool,
    cx: &Context<RootView>,
    command: impl Fn(f32) -> Command + 'static,
) -> gpui::Stateful<Div> {
    let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    let measure = bounds.clone();
    div()
        .id(id)
        .relative()
        .h(px(20.0))
        .min_w(px(20.0))
        .flex()
        .items_center()
        .when(enabled, |this| this.cursor_pointer())
        .child(
            div()
                .w_full()
                .h(px(4.0))
                .rounded_full()
                .bg(rgb(theme.border))
                .child(
                    div()
                        .relative()
                        .h_full()
                        .w(relative(fraction.clamp(0.0, 1.0)))
                        .rounded_full()
                        .bg(rgb(theme.accent))
                        .child(
                            div()
                                .absolute()
                                .right(px(-3.0))
                                .top(px(if id == "playback-volume" { -3.0 } else { -1.0 }))
                                .size(px(if id == "playback-volume" { 10.0 } else { 6.0 }))
                                .rounded_full()
                                .bg(rgb(theme.accent)),
                        ),
                ),
        )
        .child(
            canvas(move |rect, _, _| measure.set(Some(rect)), |_, _, _, _| {})
                .absolute()
                .size_full(),
        )
        .on_click(cx.listener(move |view, event: &gpui::ClickEvent, _, cx| {
            if enabled && let Some(bounds) = bounds.get() {
                let width = f32::from(bounds.size.width);
                if width > 0.0 {
                    let fraction =
                        (f32::from(event.position().x - bounds.origin.x) / width).clamp(0.0, 1.0);
                    view.playback_command(command(fraction), cx);
                }
            }
        }))
}

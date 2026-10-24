use super::{layout::Layout, root::RootView, theme::Palette};
use crate::{equalizer::LABELS, playback::Command};
use gpui::{
    Context, Div, DragMoveEvent, FontWeight, MouseButton, Render, Styled, Window, div, prelude::*,
    px, relative, rgb,
};

struct BandDrag(usize);

struct DragPreview;
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl RootView {
    fn preview_equalizer_gain(&mut self, index: usize, gain: i8, cx: &mut Context<Self>) {
        let gain = gain.clamp(-12, 12);
        if self.settings.equalizer.gains[index] != gain {
            self.settings.equalizer.gains[index] = gain;
            self.playback_command(Command::Equalizer(self.settings.equalizer), cx);
            cx.notify();
        }
    }

    fn commit_equalizer(&mut self, cx: &mut Context<Self>) {
        if self.settings.equalizer != self.config.snapshot().equalizer {
            self.save_settings(cx);
        }
    }
}

pub(super) fn equalizer_view(
    view: &RootView,
    theme: Palette,
    layout: Layout,
    cx: &Context<RootView>,
) -> Div {
    let settings = view.settings.equalizer;
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Equalizer"),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            div()
                                .id("equalizer-reset")
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(move |style| style.bg(rgb(theme.surface)))
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.settings.equalizer.gains = [0; 9];
                                    view.save_settings(cx);
                                }))
                                .child("Reset"),
                        )
                        .child(
                            div()
                                .id("equalizer-enabled")
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .cursor_pointer()
                                .bg(rgb(if settings.enabled {
                                    theme.accent
                                } else {
                                    theme.surface
                                }))
                                .text_color(rgb(if settings.enabled {
                                    0xffffff
                                } else {
                                    theme.text
                                }))
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.settings.equalizer.enabled =
                                        !view.settings.equalizer.enabled;
                                    view.save_settings(cx);
                                }))
                                .child(if settings.enabled { "On" } else { "Off" }),
                        ),
                ),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(theme.muted))
                .child("Nine bands · −12 to +12 dB · automatic headroom when boosting"),
        )
        .child(
            div()
                .id("equalizer-bands")
                .overflow_x_scroll()
                .flex()
                .gap_2()
                .child(
                    div()
                        .h(px(layout.equalizer_height))
                        .mt(px(28.0))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .text_xs()
                        .text_color(rgb(theme.muted))
                        .children(["+12", "0 dB", "−12"]),
                )
                .children(LABELS.into_iter().enumerate().map(|(index, label)| {
                    let gain = settings.gains[index];
                    div()
                        .flex_1()
                        .min_w(px(layout.equalizer_band_width))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme.accent))
                                .child(format!("{gain:+} dB")),
                        )
                        .child(
                            div()
                                .id(("equalizer-band", index))
                                .relative()
                                .w_full()
                                .h(px(layout.equalizer_height))
                                .cursor_pointer()
                                .on_drag(BandDrag(index), |_, _, _, cx| cx.new(|_| DragPreview))
                                .on_drag_move(cx.listener(
                                    move |view, event: &DragMoveEvent<BandDrag>, _, cx| {
                                        if event.drag(cx).0 == index {
                                            let height = f32::from(event.bounds.size.height);
                                            let fraction = f32::from(
                                                event.event.position.y - event.bounds.origin.y,
                                            ) / height;
                                            view.preview_equalizer_gain(
                                                index,
                                                (12.0 - 24.0 * fraction).round() as i8,
                                                cx,
                                            );
                                        }
                                    },
                                ))
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| view.commit_equalizer(cx)),
                                )
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| view.commit_equalizer(cx)),
                                )
                                .on_click(cx.listener(
                                    move |view, event: &gpui::ClickEvent, _, cx| {
                                        // A double click restores this band without changing the others.
                                        if event.click_count() == 2 {
                                            view.preview_equalizer_gain(index, 0, cx);
                                            view.commit_equalizer(cx);
                                        }
                                    },
                                ))
                                .child(
                                    div()
                                        .absolute()
                                        .left(relative(0.5))
                                        .h_full()
                                        .w(px(4.0))
                                        .rounded_full()
                                        .bg(rgb(theme.border)),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .top(relative(0.5))
                                        .w_full()
                                        .h(px(1.0))
                                        .bg(rgb(theme.border)),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left(relative(0.5))
                                        .ml(px(-9.0))
                                        .top(relative((12.0 - f32::from(gain)) / 24.0))
                                        .mt(px(-4.0))
                                        .w(px(18.0))
                                        .h(px(8.0))
                                        .rounded_md()
                                        .bg(rgb(theme.accent)),
                                ),
                        )
                        .child(div().text_xs().child(label))
                })),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme.muted))
                .child("Hz · Drag a band to adjust. Double-click to set it to 0 dB."),
        )
}

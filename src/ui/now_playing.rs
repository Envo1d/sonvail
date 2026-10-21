use super::{
    components::{duration_label, track_artwork, track_info},
    icons::icon,
    layout::Layout,
    root::{Page, RootView},
    theme::Palette,
};
use crate::playback::Command;
use gpui::{Context, FontWeight, Styled, div, prelude::*, px, rgb, rgba, uniform_list};

pub fn now_playing(
    view: &RootView,
    theme: Palette,
    layout: Layout,
    cx: &Context<RootView>,
) -> impl IntoElement {
    let state = &view.playback.snapshot;
    let track = state.track();
    let directory = view.config.library_path().with_file_name("artwork");
    let entity = cx.entity();
    div()
        .id("now-playing")
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .px(px(layout.padding))
        .pb_6()
        .child(
            div()
                .text_sm()
                .text_color(rgb(theme.accent))
                .mb_6()
                .child("NOW PLAYING"),
        )
        .when(track.is_none(), |this| {
            this.child(div().text_size(px(28.0)).child("Choose your music"))
                .child(
                    div()
                        .mt_3()
                        .text_color(rgb(theme.muted))
                        .child("Open Library and select a track to start listening."),
                )
                .child(
                    div()
                        .id("now-playing-library")
                        .mt_6()
                        .text_color(rgb(theme.accent))
                        .cursor_pointer()
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.page = Page::Library;
                            cx.notify();
                        }))
                        .child("Open Library"),
                )
        })
        .when_some(track, |this, track| {
            let metadata = &track.metadata;
            let mut badges = Vec::new();
            if let Some(date) = &metadata.release_date {
                badges.push(date.clone());
            }
            badges.extend(metadata.genres.iter().take(2).cloned());
            badges.push(format!("{} Tracks", state.tracks.len()));
            let progress = state
                .duration
                .filter(|duration| !duration.is_zero())
                .map_or(0.0, |duration| {
                    (state.position.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
                });
            this.child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_start()
                    .gap(px(layout.hero_gap))
                    .min_w_0()
                    .child(track_artwork(
                        theme,
                        Some(track),
                        &directory,
                        layout.artwork,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .pt(px(layout.hero_text_top))
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .text_size(px(32.0))
                                    .line_height(px(38.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(track.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(20.0))
                                    .text_color(rgb(theme.accent))
                                    .truncate()
                                    .child(track.artist()),
                            )
                            .child(
                                div()
                                    .text_color(rgb(theme.muted))
                                    .truncate()
                                    .child(track.album().to_owned()),
                            )
                            .child(div().flex().flex_wrap().gap_2().children(
                                badges.into_iter().map(|badge| {
                                    div()
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .bg(rgba((theme.text << 8) | 0x08))
                                        .text_xs()
                                        .text_color(rgb(theme.muted))
                                        .child(badge)
                                }),
                            ))
                            // A segmented playback timeline, not fabricated audio analysis.
                            .child(
                                div()
                                    .h(px(28.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(2.0))
                                    .overflow_hidden()
                                    .children((0..64).map(|index| {
                                        div()
                                            .flex_1()
                                            .min_w(px(2.0))
                                            .max_w(px(4.0))
                                            .h(px(12.0))
                                            .rounded_full()
                                            .bg(rgb(if (index as f32) < progress * 64.0 {
                                                theme.accent
                                            } else {
                                                theme.border
                                            }))
                                    })),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .id("now-playing-toggle")
                                            .h(px(39.0))
                                            .px_6()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .gap_2()
                                            .rounded_md()
                                            .bg(rgb(theme.accent))
                                            .text_color(rgb(0xffffff))
                                            .cursor_pointer()
                                            .on_click(cx.listener(|view, _, _, cx| {
                                                view.toggle_playback(cx)
                                            }))
                                            .child(
                                                icon(
                                                    theme,
                                                    if view.is_playing() {
                                                        "pause"
                                                    } else {
                                                        "play"
                                                    },
                                                )
                                                .size(px(16.0))
                                                .text_color(rgb(0xffffff)),
                                            )
                                            .child(if view.is_playing() {
                                                "Pause"
                                            } else {
                                                "Play"
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id("now-playing-stop")
                                            .h(px(39.0))
                                            .w(px(48.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_md()
                                            .bg(rgba((theme.text << 8) | 0x08))
                                            .cursor_pointer()
                                            .on_click(cx.listener(|view, _, _, cx| {
                                                view.playback_command(Command::Stop, cx)
                                            }))
                                            .child(icon(theme, "stop").size(px(16.0))),
                                    )
                                    .child(
                                        div()
                                            .id("now-playing-details")
                                            .h(px(39.0))
                                            .w(px(48.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_md()
                                            .bg(rgba((theme.text << 8) | 0x08))
                                            .cursor_pointer()
                                            .on_click(cx.listener(|view, _, _, cx| {
                                                view.playback.show_track_details =
                                                    !view.playback.show_track_details;
                                                cx.notify();
                                            }))
                                            .child(icon(theme, "more")),
                                    ),
                            ),
                    ),
            )
            .when(view.playback.show_track_details, |this| {
                this.child(
                    div()
                        .id("track-details")
                        .max_h(px(160.0))
                        .overflow_y_scroll()
                        .mt_4()
                        .text_sm()
                        .text_color(rgb(theme.muted))
                        .child(div().child(format!(
                                "Audio: {}",
                                [
                                    metadata.sample_rate.map(|rate| format!("{rate} Hz")),
                                    metadata.bit_depth.map(|depth| format!("{depth} bit")),
                                    metadata.channels.map(|channels| format!("{channels} ch")),
                                    metadata.bitrate_kbps.map(|rate| format!("{rate} kbps"))
                                ]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join(" · ")
                            )))
                        .child(div().mt_2().child(format!(
                            "File: {}",
                            super::library::display_path(&track.path)
                        )))
                        .when(!metadata.album_artists.is_empty(), |this| {
                            this.child(div().mt_2().child(format!(
                                "Album artist: {}",
                                metadata.album_artists.join(", ")
                            )))
                        })
                        .when(!metadata.composers.is_empty(), |this| {
                            this.child(
                                div()
                                    .mt_2()
                                    .child(format!("Composer: {}", metadata.composers.join(", "))),
                            )
                        })
                        .when_some(metadata.publisher.as_ref(), |this, value| {
                            this.child(div().mt_2().child(format!("Publisher: {value}")))
                        })
                        .when_some(metadata.copyright.as_ref(), |this, value| {
                            this.child(div().mt_2().child(value.clone()))
                        })
                        .when_some(metadata.original_release_date.as_ref(), |this, value| {
                            this.child(div().mt_2().child(format!("Original release: {value}")))
                        })
                        .when_some(metadata.error.as_ref(), |this, value| {
                            this.child(div().mt_2().child(value.clone()))
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .mt_6()
                    .px_3()
                    .py_3()
                    .border_b_1()
                    .border_color(rgba(theme.hairline))
                    .child(
                        div()
                            .w(px(26.0))
                            .text_xs()
                            .text_color(rgb(theme.muted))
                            .child("#"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(rgb(theme.muted))
                            .child("TITLE"),
                    )
                    .child(icon(theme, "clock").size(px(15.0)))
                    .child(div().w(px(16.0))),
            )
            .child(
                uniform_list(
                    "now-playing-tracks",
                    state.tracks.len(),
                    move |range, _, cx| {
                        let view = entity.read(cx);
                        let snapshot = &view.playback.snapshot;
                        range
                            .filter_map(|index| {
                                snapshot.tracks.get(index).map(|track| (index, track))
                            })
                            .map(|(index, track)| {
                                let entity = entity.clone();
                                let selected = snapshot.current == Some(index);
                                div()
                                    .id(("now-playing-track", index))
                                    .h(px(55.0))
                                    .w_full()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when(selected, |this| this.bg(rgba((theme.text << 8) | 0x08)))
                                    .when(!selected, |this| {
                                        this.border_b_1().border_color(rgba(theme.hairline))
                                    })
                                    .hover(move |style| style.bg(rgba((theme.text << 8) | 0x0c)))
                                    .on_click(move |_, _, cx| {
                                        entity.update(cx, |view, cx| {
                                            view.playback_command(Command::Select(index), cx)
                                        })
                                    })
                                    .child(
                                        div()
                                            .w(px(26.0))
                                            .flex_shrink_0()
                                            .text_xs()
                                            .text_color(rgb(theme.muted))
                                            .when(selected, |this| {
                                                this.child(
                                                    icon(theme, "playing")
                                                        .size(px(16.0))
                                                        .text_color(rgb(theme.accent)),
                                                )
                                            })
                                            .when(!selected, |this| {
                                                this.child((index + 1).to_string())
                                            }),
                                    )
                                    .child(track_info(theme, &track.title, &track.artist()))
                                    .child(
                                        div()
                                            .w(px(48.0))
                                            .flex_shrink_0()
                                            .text_right()
                                            .text_xs()
                                            .text_color(rgb(if selected {
                                                theme.accent
                                            } else {
                                                theme.muted
                                            }))
                                            .child(duration_label(track.metadata.duration_ms)),
                                    )
                                    .child(div().w(px(16.0)))
                            })
                            .collect()
                    },
                )
                .track_scroll(view.playback.track_scroll.clone())
                .flex_1()
                .min_h_0(),
            )
        })
}

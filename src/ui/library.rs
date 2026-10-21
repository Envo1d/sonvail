use gpui::{Context, FontWeight, Styled, Task, div, prelude::*, px, rgb, rgba, uniform_list};
use std::sync::Arc;

use crate::library::{
    Track,
    service::{LibraryService, Update},
};

use super::{
    components::{duration_label, track_artwork},
    layout::Layout,
    root::{Page, RootView},
    theme::Palette,
};

#[derive(Default)]
pub(super) struct Library {
    pub(super) tracks: Arc<Vec<Track>>,
    errors: usize,
    message: Option<String>,
    scanning: bool,
    revision: u64,
    service: Option<LibraryService>,
    subscription: Option<Task<()>>,
}

impl RootView {
    pub(super) fn start_library(&mut self, cx: &mut Context<Self>) {
        self.library.scanning = true;
        match LibraryService::start(
            self.config.library_path().to_owned(),
            self.config.snapshot().library_folders.clone(),
        ) {
            Ok((service, updates)) => {
                self.library.service = Some(service);
                self.library.subscription = Some(cx.spawn(async move |view, cx| {
                    while let Ok(update) = updates.recv().await {
                        if view
                            .update(cx, |view, cx| {
                                match update {
                                    Update::Snapshot(snapshot)
                                        if snapshot.revision == view.library.revision =>
                                    {
                                        view.library.tracks = Arc::new(snapshot.tracks);
                                        view.library.errors = snapshot.errors;
                                        view.library.message = snapshot.warning;
                                        view.library.scanning = snapshot.scanning;
                                        view.playback_command(
                                            crate::playback::Command::RefreshLibrary(
                                                view.library.tracks.clone(),
                                            ),
                                            cx,
                                        );
                                    }
                                    Update::Failed { revision, message } => {
                                        tracing::error!(revision, "Library worker stopped");
                                        view.library.message = Some(message);
                                        view.library.scanning = false;
                                    }
                                    _ => return,
                                }
                                cx.notify();
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }));
            }
            Err(error) => {
                tracing::error!(%error, "Cannot start library");
                self.library.message = Some(format!(
                    "Cannot start library: {error}. Restart Sonvail to try again."
                ));
                self.library.scanning = false;
            }
        }
    }

    pub(super) fn update_library_sources(&mut self, cx: &mut Context<Self>) {
        self.library.revision += 1;
        self.library.tracks = Arc::default();
        self.library.scanning = true;
        if let Some(service) = &self.library.service {
            if let Err(error) = service.set_sources(
                self.library.revision,
                self.config.snapshot().library_folders.clone(),
            ) {
                self.library.message = Some(format!(
                    "{error}. Restart Sonvail to reconnect the library."
                ));
                self.library.scanning = false;
            }
        } else {
            self.library.scanning = false;
        }
        cx.notify();
    }
}

pub(super) fn library_view(
    view: &RootView,
    theme: Palette,
    layout: Layout,
    cx: &Context<RootView>,
) -> impl IntoElement {
    let library = &view.library;
    let entity = cx.entity();
    div().size_full().min_w_0().flex().flex_col()
        .px(px(layout.padding)).pb_6().gap_4()
        .child(div()
            .child(div().text_size(px(30.0)).font_weight(FontWeight::SEMIBOLD).child("Library"))
            .child(div().text_sm().text_color(rgb(theme.muted)).child(
                if library.scanning { format!("{} tracks · Checking music folders…", library.tracks.len()) }
                else { format!("{} tracks · MP3, WAV, FLAC, OGG", library.tracks.len()) }
            )))
        .when_some(library.message.as_ref(), |this, message| {
            this.child(div().text_sm().text_color(rgb(theme.muted)).child(message.clone()))
        })
        .when(library.errors > 0, |this| {
            this.child(div().text_sm().text_color(rgb(theme.muted)).child(format!(
                "{} entries could not be scanned. Previously indexed files are kept for unavailable folders. Check the logs.", library.errors
            )))
        })
        .when(!library.scanning && library.tracks.is_empty(), |this| {
            this.child(div().p_6().rounded_lg().border_1().border_color(rgba(theme.hairline))
                .child(if view.config.snapshot().library_folders.is_empty() { "Add music folders in Settings to start your library." }
                    else { "No supported audio files found. New files will appear automatically." })
                .child(div().id("library-settings").mt_4().text_color(rgb(theme.accent)).cursor_pointer()
                    .on_click(cx.listener(|view, _, _, cx| { view.page = Page::Settings; cx.notify(); }))
                    .child("Open Settings")))
        })
        .child(uniform_list("library-tracks", library.tracks.len(), move |range, _, cx| {
            let view = entity.read(cx);
            range.filter_map(|index| view.library.tracks.get(index).map(|track| (index, track))).map(|(index, track)| {
                let root = entity.clone();
                let tracks = view.library.tracks.clone();
                let artwork = view.config.library_path().with_file_name("artwork");
                let selected = view.playback.snapshot.track().is_some_and(|playing| playing.path == track.path);
                div().id(("library-track", index)).h(px(90.0)).w_full().min_w_0().flex().items_center().gap_3().cursor_pointer()
                    .px_3().border_b_1().border_color(rgba(theme.hairline))
                    .when(selected, |this| this.bg(rgba((theme.accent << 8) | 0x18)))
                    .hover(move |style| style.bg(rgba((theme.text << 8) | 0x0c)))
                    .on_click(move |_, _, cx| root.update(cx, |view, cx| {
                        view.playback_command(crate::playback::Command::Play { tracks: tracks.clone(), index }, cx);
                    }))
                    .child(track_artwork(theme, Some(track), &artwork, 52.0))
                    .child(div().flex_1().min_w_0()
                        .child(div().truncate().child(track.title.clone()))
                        .child(div().truncate().text_sm().text_color(rgb(theme.muted)).child(format!("{} · {}", track.artist(), track.album())))
                        .child(div().truncate().text_xs().text_color(rgb(theme.muted)).child(display_path(&track.path))))
                    .child(div().w(px(130.0)).truncate().text_sm().text_color(rgb(theme.muted)).child(track.metadata.release_date.clone().unwrap_or_default()))
                    .child(div().w(px(120.0)).truncate().text_sm().text_color(rgb(theme.muted)).child(track.metadata.genres.join(", ")))
                    .child(div().w(px(56.0)).text_right().text_sm().child(duration_label(track.metadata.duration_ms)))
            }).collect()
        }).flex_1().min_h_0())
}

// Canonical paths retain their Windows extended prefix for filesystem access.
// Only ordinary disk/UNC paths get a friendlier representation in the UI.
pub(super) fn display_path(path: &std::path::Path) -> String {
    let display = path.display().to_string();
    #[cfg(windows)]
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        match prefix.kind() {
            std::path::Prefix::VerbatimDisk(_) => return display[4..].to_owned(),
            std::path::Prefix::VerbatimUNC(_, _) => {
                return format!(r"\\{}", &display[8..]);
            }
            _ => {}
        }
    }
    display
}

#[cfg(all(test, windows))]
mod tests {
    use super::display_path;
    use std::path::Path;

    #[test]
    fn formats_windows_paths_without_changing_device_namespaces() {
        for (path, expected) in [
            (r"\\?\C:\Music\Музика\song.mp3", r"C:\Music\Музика\song.mp3"),
            (r"\\?\UNC\server\share\song.mp3", r"\\server\share\song.mp3"),
            (r"C:\Music\song.mp3", r"C:\Music\song.mp3"),
            (r"\\server\share\song.mp3", r"\\server\share\song.mp3"),
            (
                r"\\?\Volume{example}\song.mp3",
                r"\\?\Volume{example}\song.mp3",
            ),
            (r"\\.\device", r"\\.\device"),
        ] {
            assert_eq!(display_path(Path::new(path)), expected);
        }
    }
}

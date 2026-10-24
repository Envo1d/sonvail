use std::path::PathBuf;

use crate::config::AppConfig;
use gpui::{Context, Div, FontWeight, PathPromptOptions, Styled, div, prelude::*, px, rgb, rgba};

use super::{
    icons::icon,
    layout::Layout,
    root::RootView,
    theme::{Palette, ThemeMode},
};

#[derive(Default)]
pub(super) struct Settings {
    pub theme: ThemeMode,
    pub equalizer: crate::equalizer::EqualizerSettings,
    folders: Vec<LibraryFolder>,
    choosing_folders: bool,
    message: Option<String>,
}

struct LibraryFolder {
    path: PathBuf,
    canonical_path: PathBuf,
}

impl Settings {
    pub(super) fn from_config(config: &AppConfig) -> Self {
        Self {
            theme: config.theme,
            equalizer: config.equalizer,
            folders: config
                .library_folders
                .iter()
                .map(|path| LibraryFolder {
                    path: path.clone(),
                    // Keep unavailable sources so removable drives survive restarts.
                    canonical_path: path.canonicalize().unwrap_or_else(|_| path.clone()),
                })
                .collect(),
            ..Self::default()
        }
    }

    fn add_folders(&mut self, folders: Vec<LibraryFolder>) {
        for folder in folders {
            if !self
                .folders
                .iter()
                .any(|existing| existing.canonical_path == folder.canonical_path)
            {
                self.folders.push(folder);
            }
        }
    }
}

impl RootView {
    pub(super) fn save_settings(&mut self, cx: &mut Context<Self>) {
        let previous_folders = self.config.snapshot().library_folders.clone();
        let draft = AppConfig {
            theme: self.settings.theme,
            equalizer: self.settings.equalizer,
            library_folders: self
                .settings
                .folders
                .iter()
                .map(|folder| folder.path.clone())
                .collect(),
        };
        match self.config.save(draft) {
            Ok(()) => self.settings.message = None,
            Err(error) => {
                tracing::error!(error = %format!("{error:#}"), "Settings change was not saved");
                let previous = Settings::from_config(self.config.snapshot());
                self.settings.theme = previous.theme;
                self.settings.equalizer = previous.equalizer;
                self.settings.folders = previous.folders;
                self.settings.message = Some("Could not save settings. The change was reverted. Check the logs and try again.".into());
            }
        }
        if previous_folders != self.config.snapshot().library_folders {
            self.update_library_sources(cx);
        }
        self.playback_command(
            crate::playback::Command::Equalizer(self.settings.equalizer),
            cx,
        );
        cx.notify();
    }

    fn choose_library_folders(&mut self, cx: &mut Context<Self>) {
        if self.settings.choosing_folders {
            return;
        }
        self.settings.choosing_folders = true;
        self.settings.message = None;
        cx.notify();
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Add music folders".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = selection.await;
            let (folders, message) = match result {
                Ok(Ok(Some(paths))) => {
                    // Resolve aliases and validate paths away from the UI thread.
                    cx.background_executor()
                        .spawn(async move {
                            let mut folders = Vec::new();
                            let mut message = None;
                            for path in paths {
                                match path.canonicalize() {
                                    Ok(canonical_path) if canonical_path.is_dir() => {
                                        folders.push(LibraryFolder { path, canonical_path });
                                    }
                                    _ => {
                                        tracing::warn!(path = %path.display(), "Library folder is unavailable");
                                        message = Some("Some folders could not be added. Check that they are available and try again.");
                                    }
                                }
                            }
                            (folders, message)
                        })
                        .await
                }
                Ok(Ok(None)) => (Vec::new(), None),
                error => {
                    tracing::error!(?error, "Folder picker failed");
                    (Vec::new(), Some("Could not open the folder picker. Please try again."))
                }
            };
            let _ = view.update(cx, |view, cx| {
                view.settings.add_folders(folders);
                view.settings.choosing_folders = false;
                view.save_settings(cx);
                if view.settings.message.is_none() {
                    view.settings.message = message.map(str::to_owned);
                }
                cx.notify();
            });
        })
        .detach();
    }
}

pub(super) fn settings_view(
    view: &RootView,
    theme: Palette,
    layout: Layout,
    cx: &Context<RootView>,
) -> impl IntoElement {
    div()
        .id("settings-view")
        .size_full()
        .min_w_0()
        .overflow_y_scroll()
        .px(px(layout.padding))
        .pb_8()
        .child(
            div()
                .w_full()
                .max_w(px(layout.settings_max_width))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_6()
                .child(
                    div()
                        .child(div().text_size(px(30.0)).font_weight(FontWeight::SEMIBOLD).child("Settings"))
                        .child(description(theme, "Make Sonvail feel like home.")),
                )
                .child(
                    section(theme)
                        .child(section_title("Appearance"))
                        .child(description(theme, "Choose your theme. System follows your device's appearance."))
                        .child(
                            div()
                                .mt_4()
                                .flex()
                                .gap_1()
                                .p_1()
                                .rounded_lg()
                                .border_1()
                                .border_color(rgb(theme.border))
                                .children([
                                    (ThemeMode::Dark, "Dark"),
                                    (ThemeMode::Light, "Light"),
                                    (ThemeMode::System, "System"),
                                ].into_iter().enumerate().map(|(index, (mode, title))| {
                                    let selected = view.settings.theme == mode;
                                    div()
                                        .id(("theme-mode", index))
                                        .flex_1()
                                        .h(px(42.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_color(rgb(if selected { 0xffffff } else { theme.muted }))
                                        .when(selected, |this| this.bg(rgb(theme.accent)))
                                        .when(!selected, |this| this.hover(move |style| style.bg(rgb(theme.surface))))
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.settings.theme = mode;
                                            view.save_settings(cx);
                                        }))
                                        .child(title)
                                })),
                        ),
                )
                .child(
                    section(theme)
                        .child(section_title("Music library"))
                        .child(description(theme, "Choose the folders Sonvail will scan for music."))
                        .child(
                            div()
                                .mt_4()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_4()
                                .child(description(theme, &format!("{} folders added", view.settings.folders.len())))
                                .child(
                                    div()
                                        .id("add-library-folders")
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .px_4()
                                        .py_2()
                                        .rounded_md()
                                        .bg(rgb(theme.accent))
                                        .text_color(rgb(0xffffff))
                                        .when(view.settings.choosing_folders, |this| this.opacity(0.6))
                                        .when(!view.settings.choosing_folders, |this| this.cursor_pointer())
                                        .on_click(cx.listener(|view, _, _, cx| view.choose_library_folders(cx)))
                                        .child(icon(theme, "plus").size(px(16.0)).text_color(rgb(0xffffff)))
                                        .child(if view.settings.choosing_folders { "Choosing folders..." } else { "Add folders" }),
                                ),
                        )
                        .when(view.settings.folders.is_empty(), |this| {
                            this.child(
                                div()
                                    .mt_4()
                                    .p_6()
                                    .border_1()
                                    .border_color(rgb(theme.border))
                                    .rounded_md()
                                    .child(div().child("Your library starts here"))
                                    .child(description(theme, "Add one or more music folders to get started.")),
                            )
                        })
                        .children(view.settings.folders.iter().enumerate().map(|(index, folder)| {
                            let path = folder.path.clone();
                            div()
                                .mt_3()
                                .flex()
                                .items_center()
                                .gap_3()
                                .p_3()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(theme.border))
                                .child(icon(theme, "library").text_color(rgb(theme.accent)))
                                .child(div().flex_1().min_w_0().overflow_hidden().text_sm().child(folder.path.display().to_string()))
                                .child(
                                    div()
                                        .id(("remove-library-folder", index))
                                        .flex_shrink_0()
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .text_sm()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(rgb(theme.surface)))
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.settings.folders.retain(|folder| folder.path != path);
                                            view.save_settings(cx);
                                        }))
                                        .child("Remove"),
                                )
                        }))
                        .child(div().mt_4().child(description(theme, "Folders are scanned at startup. Library updates automatically when files change."))),
                )
                .child(section(theme).child(super::equalizer::equalizer_view(view, theme, layout, cx)))
                .when_some(view.settings.message.as_deref(), |this, message| this.child(description(theme, message)))
                .child(section(theme).child(super::updater::update_settings(view, theme, cx)))
                .child(description(theme, "Changes are saved automatically.")),
        )
}

fn section(theme: Palette) -> Div {
    div()
        .p_6()
        .rounded_lg()
        .border_1()
        .border_color(rgba(theme.hairline))
}

fn section_title(title: &'static str) -> Div {
    div()
        .text_size(px(18.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(title)
}

fn description(theme: Palette, text: &str) -> Div {
    div()
        .mt_2()
        .text_sm()
        .text_color(rgb(theme.muted))
        .child(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(path: &str, canonical: &str) -> LibraryFolder {
        LibraryFolder {
            path: path.into(),
            canonical_path: canonical.into(),
        }
    }

    #[test]
    fn repeated_selection_and_aliases_do_not_duplicate_folders() {
        let mut settings = Settings::default();
        settings.add_folders(vec![
            folder("Music", "canonical-music"),
            folder("Other", "canonical-other"),
        ]);
        settings.add_folders(vec![
            folder("Music shortcut", "canonical-music"),
            folder("New", "canonical-new"),
            folder("New", "canonical-new"),
        ]);
        assert_eq!(
            settings
                .folders
                .iter()
                .map(|folder| folder.path.clone())
                .collect::<Vec<_>>(),
            vec![
                PathBuf::from("Music"),
                PathBuf::from("Other"),
                PathBuf::from("New")
            ]
        );
    }
}

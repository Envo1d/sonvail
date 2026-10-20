use std::borrow::Cow;

use gpui::{AssetSource, SharedString, Styled, Svg, px, rgb, svg};

use super::theme;

// Small, embedded vector assets keep the shell independent of the working directory.
const ICONS: &[(&str, &str)] = &[
    ("pause", r#"<path d="M8 5v14M16 5v14" stroke-width="3.5"/>"#),
    (
        "heart-filled",
        r#"<path fill="black" d="M20.5 5.5a5 5 0 0 0-7 0L12 7l-1.5-1.5a5 5 0 0 0-7 7L12 21l8.5-8.5a5 5 0 0 0 0-7Z"/>"#,
    ),
    ("minimize", r#"<path d="M4 12h16"/>"#),
    (
        "maximize",
        r#"<rect x="4" y="4" width="16" height="16" rx="1"/>"#,
    ),
    (
        "restore",
        r#"<path d="M8 4h12v12M8 4v4m12 8h-4"/><rect x="4" y="8" width="12" height="12" rx="1"/>"#,
    ),
    ("close", r#"<path d="m5 5 14 14M19 5 5 19"/>"#),
    (
        "search",
        r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>"#,
    ),
    ("playing", r#"<path d="M5 15V9m7 11V4m7 13V7"/>"#),
    (
        "library",
        r#"<rect x="4" y="4" width="16" height="17" rx="2"/><path d="M8 2v4m8-4v4M8 11h8m-8 4h5"/>"#,
    ),
    (
        "playlist",
        r#"<path d="M4 5h12M4 10h9M4 15h6m8-5v9m0-7 4-1"/><circle cx="15" cy="19" r="3"/>"#,
    ),
    (
        "album",
        r#"<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="3"/>"#,
    ),
    (
        "artist",
        r#"<circle cx="12" cy="7" r="4"/><path d="M4 21v-2a8 8 0 0 1 16 0v2"/>"#,
    ),
    (
        "heart",
        r#"<path d="M20.5 5.5a5 5 0 0 0-7 0L12 7l-1.5-1.5a5 5 0 0 0-7 7L12 21l8.5-8.5a5 5 0 0 0 0-7Z"/>"#,
    ),
    ("plus", r#"<path d="M12 5v14M5 12h14"/>"#),
    (
        "settings",
        r#"<path d="m9 3-1 3-3 1-2 5 2 5 3 1 1 3h6l1-3 3-1 2-5-2-5-3-1-1-3Z"/><circle cx="12" cy="12" r="3"/>"#,
    ),
    (
        "sun",
        r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>"#,
    ),
    (
        "more",
        r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#,
    ),
    ("play", r#"<path d="m9 5 11 7-11 7Z"/>"#),
    ("previous", r#"<path d="M5 5v14m14-14L8 12l11 7Z"/>"#),
    ("next", r#"<path d="M19 5v14M5 5l11 7-11 7Z"/>"#),
    (
        "shuffle",
        r#"<path d="M3 6h3c5 0 7 12 12 12h3m-4-4 4 4-4 4M3 18h3c2 0 4-3 6-6s4-6 6-6h3m-4-4 4 4-4 4"/>"#,
    ),
    (
        "repeat",
        r#"<path d="m17 2 4 4-4 4M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4m14-1v2a3 3 0 0 1-3 3H3"/>"#,
    ),
    (
        "volume",
        r#"<path d="M4 9v6h4l5 4V5L8 9Zm12-1a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14"/>"#,
    ),
    (
        "device",
        r#"<rect x="3" y="3" width="13" height="17" rx="2"/><rect x="13" y="10" width="8" height="11" rx="2"/><path d="M7 7h5m4 10h2"/>"#,
    ),
    (
        "expand",
        r#"<path d="M9 3H3v6m12-6h6v6M3 15v6h6m12-6v6h-6"/>"#,
    ),
    ("grip", r#"<path d="M5 8h14M5 12h14M5 16h14"/>"#),
    (
        "clock",
        r#"<circle cx="12" cy="12" r="9"/><path d="M12 6v6l4 2"/>"#,
    ),
];

pub struct Assets;

pub const APP_ICON_SIZE: f32 = 32.0;
pub const ALBUM_ART: &str = "artwork/twilight-ring.png";
const ALBUM_BYTES: &[u8] = include_bytes!("../../assets/artwork/twilight-ring.png");
const APP_ICONS: &[(u32, &str, &[u8])] = &[
    (
        32,
        "icons/soundora-32.png",
        include_bytes!("../../assets/icons/soundora-32.png"),
    ),
    (
        64,
        "icons/soundora-64.png",
        include_bytes!("../../assets/icons/soundora-64.png"),
    ),
    (
        128,
        "icons/soundora-128.png",
        include_bytes!("../../assets/icons/soundora-128.png"),
    ),
    (
        256,
        "icons/soundora-256.png",
        include_bytes!("../../assets/icons/soundora-256.png"),
    ),
];

pub fn app_icon_path(scale_factor: f32) -> &'static str {
    let physical_size = APP_ICON_SIZE * scale_factor.max(1.0);
    APP_ICONS
        .iter()
        .find(|(size, _, _)| *size as f32 >= physical_size)
        .unwrap_or(&APP_ICONS[APP_ICONS.len() - 1])
        .1
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if path == ALBUM_ART {
            return Ok(Some(Cow::Borrowed(ALBUM_BYTES)));
        }
        if let Some((_, _, bytes)) = APP_ICONS.iter().find(|(_, name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        let Some(name) = path
            .strip_prefix("icons/")
            .and_then(|p| p.strip_suffix(".svg"))
        else {
            return Ok(None);
        };
        Ok(ICONS.iter().find(|(key, _)| *key == name).map(|(_, body)| {
            Cow::Owned(format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#
            ).into_bytes())
        }))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .map(|(name, _)| format!("icons/{name}.svg"))
            .chain(APP_ICONS.iter().map(|(_, path, _)| (*path).to_owned()))
            .chain(std::iter::once(ALBUM_ART.to_owned()))
            .filter(|name| name.starts_with(path))
            .map(Into::into)
            .collect())
    }
}

pub fn icon(name: &'static str) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(px(19.0))
        .flex_shrink_0()
        .text_color(rgb(theme::MUTED))
}

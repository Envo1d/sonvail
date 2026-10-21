use gpui::WindowAppearance;

pub use crate::config::ThemeMode;

impl ThemeMode {
    pub fn palette(self, appearance: WindowAppearance) -> Palette {
        match self {
            Self::Light => LIGHT,
            Self::Dark => DARK,
            Self::System => match appearance {
                WindowAppearance::Light | WindowAppearance::VibrantLight => LIGHT,
                WindowAppearance::Dark | WindowAppearance::VibrantDark => DARK,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub surface: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub selected: u32,
    pub window_tint: u32,
    pub sidebar_tint: u32,
    pub player_tint: u32,
    pub hairline: u32,
}

// Alpha is intentional: opaque root and panel fills would conceal native acrylic.
const DARK: Palette = Palette {
    surface: 0x182230,
    border: 0x1c2736,
    text: 0xe8edf5,
    muted: 0x929dab,
    accent: 0x2f6bff,
    selected: 0x1b2c48,
    window_tint: 0x081321c2,
    sidebar_tint: 0x141a2366,
    player_tint: 0x18202e80,
    hairline: 0xffffff0c,
};

const LIGHT: Palette = Palette {
    surface: 0xe4eaf3,
    border: 0xc5cede,
    text: 0x182438,
    muted: 0x52617a,
    accent: 0x245de0,
    selected: 0xd3e1fa,
    window_tint: 0xf4f7fce0,
    sidebar_tint: 0xe2e9f366,
    player_tint: 0xe4eaf399,
    hairline: 0x18243818,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_follows_all_native_appearances() {
        for appearance in [WindowAppearance::Light, WindowAppearance::VibrantLight] {
            assert_eq!(ThemeMode::System.palette(appearance).text, LIGHT.text);
        }
        for appearance in [WindowAppearance::Dark, WindowAppearance::VibrantDark] {
            assert_eq!(ThemeMode::System.palette(appearance).text, DARK.text);
        }
    }

    #[test]
    fn explicit_modes_ignore_system_appearance() {
        assert_eq!(
            ThemeMode::Dark.palette(WindowAppearance::Light).text,
            DARK.text
        );
        assert_eq!(
            ThemeMode::Light.palette(WindowAppearance::Dark).text,
            LIGHT.text
        );
    }
}

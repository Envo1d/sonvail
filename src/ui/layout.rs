// Dimensions follow the reference window while keeping the minimum window usable.
#[derive(Clone, Copy)]
pub struct Layout {
    pub sidebar: f32,
    pub queue: f32,
    pub artwork: f32,
    pub padding: f32,
    pub settings_max_width: f32,
    pub hero_gap: f32,
    pub hero_text_top: f32,
}

impl Layout {
    pub fn for_width(width: f32) -> Self {
        let sidebar = (width * 0.188).clamp(232.0, 268.0);
        let queue = (width * 0.278).clamp(300.0, 394.0);
        let padding = if width < 1280.0 { 24.0 } else { 34.0 };
        let content = width - sidebar - queue - padding * 2.0;
        Self {
            sidebar,
            queue,
            artwork: (content * 0.4).clamp(196.0, 276.0),
            padding,
            settings_max_width: 880.0,
            hero_gap: if width < 1280.0 { 22.0 } else { 32.0 },
            hero_text_top: if width < 1280.0 { 16.0 } else { 44.0 },
        }
    }
}

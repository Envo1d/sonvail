use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

use crate::ui::{Assets, RootView};

pub fn run() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1440.0), px(940.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1120.0), px(720.0))),
                window_background: WindowBackgroundAppearance::Blurred,
                titlebar: Some(TitlebarOptions {
                    title: Some("Soundora".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| RootView),
        )
        .expect("failed to open the Soundora window");

        cx.activate(true);
    });
}

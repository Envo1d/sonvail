use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

use crate::{
    config::ConfigStore,
    ui::{Assets, RootView},
};

pub fn run(store: ConfigStore) {
    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            cx.on_app_quit(move |_| {
                tracing::info!("Shutting down Soundora");
                std::future::ready(())
            })
            .detach();
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
                |window, cx| {
                    window.on_window_should_close(cx, |_, cx| {
                        if cx.windows().len() == 1 {
                            // Quit while the native window is still alive. GPUI 0.2.2's
                            // WM_CLOSE path otherwise destroys HWND before deferred
                            // activation callbacks and WindowsWindow::drop finish.
                            tracing::info!("Main window close requested");
                            cx.quit();
                            false
                        } else {
                            true
                        }
                    });
                    cx.new(|cx| RootView::new(store, window, cx))
                },
            )
            .expect("failed to open the Soundora window");

            cx.activate(true);
        });
}

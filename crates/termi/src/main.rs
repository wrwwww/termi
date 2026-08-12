use std::sync::Mutex;

use assets::Assets;
use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use gpui_component::{Root, Theme, ThemeMode, TitleBar};
use gpui_platform;

use log::info;
use settings::Settings;
use terminal::terminal_settings::TerminalSettings;
use workspace::WorkspaceView;
fn build_application() -> Application {
    let platform = gpui_platform::current_platform(false);
    if std::env::var("ZED_EXPERIMENTAL_A11Y").as_deref() == Ok("1") {
        Application::with_platform(platform)
    } else {
        Application::new_inaccessible(platform)
    }
}

fn main() {
    // log initialization
    env_logger::init();
    let app = build_application()
        .with_assets(Assets)
        .with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        settings::init(cx);
        // extension::init(cx);
        theme_settings::init(theme::LoadThemes::All(Box::new(Assets)), cx);
        load_embedded_fonts(cx);
        let settings = TerminalSettings::get_global(cx);
        info!("terminal settings {:#?}", settings);
        gpui_component::init(cx);
        // 在应用初始化时，将主题模式切换为 Dark
        Theme::change(ThemeMode::Dark, None, cx);

        // terminal_view::init(cx);
        // theme_selector::init(cx);
        open_window(cx);
    });
}
fn load_embedded_fonts(cx: &App) {
    let asset_source = cx.asset_source();
    let font_paths = asset_source.list("fonts").unwrap();
    let embedded_fonts = Mutex::new(Vec::new());
    let executor = cx.background_executor();

    cx.foreground_executor().block_on(executor.scoped(|scope| {
        for font_path in &font_paths {
            if !font_path.ends_with(".ttf") {
                continue;
            }

            scope.spawn(async {
                let font_bytes = asset_source.load(font_path).unwrap().unwrap();
                embedded_fonts.lock().unwrap().push(font_bytes);
            });
        }
    }));

    cx.text_system()
        .add_fonts(embedded_fonts.into_inner().unwrap())
        .unwrap();
}

fn open_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1200.), px(600.0)), cx);
    cx.spawn(async move |cx| {
        let state = cx.new(|cx| workspace::state::AppState::load());

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| WorkspaceView::new(state, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
                // view
            },
        )
        .expect("Failed to open window");
    })
    .detach();
}

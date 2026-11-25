mod app;
mod assets;
mod autostart;
mod dicts;
mod home;
mod recent;
mod settings_page;
mod theme;
mod widgets;

use gpui::*;
use gpui_base::Theme;

use theme::Palette;

/// Give the text input our colours (caret, selection, placeholder) so it matches the cards around it.
pub fn apply_input_theme(p: &Palette, cx: &mut App) {
    let t = Theme::global_mut(cx);
    t.appearance = if p.dark { gpui_base::ThemeAppearance::Dark } else { gpui_base::ThemeAppearance::Light };
    let c = &mut t.tokens.colors;
    c.background = p.card;
    c.foreground = p.text;
    c.muted_foreground = p.muted;
    c.primary = p.accent;
    c.ring = p.accent;
    c.input = p.border;
    c.border = p.border;
    c.selection = Hsla { a: 0.28, ..p.accent };
}

fn main() {
    gpui_platform::application().with_assets(assets::Assets).run(|cx: &mut App| {
        gpui_base::init(cx);
        let _ = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(assets::PIXEL_FONT)]);
        let bounds = Bounds::centered(None, size(px(940.), px(660.)), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(820.), px(560.))),
            titlebar: Some(TitlebarOptions {
                title: Some("Spelly".into()),
                // macOS: draw our own top edge and keep the traffic lights.
                appears_transparent: cfg!(target_os = "macos"),
                traffic_light_position: Some(point(px(18.), px(18.))),
            }),
            ..Default::default()
        };
        // Closing the window only closes the window program. Spelly itself keeps running.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.open_window(opts, |window, cx| cx.new(|cx| app::SpellyApp::new(window, cx))).unwrap();
        cx.activate(true);
    });
}

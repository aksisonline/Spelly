use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::*;
use gpui_base::input::{Input, InputEvent, InputState};
use spelly_core::{start_of_today, LogEntry, PackFile, Settings, Store};

use crate::theme::Palette;
use crate::widgets::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Recent,
    Dictionaries,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RecentTab {
    Fixes,
    WontFix,
}

pub struct SpellyApp {
    pub store: Store,
    pub pal: Palette,
    pub page: Page,
    pub settings: Settings,
    pub dark: bool,
    pub recent: Vec<LogEntry>,
    pub today: usize,
    pub total: usize,
    pub rejected: Vec<(String, String)>,
    pub packs: Vec<PackFile>,
    pub selected_pack: usize,
    pub alive: bool,
    pub autostart: bool,
    pub recent_tab: RecentTab,
    pub creating_pack: bool,
    /// "Advanced" folds, closed by default.
    pub adv_dict: bool,
    pub adv_settings: bool,

    pub search: Entity<InputState>,
    pub add_word: Entity<InputState>,
    pub alias_wrong: Entity<InputState>,
    pub alias_right: Entity<InputState>,
    pub new_pack: Entity<InputState>,
    pub app_input: Entity<InputState>,

    pub notice: Option<(String, bool, Instant)>,
    log_len: u64,
    _subs: Vec<Subscription>,
}

/// Light or dark: the system setting (`SPELLY_THEME=light|dark` forces one, for development).
fn is_dark(window: &Window) -> bool {
    match std::env::var("SPELLY_THEME").as_deref() {
        Ok("light") => false,
        Ok("dark") => true,
        _ => matches!(window.appearance(), WindowAppearance::Dark | WindowAppearance::VibrantDark),
    }
}

fn make_input(placeholder: &'static str, window: &mut Window, cx: &mut Context<SpellyApp>) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}

impl SpellyApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = Store::open_default();
        let dark = is_dark(window);
        let settings = store.settings();
        let pal = Palette::build(settings.accent, dark);
        crate::apply_input_theme(&pal, cx);

        let search = make_input("Search this pack", window, cx);
        let add_word = make_input("Add a word or name", window, cx);
        let alias_wrong = make_input("When I type…", window, cx);
        let alias_right = make_input("…change it to", window, cx);
        let new_pack = make_input("Pack name, e.g. friends", window, cx);
        let app_input = make_input("App name, e.g. Code.exe", window, cx);

        let mut subs = vec![];
        subs.push(cx.observe_window_appearance(window, |this, window, cx| {
            this.dark = is_dark(window);
            this.sync_palette(cx);
        }));
        subs.push(cx.subscribe_in(&search, window, |_, _, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        }));
        subs.push(cx.subscribe_in(&add_word, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.submit_word(window, cx);
            }
        }));
        subs.push(cx.subscribe_in(&alias_right, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.submit_alias(window, cx);
            }
        }));
        subs.push(cx.subscribe_in(&new_pack, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.submit_pack(window, cx);
            }
        }));
        subs.push(cx.subscribe_in(&app_input, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.submit_app(window, cx);
            }
        }));

        let mut app = Self {
            store,
            pal,
            page: match std::env::var("SPELLY_PAGE").as_deref() {
                Ok("recent") => Page::Recent,
                Ok("dicts") => Page::Dictionaries,
                Ok("settings") => Page::Settings,
                _ => Page::Home,
            },
            settings,
            dark,
            recent: vec![],
            today: 0,
            total: 0,
            rejected: vec![],
            packs: vec![],
            selected_pack: 0,
            alive: false,
            autostart: crate::autostart::is_enabled(),
            recent_tab: RecentTab::Fixes,
            creating_pack: false,
            adv_dict: std::env::var_os("SPELLY_ADV").is_some(), // dev: start with the folds open
            adv_settings: std::env::var_os("SPELLY_ADV").is_some(),
            search,
            add_word,
            alias_wrong,
            alias_right,
            new_pack,
            app_input,
            notice: None,
            log_len: u64::MAX,
            _subs: subs,
        };
        app.refresh(true);

        // Once a second: see what the background app wrote.
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        app
    }

    /// Rebuild the colours from the chosen accent and the system light/dark setting.
    pub fn sync_palette(&mut self, cx: &mut Context<Self>) {
        self.pal = Palette::build(self.settings.accent, self.dark);
        crate::apply_input_theme(&self.pal, cx);
        cx.notify();
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let before = (self.alive, self.settings.clone(), self.log_len, self.packs.len());
        let accent_before = self.settings.accent;
        self.refresh(false);
        if self.settings.accent != accent_before {
            self.sync_palette(cx);
        }
        if let Some((_, _, at)) = &self.notice {
            if at.elapsed() > Duration::from_millis(3200) {
                self.notice = None;
                cx.notify();
            }
        }
        if before != (self.alive, self.settings.clone(), self.log_len, self.packs.len()) {
            cx.notify();
        }
    }

    /// Read everything from disk. `force` also re-reads packs (cheap, but not every tick).
    pub fn refresh(&mut self, force: bool) {
        self.alive = self.store.daemon_alive();
        self.settings = self.store.settings();
        self.rejected = self.store.rejected();
        let len = self.store.log_len();
        if force || len != self.log_len {
            self.log_len = len;
            self.recent = self.store.recent(300);
            self.today = self.store.fixes_since(start_of_today());
            self.total = self.store.log_count();
        }
        self.packs = self.store.pack_files();
        if self.selected_pack >= self.packs.len() {
            self.selected_pack = self.packs.len().saturating_sub(1);
        }
    }

    pub fn say(&mut self, msg: impl Into<String>, error: bool, cx: &mut Context<Self>) {
        self.notice = Some((msg.into(), error, Instant::now()));
        cx.notify();
    }

    // ---------------------------------------------------------------- actions

    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.settings.paused = paused;
        let _ = self.store.save_settings(&self.settings);
        cx.notify();
    }

    pub fn mark_right(&mut self, e: &LogEntry, cx: &mut Context<Self>) {
        self.store.add_rejected(&[(e.original.clone(), e.replacement.clone())]);
        self.rejected = self.store.rejected();
        self.say(format!("Got it. I won't change “{}” again.", e.original), false, cx);
    }

    pub fn unreject(&mut self, original: &str, replacement: &str, cx: &mut Context<Self>) {
        self.store.remove_rejected(original, replacement);
        self.rejected = self.store.rejected();
        self.say(format!("“{original}” can be fixed again."), false, cx);
    }

    pub fn is_rejected(&self, e: &LogEntry) -> bool {
        self.rejected
            .iter()
            .any(|(a, b)| a.eq_ignore_ascii_case(&e.original) && b.eq_ignore_ascii_case(&e.replacement))
    }

    /// Edit the selected pack on disk, then report.
    pub fn edit_pack(&mut self, ok_msg: &str, cx: &mut Context<Self>, f: impl FnOnce(&mut PackFile) -> Result<(), String>) {
        let Some(pack) = self.packs.get_mut(self.selected_pack) else { return };
        match f(pack).and_then(|_| pack.save().map_err(|e| e.to_string())) {
            Ok(()) => self.say(ok_msg.to_string(), false, cx),
            Err(e) => self.say(e, true, cx),
        }
        self.refresh(false);
    }

    pub fn submit_word(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let w = self.add_word.read(cx).value().to_string();
        let mut done = false;
        self.edit_pack("Added.", cx, |p| {
            p.add_word(&w)?;
            done = true;
            Ok(())
        });
        if done {
            self.add_word.update(cx, |i, cx| i.set_value("", window, cx));
        }
    }

    pub fn submit_alias(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (a, b) = (self.alias_wrong.read(cx).value().to_string(), self.alias_right.read(cx).value().to_string());
        let mut done = false;
        self.edit_pack("Correction added.", cx, |p| {
            p.add_alias(&a, &b)?;
            done = true;
            Ok(())
        });
        if done {
            self.alias_wrong.update(cx, |i, cx| i.set_value("", window, cx));
            self.alias_right.update(cx, |i, cx| i.set_value("", window, cx));
        }
    }

    pub fn submit_pack(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.new_pack.read(cx).value().to_string();
        match self.store.create_pack(&name) {
            Ok(p) => {
                self.refresh(false);
                self.selected_pack = self.packs.iter().position(|x| x.name == p.name).unwrap_or(0);
                self.creating_pack = false;
                self.new_pack.update(cx, |i, cx| i.set_value("", window, cx));
                self.say(format!("Created “{}”.", p.name), false, cx);
            }
            Err(e) => self.say(e.to_string(), true, cx),
        }
    }

    pub fn submit_app(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.app_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        if !self.settings.ignored_apps.iter().any(|a| a.eq_ignore_ascii_case(&name)) {
            self.settings.ignored_apps.push(name);
            let _ = self.store.save_settings(&self.settings);
        }
        self.app_input.update(cx, |i, cx| i.set_value("", window, cx));
        cx.notify();
    }
}

// -------------------------------------------------------------------- chrome

impl SpellyApp {
    fn nav_item(&self, page: Page, glyph: &'static str, label: &'static str, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let active = self.page == page;
        let (hover, accent_soft, text, muted, ink) = (p.hover, p.accent_soft, p.text, p.muted, p.ink);
        div()
            .id(label)
            .flex()
            .items_center()
            .gap(px(12.))
            .h(px(42.))
            .px(px(12.))
            .rounded(px(R_ROW))
            .cursor_pointer()
            .border_2()
            .border_color(if active { ink } else { gpui::transparent_black() })
            .text_size(px(14.))
            .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
            .text_color(text)
            .when(active, |d| d.bg(accent_soft).shadow(hard_shadow(&p, 3.)))
            .when(!active, |d| d.hover(move |s| s.bg(hover)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.page = page;
                this.refresh(true);
                cx.notify();
            }))
            .child(icon(glyph, 18., if active { text } else { muted }))
            .child(label)
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let top_pad = if cfg!(target_os = "macos") { 44. } else { 20. };
        let status = if self.settings.paused {
            ("Napping", p.muted, p.card)
        } else if self.alive {
            ("Running", p.ok, p.ok_soft)
        } else {
            ("Not running", p.warn, p.warn_soft)
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(236.))
            .h_full()
            .bg(p.panel)
            .border_r_2()
            .border_color(p.ink)
            .px(px(16.))
            .pt(px(top_pad))
            .pb(px(16.))
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(4.))
                    .pb(px(18.))
                    .window_control_area(WindowControlArea::Drag)
                    .child(img("mascot/kitten.png").size(px(38.)).flex_none())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(pixel("Spelly", 24., p.text))
                            .child(div().text_size(px(11.)).text_color(p.muted).child("Powered by Harper")),
                    ),
            )
            .child(self.nav_item(Page::Home, "home", "Home", cx))
            .child(self.nav_item(Page::Recent, "clock", "Recent fixes", cx))
            .child(self.nav_item(Page::Dictionaries, "book", "Dictionaries", cx))
            .child(self.nav_item(Page::Settings, "sliders", "Settings", cx))
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .h(px(34.))
                    .rounded(px(R_PILL))
                    .bg(status.2)
                    .border_2()
                    .border_color(p.border)
                    .child(div().size(px(8.)).rounded(px(R_FULL)).bg(status.1))
                    .child(div().text_size(px(12.)).font_weight(FontWeight::MEDIUM).text_color(status.1).child(status.0)),
            )
    }

    fn toast(&self) -> Option<impl IntoElement> {
        let p = self.pal;
        self.notice.as_ref().map(|(msg, err, _)| {
            let (bg, fg) = if *err { (p.warn_soft, p.warn) } else { (p.card, p.text) };
            div().absolute().bottom(px(24.)).left_0().right_0().flex().justify_center().child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(16.))
                    .h(px(40.))
                    .rounded(px(R_PILL))
                    .bg(bg)
                    .border_2()
                    .border_color(p.ink)
                    .shadow(hard_shadow(&p, 3.))
                    .text_color(fg)
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(icon(if *err { "alert" } else { "check" }, 14., if *err { fg } else { p.ok }))
                    .child(msg.clone()),
            )
        })
    }
}

impl Render for SpellyApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let page = match self.page {
            Page::Home => self.home(cx).into_any_element(),
            Page::Recent => self.recent_page(cx).into_any_element(),
            Page::Dictionaries => self.dictionaries(cx).into_any_element(),
            Page::Settings => self.settings_page(cx).into_any_element(),
        };
        div()
            .relative()
            .flex()
            .size_full()
            .bg(p.bg)
            .text_color(p.text)
            .child(self.sidebar(cx))
            .child(
                div()
                    .id("page")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .px(px(40.))
                    .pt(px(if cfg!(target_os = "macos") { 44. } else { 32. }))
                    .pb(px(48.))
                    .child(div().w_full().max_w(px(720.)).mx_auto().child(page)),
            )
            .children(self.toast())
    }
}

/// A bordered text field around one of our inputs.
pub fn input_field(state: &Entity<InputState>, p: &Palette) -> impl IntoElement {
    field(p).flex_1().child(div().flex_1().text_size(px(14.)).child(Input::new(state)))
}

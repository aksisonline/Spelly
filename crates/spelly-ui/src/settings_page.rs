use gpui::prelude::*;
use gpui::*;
use spelly_core::{Accent, Strictness};

use crate::app::{input_field, SpellyApp};
use crate::theme::swatch;
use crate::widgets::*;

impl SpellyApp {
    pub fn settings_page(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;

        // ---- colour: round swatches, the one place circles are the right shape
        let swatches = div().flex().flex_col().gap(px(10.)).child(div().flex().flex_wrap().gap(px(10.)).children(Accent::ALL.into_iter().map(|a| {
            let on = self.settings.accent == a;
            let color = swatch(a);
            let check = if a == Accent::Neutral && p.dark { p.bg } else { gpui::white() };
            div()
                .id(a.label())
                .flex()
                .items_center()
                .justify_center()
                .size(px(44.))
                .rounded(px(R_FULL))
                .cursor_pointer()
                .border_2()
                .border_color(if on { p.ink } else { gpui::transparent_black() })
                .hover(move |d| d.border_color(Hsla { a: 0.5, ..color }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.accent = a;
                    let _ = this.store.save_settings(&this.settings);
                    this.sync_palette(cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(32.))
                        .rounded(px(R_FULL))
                        .bg(color)
                        .when(on, |d| d.child(icon("check", 16., check))),
                )
        })))
        .child(caption(self.settings.accent.label(), &p));

        // ---- boldness
        let strictness = div().flex().gap(px(12.)).children(Strictness::ALL.into_iter().map(|s| {
            let active = self.settings.strictness == s;
            div()
                .id(s.label())
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(6.))
                .p(px(14.))
                .rounded(px(R_ROW))
                .cursor_pointer()
                .border_2()
                .border_color(if active { p.ink } else { p.border })
                .bg(if active { p.accent_soft } else { p.card })
                .when(active, |d| d.shadow(hard_shadow(&p, 3.)))
                .hover(move |d| if active { d } else { d.border_color(p.ink) })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings.strictness = s;
                    let _ = this.store.save_settings(&this.settings);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pixel(s.label(), 17., p.text))
                        .when(active, |d| d.child(icon("check", 16., p.text))),
                )
                .child(div().text_size(px(12.)).line_height(px(17.)).text_color(p.muted).child(s.blurb()))
        }));

        // ---- general
        let autostart = switch("autostart", self.autostart, &p).on_click(cx.listener(|this, _, _, cx| {
            this.autostart = !this.autostart;
            crate::autostart::set_enabled(this.autostart);
            cx.notify();
        }));
        let general = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_size(px(14.)).font_weight(FontWeight::MEDIUM).child("Start Spelly when I sign in"))
                    .child(caption("So it's just always there.", &p)),
            )
            .child(autostart);

        // ---- advanced
        let open = self.adv_settings;
        let head = disclosure("adv-settings", "Advanced", open, &p).on_click(cx.listener(|this, _, _, cx| {
            this.adv_settings = !this.adv_settings;
            cx.notify();
        }));

        let apps = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("Apps to leave alone"))
            .child(caption("Spelly won't touch anything typed in these. Use the name Windows or macOS shows for the app.", &p))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .children(self.settings.ignored_apps.clone().into_iter().enumerate().map(|(i, a)| {
                        let name = a.clone();
                        row(&p)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .pl(px(12.))
                            .pr(px(4.))
                            .h(px(32.))
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(a)
                            .child(icon_button(("rm-app", i), "x", &p).on_click(cx.listener(move |this, _, _, cx| {
                                this.settings.ignored_apps.retain(|x| *x != name);
                                let _ = this.store.save_settings(&this.settings);
                                cx.notify();
                            })))
                    }))
                    .when(self.settings.ignored_apps.is_empty(), |d| d.child(caption("None. Spelly works everywhere.", &p))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(input_field(&self.app_input, &p))
                    .child(button_ghost("add-app", "Ignore", Some("plus"), &p).h(px(40.)).on_click(cx.listener(|this, _, window, cx| this.submit_app(window, cx)))),
            );

        let files = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child("My Spelly files"))
                    .child(caption("Dictionaries, history and settings. All plain text.", &p)),
            )
            .child(button_ghost("open-data", "Open", Some("folder"), &p).on_click(cx.listener(|this, _, _, _| {
                let root = this.store.root().to_path_buf();
                let _ = std::fs::create_dir_all(&root);
                #[cfg(windows)]
                let _ = std::process::Command::new("explorer").arg(&root).spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&root).spawn();
                #[cfg(not(any(windows, target_os = "macos")))]
                let _ = std::process::Command::new("xdg-open").arg(&root).spawn();
            })));

        let section = |title: &str, hint: &str, body: AnyElement| {
            tile(&p)
                .flex()
                .flex_col()
                .gap(px(14.))
                .p(px(20.))
                .child(div().flex().flex_col().gap(px(3.)).child(h2(title.to_string(), &p)).when(!hint.is_empty(), |d| d.child(caption(hint.to_string(), &p))))
                .child(body)
        };

        div()
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(div().flex().flex_col().gap(px(6.)).child(h1("Settings", &p)).child(subtitle("Changes apply right away.", &p)))
            .child(section("Colour", "", swatches.into_any_element()))
            .child(section("How bold should I be?", "Anything I skip is still underlined, never silently left wrong.", strictness.into_any_element()))
            .child(section("General", "", general.into_any_element()))
            .child(head)
            .when(open, |d| {
                d.child(tile(&p).flex().flex_col().gap(px(20.)).p(px(20.)).child(apps).child(files))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(4.))
                    .child(img("mascot/kitten.png").size(px(30.)).flex_none())
                    .child(caption(format!("Spelly {} · Powered by Harper · Apache-2.0", env!("CARGO_PKG_VERSION")), &p)),
            )
    }
}

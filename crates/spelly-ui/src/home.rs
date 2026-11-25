use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use crate::app::{Page, SpellyApp};
use crate::theme::Palette;
use crate::widgets::*;

fn stat(value: String, label: &str, p: &Palette) -> Div {
    tile(p)
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(16.))
        .child(div().text_size(px(32.)).line_height(px(38.)).font_weight(FontWeight::BOLD).text_color(p.text).child(value))
        .child(caption(label.to_string(), p))
}

pub fn ago(ts: u64) -> String {
    let secs = spelly_core::now().saturating_sub(ts);
    match secs {
        0..=45 => "just now".into(),
        46..=3599 => format!("{} min ago", (secs / 60).max(1)),
        3600..=86_399 => format!("{} h ago", secs / 3600),
        86_400..=172_799 => "yesterday".into(),
        _ => format!("{} days ago", secs / 86_400),
    }
}

impl SpellyApp {
    pub fn home(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let paused = self.settings.paused;
        let words: usize = self.packs.iter().map(|k| k.word_count()).sum();

        let (title, blurb) = if paused {
            ("Spelly is napping", "Nothing is being fixed. Wake me up when you're ready.")
        } else if !self.alive {
            ("Spelly isn't running", "Start the background app and I'll fix typos as you type.")
        } else {
            ("Spelly is on", "I fix typos as you type. If I get one wrong, press Backspace right away and I'll put it back.")
        };

        // The mascot breathes while Spelly is on, and sleeps when paused.
        let mascot = if paused {
            img("mascot/sleepy.png").h(px(112.)).flex_none().into_any_element()
        } else {
            img("mascot/kitten.png")
                .h(px(124.))
                .flex_none()
                .with_animation(
                    "bob",
                    Animation::new(Duration::from_millis(2600)).repeat().with_easing(ease_in_out),
                    |el, t| el.relative().top(px(-4. * (t * std::f32::consts::TAU).sin())),
                )
                .into_any_element()
        };

        let action = if paused {
            button_primary("wake", "Wake Spelly up", &p).on_click(cx.listener(|this, _, _, cx| this.set_paused(false, cx)))
        } else if !self.alive {
            button_primary("start", "Start Spelly", &p).on_click(cx.listener(|_, _, _, _| crate::autostart::start_daemon()))
        } else {
            button_primary("pause", "Pause", &p).on_click(cx.listener(|this, _, _, cx| this.set_paused(true, cx)))
        };

        let latest = self.recent.iter().take(3).cloned().collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(h1("Home", &p))
            .child(
                card(&p)
                    .flex()
                    .items_center()
                    .gap(px(28.))
                    .p(px(24.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_none()
                            .size(px(164.))
                            .rounded(px(R_CARD))
                            .bg(p.accent_soft)
                            .border_2()
                            .border_color(p.ink)
                            .child(mascot),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_start()
                            .flex_1()
                            .min_w_0()
                            .gap(px(10.))
                            .child(pixel(title, 28., p.text))
                            .child(div().text_size(px(14.)).line_height(px(21.)).text_color(p.muted).child(blurb))
                            .child(div().pt(px(8.)).pr(px(4.)).child(action)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(14.))
                    .child(stat(self.today.to_string(), "fixes today", &p))
                    .child(stat(self.total.to_string(), "fixes in total", &p))
                    .child(stat(words.to_string(), "words I know", &p)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(h2("Latest Fixes", &p))
                            .child(button_ghost("see-all", "See all", Some("arrow-right"), &p).on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Recent;
                                cx.notify();
                            }))),
                    )
                    .children(latest.iter().map(|e| {
                        row(&p)
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .px(px(16.))
                            .h(px(46.))
                            .child(div().text_size(px(14.)).text_color(p.muted).line_through().child(e.original.clone()))
                            .child(icon("arrow-right", 14., p.muted))
                            .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(e.replacement.clone()))
                            .child(div().flex_1())
                            .child(caption(ago(e.ts), &p))
                    }))
                    .when(latest.is_empty(), |d| {
                        d.child(caption("Nothing yet. Type something with a typo in any app and it'll show up here.", &p))
                    }),
            )
    }
}

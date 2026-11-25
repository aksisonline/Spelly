use gpui::prelude::*;
use gpui::*;

use crate::app::{RecentTab, SpellyApp};
use crate::home::ago;
use crate::widgets::*;

impl SpellyApp {
    fn tab(&self, id: &'static str, label: String, tab: RecentTab, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let active = self.recent_tab == tab;
        div()
            .id(id)
            .px(px(16.))
            .h(px(32.))
            .flex()
            .items_center()
            .rounded(px(2.))
            .cursor_pointer()
            .text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(if active { p.on_accent } else { p.muted })
            .when(active, |d| d.bg(p.accent))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.recent_tab = tab;
                cx.notify();
            }))
            .child(label)
    }

    pub fn recent_page(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let tabs = div()
            .flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(R_PILL))
            .bg(p.card)
            .border_2()
            .border_color(p.ink)
            .child(self.tab("t-fixes", "Fixes".into(), RecentTab::Fixes, cx))
            .child(self.tab("t-wont", format!("Won't fix ({})", self.rejected.len()), RecentTab::WontFix, cx));

        let body = match self.recent_tab {
            RecentTab::Fixes => self.fixes_list(cx).into_any_element(),
            RecentTab::WontFix => self.wont_fix_list(cx).into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(h1("Recent Fixes", &p))
                            .child(subtitle("Tell me when I got it wrong and I'll never do it again.", &p)),
                    )
                    .child(div().flex().flex_none().child(tabs)),
            )
            .child(body)
    }

    fn fixes_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        if self.recent.is_empty() {
            return empty_state(
                "mascot/yawn.png",
                "Nothing fixed yet",
                "Type with a typo in any app. Fixes show up here.",
                &p,
            )
            .into_any_element();
        }
        let rows: Vec<_> = self.recent.iter().take(100).cloned().collect();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .children(rows.into_iter().enumerate().map(|(i, e)| {
                let rejected = self.is_rejected(&e);
                let for_click = e.clone();
                let action = if rejected {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ok)
                        .child(icon("check", 14., p.ok))
                        .child("Won't fix again")
                        .into_any_element()
                } else {
                    button_ghost(("right", i), "That was right", Some("undo"), &p)
                        .on_click(cx.listener(move |this, _, _, cx| this.mark_right(&for_click, cx)))
                        .into_any_element()
                };
                row(&p)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(16.))
                    .min_h(px(60.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(div().text_size(px(15.)).text_color(p.muted).line_through().child(e.original.clone()))
                                    .child(icon("arrow-right", 14., p.muted))
                                    .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(e.replacement.clone())),
                            )
                            .child(caption(format!("{} · {}", ago(e.ts), if e.app.is_empty() { "unknown app".into() } else { e.app.clone() }), &p)),
                    )
                    .child(chip(friendly_reason(&e.reason), p.accent_soft, p.text, p.ink))
                    .child(action)
            }))
            .into_any_element()
    }

    fn wont_fix_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        if self.rejected.is_empty() {
            return empty_state(
                "mascot/tuxedo.png",
                "Nothing on the list",
                "When you press Backspace right after a fix, or tap “That was right”, it lands here.",
                &p,
            )
            .into_any_element();
        }
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .children(self.rejected.clone().into_iter().enumerate().map(|(i, (a, b))| {
                let (a2, b2) = (a.clone(), b.clone());
                row(&p)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .px(px(16.))
                    .h(px(52.))
                    .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(a))
                    .child(caption("stays as is, never", &p))
                    .child(div().text_size(px(15.)).text_color(p.muted).line_through().child(b))
                    .child(div().flex_1())
                    .child(
                        button_ghost(("fixable", i), "Allow fixing again", None, &p)
                            .on_click(cx.listener(move |this, _, _, cx| this.unreject(&a2, &b2, cx))),
                    )
            }))
            .into_any_element()
    }
}

fn friendly_reason(r: &str) -> &'static str {
    match r {
        "alias" => "Your correction",
        "Spelling" | "Typo" => "Spelling",
        "Capitalization" => "Capital",
        "Punctuation" | "Spacing" | "Formatting" => "Spacing",
        "Repetition" => "Repeated word",
        "Grammar" | "Agreement" => "Grammar",
        "Malapropism" | "Eggcorn" | "Usage" | "Nonstandard" => "Word choice",
        "BoundaryError" => "Joined words",
        _ => "Polish",
    }
}

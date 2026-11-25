use gpui::prelude::*;
use gpui::*;
use spelly_core::{Entry, PackFile};

use crate::app::{input_field, SpellyApp};
use crate::widgets::*;

impl SpellyApp {
    /// Dictionaries, kept simple: pick a pack, add a word, see the words. Everything else
    /// (search, corrections, pack type, new packs, file location, problems) lives under Advanced.
    pub fn dictionaries(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let header = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(h1("Dictionaries", &p))
            .child(subtitle("Words and names I should never “fix”. Add one and I'll leave it alone.", &p));

        let Some(pack) = self.packs.get(self.selected_pack).cloned() else {
            return div()
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(header)
                .child(empty_state("mascot/tuxedo.png", "No packs yet", "Open Advanced and create one.", &p))
                .child(self.advanced_dict(None, cx))
                .into_any_element();
        };

        let q = self.search.read(cx).value().to_string().to_lowercase();
        let all = pack.entries();
        let total = all.len();
        let entries: Vec<Entry> = all
            .into_iter()
            .filter(|e| match e {
                Entry::Word { word, .. } => word.to_lowercase().contains(&q),
                Entry::Alias { wrong, right, .. } => wrong.to_lowercase().contains(&q) || right.to_lowercase().contains(&q),
            })
            .collect();
        let issues = pack.validate();

        // pack picker: only when there's a choice to make
        let tabs = (self.packs.len() > 1).then(|| {
            div().flex().flex_wrap().gap(px(8.)).children(self.packs.iter().enumerate().map(|(i, pk)| {
                let active = i == self.selected_pack;
                div()
                    .id(("pack", i))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .h(px(34.))
                    .px(px(12.))
                    .rounded(px(R_PILL))
                    .cursor_pointer()
                    .border_2()
                    .border_color(if active { p.ink } else { p.border })
                    .bg(if active { p.accent_soft } else { p.card })
                    .hover(move |s| if active { s } else { s.border_color(p.ink) })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_pack = i;
                        cx.notify();
                    }))
                    .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(pk.name.clone()))
                    .child(caption(pk.entries().len().to_string(), &p))
            }))
        });

        let adder = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(input_field(&self.add_word, &p))
            .child(button_primary("add-word", "Add", &p).on_click(cx.listener(|this, _, window, cx| this.submit_word(window, cx))));

        let warn = (!issues.is_empty()).then(|| {
            let n = issues.len();
            div()
                .id("warn")
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(14.))
                .h(px(38.))
                .rounded(px(R_PILL))
                .bg(p.warn_soft)
                .border_2()
                .border_color(p.warn)
                .cursor_pointer()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.warn)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.adv_dict = true;
                    cx.notify();
                }))
                .child(icon("alert", 14., p.warn))
                .child(format!("{n} thing{} to look at in this pack. See Advanced.", if n == 1 { "" } else { "s" }))
        });

        let shown = entries.len();
        let chips = div().flex().flex_wrap().gap(px(8.)).children(entries.into_iter().take(400).enumerate().map(|(i, e)| {
            let line = e.line();
            let (label, tag) = match &e {
                Entry::Word { word, .. } => (div().text_size(px(14.)).font_weight(FontWeight::MEDIUM).child(word.clone()).into_any_element(), false),
                Entry::Alias { wrong, right, .. } => (
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().text_size(px(14.)).text_color(p.muted).line_through().child(wrong.clone()))
                        .child(icon("arrow-right", 12., p.muted))
                        .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(right.clone()))
                        .into_any_element(),
                    true,
                ),
            };
            row(&p)
                .flex()
                .items_center()
                .gap(px(4.))
                .h(px(36.))
                .pl(px(12.))
                .pr(px(4.))
                .when(tag, |d| d.bg(p.accent_soft))
                .child(label)
                .child(icon_button(("del", i), "x", &p).on_click(cx.listener(move |this, _, _, cx| {
                    this.edit_pack("Removed.", cx, |pk| {
                        pk.remove_line(line);
                        Ok(())
                    })
                })))
        }));

        div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(header)
            .children(tabs)
            .child(adder)
            .children(warn)
            .child(caption(
                if q.is_empty() { format!("{total} in {}", pack.name) } else { format!("{shown} of {total} match") },
                &p,
            ))
            .child(if shown == 0 {
                empty_state(
                    "mascot/tuxedo.png",
                    if q.is_empty() { "Nothing here yet" } else { "No matches" },
                    if q.is_empty() { "Type a word above and press Enter." } else { "Try another spelling, or add it." },
                    &p,
                )
                .into_any_element()
            } else {
                chips.into_any_element()
            })
            .when(shown > 400, |d| d.child(caption(format!("…and {} more. Use search under Advanced.", shown - 400), &p)))
            .child(self.advanced_dict(Some(&pack), cx))
            .into_any_element()
    }

    fn advanced_dict(&mut self, pack: Option<&PackFile>, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.pal;
        let open = self.adv_dict;
        let head = disclosure("adv-dict", "Advanced", open, &p).on_click(cx.listener(|this, _, _, cx| {
            this.adv_dict = !this.adv_dict;
            cx.notify();
        }));
        if !open {
            return div().child(head).into_any_element();
        }

        let labeled = |title: &str, body: AnyElement| {
            div().flex().flex_col().gap(px(8.)).child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(title.to_string())).child(body)
        };

        let search = labeled(
            "Search",
            field(&p)
                .child(icon("search", 14., p.muted))
                .child(div().flex_1().text_size(px(14.)).child(gpui_base::input::Input::new(&self.search)))
                .into_any_element(),
        );

        let correction = labeled(
            "Always correct this",
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(input_field(&self.alias_wrong, &p))
                .child(icon("arrow-right", 16., p.muted))
                .child(input_field(&self.alias_right, &p))
                .child(button_ghost("add-alias", "Save", None, &p).h(px(40.)).on_click(cx.listener(|this, _, window, cx| this.submit_alias(window, cx))))
                .into_any_element(),
        );

        let new_pack = labeled(
            "New pack",
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(input_field(&self.new_pack, &p))
                .child(button_ghost("make-pack", "Create", Some("plus"), &p).h(px(40.)).on_click(cx.listener(|this, _, window, cx| this.submit_pack(window, cx))))
                .into_any_element(),
        );

        let pack_section = pack.map(|pk| {
            let proper = pk.is_proper();
            let path = pk.path.clone();
            let issues = pk.validate();
            labeled(
                &format!("About “{}”", pk.name),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                button_ghost("kind", if proper { "Names & titles" } else { "Everyday words" }, None, &p).on_click(cx.listener(move |this, _, _, cx| {
                                    this.edit_pack("Pack type changed.", cx, |pk| {
                                        pk.set_proper(!proper);
                                        Ok(())
                                    })
                                })),
                            )
                            .child(caption(
                                if proper { "Keeps capitals and is never “fixed”." } else { "Just added to the vocabulary." },
                                &p,
                            ))
                            .child(div().flex_1())
                            .child(button_ghost("reveal", "Show file", Some("folder"), &p).on_click(cx.listener(move |_, _, _, _| {
                                let dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
                                #[cfg(windows)]
                                let _ = std::process::Command::new("explorer").arg(dir).spawn();
                                #[cfg(target_os = "macos")]
                                let _ = std::process::Command::new("open").arg(dir).spawn();
                                #[cfg(not(any(windows, target_os = "macos")))]
                                let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                            }))),
                    )
                    .children(issues.iter().take(6).map(|i| {
                        div().text_size(px(12.)).text_color(p.warn).child(format!("Line {}: {}", i.line + 1, i.message))
                    }))
                    .into_any_element(),
            )
        });

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(head)
            .child(
                tile(&p)
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .p(px(18.))
                    .when(pack.is_some(), |d| d.child(search))
                    .when(pack.is_some(), |d| d.child(correction))
                    .child(new_pack)
                    .children(pack_section),
            )
            .into_any_element()
    }
}

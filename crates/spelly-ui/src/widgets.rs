//! Our own small component kit. Plain functions over GPUI primitives.
//!
//! The look is "pixel sticker": the mascots are crisp pixel art with dark outlines, so the UI
//! uses 2px ink outlines, hard offset shadows (no blur), small radii and a pixel display face.
//! Round shapes are kept only where the shape means something (colour swatches, status dots).

use gpui::prelude::*;
use gpui::*;

use crate::assets::PIXEL;
use crate::theme::Palette;

pub const R_CARD: f32 = 4.0;
pub const R_ROW: f32 = 4.0;
pub const R_PILL: f32 = 3.0;
pub const R_FULL: f32 = 999.0;

pub fn icon(name: &str, size: f32, color: Hsla) -> Svg {
    svg().path(format!("icons/{name}.svg")).size(px(size)).flex_none().text_color(color)
}

/// A hard, blur-free drop shadow in the ink colour: the sticker-on-paper effect.
pub fn hard_shadow(p: &Palette, d: f32) -> Vec<BoxShadow> {
    vec![BoxShadow { color: p.shade, offset: point(px(d), px(d)), blur_radius: px(0.), spread_radius: px(0.), inset: false }]
}

pub fn card(p: &Palette) -> Div {
    div().bg(p.card).border_2().border_color(p.ink).rounded(px(R_CARD)).shadow(hard_shadow(p, 4.))
}

/// Flat bordered tile, for things that sit inside a page rather than stand out.
pub fn tile(p: &Palette) -> Div {
    div().bg(p.card).border_2().border_color(p.ink).rounded(px(R_CARD))
}

pub fn row(p: &Palette) -> Div {
    div().bg(p.card).border_2().border_color(p.border).rounded(px(R_ROW))
}

pub fn pixel(text: impl Into<SharedString>, size: f32, color: Hsla) -> Div {
    div().font_family(PIXEL).text_size(px(size)).line_height(px(size * 1.2)).text_color(color).child(text.into())
}

pub fn h1(text: impl Into<SharedString>, p: &Palette) -> Div {
    pixel(text, 32., p.text)
}

pub fn subtitle(text: impl Into<SharedString>, p: &Palette) -> Div {
    div().text_size(px(14.)).text_color(p.muted).child(text.into())
}

pub fn h2(text: impl Into<SharedString>, p: &Palette) -> Div {
    pixel(text, 18., p.text)
}

pub fn caption(text: impl Into<SharedString>, p: &Palette) -> Div {
    div().text_size(px(12.)).text_color(p.muted).child(text.into())
}

/// The one loud thing on a screen. Presses down into its shadow.
pub fn button_primary(id: impl Into<ElementId>, label: &str, p: &Palette) -> Stateful<Div> {
    let (hover, shadow) = (p.accent_hover, hard_shadow(p, 3.));
    div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .h(px(44.))
        .px(px(22.))
        .rounded(px(R_PILL))
        .bg(p.accent)
        .border_2()
        .border_color(p.ink)
        .shadow(shadow)
        .text_color(p.on_accent)
        .font_family(PIXEL)
        .text_size(px(16.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .active(|s| s.shadow(vec![]).top(px(3.)).left(px(3.)))
        .child(label.to_string())
}

/// Quiet button with an optional leading icon.
pub fn button_ghost(id: impl Into<ElementId>, label: &str, glyph: Option<&str>, p: &Palette) -> Stateful<Div> {
    let (hover, ink) = (p.hover, p.ink);
    let mut b = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.))
        .h(px(34.))
        .px(px(12.))
        .rounded(px(R_PILL))
        .bg(p.card)
        .border_2()
        .border_color(p.border)
        .text_color(p.text)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(move |s| s.bg(hover).border_color(ink));
    if let Some(g) = glyph {
        b = b.child(icon(g, 14., p.muted));
    }
    b.child(label.to_string())
}

pub fn icon_button(id: impl Into<ElementId>, glyph: &str, p: &Palette) -> Stateful<Div> {
    let (hover, muted, accent) = (p.hover, p.muted, p.accent);
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.))
        .rounded(px(2.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover).text_color(accent))
        .child(icon(glyph, 14., muted))
}

pub fn chip(text: impl Into<SharedString>, bg: Hsla, fg: Hsla, ink: Hsla) -> Div {
    div()
        .px(px(8.))
        .py(px(2.))
        .rounded(px(2.))
        .bg(bg)
        .border_1()
        .border_color(ink)
        .text_color(fg)
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

/// Chrome for a text input (the input itself comes from gpui-base).
pub fn field(p: &Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(px(40.))
        .px(px(12.))
        .rounded(px(R_PILL))
        .bg(p.card)
        .border_2()
        .border_color(p.ink)
}

pub fn switch(id: impl Into<ElementId>, on: bool, p: &Palette) -> Stateful<Div> {
    let knob = div().size(px(16.)).rounded(px(2.)).bg(if on { p.on_accent } else { p.ink });
    div()
        .id(id)
        .flex()
        .items_center()
        .w(px(48.))
        .h(px(26.))
        .px(px(3.))
        .rounded(px(R_PILL))
        .border_2()
        .border_color(p.ink)
        .cursor_pointer()
        .bg(if on { p.accent } else { p.card })
        .when(on, |d| d.justify_end())
        .child(knob)
}

/// "Advanced" style fold: label + chevron. The caller shows the body when open.
pub fn disclosure(id: impl Into<ElementId>, label: &str, open: bool, p: &Palette) -> Stateful<Div> {
    let hover = p.hover;
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.))
        .h(px(32.))
        .px(px(8.))
        .rounded(px(R_PILL))
        .cursor_pointer()
        .text_color(p.muted)
        .hover(move |s| s.bg(hover))
        .child(icon(if open { "chevron-down" } else { "chevron-right" }, 14., p.muted))
        .child(pixel(label.to_string(), 15., p.muted))
}

pub fn empty_state(image: &'static str, title: &str, hint: &str, p: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(10.))
        .py(px(32.))
        .child(img(image).h(px(104.)).flex_none())
        .child(pixel(title.to_string(), 18., p.text))
        .child(div().text_size(px(13.)).text_color(p.muted).max_w(px(340.)).text_center().child(hint.to_string()))
}

//! Spelly's look. Same two palettes everywhere; the window follows the system light/dark setting.
//!
//! Principles (borrowed from how Discord describes its GPUI rewrite): our own components built
//! from primitives, no webview, tokens instead of one-off colours, an 8px grid, soft layered
//! surfaces, and motion only where it explains something.

use gpui::{hsla, rgb, Hsla, Rgba, WindowAppearance};
use spelly_core::Accent;

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    /// Window background.
    pub bg: Hsla,
    /// Sidebar and quiet panels.
    pub panel: Hsla,
    /// Cards.
    pub card: Hsla,
    /// Hover wash on cards and rows.
    pub hover: Hsla,
    /// Soft outline for quiet rows.
    pub border: Hsla,
    /// Strong outline for cards and buttons, and the colour of the hard drop shadow.
    pub ink: Hsla,
    pub shade: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub accent_soft: Hsla,
    pub on_accent: Hsla,
    pub ok: Hsla,
    pub ok_soft: Hsla,
    pub warn: Hsla,
    pub warn_soft: Hsla,
    pub shadow: Hsla,
}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// (hue in degrees, light-mode accent, dark-mode accent). Neutral has no hue: greys all round.
fn spec(a: Accent) -> (f32, u32, u32) {
    match a {
        Accent::Pink => (336., 0xF0508F, 0xFF7BB0),
        Accent::Cherry => (356., 0xE5484D, 0xFF7A7E),
        Accent::Peach => (18., 0xF2784B, 0xFF9A6E),
        Accent::Honey => (40., 0xD99207, 0xF2B84B),
        Accent::Mint => (160., 0x2FA374, 0x5FD1A0),
        Accent::Sky => (212., 0x3B8EEA, 0x6DB0FF),
        Accent::Lavender => (255., 0x8B6CEF, 0xAC93FF),
        Accent::Neutral => (0., 0x3F3A45, 0xE9E4EC),
    }
}

/// The swatch colour shown in Settings.
pub fn swatch(a: Accent) -> Hsla {
    c(spec(a).1)
}

fn hsl(h: f32, s: f32, l: f32) -> Hsla {
    hsla(h / 360., s, l, 1.)
}

impl Palette {
    /// Everything (surfaces, borders, soft fills) is tinted from the accent's hue, so a theme
    /// feels like one colour rather than a pink app with a blue button.
    pub fn build(accent: Accent, dark: bool) -> Self {
        let (hue, light_hex, dark_hex) = spec(accent);
        let neutral = accent == Accent::Neutral;
        let accent_color = c(if dark { dark_hex } else { light_hex });
        let lum = {
            let r: Rgba = accent_color.into();
            0.299 * r.r + 0.587 * r.g + 0.114 * r.b
        };
        let on_accent = if lum > 0.62 { hsl(hue, if neutral { 0. } else { 0.5 }, 0.12) } else { gpui::white() };
        let shift = if dark { 0.06 } else { -0.06 };
        let accent_hover = Hsla { l: (accent_color.l + shift).clamp(0., 1.), ..accent_color };

        if dark {
            let t = if neutral { 0. } else { 1. };
            Self {
                dark: true,
                bg: hsl(hue, 0.20 * t, 0.09),
                panel: hsl(hue, 0.20 * t, 0.12),
                card: hsl(hue, 0.22 * t, 0.155),
                hover: hsl(hue, 0.22 * t, 0.195),
                border: hsl(hue, 0.20 * t, 0.26),
                ink: hsl(hue, 0.25 * t, 0.62),
                shade: hsl(hue, 0.30 * t, 0.045),
                text: hsl(hue, 0.60 * t, 0.95),
                muted: hsl(hue, 0.17 * t, 0.65),
                accent: accent_color,
                accent_hover,
                accent_soft: hsl(hue, 0.34 * t, 0.21),
                on_accent,
                ok: c(0x5FD1A0),
                ok_soft: c(0x1F3B30),
                warn: c(0xF2B45A),
                warn_soft: c(0x3F3016),
                shadow: hsla(0., 0., 0., 0.35),
            }
        } else {
            let t = if neutral { 0. } else { 1. };
            Self {
                dark: false,
                bg: hsl(hue, t, 0.985),
                panel: hsl(hue, t, 0.957),
                card: hsl(0., 0., 1.),
                hover: hsl(hue, t, 0.972),
                border: hsl(hue, 0.70 * t, 0.89),
                ink: hsl(hue, 0.30 * t, 0.20),
                shade: hsl(hue, 0.30 * t, 0.20),
                text: hsl(hue, 0.30 * t, 0.16),
                muted: hsl(hue, 0.15 * t, 0.55),
                accent: accent_color,
                accent_hover,
                accent_soft: hsl(hue, t, 0.93),
                on_accent,
                ok: c(0x2FA374),
                ok_soft: c(0xDDF6EA),
                warn: c(0xC27A12),
                warn_soft: c(0xFFEFD0),
                shadow: hsla(hue / 360., 0.5 * t, 0.35, 0.10),
            }
        }
    }

    pub fn for_appearance(accent: Accent, a: WindowAppearance) -> Self {
        Self::build(accent, matches!(a, WindowAppearance::Dark | WindowAppearance::VibrantDark))
    }
}

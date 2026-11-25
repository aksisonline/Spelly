use serde::{Deserialize, Serialize};

use crate::Policy;
use harper_core::linting::LintKind;

/// How eagerly Spelly rewrites. Anything it skips is still underlined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Strictness {
    /// Spelling, typos, capitals, spacing. Nothing else.
    Gentle,
    /// Plus grammar and word-choice fixes.
    #[default]
    Balanced,
    /// Everything Harper is confident about, and bolder spelling guesses.
    Bold,
}

impl Strictness {
    pub const ALL: [Strictness; 3] = [Strictness::Gentle, Strictness::Balanced, Strictness::Bold];

    pub fn label(self) -> &'static str {
        match self {
            Strictness::Gentle => "Gentle",
            Strictness::Balanced => "Balanced",
            Strictness::Bold => "Bold",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Strictness::Gentle => "Spelling, capitals and spacing only.",
            Strictness::Balanced => "Also fixes grammar and mix-ups like their / they're.",
            Strictness::Bold => "Fixes anything it's fairly sure about, even bigger typos.",
        }
    }

    pub fn policy(self) -> Policy {
        let mut p = Policy::default();
        match self {
            Strictness::Gentle => {
                p.auto_apply = vec![
                    LintKind::Spelling,
                    LintKind::Typo,
                    LintKind::Capitalization,
                    LintKind::Punctuation,
                    LintKind::Formatting,
                    LintKind::Repetition,
                ];
                p.max_spelling_distance = 1;
            }
            Strictness::Balanced => {}
            Strictness::Bold => {
                p.auto_apply.extend([LintKind::Style, LintKind::Redundancy, LintKind::Regionalism]);
                p.max_spelling_distance = 3;
            }
        }
        p
    }
}

/// The window's colour theme. Pink is Spelly's own; the rest are alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Pink,
    Cherry,
    Peach,
    Honey,
    Mint,
    Sky,
    Lavender,
    Neutral,
}

impl Accent {
    pub const ALL: [Accent; 8] = [
        Accent::Pink,
        Accent::Cherry,
        Accent::Peach,
        Accent::Honey,
        Accent::Mint,
        Accent::Sky,
        Accent::Lavender,
        Accent::Neutral,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Accent::Pink => "Pink",
            Accent::Cherry => "Cherry",
            Accent::Peach => "Peach",
            Accent::Honey => "Honey",
            Accent::Mint => "Mint",
            Accent::Sky => "Sky",
            Accent::Lavender => "Lavender",
            Accent::Neutral => "Neutral",
        }
    }
}

/// `settings.json`: the one place the tray app, the window and the daemon agree on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub paused: bool,
    pub strictness: Strictness,
    /// App names Spelly leaves alone (`Code.exe`, `Terminal`...). Case-insensitive.
    pub ignored_apps: Vec<String>,
    /// Window colour theme (the background app ignores it).
    pub accent: Accent,
}

impl Default for Settings {
    fn default() -> Self {
        Self { paused: false, strictness: Strictness::default(), ignored_apps: vec![], accent: Accent::default() }
    }
}

impl Settings {
    pub fn is_ignored(&self, app: &str) -> bool {
        let app = app.trim().to_lowercase();
        !app.is_empty() && self.ignored_apps.iter().any(|a| a.trim().to_lowercase() == app)
    }
}

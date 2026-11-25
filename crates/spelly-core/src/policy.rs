use harper_core::linting::LintKind;

/// What Spelly may change silently vs. only underline.
#[derive(Debug, Clone)]
pub struct Policy {
    /// Lint kinds applied without asking (first suggestion wins).
    pub auto_apply: Vec<LintKind>,
    /// Misspelled capitalized words mid-sentence are probably names: highlight only.
    pub protect_unknown_capitalized: bool,
    /// Max edit distance (chars) between typo and auto-applied spelling fix.
    pub max_spelling_distance: usize,
    /// How many trailing words a fix may reach back to (context-aware fixes need >= 2).
    pub lookback_words: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            auto_apply: vec![
                LintKind::Spelling,
                LintKind::Typo,
                LintKind::Capitalization,
                LintKind::Punctuation,
                LintKind::Formatting,
                LintKind::Repetition,
                LintKind::Grammar,
                LintKind::Agreement,
                LintKind::BoundaryError,
                LintKind::Malapropism,
                LintKind::Usage,
                LintKind::Eggcorn,
                LintKind::Nonstandard,
                // then/than and friends are filed as Miscellaneous upstream.
                LintKind::Miscellaneous,
            ],
            protect_unknown_capitalized: true,
            max_spelling_distance: 2,
            lookback_words: 3,
        }
    }
}

use std::sync::Arc;

use harper_core::{
    linting::{LintGroup, LintKind, Linter},
    spell::{FstDictionary, MergedDictionary, MutableDictionary},
    Dialect, Document, DictWordMetadata, NounData, Span, TokenStringExt,
};

use crate::{Pack, Policy};

/// A change Spelly wants to make. Spans are char offsets into the text that was checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Correction {
    pub span: Span<char>,
    pub original: String,
    pub replacement: String,
    /// Lint rule category, or "alias" for forced corrections.
    pub reason: String,
}

/// Misspelling/issue we are *not* confident enough to fix: underline it.
#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    pub span: Span<char>,
    pub message: String,
    pub suggestions: Vec<String>,
}

pub struct Spelly {
    dict: Arc<MergedDictionary>,
    lints: LintGroup,
    aliases: std::collections::HashMap<String, String>,
    pub policy: Policy,
}

impl Spelly {
    /// Custom packs are merged *in front of* Harper's curated dictionary.
    pub fn new(packs: &[Pack], dialect: Dialect) -> Self {
        let mut custom = MutableDictionary::new();
        let mut aliases = std::collections::HashMap::new();
        for pack in packs {
            for w in &pack.words {
                let meta = if pack.proper {
                    DictWordMetadata {
                        noun: Some(NounData { is_proper: Some(true), ..Default::default() }),
                        ..Default::default()
                    }
                } else {
                    DictWordMetadata::default()
                };
                custom.append_word_str(w, meta);
            }
            aliases.extend(pack.aliases.iter().cloned());
        }
        let mut merged = MergedDictionary::new();
        merged.add_dictionary(Arc::new(custom));
        merged.add_dictionary(FstDictionary::curated());
        let dict = Arc::new(merged);
        let lints = LintGroup::new_curated(dict.clone(), dialect);
        Self { dict, lints, aliases, policy: Policy::default() }
    }

    fn document(&self, text: &str) -> Document {
        Document::new_plain_english(text, self.dict.as_ref())
    }

    /// Everything Spelly would fix in `text`, plus what it would only underline.
    pub fn analyze(&mut self, text: &str) -> (Vec<Correction>, Vec<Highlight>) {
        let doc = self.document(text);
        self.analyze_doc(&doc, 0)
    }

    /// Call when a word boundary was just typed. `text` is everything up to and including
    /// the boundary. Only changes touching the last `lookback_words` words are returned,
    /// so earlier text the user left alone (or reverted) is never rewritten.
    pub fn on_boundary(&mut self, text: &str) -> Vec<Correction> {
        let doc = self.document(text);
        let words: Vec<usize> = doc.iter_word_indices().collect();
        let from = words
            .len()
            .checked_sub(self.policy.lookback_words)
            .map(|i| doc.get_tokens()[words[i]].span.start)
            .unwrap_or(0);
        self.analyze_doc(&doc, from).0
    }

    fn analyze_doc(&mut self, doc: &Document, from: usize) -> (Vec<Correction>, Vec<Highlight>) {
        let src = doc.get_source();
        let tokens = doc.get_tokens();
        let mut fixes: Vec<Correction> = vec![];
        let mut marks: Vec<Highlight> = vec![];

        // 1. Forced corrections (aliases) always win.
        for i in doc.iter_word_indices() {
            let span = tokens[i].span;
            if span.end <= from {
                continue;
            }
            let word: String = span.get_content(src).iter().collect();
            if let Some(to) = self.aliases.get(&word.to_lowercase()) {
                if *to != word {
                    fixes.push(Correction {
                        span,
                        original: word,
                        replacement: to.clone(),
                        reason: "alias".into(),
                    });
                }
            }
        }

        // 1b. Missing space after punctuation ("test.It", "yes,no"): Harper doesn't cover it.
        for span in missing_spaces(src, from) {
            if !fixes.iter().any(|f| overlaps(f.span, span)) {
                let p: String = span.get_content(src).iter().collect();
                fixes.push(Correction {
                    span,
                    original: p.clone(),
                    replacement: format!("{p} "),
                    reason: "Spacing".into(),
                });
            }
        }

        // 2. Harper's context-aware lints.
        let mut lints = self.lints.lint(doc);
        lints.sort_by_key(|l| (l.span.start, l.priority));
        for lint in lints {
            if lint.span.end <= from || fixes.iter().any(|f| overlaps(f.span, lint.span)) {
                continue;
            }
            let original: String = lint.span.get_content(src).iter().collect();
            let suggestions: Vec<String> = lint
                .suggestions
                .iter()
                .map(|s| {
                    let mut buf = src.to_vec();
                    s.apply(lint.span, &mut buf);
                    let delta = buf.len() as isize - src.len() as isize;
                    let end = (lint.span.end as isize + delta).max(lint.span.start as isize) as usize;
                    buf[lint.span.start..end].iter().collect()
                })
                .collect();
            let confident = self.policy.auto_apply.contains(&lint.lint_kind)
                && !suggestions.is_empty()
                && self.safe(&lint.lint_kind, &original, &suggestions[0], doc, lint.span);
            if confident {
                fixes.push(Correction {
                    span: lint.span,
                    original,
                    replacement: suggestions[0].clone(),
                    reason: format!("{:?}", lint.lint_kind),
                });
            } else {
                marks.push(Highlight { span: lint.span, message: lint.message, suggestions });
            }
        }
        fixes.sort_by_key(|f| f.span.start);
        (fixes, marks)
    }

    fn safe(&self, kind: &LintKind, from: &str, to: &str, doc: &Document, span: Span<char>) -> bool {
        if *kind != LintKind::Spelling {
            return true;
        }
        if edit_distance(&from.to_lowercase(), &to.to_lowercase()) > self.policy.max_spelling_distance {
            return false;
        }
        // Capitalized, not sentence-initial, and unknown => most likely a name we haven't seen.
        if self.policy.protect_unknown_capitalized
            && from.chars().next().is_some_and(char::is_uppercase)
            && !at_sentence_start(doc, span)
        {
            return false;
        }
        true
    }

    /// Apply corrections to `text`.
    pub fn apply(text: &str, fixes: &[Correction]) -> String {
        let mut chars: Vec<char> = text.chars().collect();
        let mut sorted: Vec<&Correction> = fixes.iter().collect();
        sorted.sort_by_key(|f| std::cmp::Reverse(f.span.start));
        for f in sorted {
            let rep: Vec<char> = f.replacement.chars().collect();
            chars.splice(f.span.start..f.span.end, rep);
        }
        chars.into_iter().collect()
    }
}

fn overlaps(a: Span<char>, b: Span<char>) -> bool {
    a.start < b.end && b.start < a.end
}

fn at_sentence_start(doc: &Document, span: Span<char>) -> bool {
    let toks = doc.get_tokens();
    let Some(idx) = toks.iter().position(|t| t.span.start == span.start) else { return true };
    toks[..idx].iter().rev().find(|t| !t.kind.is_whitespace()).map_or(true, |t| {
        t.kind.is_sentence_terminator() || t.kind.is_paragraph_break() || t.kind.is_newline()
    })
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + (a[i - 1] != b[j - 1]) as usize);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Spans of a single punctuation char that is glued to the next word.
/// `.` only when it looks like a sentence end (lowercase letter before, uppercase after),
/// so `file.txt`, `3.5`, `e.g.` and URLs stay untouched.
fn missing_spaces(src: &[char], from: usize) -> Vec<Span<char>> {
    let mut out = vec![];
    for i in 1..src.len().saturating_sub(1) {
        let (prev, c, next) = (src[i - 1], src[i], src[i + 1]);
        let glued = match c {
            '.' => prev.is_lowercase() && next.is_uppercase(),
            ',' | ';' | '!' | '?' => prev.is_alphabetic() && next.is_alphabetic(),
            _ => false,
        };
        if glued && i + 1 > from && !in_url_or_email(src, i) {
            out.push(Span::new(i, i + 1));
        }
    }
    out
}

fn in_url_or_email(src: &[char], i: usize) -> bool {
    let start = src[..i].iter().rposition(|c| c.is_whitespace()).map_or(0, |p| p + 1);
    let end = src[i..].iter().position(|c| c.is_whitespace()).map_or(src.len(), |p| i + p);
    let chunk: String = src[start..end].iter().collect();
    chunk.contains("://") || chunk.contains('@') || chunk.starts_with("www.")
}

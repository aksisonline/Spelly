use std::collections::HashSet;

use crate::{Correction, Spelly};

/// Per-text-field state implementing "backspace right after a correction undoes it".
pub struct Session {
    last: Vec<Correction>,
    /// (original, replacement), lowercase. Reverted pairs are never re-applied.
    rejected: HashSet<(String, String)>,
}

impl Session {
    pub fn new() -> Self {
        Self { last: vec![], rejected: HashSet::new() }
    }

    /// Word boundary typed. Returns corrections for the host to apply (see [`Spelly::apply`]).
    pub fn on_boundary(&mut self, spelly: &mut Spelly, text: &str) -> Vec<Correction> {
        let fixes = spelly.on_boundary(text);
        self.accept(fixes)
    }

    /// Drop corrections the user already reverted, remember the rest for undo.
    pub fn accept(&mut self, fixes: Vec<Correction>) -> Vec<Correction> {
        let fixes: Vec<Correction> = fixes
            .into_iter()
            .filter(|f| !self.rejected.contains(&key(&f.original, &f.replacement)))
            .collect();
        self.last = fixes.clone();
        fixes
    }

    /// Backspace pressed. If it immediately follows corrections, returns the reversal
    /// (each item's `replacement` is the original text, in post-correction coordinates).
    pub fn on_backspace(&mut self) -> Vec<Correction> {
        let fixes = std::mem::take(&mut self.last);
        let mut shift: isize = 0;
        let mut out = vec![];
        for f in fixes {
            self.rejected.insert(key(&f.original, &f.replacement));
            let start = (f.span.start as isize + shift) as usize;
            let new_len = f.replacement.chars().count();
            shift += new_len as isize - (f.span.end - f.span.start) as isize;
            out.push(Correction {
                span: harper_core::Span::new(start, start + new_len),
                original: f.replacement,
                replacement: f.original,
                reason: "revert".into(),
            });
        }
        out
    }

    /// Any other keystroke closes the undo window.
    pub fn on_other_input(&mut self) {
        self.last.clear();
    }

    pub fn reject(&mut self, original: &str, replacement: &str) {
        self.rejected.insert(key(original, replacement));
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

fn key(a: &str, b: &str) -> (String, String) {
    (a.to_lowercase(), b.to_lowercase())
}

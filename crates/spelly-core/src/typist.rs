use crate::{Correction, Session, Settings, Spelly};

/// What the platform shell tells us happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    /// A printable character the user typed. `'\n'` is Enter, `'\t'` is Tab.
    Char(char),
    Backspace,
    /// Cursor moved or focus changed: we no longer know what is on screen.
    Reset,
}

/// Delete `backspaces` characters before the cursor, then type `text`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edit {
    pub backspaces: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Let the key through untouched.
    Pass,
    /// Swallow the key and do this instead.
    Swallow(Edit),
}

struct Undo {
    /// What the user actually typed (before the fix and the boundary).
    before: Vec<char>,
    /// What is on screen after the fix, boundary included.
    after: Vec<char>,
}

const MAX_BUF: usize = 400;

/// Turns a stream of keys into edits. No OS calls: the shell injects the `Edit`s.
pub struct Typist {
    spelly: Spelly,
    session: Session,
    buf: Vec<char>,
    undo: Option<Undo>,
    /// True when `buf` starts at the beginning of a line we saw begin (after Enter),
    /// so a lowercase first word really is a sentence start.
    anchored: bool,
    pub paused: bool,
    applied: Vec<Correction>,
}

impl Typist {
    pub fn new(spelly: Spelly) -> Self {
        Self { spelly, session: Session::new(), buf: vec![], undo: None, anchored: false, paused: false, applied: vec![] }
    }

    /// Swap in freshly loaded dictionaries. Keeps what the user has reverted.
    pub fn reload(&mut self, spelly: Spelly) {
        self.spelly = spelly;
    }

    /// Pause state and strictness from `settings.json`.
    pub fn apply_settings(&mut self, s: &Settings) {
        self.paused = s.paused;
        self.spelly.policy = s.strictness.policy();
    }

    /// Persisted "that was right" pairs, loaded at startup and on change.
    pub fn set_rejected(&mut self, pairs: &[(String, String)]) {
        for (a, b) in pairs {
            self.session.reject(a, b);
        }
    }

    /// Fixes applied since the last call (for the log).
    pub fn drain_applied(&mut self) -> Vec<Correction> {
        std::mem::take(&mut self.applied)
    }

    /// Fixes the user reverted with Backspace since the last call.
    pub fn drain_rejections(&mut self) -> Vec<(String, String)> {
        self.session.take_new_rejections()
    }

    pub fn on_key(&mut self, key: Key) -> Outcome {
        if self.paused {
            self.reset();
            return Outcome::Pass;
        }
        match key {
            Key::Reset => {
                self.reset();
                Outcome::Pass
            }
            Key::Backspace => self.backspace(),
            Key::Char(c) if is_boundary(c) => self.boundary(c),
            Key::Char(c) => {
                self.close_undo();
                self.push(c);
                Outcome::Pass
            }
        }
    }

    fn boundary(&mut self, c: char) -> Outcome {
        self.close_undo();
        let mut text = self.buf.clone();
        text.push(c);
        let s: String = text.iter().collect();

        let fixes: Vec<_> = self
            .spelly
            .on_boundary(&s)
            .into_iter()
            // After a click we don't know if the first word starts a sentence.
            .filter(|f| self.anchored || !(f.reason == "Capitalization" && f.span.start == 0))
            .collect();
        let fixes = self.session.accept(fixes);

        let outcome = if fixes.is_empty() {
            self.push(c);
            Outcome::Pass
        } else {
            let new: Vec<char> = Spelly::apply(&s, &fixes).chars().collect();
            let keep = common_prefix(&self.buf, &new);
            let edit = Edit {
                backspaces: self.buf.len() - keep,
                text: new[keep..].iter().collect(),
            };
            if edit.backspaces == 0 && edit.text == c.to_string() {
                self.push(c);
                Outcome::Pass
            } else {
                self.applied.extend(fixes.iter().cloned());
                self.undo = Some(Undo { before: self.buf.clone(), after: new.clone() });
                self.buf = new;
                Outcome::Swallow(edit)
            }
        };
        if c == '\n' {
            self.reset();
            self.anchored = true;
        }
        outcome
    }

    fn backspace(&mut self) -> Outcome {
        if let Some(u) = self.undo.take() {
            if self.buf == u.after {
                self.session.on_backspace();
                let keep = common_prefix(&u.after, &u.before);
                let edit = Edit {
                    backspaces: u.after.len() - keep,
                    text: u.before[keep..].iter().collect(),
                };
                self.buf = u.before;
                return Outcome::Swallow(edit);
            }
        }
        self.session.on_other_input();
        self.buf.pop();
        Outcome::Pass
    }

    fn push(&mut self, c: char) {
        self.buf.push(c);
        if self.buf.len() > MAX_BUF {
            self.buf.drain(..MAX_BUF / 4);
            self.anchored = false;
        }
    }

    fn close_undo(&mut self) {
        self.undo = None;
        self.session.on_other_input();
    }

    fn reset(&mut self) {
        self.close_undo();
        self.buf.clear();
        self.anchored = false;
    }
}

fn is_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '!' | '?')
}

fn common_prefix(a: &[char], b: &[char]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

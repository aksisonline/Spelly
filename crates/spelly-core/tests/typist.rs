use harper_core::Dialect;
use spelly_core::*;

/// A fake text field: applies what the shell would inject.
struct Screen {
    typist: Typist,
    text: String,
}

impl Screen {
    fn new() -> Self {
        let packs = [Pack::parse("anime", include_str!("../../../dictionaries/anime.txt"))];
        Self { typist: Typist::new(Spelly::new(&packs, Dialect::Indian)), text: String::new() }
    }
    fn key(&mut self, k: Key) {
        match self.typist.on_key(k) {
            Outcome::Pass => match k {
                Key::Char(c) => self.text.push(c),
                Key::Backspace => {
                    self.text.pop();
                }
                Key::Reset => {}
            },
            Outcome::Swallow(e) => {
                for _ in 0..e.backspaces {
                    self.text.pop();
                }
                self.text.push_str(&e.text);
            }
        }
    }
    fn type_str(&mut self, s: &str) {
        s.chars().for_each(|c| self.key(Key::Char(c)));
    }
}

#[test]
fn fixes_on_space() {
    let mut s = Screen::new();
    s.type_str("I like teh ");
    assert_eq!(s.text, "I like the ");
}

#[test]
fn backspace_restores_original_and_sticks() {
    let mut s = Screen::new();
    s.type_str("I like teh ");
    s.key(Key::Backspace);
    assert_eq!(s.text, "I like teh");
    s.type_str(" ");
    assert_eq!(s.text, "I like teh ");
}

#[test]
fn second_backspace_is_normal() {
    let mut s = Screen::new();
    s.type_str("I like teh ");
    s.key(Key::Backspace);
    s.key(Key::Backspace);
    assert_eq!(s.text, "I like te");
}

#[test]
fn context_fix_reaches_back() {
    let mut s = Screen::new();
    s.type_str("Their going to the ");
    assert_eq!(s.text, "They're going to the ");
}

#[test]
fn click_does_not_capitalize_first_word() {
    let mut s = Screen::new();
    s.key(Key::Reset);
    s.type_str("teh ");
    assert_eq!(s.text, "the ");
}

#[test]
fn enter_applies_fix_and_starts_fresh_line() {
    let mut s = Screen::new();
    s.type_str("I like teh\n");
    assert_eq!(s.text, "I like the\n");
    s.type_str("narto ");
    assert_eq!(s.text, "I like the\nNaruto ");
}

#[test]
fn paused_does_nothing() {
    let mut s = Screen::new();
    s.typist.paused = true;
    s.type_str("teh ");
    assert_eq!(s.text, "teh ");
}

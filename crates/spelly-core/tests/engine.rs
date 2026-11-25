use harper_core::Dialect;
use spelly_core::*;

fn spelly() -> Spelly {
    let packs = [
        Pack::parse("anime", include_str!("../../../dictionaries/anime.txt")),
        Pack::parse("names", include_str!("../../../dictionaries/indian-names.txt")),
    ];
    Spelly::new(&packs, Dialect::Indian)
}

fn fix(s: &mut Spelly, text: &str) -> String {
    let (fixes, _) = s.analyze(text);
    Spelly::apply(text, &fixes)
}

#[test]
fn spelling_and_case() {
    let mut s = spelly();
    assert_eq!(fix(&mut s, "I recieve teh mail."), "I receive the mail.");
}

#[test]
fn custom_dictionary_aliases() {
    let mut s = spelly();
    assert_eq!(fix(&mut s, "My favourite is narto and Sasuke."), "My favourite is Naruto and Sasuke.");
}

#[test]
fn names_are_left_alone() {
    let mut s = spelly();
    let t = "Venkatesh met Karthik and Meenakshi.";
    assert_eq!(fix(&mut s, t), t);
}

#[test]
fn context_aware_grammar() {
    let mut s = spelly();
    assert_eq!(
        fix(&mut s, "Their going to the park. I like cats more then dogs. i am here"),
        "They're going to the park. I like cats more than dogs. I am here"
    );
}

#[test]
fn spacing_and_punctuation() {
    let mut s = spelly();
    assert_eq!(
        fix(&mut s, "Hello  world ,this is a test.It works, yes,no. See file.txt or a@b.com"),
        "Hello world, this is a test. It works, yes, no. See file.txt or a@b.com"
    );
}

#[test]
fn backspace_reverts() {
    let mut s = spelly();
    let mut sess = Session::new();
    let text = "I like teh ";
    let fixes = sess.on_boundary(&mut s, text);
    assert_eq!(Spelly::apply(text, &fixes), "I like the ");
    let rev = sess.on_backspace();
    assert_eq!(rev.len(), 1);
    assert_eq!(rev[0].replacement, "teh");
    assert!(sess.on_backspace().is_empty());
    // rejected: not corrected again
    assert!(sess.on_boundary(&mut s, "I like teh ").is_empty());
}

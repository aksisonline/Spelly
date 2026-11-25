use spelly_core::*;

fn tmp(name: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("spelly-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::new(dir)
}

#[test]
fn settings_roundtrip_and_defaults() {
    let st = tmp("settings");
    assert_eq!(st.settings(), Settings::default());
    let s = Settings { paused: true, strictness: Strictness::Bold, ignored_apps: vec!["Code.exe".into()], accent: Accent::Sky };
    st.save_settings(&s).unwrap();
    assert_eq!(st.settings(), s);
    assert!(s.is_ignored("code.EXE"));
    assert!(!s.is_ignored("notepad.exe"));
}

#[test]
fn log_recent_newest_first_and_counts() {
    let st = tmp("log");
    let e = |o: &str, ts| LogEntry { ts, original: o.into(), replacement: "x".into(), reason: "Spelling".into(), app: "a".into() };
    st.append_log(&[e("one", 10), e("two", 20), e("three", now())]);
    let r = st.recent(2);
    assert_eq!(r[0].original, "three");
    assert_eq!(r[1].original, "two");
    assert_eq!(st.fixes_since(start_of_today()), 1);
}

#[test]
fn rejected_dedupes_and_removes() {
    let st = tmp("rej");
    st.add_rejected(&[("teh".into(), "the".into())]);
    st.add_rejected(&[("TEH".into(), "The".into()), ("recieve".into(), "receive".into())]);
    assert_eq!(st.rejected().len(), 2);
    st.remove_rejected("teh", "the");
    assert_eq!(st.rejected(), vec![("recieve".to_string(), "receive".to_string())]);
}

#[test]
fn fingerprint_changes_on_edit() {
    let st = tmp("fp");
    let mut packs = st.pack_files();
    let before = st.fingerprint();
    std::thread::sleep(std::time::Duration::from_millis(20));
    packs[0].add_word("Zorblax").unwrap();
    packs[0].save().unwrap();
    assert_ne!(before, st.fingerprint());
}

#[test]
fn pack_editing_keeps_comments_and_validates() {
    let st = tmp("pack");
    let mut p = st.create_pack("my pack!").unwrap();
    assert_eq!(p.name, "my-pack");
    p.add_word("Zorblax").unwrap();
    assert!(p.add_word("zorblax").is_err());
    p.add_alias("zorbalx", "Zorblax").unwrap();
    p.save().unwrap();
    let again = PackFile::load(&p.path).unwrap();
    assert_eq!(again.entries().len(), 2);
    assert!(again.text().starts_with("@kind proper"));
    assert!(again.validate().is_empty());
    let mut bad = PackFile::from_text(&p.path, "A\na\nx -> x\nq ->\n");
    assert_eq!(bad.validate().len(), 3);
    bad.remove_line(1);
    assert_eq!(bad.validate().len(), 2);
}

#[test]
fn typist_logs_and_persists_rejections() {
    use harper_core::Dialect;
    let mut t = Typist::new(Spelly::new(&[], Dialect::Indian));
    for c in "I like teh ".chars() {
        t.on_key(Key::Char(c));
    }
    let applied = t.drain_applied();
    assert_eq!(applied.len(), 1);
    t.on_key(Key::Backspace);
    assert_eq!(t.drain_rejections(), vec![("teh".to_string(), "the".to_string())]);
}

#[test]
fn strictness_changes_policy() {
    assert!(Strictness::Gentle.policy().auto_apply.len() < Strictness::Balanced.policy().auto_apply.len());
    assert!(Strictness::Bold.policy().max_spelling_distance > Strictness::Balanced.policy().max_spelling_distance);
}

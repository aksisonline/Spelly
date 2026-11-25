use std::time::{Duration, Instant};

use spelly_core::*;

fn store(name: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("spelly-rt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::new(dir)
}

fn type_str(rt: &mut Runtime, app: &str, s: &str) -> String {
    let mut screen = String::new();
    for c in s.chars() {
        match rt.on_key(Key::Char(c), app) {
            Outcome::Pass => screen.push(c),
            Outcome::Swallow(e) => {
                (0..e.backspaces).for_each(|_| {
                    screen.pop();
                });
                screen.push_str(&e.text);
            }
        }
    }
    screen
}

#[test]
fn picks_up_pack_edits_without_restart() {
    let st = store("reload");
    let mut rt = Runtime::new(st.clone());
    assert_eq!(type_str(&mut rt, "app", "I met zorbalx "), "I met zorbalx ");

    let mut pack = st.create_pack("mine").unwrap();
    pack.add_alias("zorbalx", "Zorblax").unwrap();
    pack.save().unwrap();

    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(20) {
        rt.tick();
        std::thread::sleep(Duration::from_millis(100));
        rt.on_key(Key::Reset, "app");
        if type_str(&mut rt, "app", "zorbalx ") == "Zorblax " {
            return;
        }
    }
    panic!("pack edit never took effect");
}

#[test]
fn ignored_apps_and_pause_and_log() {
    let st = store("ign");
    st.save_settings(&Settings { ignored_apps: vec!["Terminal".into()], ..Default::default() }).unwrap();
    let mut rt = Runtime::new(st.clone());
    assert_eq!(type_str(&mut rt, "terminal", "teh "), "teh ");
    assert_eq!(type_str(&mut rt, "Notes", "teh "), "the ");
    assert_eq!(st.recent(5).len(), 1);
    assert_eq!(st.recent(1)[0].app, "Notes");

    rt.set_paused(true);
    assert_eq!(type_str(&mut rt, "Notes", "teh "), "teh ");
    assert!(st.settings().paused);
}

#[test]
fn backspace_revert_is_remembered_on_disk() {
    let st = store("rev");
    let mut rt = Runtime::new(st.clone());
    type_str(&mut rt, "Notes", "teh ");
    rt.on_key(Key::Backspace, "Notes");
    assert_eq!(st.rejected(), vec![("teh".to_string(), "the".to_string())]);
    // a fresh process remembers too
    let mut rt2 = Runtime::new(st);
    assert_eq!(type_str(&mut rt2, "Notes", "teh "), "teh ");
}

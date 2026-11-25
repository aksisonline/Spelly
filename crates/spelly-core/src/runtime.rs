use std::sync::mpsc::{channel, Receiver};

use harper_core::Dialect;

use crate::{now, Key, LogEntry, Outcome, Settings, Spelly, Store, Typist};

/// Everything a background app needs: the typing engine, kept in sync with the files on disk.
/// The OS shells call `on_key` for every key and `tick` about once a second.
pub struct Runtime {
    store: Store,
    typist: Typist,
    settings: Settings,
    packs_fp: u64,
    prefs_fp: u64,
    loading: Option<Receiver<Spelly>>,
    ticks: u32,
}

fn build(store: &Store) -> Spelly {
    Spelly::new(&store.load_packs(), Dialect::Indian)
}

impl Runtime {
    pub fn new(store: Store) -> Self {
        crate::Pack::seed_dir(&store.packs_dir());
        let settings = store.settings();
        let mut typist = Typist::new(build(&store));
        typist.apply_settings(&settings);
        typist.set_rejected(&store.rejected());
        let (packs_fp, prefs_fp) = store.fingerprints();
        Self { store, typist, settings, packs_fp, prefs_fp, loading: None, ticks: 0 }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// `app` is the name of the focused app (cached by the shell); ignored apps are skipped.
    pub fn on_key(&mut self, key: Key, app: &str) -> Outcome {
        if self.settings.is_ignored(app) {
            self.typist.on_key(Key::Reset);
            return Outcome::Pass;
        }
        let outcome = self.typist.on_key(key);
        let applied = self.typist.drain_applied();
        if !applied.is_empty() {
            let ts = now();
            let entries: Vec<LogEntry> = applied
                .into_iter()
                .map(|c| LogEntry { ts, original: c.original, replacement: c.replacement, reason: c.reason, app: app.to_string() })
                .collect();
            self.store.append_log(&entries);
        }
        let reverted = self.typist.drain_rejections();
        if !reverted.is_empty() {
            self.store.add_rejected(&reverted);
            self.prefs_fp = self.store.fingerprints().1; // our own write: no reload needed
        }
        outcome
    }

    /// Toggle pause and write it to `settings.json` so the window sees it too.
    pub fn set_paused(&mut self, paused: bool) {
        self.settings.paused = paused;
        let _ = self.store.save_settings(&self.settings);
        self.typist.apply_settings(&self.settings);
        self.prefs_fp = self.store.fingerprints().1;
    }

    /// Call about once a second. Picks up edits made in the window or by hand.
    /// Returns true if anything changed (so the shell can refresh its menu).
    pub fn tick(&mut self) -> bool {
        if self.ticks % 3 == 0 {
            self.store.touch_alive();
        }
        self.ticks = self.ticks.wrapping_add(1);
        let mut changed = false;
        if let Some(rx) = &self.loading {
            if let Ok(fresh) = rx.try_recv() {
                self.typist.reload(fresh);
                self.typist.apply_settings(&self.settings);
                self.loading = None;
                changed = true;
            }
        }
        let (packs, prefs) = self.store.fingerprints();
        if prefs != self.prefs_fp {
            self.prefs_fp = prefs;
            self.settings = self.store.settings();
            self.typist.apply_settings(&self.settings);
            self.typist.set_rejected(&self.store.rejected());
            changed = true;
        }
        if packs != self.packs_fp && self.loading.is_none() {
            self.packs_fp = packs;
            // Building the dictionary takes a moment: do it off-thread so typing never stalls.
            let (tx, rx) = channel();
            let store = self.store.clone();
            std::thread::spawn(move || {
                let _ = tx.send(build(&store));
            });
            self.loading = Some(rx);
        }
        changed
    }
}

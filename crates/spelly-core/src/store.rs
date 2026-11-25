use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{PackFile, Settings};

/// One automatic fix, as written to `log.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unix seconds.
    pub ts: u64,
    pub original: String,
    pub replacement: String,
    pub reason: String,
    pub app: String,
}

/// Everything Spelly keeps on disk. Plain files, so the tray app, the window and
/// people (or agents) can all read and edit them.
///
/// ```text
/// <root>/settings.json
/// <root>/log.jsonl          recent fixes
/// <root>/rejected.txt       "That was right" list: `original -> replacement`
/// <root>/dictionaries/*.txt packs
/// ```
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

const MAX_LOG_BYTES: u64 = 512 * 1024;

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `%APPDATA%\Spelly`, `~/Library/Application Support/Spelly`, or `~/.config/spelly`.
    pub fn default_dir() -> PathBuf {
        if let Some(d) = std::env::var_os("SPELLY_DATA_DIR") {
            return PathBuf::from(d); // dev and tests
        }
        #[cfg(windows)]
        {
            std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default().join("Spelly")
        }
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Library/Application Support/Spelly")
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config/spelly")
        }
    }

    pub fn open_default() -> Self {
        Self::new(Self::default_dir())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn packs_dir(&self) -> PathBuf {
        self.root.join("dictionaries")
    }
    fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    fn log_path(&self) -> PathBuf {
        self.root.join("log.jsonl")
    }
    fn rejected_path(&self) -> PathBuf {
        self.root.join("rejected.txt")
    }

    // ------------------------------------------------------------ settings

    pub fn settings(&self) -> Settings {
        fs::read_to_string(self.settings_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, s: &Settings) -> std::io::Result<()> {
        fs::create_dir_all(&self.root)?;
        write_atomic(&self.settings_path(), &serde_json::to_string_pretty(s).unwrap_or_default())
    }

    // ------------------------------------------------------------ log

    pub fn append_log(&self, entries: &[LogEntry]) {
        if entries.is_empty() {
            return;
        }
        let _ = fs::create_dir_all(&self.root);
        if fs::metadata(self.log_path()).map(|m| m.len()).unwrap_or(0) > MAX_LOG_BYTES {
            let keep: Vec<String> = self.read_log_lines().into_iter().rev().take(500).rev().collect();
            let _ = write_atomic(&self.log_path(), &(keep.join("\n") + "\n"));
        }
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(self.log_path()) {
            for e in entries {
                if let Ok(line) = serde_json::to_string(e) {
                    let _ = writeln!(f, "{line}");
                }
            }
        }
    }

    fn read_log_lines(&self) -> Vec<String> {
        fs::read_to_string(self.log_path())
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// Newest first.
    pub fn recent(&self, limit: usize) -> Vec<LogEntry> {
        self.read_log_lines()
            .iter()
            .rev()
            .filter_map(|l| serde_json::from_str(l).ok())
            .take(limit)
            .collect()
    }

    /// How many fixes were made since `since` (unix seconds).
    pub fn fixes_since(&self, since: u64) -> usize {
        self.read_log_lines()
            .iter()
            .filter_map(|l| serde_json::from_str::<LogEntry>(l).ok())
            .filter(|e| e.ts >= since)
            .count()
    }

    // ------------------------------------------------------------ "that was right"

    /// Pairs Spelly must never apply again.
    pub fn rejected(&self) -> Vec<(String, String)> {
        fs::read_to_string(self.rejected_path())
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once("->"))
            .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            .collect()
    }

    pub fn add_rejected(&self, pairs: &[(String, String)]) {
        let existing = self.rejected();
        let fresh: Vec<_> = pairs
            .iter()
            .filter(|(a, b)| {
                !existing.iter().any(|(x, y)| x.eq_ignore_ascii_case(a) && y.eq_ignore_ascii_case(b))
            })
            .collect();
        if fresh.is_empty() {
            return;
        }
        let _ = fs::create_dir_all(&self.root);
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(self.rejected_path()) {
            for (a, b) in fresh {
                let _ = writeln!(f, "{a} -> {b}");
            }
        }
    }

    pub fn remove_rejected(&self, original: &str, replacement: &str) {
        let kept: Vec<String> = self
            .rejected()
            .into_iter()
            .filter(|(a, b)| !(a.eq_ignore_ascii_case(original) && b.eq_ignore_ascii_case(replacement)))
            .map(|(a, b)| format!("{a} -> {b}"))
            .collect();
        let _ = write_atomic(&self.rejected_path(), &(kept.join("\n") + "\n"));
    }

    // ------------------------------------------------------------ packs

    pub fn load_packs(&self) -> Vec<crate::Pack> {
        crate::Pack::load_dir(&self.packs_dir())
    }

    /// Editable pack files; seeds the starter packs the first time.
    pub fn pack_files(&self) -> Vec<PackFile> {
        crate::Pack::seed_dir(&self.packs_dir());
        let mut files: Vec<PackFile> = fs::read_dir(self.packs_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "txt"))
            .filter_map(|p| PackFile::load(&p).ok())
            .collect();
        files.sort_by(|a, b| a.name.cmp(&b.name));
        files
    }

    pub fn create_pack(&self, name: &str) -> std::io::Result<PackFile> {
        let clean: String = name
            .trim()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect();
        let clean = clean.trim_matches('-');
        if clean.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Give the pack a name"));
        }
        fs::create_dir_all(self.packs_dir())?;
        let path = self.packs_dir().join(format!("{clean}.txt"));
        if path.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "A pack with that name exists"));
        }
        let pf = PackFile::from_text(&path, "@kind proper\n");
        pf.save()?;
        Ok(pf)
    }

    // ------------------------------------------------------------ heartbeat

    /// The background app calls this every few seconds so the window can say "Running".
    pub fn touch_alive(&self) {
        let _ = fs::create_dir_all(&self.root);
        let _ = fs::write(self.root.join("alive"), now().to_string());
    }

    pub fn daemon_alive(&self) -> bool {
        fs::read_to_string(self.root.join("alive"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .is_some_and(|t| now().saturating_sub(t) <= 8)
    }

    /// Size of the log file: changes whenever a fix is appended.
    pub fn log_len(&self) -> u64 {
        fs::metadata(self.log_path()).map(|m| m.len()).unwrap_or(0)
    }

    /// Lines in the log (cheap total-fixes figure).
    pub fn log_count(&self) -> usize {
        self.read_log_lines().len()
    }

    // ------------------------------------------------------------ change detection

    /// `(packs, prefs)` fingerprints. Packs change when any dictionary file does; prefs when
    /// settings or the rejected list do. Cheap enough to poll once a second.
    pub fn fingerprints(&self) -> (u64, u64) {
        let hash = |paths: &[PathBuf]| {
            let mut h: u64 = 1469598103934665603;
            for p in paths {
                if let Ok(m) = fs::metadata(p) {
                    let t = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_nanos() as u64);
                    for v in [t, m.len()] {
                        h = (h ^ v).wrapping_mul(1099511628211);
                    }
                }
            }
            h
        };
        let mut packs: Vec<PathBuf> =
            fs::read_dir(self.packs_dir()).into_iter().flatten().flatten().map(|e| e.path()).collect();
        packs.sort();
        (hash(&packs), hash(&[self.settings_path(), self.rejected_path()]))
    }

    pub fn fingerprint(&self) -> u64 {
        let (a, b) = self.fingerprints();
        a ^ b.rotate_left(1)
    }
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Start of the current UTC day. (Good enough for "fixes today".)
pub fn start_of_today() -> u64 {
    now() / 86_400 * 86_400
}

fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

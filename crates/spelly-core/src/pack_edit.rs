use std::path::{Path, PathBuf};

/// One meaningful line of a pack.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Word { line: usize, word: String },
    Alias { line: usize, wrong: String, right: String },
}

impl Entry {
    pub fn line(&self) -> usize {
        match self {
            Entry::Word { line, .. } | Entry::Alias { line, .. } => *line,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub line: usize,
    pub message: String,
}

/// A pack file you can edit without losing its comments.
#[derive(Debug, Clone)]
pub struct PackFile {
    pub name: String,
    pub path: PathBuf,
    text: String,
}

impl PackFile {
    pub fn load(path: &Path) -> std::io::Result<Self> {
        Ok(Self::from_text(path, &std::fs::read_to_string(path)?))
    }

    pub fn from_text(path: &Path, text: &str) -> Self {
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        Self { name, path: path.to_path_buf(), text: text.to_string() }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, &self.text)?;
        std::fs::rename(&tmp, &self.path)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_proper(&self) -> bool {
        !self.text.lines().any(|l| l.split('#').next().unwrap_or("").trim() == "@kind common")
    }

    /// Names and titles (`true`) are protected from "fixes" and keep their capitals;
    /// ordinary words (`false`) are just added to the vocabulary.
    pub fn set_proper(&mut self, proper: bool) {
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        lines.retain(|l| !l.trim_start().starts_with("@kind"));
        if !proper {
            lines.insert(0, "@kind common".to_string());
        } else {
            lines.insert(0, "@kind proper".to_string());
        }
        self.text = lines.join("\n") + "\n";
    }

    pub fn entries(&self) -> Vec<Entry> {
        self.text
            .lines()
            .enumerate()
            .filter_map(|(i, raw)| {
                let line = raw.split('#').next().unwrap_or("").trim();
                if line.is_empty() || line.starts_with('@') {
                    None
                } else if let Some((w, r)) = line.split_once("->") {
                    Some(Entry::Alias { line: i, wrong: w.trim().into(), right: r.trim().into() })
                } else {
                    Some(Entry::Word { line: i, word: line.into() })
                }
            })
            .collect()
    }

    pub fn word_count(&self) -> usize {
        self.entries().iter().filter(|e| matches!(e, Entry::Word { .. })).count()
    }

    pub fn add_word(&mut self, word: &str) -> Result<(), String> {
        let word = clean(word)?;
        if self.entries().iter().any(|e| matches!(e, Entry::Word { word: w, .. } if w.eq_ignore_ascii_case(&word))) {
            return Err(format!("“{word}” is already in {}", self.name));
        }
        self.push_line(&word);
        Ok(())
    }

    pub fn add_alias(&mut self, wrong: &str, right: &str) -> Result<(), String> {
        let (wrong, right) = (clean(wrong)?, clean(right)?);
        if wrong.eq_ignore_ascii_case(&right) && wrong == right {
            return Err("Both sides are the same".into());
        }
        if self.entries().iter().any(|e| matches!(e, Entry::Alias { wrong: w, .. } if w.eq_ignore_ascii_case(&wrong))) {
            return Err(format!("“{wrong}” already has a correction"));
        }
        self.push_line(&format!("{wrong} -> {right}"));
        Ok(())
    }

    /// Replace the text of a line (used for inline edits).
    pub fn replace_line(&mut self, line: usize, new: &str) {
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        if line < lines.len() {
            lines[line] = new.trim().to_string();
            self.text = lines.join("\n") + "\n";
        }
    }

    pub fn remove_line(&mut self, line: usize) {
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        if line < lines.len() {
            lines.remove(line);
            self.text = if lines.is_empty() { String::new() } else { lines.join("\n") + "\n" };
        }
    }

    /// Problems worth showing, e.g. after an agent edited the file.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = vec![];
        let mut seen_words = std::collections::HashMap::<String, usize>::new();
        let mut seen_alias = std::collections::HashMap::<String, usize>::new();
        for (i, raw) in self.text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('@') {
                continue;
            }
            if line.matches("->").count() > 1 {
                issues.push(Issue { line: i, message: "More than one “->” on a line".into() });
            } else if let Some((w, r)) = line.split_once("->") {
                let (w, r) = (w.trim(), r.trim());
                if w.is_empty() || r.is_empty() {
                    issues.push(Issue { line: i, message: "A correction needs both sides".into() });
                } else if w == r {
                    issues.push(Issue { line: i, message: format!("“{w}” corrects to itself") });
                } else if let Some(first) = seen_alias.insert(w.to_lowercase(), i) {
                    issues.push(Issue { line: i, message: format!("“{w}” also appears on line {}", first + 1) });
                }
            } else if let Some(first) = seen_words.insert(line.to_lowercase(), i) {
                issues.push(Issue { line: i, message: format!("“{line}” also appears on line {}", first + 1) });
            }
        }
        issues
    }

    fn push_line(&mut self, line: &str) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        self.text.push_str(line);
        self.text.push('\n');
    }
}

fn clean(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.is_empty() {
        Err("Type something first".into())
    } else if s.contains('#') || s.contains("->") || s.starts_with('@') || s.contains('\n') {
        Err("Words can't contain #, -> or start with @".into())
    } else {
        Ok(s.to_string())
    }
}

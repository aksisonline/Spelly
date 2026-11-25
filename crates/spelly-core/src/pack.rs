use std::path::Path;

/// Defaults baked into the app, so Spelly works with nothing next to it.
const BUILT_IN: [(&str, &str); 2] = [
    ("anime", include_str!("../../../dictionaries/anime.txt")),
    ("indian-names", include_str!("../../../dictionaries/indian-names.txt")),
];

/// A plain-text dictionary pack (easy for humans and AI agents to extend).
///
/// ```text
/// # comment
/// @kind proper        # optional header: `proper` (names, titles) or `common`
/// Naruto              # a known word, canonical casing
/// narto -> Naruto     # forced correction
/// ```
#[derive(Debug, Clone)]
pub struct Pack {
    pub name: String,
    pub proper: bool,
    pub words: Vec<String>,
    /// (lowercase wrong form, replacement)
    pub aliases: Vec<(String, String)>,
}

impl Pack {
    pub fn parse(name: &str, text: &str) -> Pack {
        let mut pack = Pack { name: name.into(), proper: true, words: vec![], aliases: vec![] };
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(kind) = line.strip_prefix("@kind") {
                pack.proper = kind.trim() != "common";
            } else if let Some((wrong, right)) = line.split_once("->") {
                pack.aliases.push((wrong.trim().to_lowercase(), right.trim().to_string()));
            } else {
                pack.words.push(line.to_string());
            }
        }
        pack
    }
}

impl Pack {
    pub fn built_in() -> Vec<Pack> {
        BUILT_IN.iter().map(|(n, t)| Pack::parse(n, t)).collect()
    }

    /// Every `*.txt` in `dir`; the built-in packs if there are none.
    pub fn load_dir(dir: &Path) -> Vec<Pack> {
        let found: Vec<Pack> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "txt"))
            .filter_map(|p| {
                let name = p.file_stem()?.to_string_lossy().into_owned();
                Some(Pack::parse(&name, &std::fs::read_to_string(&p).ok()?))
            })
            .collect();
        if found.is_empty() { Self::built_in() } else { found }
    }

    /// Create `dir` and drop the built-in packs in it, if it has no packs yet.
    /// Gives people (and agents) real files to edit.
    pub fn seed_dir(dir: &Path) {
        let _ = std::fs::create_dir_all(dir);
        let has_packs = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "txt"));
        if !has_packs {
            for (name, text) in BUILT_IN {
                let _ = std::fs::write(dir.join(format!("{name}.txt")), text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_dir_falls_back_to_built_in() {
        let packs = Pack::load_dir(Path::new("/definitely/not/here"));
        assert_eq!(packs.len(), 2);
        assert!(packs.iter().any(|p| p.words.iter().any(|w| w == "Naruto")));
    }

    #[test]
    fn seed_then_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("spelly-seed-{}", std::process::id()));
        Pack::seed_dir(&dir);
        assert_eq!(Pack::load_dir(&dir).len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

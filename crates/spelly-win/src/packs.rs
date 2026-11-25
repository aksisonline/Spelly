use std::path::Path;

use spelly_core::Pack;

/// Defaults baked into the exe, so Spelly works with nothing next to it.
const BUILT_IN: [(&str, &str); 2] = [
    ("anime", include_str!("../../../dictionaries/anime.txt")),
    ("indian-names", include_str!("../../../dictionaries/indian-names.txt")),
];

/// Every `*.txt` in `dir` if there is one, otherwise the built-in packs.
/// Edit the files, pick "Reload dictionaries" in the tray, done.
pub fn load(dir: &Path) -> Vec<Pack> {
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
    if found.is_empty() {
        BUILT_IN.iter().map(|(n, t)| Pack::parse(n, t)).collect()
    } else {
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_built_in() {
        let packs = load(Path::new("/definitely/not/here"));
        assert_eq!(packs.len(), 2);
        assert!(packs.iter().any(|p| p.words.iter().any(|w| w == "Naruto")));
    }
}

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

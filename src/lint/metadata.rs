use super::{Issue, Kind, Severity};
use regex::Regex;
use std::sync::LazyLock;

pub(super) static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&(?:[a-zA-Z]{2,8}|#\d{2,5});").unwrap());
/// Top-level `key: value` line.
pub(super) static TOP_LEVEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9_ ]*):\s*(.*?)\s*$").unwrap());
pub(super) const PLACEHOLDERS: &[&str] = &["", "''", "\"\"", "null", "'null'", "\"null\"", "~"];
pub(super) const KEY_RENAMES: &[(&str, &str)] = &[
    ("source_url", "source"),
    ("prep_time", "prep time"),
    ("cook_time", "cook time"),
    ("total_time", "time required"),
    ("image_url", "image"),
];

pub(super) fn check(frontmatter: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    if serde_yaml::from_str::<serde_yaml::Value>(frontmatter).is_err() {
        issues.push(Issue::new(Kind::InvalidFrontmatter, Severity::Error, frontmatter, false));
    }
    let lines: Vec<&str> = frontmatter.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(cap) = TOP_LEVEL.captures(line) else { continue };
        let (key, value) = (&cap[1], &cap[2]);
        let has_children = lines.get(i + 1).is_some_and(|next| next.starts_with(' '));
        if PLACEHOLDERS.contains(&value) && !has_children {
            issues.push(Issue::new(Kind::EmptyMetadata, Severity::Warning, *line, true));
        }
        if KEY_RENAMES.iter().any(|(from, _)| *from == key) {
            issues.push(Issue::new(Kind::MetadataKeyDrift, Severity::Warning, *line, true));
        }
    }
    for m in ENTITY.find_iter(frontmatter) {
        issues.push(Issue::new(Kind::HtmlEntity, Severity::Warning, m.as_str(), true));
    }
    issues
}

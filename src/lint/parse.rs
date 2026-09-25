use super::{Issue, Kind, Severity};
use cooklang::Recipe;

/// Warnings that come from scraper metadata or are cosmetic — out of scope
/// for the converter. Frontmatter validity is checked in `metadata`.
const IGNORED: &[&str] = &[
    "Unsupported value for key",
    "Time overriden",
    "Unnecessary scaling lock",
    "frontmatter",
];

pub(super) fn parse(doc: &str) -> Result<(Recipe, Vec<Issue>), Vec<Issue>> {
    match cooklang::parse(doc).into_result() {
        Ok((recipe, report)) => {
            let issues = report
                .warnings()
                .map(|d| d.to_string())
                .filter(|m| !IGNORED.iter().any(|s| m.contains(s)))
                .map(|m| Issue::new(Kind::ParseWarning, Severity::Warning, m, false))
                .collect();
            Ok((recipe, issues))
        }
        Err(report) => Err(report
            .errors()
            .map(|d| Issue::new(Kind::ParseError, Severity::Error, d.to_string(), false))
            .collect()),
    }
}

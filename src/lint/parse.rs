use super::{Issue, Kind, Severity};
use cooklang::Recipe;

/// Warnings that come from scraper metadata or are cosmetic — out of scope
/// for the converter. Frontmatter validity is checked in `metadata`.
const IGNORED: &[&str] = &[
    // e.g. "Unsupported value for key: 'servings'" — a metadata value shaped
    // in a way cooklang's typed fields can't accept (arrays, nested maps).
    "Unsupported value for key",
    // "Time overriden" — an explicit `time required` conflicts with
    // `prep time`/`cook time` and cooklang keeps one of them.
    "Time overriden",
    // "Unnecessary scaling lock modifier" — a `%` lock on a quantity that
    // didn't need one; cosmetic.
    "Unnecessary scaling lock",
    // Cooklang's own frontmatter diagnostics: "Invalid YAML frontmatter
    // syntax: ...", "The frontmatter will be ignored...", and the deprecated
    // `>>` metadata warning. Frontmatter validity is checked by `metadata`,
    // not here.
    "frontmatter",
];

/// Parses `doc` and returns whatever recipe cooklang could build (even when
/// there were errors — cooklang is a best-effort parser) alongside the
/// diagnostics. Callers should still run consistency checks against the
/// recipe whenever it's `Some`, regardless of parse errors.
pub(super) fn parse(doc: &str) -> (Option<Recipe>, Vec<Issue>) {
    let (recipe, report) = cooklang::parse(doc).into_tuple();
    let mut issues: Vec<Issue> = report
        .errors()
        .map(|d| Issue::new(Kind::ParseError, Severity::Error, d.to_string(), false))
        .collect();
    issues.extend(
        report
            .warnings()
            .map(|d| d.to_string())
            .filter(|m| !IGNORED.iter().any(|s| m.contains(s)))
            .map(|m| Issue::new(Kind::ParseWarning, Severity::Warning, m, false)),
    );
    (recipe, issues)
}

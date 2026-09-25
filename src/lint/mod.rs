//! Deterministic quality checks for Cooklang conversions.
//!
//! `lint` compares a conversion against its original recipe text and returns
//! typed issues; `autofix` applies the mechanical rewrites that never change a
//! number, drop a step, or add an ingredient.

mod consistency;
mod fix;
mod metadata;
mod parse;
mod source;
mod tokens;

pub use fix::autofix;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    ParseError,
    ParseWarning,
    InvalidFrontmatter,
    MissingPercent,
    NoteInBraces,
    StrayBraces,
    OptionalColon,
    UnitAsNote,
    CookwareQuantity,
    CookwareJunk,
    TokenInNote,
    PercentInText,
    PhantomToken,
    TildeApprox,
    EmptyMetadata,
    HtmlEntity,
    MetadataKeyDrift,
    MissingIngredient,
    DoubleCount,
    SpellingVariant,
    LanguageMismatch,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Issue {
    pub kind: Kind,
    pub severity: Severity,
    pub evidence: String,
    pub autofixable: bool,
}

impl Issue {
    pub fn new(kind: Kind, severity: Severity, evidence: impl Into<String>, autofixable: bool) -> Self {
        let mut evidence: String = evidence.into();
        if evidence.chars().count() > 120 {
            evidence = evidence.chars().take(120).collect();
        }
        Self { kind, severity, evidence, autofixable }
    }
}

/// Splits `---\n<yaml>\n---` frontmatter from the body.
pub fn split_frontmatter(doc: &str) -> (Option<&str>, &str) {
    if let Some(rest) = doc.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let after = &rest[end + 4..];
            return (Some(&rest[..end]), after.trim_start_matches('\n'));
        }
    }
    (None, doc)
}

/// Checks a Cooklang conversion against the original recipe text.
pub fn lint(original: &str, cooklang: &str) -> Vec<Issue> {
    let (frontmatter, body) = split_frontmatter(cooklang);
    let mut issues = Vec::new();
    if let Some(fm) = frontmatter {
        issues.extend(metadata::check(fm));
    }
    issues.extend(tokens::check(body));
    let (recipe, diags) = parse::parse(cooklang);
    issues.extend(diags);
    if let Some(recipe) = &recipe {
        issues.extend(consistency::check(original, body, recipe));
    }
    issues
}

/// Weighted issue count used to compare two drafts: errors 3, warnings 1.
pub fn score(issues: &[Issue]) -> u32 {
    issues
        .iter()
        .map(|i| match i.severity {
            Severity::Error => 3,
            Severity::Warning => 1,
        })
        .sum()
}

/// Human-readable language of the original text (same detector as the prompt).
pub fn source_language(original: &str) -> Option<String> {
    whatlang::detect(original).map(|info| info.lang().eng_name().to_string())
}

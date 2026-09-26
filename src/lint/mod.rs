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
    UnderCount,
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
///
/// Tolerates a leading BOM and `\r\n` line endings, and requires the closing
/// fence to be exactly `---` on its own line (`\n---\n`, or `\n---` at EOF) —
/// a substring match would wrongly treat "---more" or "----" as a close.
pub fn split_frontmatter(doc: &str) -> (Option<&str>, &str) {
    let doc = doc.strip_prefix('\u{FEFF}').unwrap_or(doc);
    let Some(after_open) = doc.strip_prefix("---\r\n").or_else(|| doc.strip_prefix("---\n"))
    else {
        return (None, doc);
    };

    let mut pos = 0usize;
    loop {
        let rest = &after_open[pos..];
        let (line, next_pos, is_last) = match rest.find('\n') {
            Some(rel) => (&rest[..rel], pos + rel + 1, false),
            None => (rest, after_open.len(), true),
        };
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line == "---" {
            let fm = if pos == 0 {
                ""
            } else {
                let raw = &after_open[..pos - 1];
                raw.strip_suffix('\r').unwrap_or(raw)
            };
            let body = &after_open[next_pos.min(after_open.len())..];
            return (Some(fm), body.trim_start_matches(['\n', '\r']));
        }
        if is_last {
            break;
        }
        pos = next_pos;
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

/// Words in the original outside its extracted ingredient lines (bullets
/// stripped): `(total, longest line)`. Unspaced CJK text counts one word per
/// two CJK characters. An original that is only an ingredient list has
/// (almost) none — no method to convert, so a draft with steps invented them.
/// Not a lint issue; callers pick the thresholds.
pub fn method_words(original: &str) -> (usize, usize) {
    let listed: std::collections::HashSet<String> =
        source::source_ingredients(original).into_iter().map(|s| s.line).collect();
    let per_line: Vec<usize> = original
        .lines()
        .map(|l| l.trim().trim_start_matches(['▢', '•', '-', '*', '·', '–', '\u{200b}']).trim())
        // A line of 20+ CJK characters is a sentence even if the extractor listed it
        // (unspaced text has no word count or "." for its prose check to see).
        .filter(|l| !listed.contains(*l) || l.chars().filter(|c| consistency::is_cjk(*c)).count() >= 20)
        .map(|l| {
            let cjk = l.chars().filter(|c| consistency::is_cjk(*c)).count();
            let spaced = l.split_whitespace().filter(|w| w.chars().any(|c| c.is_alphabetic() && !consistency::is_cjk(c))).count();
            spaced + cjk / 2
        })
        .collect();
    (per_line.iter().sum(), per_line.iter().copied().max().unwrap_or(0))
}

/// Human-readable language of the original text (same detector as the prompt).
pub fn source_language(original: &str) -> Option<String> {
    whatlang::detect(original).map(|info| info.lang().eng_name().to_string())
}

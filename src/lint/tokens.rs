use super::{Issue, Kind, Severity};
use regex::Regex;
use std::sync::LazyLock;

/// `@name{content}(note)` — name may contain spaces, never another sigil.
pub(super) static INGREDIENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@\??([^@#~{}\n]+?)\{([^}\n]*)\}(\([^)\n]*\))?").unwrap()
});
static COOKWARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#([^@#~{}\n]+?)\{([^}\n]*)\}").unwrap());
static ANY_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[@#~]\??[^@#~{}\n]*\{[^}\n]*\}(\([^)\n]*\))?").unwrap());
/// Number, space, word(s), no `%`: "2 cups", "1 1/2 tbsp", "8 rashers".
pub(super) static SPACED_QTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(\d[\d./,]*(?:\s+\d+/\d+)?(?:\s*-\s*\d[\d./,]*)?)\s+([^\d%(),][^%(),]*?)\s*$").unwrap()
});
/// Number glued to unit: "200g", "15ml".
pub(super) static GLUED_QTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d[\d.,]*)([a-zA-Zµ]{1,5})\s*$").unwrap());
pub(super) static UNIT_AS_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"\{(\d[\d./\-]*)\}\((",
        r"cloves?|slices?|large|small|medium|sprigs?|heads?|bunch(?:es)?|sticks?|cans?|leaves|",
        r"gousses?|tranches?|bâtons?|brins?|têtes?|",
        r"Zehen?|Scheiben?|Stängel|Bund|Dosen?",
        r")\)"
    ))
    .unwrap()
});
static STRAY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\}\{[^}\n]*\}").unwrap());
/// `\d%<unit>` glued together in prose, e.g. "200%ml water" — a converter
/// artifact where the `%` scaling sigil leaked into plain text. Restricted to
/// a unit whitelist (rather than any letter) so it doesn't fire on languages
/// where a percentage is followed directly by a suffix, e.g. German
/// "70%ige Schokolade".
static PCT_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\d%(?:g|kg|mg|ml|cl|dl|l|tsp|tbsp|cups?|oz|lb|el|tl|min|minutes?|hours?)\b")
        .unwrap()
});
static PHANTOM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@\d|@[^@#~{}\n]*\d+\s*(?:°|g|ml|C|F)\{").unwrap());
static TILDE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"~\s*\d").unwrap());

pub(super) fn strip_tokens(text: &str) -> String {
    ANY_TOKEN.replace_all(text, "").into_owned()
}

pub(super) fn check(body: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |kind, severity, evidence: &str, fix| {
        issues.push(Issue::new(kind, severity, evidence, fix))
    };

    for cap in INGREDIENT.captures_iter(body) {
        let whole = &cap[0];
        let content = &cap[2];
        if content.contains('(') {
            push(Kind::NoteInBraces, Severity::Error, whole, false);
        } else if !content.contains('%')
            && (SPACED_QTY.is_match(content) || GLUED_QTY.is_match(content))
        {
            push(Kind::MissingPercent, Severity::Error, whole, true);
        }
        if cap[1].contains('%') {
            push(Kind::PercentInText, Severity::Error, whole, false);
        }
    }
    for m in UNIT_AS_NOTE.find_iter(body) {
        push(Kind::UnitAsNote, Severity::Error, m.as_str(), true);
    }
    for m in STRAY.find_iter(body) {
        push(Kind::StrayBraces, Severity::Error, m.as_str(), true);
    }
    if body.contains("@?:") {
        push(Kind::OptionalColon, Severity::Error, "@?:", true);
    }
    for cap in COOKWARE.captures_iter(body) {
        let name = &cap[1];
        let junk = name.chars().count() > 40
            || name.contains([')', '>', ';', ':', '!', '(']);
        if junk {
            push(Kind::CookwareJunk, Severity::Error, &cap[0], false);
        } else if !cap[2].trim().is_empty() {
            push(Kind::CookwareQuantity, Severity::Warning, &cap[0], false);
        }
    }
    for line in body.lines().filter(|l| l.starts_with('>')) {
        if ANY_TOKEN.is_match(line) {
            push(Kind::TokenInNote, Severity::Warning, line, true);
        }
    }
    let prose = strip_tokens(body);
    for m in PCT_TEXT.find_iter(&prose) {
        push(Kind::PercentInText, Severity::Error, m.as_str(), false);
    }
    for m in PHANTOM.find_iter(body) {
        push(Kind::PhantomToken, Severity::Error, m.as_str(), false);
    }
    for m in TILDE.find_iter(&prose) {
        push(Kind::TildeApprox, Severity::Warning, m.as_str(), true);
    }
    issues
}

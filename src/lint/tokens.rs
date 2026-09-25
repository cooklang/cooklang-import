use super::{Issue, Kind, Severity};
use regex::Regex;
use std::sync::LazyLock;

/// `@name{content}(note)` — name may contain spaces, never another sigil, and
/// never starts with a space. Tolerates a space between `}` and the note's
/// opening `(`, which converters sometimes emit ("{4} (cloves)").
pub(super) static INGREDIENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@\??([^\s@#~{}][^@#~{}\n]*?)\{([^}\n]*)\}\s*(\([^)\n]*\))?").unwrap()
});
/// Name must not start with a space, so a stray '#' followed by prose
/// ("# Step 1 {x}") doesn't read as cookware.
static COOKWARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#([^\s@#~{}][^@#~{}\n]*?)\{([^}\n]*)\}").unwrap());
static ANY_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[@#~]\??[^@#~{}\n]*\{[^}\n]*\}(\([^)\n]*\))?").unwrap());
/// A single number: a whole number followed by a slash fraction ("1 1/2"), a
/// bare slash fraction ("1/2"), digits (dot/comma decimal) optionally
/// followed by a unicode fraction ("1 ½"), or a standalone unicode fraction
/// (½¼¾⅓⅔⅛).
const NUM: &str = r"(?:\d+\s+\d+/\d+|\d+/\d+|\d[\d.,]*(?:\s*[½¼¾⅓⅔⅛])?|[½¼¾⅓⅔⅛])";
/// A unit/word phrase: never a digit or a fraction char (that would belong
/// to the number, not the unit).
const UNIT_WORDS: &str = r"[^\d%(),½¼¾⅓⅔⅛][^%(),½¼¾⅓⅔⅛]*?";
/// Range joiners spelled with words instead of a dash, across the languages
/// seen in imports: English "to"/"or", French "à", German "bis"/"oder".
const RANGE_WORD: &str = r"(?:to|à|bis|oder|or)";

/// Number [range] space word(s), no `%`: "2 cups", "1 ½ cups", "2–3 large".
pub(super) static SPACED_QTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^\s*{NUM}(?:\s*[-–—]\s*{NUM})?\s+({UNIT_WORDS})\s*$")).unwrap()
});
/// Same shape, but the range is spelled out ("2 to 3 cups") rather than a
/// dash. Still a MissingPercent defect, but the autofixer must not touch it
/// (rewriting "2 to 3" into a single cooklang quantity isn't mechanical).
pub(super) static SPACED_QTY_WORD_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)^\s*{NUM}\s+{RANGE_WORD}\s+{NUM}\s+({UNIT_WORDS})\s*$"
    ))
    .unwrap()
});
/// Number glued to unit: "200g", "2Stück". Unicode letters so non-ASCII unit
/// abbreviations count, but a bare "x" ("2x", a scaling shorthand) is not a
/// unit — filtered by the caller.
pub(super) static GLUED_QTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d[\d.,]*)(\p{L}{1,6})\s*$").unwrap());
/// A plain quantity with no unit and no range — the shape that belongs in
/// `UnitAsNote`'s `{qty}`, e.g. "4", "1,5", "½".
static QTY_PLAIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\d[\d.,]*|[½¼¾⅓⅔⅛])$").unwrap());
/// A unit word at the start of the `(...)` note that should have been the
/// `%unit` inside the braces instead, e.g. `{4}(cloves)` or `{4}(cloves,
/// minced)` — matched at the start only, so trailing prep text after a comma
/// doesn't prevent the match.
static UNIT_WORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)^(",
        r"cloves?|slices?|large|small|medium|sprigs?|heads?|bunch(?:es)?|sticks?|cans?|leaves|",
        r"gousses?|tranches?|bâtons?|brins?|têtes?|",
        r"Zehen?|Scheiben?|Stängel|Bund|Dosen?|Stangen|Zweige|Blätter|Stück|Prise|",
        r"feuilles|pincées?|sachets?|boîtes?",
        r")\b"
    ))
    .unwrap()
});
static STRAY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\}\{([^}\n]*)\}").unwrap());
/// `\d%<unit>` glued together in prose, e.g. "200%ml water" — a converter
/// artifact where the `%` scaling sigil leaked into plain text. Restricted to
/// a unit whitelist (rather than any letter) so it doesn't fire on languages
/// where a percentage is followed directly by a suffix, e.g. German
/// "70%ige Schokolade".
static PCT_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\d%(?:g|kg|mg|ml|cl|dl|l|tsp|tbsp|cups?|oz|lb|el|tl|min|minutes?|hours?)\b")
        .unwrap()
});
/// An ingredient "name" that's actually a bare temperature or mass, e.g.
/// `@180C{}` or `@350°F{}` — the converter dropped the real ingredient name.
/// Requires the digits to be immediately (no separator) followed by the
/// unit, so a genuine name that merely starts with a digit — `@7-spice
/// mix{1%tsp}`, `@00 Mehl{500%g}` — isn't flagged.
static PHANTOM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@\d+(?:[.,]\d+)?\s*(?:°\s*[CF]?|[CF]\b|g\b|kg\b|ml\b)").unwrap()
});
static TILDE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"~\s*\d").unwrap());

pub(super) fn strip_tokens(text: &str) -> String {
    ANY_TOKEN.replace_all(text, "").into_owned()
}

/// A `-- comment` or `> note` line — never a real step, so a missing `%` or
/// an unescaped `~digit` there isn't a converter defect worth flagging.
fn is_comment_or_note_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("--") || t.starts_with('>')
}

pub(super) fn check(body: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |kind, severity, evidence: &str, fix| {
        issues.push(Issue::new(kind, severity, evidence, fix))
    };

    for line in body.lines() {
        let skip = is_comment_or_note_line(line);
        for cap in INGREDIENT.captures_iter(line) {
            let whole = &cap[0];
            let content = &cap[2];
            if content.contains('(') {
                push(Kind::NoteInBraces, Severity::Error, whole, false);
            } else if !skip && !content.contains('%') {
                if SPACED_QTY_WORD_RANGE.is_match(content) {
                    push(Kind::MissingPercent, Severity::Error, whole, false);
                } else if SPACED_QTY.is_match(content) {
                    push(Kind::MissingPercent, Severity::Error, whole, true);
                } else if let Some(g) = GLUED_QTY.captures(content) {
                    if !g[2].eq_ignore_ascii_case("x") {
                        push(Kind::MissingPercent, Severity::Error, whole, true);
                    }
                }
            }
            if cap[1].contains('%') {
                push(Kind::PercentInText, Severity::Error, whole, false);
            }
            if let Some(note) = cap.get(3) {
                let note_inner = note.as_str().trim_start_matches('(').trim_end_matches(')');
                if QTY_PLAIN.is_match(content) && UNIT_WORD.is_match(note_inner) {
                    push(Kind::UnitAsNote, Severity::Error, whole, true);
                }
            }
        }
        if !skip {
            let prose_line = strip_tokens(line);
            for m in TILDE.find_iter(&prose_line) {
                push(Kind::TildeApprox, Severity::Warning, m.as_str(), true);
            }
        }
    }
    for cap in STRAY.captures_iter(body) {
        // Only a bare `{}` stray pair is a safe delete; `{2}` might hold a
        // real (if misplaced) value the autofixer shouldn't discard.
        let fixable = cap[1].is_empty();
        push(Kind::StrayBraces, Severity::Error, &cap[0], fixable);
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
    for line in body.lines().filter(|l| l.trim_start().starts_with('>')) {
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
    issues
}

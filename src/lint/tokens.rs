use super::{Issue, Kind, Severity};
use regex::Regex;
use std::sync::LazyLock;

/// `@name{content}(note)` — name may contain spaces, never another sigil, and
/// never starts with a space. Captures the whitespace between `}` and `(`
/// separately (group 3) from the note itself (group 4): in real Cooklang a
/// note is glued to `}`, so `(` preceded by a space is ordinary prose in
/// parentheses, not a note — `check()` only treats it as a note when that
/// whole parenthetical is a single unit word.
pub(super) static INGREDIENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@\??([^\s@#~{}][^@#~{}\n]*?)\{([^}\n]*)\}(\s*)(\([^)\n]*\))?").unwrap()
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
/// The unit-word alternation shared by UNIT_WORD and UNIT_WORD_EXACT.
const UNIT_WORD_ALT: &str = concat!(
    r"cloves?|slices?|large|small|medium|sprigs?|heads?|bunch(?:es)?|sticks?|cans?|leaves|",
    r"gousses?|tranches?|bâtons?|brins?|têtes?|",
    r"Zehen?|Scheiben?|Stängel|Bund|Dosen?|Stangen|Zweige|Blätter|Stück|Prise|",
    r"feuilles|pincées?|sachets?|boîtes?",
);
/// A unit word at the start of the `(...)` note that should have been the
/// `%unit` inside the braces instead, e.g. `{4}(cloves)` or `{4}(cloves,
/// minced)` — matched at the start only, so trailing prep text after a comma
/// doesn't prevent the match. Only used for the glued `{n}(...)` form, where
/// the whole parenthetical is a genuine cooklang note.
static UNIT_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?i)^({UNIT_WORD_ALT})\b")).unwrap());
/// Same word list, but the whole (trimmed) text must be nothing but the unit
/// word — used for `{n} (...)` (a space before the paren), where the
/// parenthetical is ordinary prose unless it's exactly one unit word, e.g.
/// `{4} (cloves)` but not `{1} (medium heat)`.
static UNIT_WORD_EXACT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?i)^(?:{UNIT_WORD_ALT})$")).unwrap());
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

/// For an autofixable MissingPercent ingredient quantity, returns the fixed
/// `{qty%unit}` content — inserting the `%` between the number and the unit
/// without ever touching a digit. Returns `None` for a word-spelled range
/// ("2 to 3 cups") or anything neither SPACED_QTY nor GLUED_QTY (minus a bare
/// "x") recognizes.
///
/// Not yet called from non-test code: `fix::autofix` (a later task) will use
/// this to rewrite MissingPercent issues.
#[allow(dead_code)]
pub(super) fn fixed_quantity(raw: &str) -> Option<String> {
    let content = raw.trim();
    if SPACED_QTY_WORD_RANGE.is_match(content) {
        return None;
    }
    if let Some(cap) = SPACED_QTY.captures(content) {
        let unit = cap.get(1).unwrap();
        let number = content[..unit.start()].trim_end();
        return Some(format!("{number}%{}", unit.as_str().trim()));
    }
    if let Some(cap) = GLUED_QTY.captures(content) {
        if !cap[2].eq_ignore_ascii_case("x") {
            return Some(format!("{}%{}", &cap[1], &cap[2]));
        }
    }
    None
}

/// For an autofixable UnitAsNote ingredient — a plain quantity (`qty`, e.g.
/// "4") followed by a note (`note`, e.g. "cloves, minced") that starts with a
/// unit word — returns the fixed `{qty%unit}` content plus whatever prep text
/// is left in the note (`None` if nothing remains), e.g.
/// `("4", "cloves, minced")` -> `("4%cloves", Some("minced"))`.
///
/// Not yet called from non-test code: `fix::autofix` (a later task) will use
/// this to rewrite UnitAsNote issues.
#[allow(dead_code)]
pub(super) fn fixed_unit_as_note(qty: &str, note: &str) -> Option<(String, Option<String>)> {
    if !QTY_PLAIN.is_match(qty) {
        return None;
    }
    let note = note.trim();
    let m = UNIT_WORD.find(note)?;
    if m.start() != 0 {
        return None;
    }
    let rest = note[m.end()..].trim_start_matches(',').trim();
    let remaining = if rest.is_empty() { None } else { Some(rest.to_string()) };
    Some((format!("{qty}%{}", m.as_str()), remaining))
}

/// A `-- comment`, `> note`, or `[- block comment -]` line — never a real
/// step, so a missing `%` or an unescaped `~digit` there isn't a converter
/// defect worth flagging. Only recognizes a block comment that starts the
/// line; one inline alongside real content is left to the other checks.
fn is_comment_or_note_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("--") || t.starts_with('>') || t.starts_with("[-")
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
            if let Some(note) = cap.get(4) {
                let note_inner = note.as_str().trim_start_matches('(').trim_end_matches(')');
                let has_space = !cap[3].is_empty();
                let is_unit = if has_space {
                    UNIT_WORD_EXACT.is_match(note_inner.trim())
                } else {
                    UNIT_WORD.is_match(note_inner)
                };
                if QTY_PLAIN.is_match(content) && is_unit {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_quantity_inserts_percent_between_number_and_unit() {
        assert_eq!(fixed_quantity("2 cups"), Some("2%cups".to_string()));
        assert_eq!(fixed_quantity("1 1/2 cups"), Some("1 1/2%cups".to_string()));
        assert_eq!(fixed_quantity("½ cup"), Some("½%cup".to_string()));
        assert_eq!(fixed_quantity("200g"), Some("200%g".to_string()));
        assert_eq!(fixed_quantity("2–3 large"), Some("2–3%large".to_string()));
    }

    #[test]
    fn fixed_quantity_never_touches_word_ranges_or_bare_x() {
        assert_eq!(fixed_quantity("2 to 3 cups"), None);
        assert_eq!(fixed_quantity("2 bis 3 EL"), None);
        assert_eq!(fixed_quantity("2x"), None);
    }

    #[test]
    fn fixed_unit_as_note_splits_unit_from_remaining_prep_text() {
        assert_eq!(
            fixed_unit_as_note("4", "cloves, minced"),
            Some(("4%cloves".to_string(), Some("minced".to_string())))
        );
        assert_eq!(fixed_unit_as_note("4", "cloves"), Some(("4%cloves".to_string(), None)));
        assert_eq!(
            fixed_unit_as_note("1,5", "Zehen"),
            Some(("1,5%Zehen".to_string(), None))
        );
        assert_eq!(fixed_unit_as_note("½", "head"), Some(("½%head".to_string(), None)));
    }

    #[test]
    fn fixed_unit_as_note_rejects_non_plain_quantities_and_non_unit_notes() {
        assert_eq!(fixed_unit_as_note("2 cups", "cloves"), None);
        assert_eq!(fixed_unit_as_note("4", "diced"), None);
    }
}

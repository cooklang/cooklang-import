//! Insert cooklang markup into source text; never emit unparseable output.

use super::bio::Span;
use super::entry::Entry;
use super::linker::{is_unit_word, link};
use super::timer::parse_timer;
use super::words::Word;
use std::sync::OnceLock;

/// Amount-resolution rules per ING mention — see module tests.
pub fn assemble_paragraph(
    text: &str,
    words: &[Word],
    spans: &[Span],
    entries: &mut [Entry],
    used: &mut [bool],
) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    for span in spans {
        let (mut start, end) = (words[span.start].start, words[span.end - 1].end);
        if start < cursor {
            continue; // overlapping span — keep the earlier one
        }
        let surface = &text[start..end];
        let markup = match span.kind.as_str() {
            "ING" => {
                let linked = link(surface, entries);
                let local = local_amount(words, span.start);
                match (linked, local) {
                    (Some(i), Some((amt, amt_start))) if amt_start >= cursor => {
                        start = amt_start;
                        Some(format!("@{}{{{}}}", clean(&entries[i].name), amt))
                    }
                    (None, Some((amt, amt_start))) if amt_start >= cursor => {
                        start = amt_start;
                        Some(format!("@{}{{{}}}", clean(surface), amt))
                    }
                    (Some(i), _) if !used[i] => {
                        used[i] = true;
                        let e = &entries[i];
                        let amount = match (&e.qty, &e.unit) {
                            (Some(q), Some(u)) => format!("{q}%{u}"),
                            (Some(q), None) => q.clone(),
                            _ => String::new(),
                        };
                        let mut m = format!("@{}{{{}}}", clean(&e.name), amount);
                        if let Some(p) = &e.prep {
                            m.push_str(&format!("({})", clean(p)));
                        }
                        Some(m)
                    }
                    (Some(_), _) => None, // back-reference: plain
                    (None, _) => Some(format!("@{}{{}}", clean(surface))),
                }
            }
            "COOK" => Some(format!("#{}{{}}", clean(surface))),
            "TIMER" => parse_timer(surface).map(|(q, u)| format!("~{{{q}%{u}}}")),
            _ => None,
        };
        out.push_str(&text[cursor..start]);
        match markup {
            Some(m) => out.push_str(&m),
            None => out.push_str(&text[start..end]),
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// Detect "<qty> [unit]" immediately before word index `i`; returns the
/// amount string (qty%unit or bare qty) and the byte offset where it begins.
///
/// v1 scope: only numeric or vulgar-fraction quantities are recognized
/// ("2", "4-5", "½"). Word-quantifiers ("a", "half", "some") are not
/// detected here and intentionally fall through to the ingredient-list
/// amount rule in `assemble_paragraph`.
fn local_amount(words: &[Word], i: usize) -> Option<(String, usize)> {
    fn is_qty(t: &str) -> bool {
        t.chars().next().is_some_and(|c| c.is_ascii_digit())
            || matches!(t, "½" | "¼" | "¾" | "⅓" | "⅔" | "⅛")
    }
    if i >= 2 && is_qty(&words[i - 2].text) && is_unit_word(&words[i - 1].text) {
        return Some((
            format!("{}%{}", words[i - 2].text, words[i - 1].text),
            words[i - 2].start,
        ));
    }
    if i >= 1 && is_qty(&words[i - 1].text) {
        return Some((words[i - 1].text.clone(), words[i - 1].start));
    }
    None
}

/// Strip cooklang metachars from emitted names/notes.
fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '@' | '#' | '~' | '{' | '}' | '(' | ')' | '%'))
        .collect()
}

/// Cached parser: upstream docs note `CooklangParser` construction can be
/// heavy (it builds the unit-conversion tables), and `validate_or_plain` is
/// called once per converted paragraph — build it once, not per call.
fn parser() -> &'static cooklang::CooklangParser {
    static PARSER: OnceLock<cooklang::CooklangParser> = OnceLock::new();
    PARSER.get_or_init(|| {
        cooklang::CooklangParser::new(cooklang::Extensions::all(), cooklang::Converter::default())
    })
}

fn parses_clean(text: &str) -> bool {
    !parser().parse(text).into_tuple().1.has_errors()
}

/// Parse-check a converted paragraph; fall back to the unmarked source on
/// error. That fallback is normally safe (no markup means nothing for the
/// parser to trip on) — except when the *source recipe itself* contains a
/// literal cooklang metacharacter in ordinary prose (an old recipe footer's
/// "~or-", a stray "@handle", a "#1" callout), which can make even the
/// unmarked text fail to parse. In that last-resort case, sanitize just
/// enough of it to parse rather than giving up the "never emit unparseable
/// output" guarantee.
pub fn validate_or_plain(converted: &str, plain: &str) -> String {
    if parses_clean(converted) {
        return converted.to_string();
    }
    if parses_clean(plain) {
        return plain.to_string();
    }
    // Replace the three "opens a construct" metacharacters with readable
    // ASCII lookalikes — favors legibility over outright deletion, and
    // resolves the common case (no unmatched `{`/`}` involved).
    let lookalikes = plain
        .replace('~', "-")
        .replace('@', "at ")
        .replace('#', "No. ");
    if parses_clean(&lookalikes) {
        return lookalikes;
    }
    // Still broken — a `{`/`}` pair (or a metachar in a spot the above
    // didn't fix) is also in play. Strip every cooklang metachar outright;
    // some text loss is preferable to an unparseable recipe.
    lookalikes
        .chars()
        .filter(|c| !matches!(c, '@' | '#' | '~' | '{' | '}'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::words::word_tokenize;
    use super::*;

    fn entry(name: &str, qty: Option<&str>, unit: Option<&str>, prep: Option<&str>) -> Entry {
        Entry {
            name: name.to_string(),
            qty: qty.map(str::to_string),
            unit: unit.map(str::to_string),
            prep: prep.map(str::to_string),
        }
    }

    fn span(kind: &str, start: usize, end: usize) -> Span {
        Span {
            kind: kind.to_string(),
            start,
            end,
        }
    }

    #[test]
    fn first_mention_gets_full_markup_second_stays_plain() {
        let text = "Divide beef in half. Season beef well.";
        let words = word_tokenize(text);
        let spans = vec![span("ING", 1, 2), span("ING", 6, 7)];
        let mut entries = vec![entry("Ground Beef", Some("10"), Some("ounce"), None)];
        let mut used = vec![false];
        let out = assemble_paragraph(text, &words, &spans, &mut entries, &mut used);
        assert_eq!(
            out,
            "Divide @Ground Beef{10%ounce} in half. Season beef well."
        );
    }

    #[test]
    fn split_usage_local_amounts_mark_every_usage() {
        let text = "Combine \u{bd} TBSP mayonnaise now. Spread 1 TBSP mayonnaise on buns.";
        let words = word_tokenize(text);
        // tokens: Combine ½ TBSP mayonnaise now . Spread 1 TBSP mayonnaise on buns .
        let spans = vec![span("ING", 3, 4), span("ING", 9, 10)];
        let mut entries = vec![entry("Mayonnaise", Some("1"), Some("TBSP"), None)];
        let mut used = vec![false];
        let out = assemble_paragraph(text, &words, &spans, &mut entries, &mut used);
        assert_eq!(
            out,
            "Combine @Mayonnaise{\u{bd}%TBSP} now. Spread @Mayonnaise{1%TBSP} on buns."
        );
    }

    #[test]
    fn cookware_and_timer_markup() {
        let text = "Toast on a baking sheet for 2-3 minutes.";
        let words = word_tokenize(text);
        // tokens: Toast on a baking sheet for 2-3 minutes .
        let spans = vec![span("COOK", 3, 5), span("TIMER", 6, 8)];
        let mut entries = Vec::new();
        let mut used = Vec::new();
        let out = assemble_paragraph(text, &words, &spans, &mut entries, &mut used);
        assert_eq!(out, "Toast on a #baking sheet{} for ~{2-3%minutes}.");
    }

    #[test]
    fn unlinked_mention_gets_bare_markup() {
        let text = "Add chocolate now.";
        let words = word_tokenize(text);
        let spans = vec![span("ING", 1, 2)];
        let mut entries: Vec<Entry> = Vec::new();
        let mut used: Vec<bool> = Vec::new();
        let out = assemble_paragraph(text, &words, &spans, &mut entries, &mut used);
        assert_eq!(out, "Add @chocolate{} now.");
    }

    #[test]
    fn validated_output_parses_or_falls_back() {
        // "Empty quantity value" — a real cooklang parse error (verified against
        // the cooklang 0.18 parser directly): unit given with no value before `%`.
        let out = validate_or_plain("@x{%cup}", "plain text");
        assert!(out.contains("plain"), "{out}");
    }

    #[test]
    fn validate_or_plain_sanitizes_stray_metachars_when_plain_itself_wont_parse() {
        // Regression: real recipe source text (an old MasterCook-era footer)
        // containing a literal "~or-" and "@" addresses in plain prose, with
        // no markup ever inserted around them (the paragraph had no spans at
        // all, so `converted == plain` — the old fallback couldn't rescue it
        // since both attempts were the exact same broken string).
        let text = "jpmd44a@prodigy.com ~or- MAD-SQUAD@prodigy.net";
        let out = validate_or_plain(text, text);
        assert!(parses_clean(&out), "expected parseable output, got {out:?}");
        assert!(!out.contains('~'), "{out}");
        assert!(!out.contains('@'), "{out}");
        // Lookalike substitution should be enough here (no `{`/`}` involved),
        // so the human-readable text is still mostly recognizable.
        assert!(out.contains("prodigy.com"), "{out}");
    }

    #[test]
    fn validate_or_plain_strips_unmatched_braces_as_last_resort() {
        // A brace with no opening metachar before it (so the lookalike swap
        // alone can't fix it) still must not survive into unparseable output.
        let text = "Preheat to 350{F} and set timer, no closing brace here.";
        let out = validate_or_plain(text, text);
        assert!(parses_clean(&out), "expected parseable output, got {out:?}");
    }

    #[test]
    fn validate_or_plain_reuses_cached_parser_across_calls() {
        // The parser is a lazily-initialized OnceLock; calling validate_or_plain
        // repeatedly (as the paragraph-by-paragraph pipeline does) must keep
        // giving correct, independent results rather than reusing stale state.
        assert_eq!(
            validate_or_plain("Add @salt{1%tsp}.", "plain"),
            "Add @salt{1%tsp}."
        );
        assert_eq!(validate_or_plain("@x{%cup}", "plain"), "plain");
        assert_eq!(
            validate_or_plain("Add @salt{1%tsp}.", "plain"),
            "Add @salt{1%tsp}."
        );
    }
}

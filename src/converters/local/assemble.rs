//! Insert cooklang markup into source text; never emit unparseable output.

use super::bio::Span;
use super::entry::Entry;
use super::linker::{is_unit_word, link};
use super::timer::parse_timer;
use super::words::Word;

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

/// Parse-check a converted paragraph; fall back to plain source on error.
pub fn validate_or_plain(converted: &str, plain: &str) -> String {
    let parser =
        cooklang::CooklangParser::new(cooklang::Extensions::all(), cooklang::Converter::default());
    let (_, report) = parser.parse(converted).into_tuple();
    if report.has_errors() {
        plain.to_string()
    } else {
        converted.to_string()
    }
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
}

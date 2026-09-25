//! Pulls the ingredient list out of the original recipe text so conversions
//! can be checked against it.
//!
//! Extraction is anchored on a recognized "Ingredients" header when one is
//! present (reading until a "Method"/"Instructions" header or a prose line),
//! and falls back to runs of three or more list-like lines otherwise. Both
//! paths route every candidate line through [`looks_like_ingredient_line`],
//! which rejects prose, section headers and known junk (nav/time/yield/step
//! markers) — real-world exports mix all of these into the "ingredients"
//! text blob.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SourceIngredient {
    pub line: String,
    /// Lower-cased ingredient name without quantity, unit or trailing notes.
    pub name: String,
    pub qty: Option<f64>,
    pub unit: Option<String>,
}

/// Leading "qty [-hi] rest": captures the first number (group 1), an
/// optional dash-range upper bound (group 2), and everything after (group
/// 3). The upper bound is what gets used as `qty` when present, matching
/// `consistency::numeric`, which reads a *tagged* range's `end` too.
static LEADING_QTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d+\s+\d+/\d+|\d+/\d+|\d+(?:[.,]\d+)?|[½¼¾⅓⅔⅛])(?:\s*[-–]\s*([\d.,/]+))?\s*(.*)$").unwrap()
});
/// Trailing "name qty [unit]", e.g. "Zucchini 200 g" or "Молоко: 120 мл" — a
/// format some real exports use instead of a leading quantity.
static TRAILING_QTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<name>[^\d]+?)[\s:]+(?P<q>\d+(?:[.,]\d+)?|\d+/\d+|[½¼¾⅓⅔⅛])(?:\s*[-–]\s*(?P<qhi>\d+(?:[.,]\d+)?))?\s*(?P<u>[\p{L}.]{1,12}(?:\s[\p{L}.]{1,4})?)?\s*$",
    )
    .unwrap()
});
static PAREN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\([^()]*\)").unwrap());
/// A recognized "Ingredients" header, in the languages seen in imports.
static ING_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\W*(ingredients?|zutaten|ingrédients|ingredientes|ingrediënten|ingredienti|інгредієнти|ингредиенты|składniki|ingredienser|wat heb je nodig|you will need|what you need)\b").unwrap()
});
/// A recognized "Method"/"Instructions" header — ends the ingredients run
/// when one is found after an `ING_HEADER` anchor.
static METHOD_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\W*(instructions|method|directions|preparation|steps|zubereitung|anleitung|préparation|preparación|bereiding|спосіб|приготування|приготовление|przygotowanie)\b").unwrap()
});
/// Navigation, timing, yield, step markers and other non-ingredient noise
/// that real exports leave inline with the ingredient list.
static JUNK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:(?:yield|serves|servings|makes|portions?|prep|cook|total|time|step\s*\d+|stap|notes?|nutrition|equipment|print|imprimer|video|vidéo|jump to|pin|rate|dauer|schwierigkeit|quantité)\b|title:|\d+\s*(?:h|hr|hrs|min|mins|minutes)\b|\d+h\d*)").unwrap()
});
const UNITS: &[&str] = &[
    "g", "gr", "kg", "mg", "ml", "cl", "dl", "l", "tsp", "tbsp", "tbs", "tbls", "teaspoon", "teaspoons",
    "tablespoon", "tablespoons", "cup", "cups", "oz", "ounce", "ounces", "lb", "lbs", "lbs.", "pound",
    "pounds", "pinch", "can", "cans", "clove", "cloves", "el", "tl", "msp", "prise", "zehe",
    "zehen", "bund", "c.", "c", "cs", "cc", "càs", "càc", "cuillère", "cuillères", "gousse",
    "gousses", "pincée", "sachet", "tranche", "tranches", "stück", "stk.", "pkt.", "slice", "slices",
    "tsp.", "tbsp.", "шт.", "шт", "г", "кг", "мл", "л",
];
/// Multi-word units that a single-token lookup (`UNITS`) can't catch —
/// checked as a literal prefix of the text after the number.
const MULTI_WORD_UNITS: &[&str] = &["c. à s.", "c. à c."];
const BULLETS: &[char] = &['▢', '•', '-', '*', '·', '–', '\u{200b}'];

fn parse_number(s: &str) -> Option<f64> {
    match s {
        "½" => return Some(0.5),
        "¼" => return Some(0.25),
        "¾" => return Some(0.75),
        "⅓" => return Some(1.0 / 3.0),
        "⅔" => return Some(2.0 / 3.0),
        "⅛" => return Some(0.125),
        _ => {}
    }
    let mut total = 0.0;
    for part in s.split_whitespace() {
        if let Some((n, d)) = part.split_once('/') {
            total += n.parse::<f64>().ok()? / d.parse::<f64>().ok()?;
        } else {
            total += part.replace(',', ".").parse::<f64>().ok()?;
        }
    }
    Some(total)
}

/// A short "für/for the/pour la/para/для/voor" sub-heading with no digits —
/// distinct from `ING_HEADER`, which anchors the whole extraction.
static SUBHEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(für|for the|for|pour la|pour le|pour les|para el|para la|voor de|для|dla)\b").unwrap());

/// A section header: "Zutaten:", "## Für das Dressing", "FOR THE CAKE",
/// "Wet ingredients--", or a short "für/for the/..." sub-heading with no
/// digits. Never an ingredient line, and never allowed to break a run — it's
/// simply skipped.
fn is_header(line: &str) -> bool {
    let words = line.split_whitespace().count();
    let all_caps = line.chars().any(|c| c.is_alphabetic())
        && line.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
        && words <= 5
        && !line.chars().next().is_some_and(|c| c.is_ascii_digit());
    line.starts_with('#')
        || line.ends_with(':')
        || line.ends_with("--")
        || all_caps
        || (words <= 5 && SUBHEAD.is_match(line) && !line.chars().any(|c| c.is_ascii_digit()))
        || ING_HEADER.is_match(line)
        || METHOD_HEADER.is_match(line)
}

/// A line counts as prose only when it ends with "." and has 5+ words, or is
/// longer than 160 characters — an abbreviation like "Stk." or "tsp." ends
/// with a period too, but rarely reaches 5 words on its own line. A very
/// long, comma-strung line with no period at all (common in run-on
/// instructions) is still treated as prose once it's implausibly long for an
/// ingredient line, scaled up when it does look like a quantity line (so a
/// long ingredient name with a note doesn't get misread as instructions).
/// A period followed by whitespace and a capital letter — a new sentence
/// starting mid-line ("Kartoffeln schälen. Kartoffeln kochen.") — is prose
/// regardless of the line's total word count. An abbreviation's period
/// ("c. à s.", "2 c. à c. huile") is always followed by a *lowercase*
/// continuation, so it doesn't trip this.
static MID_SENTENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.\s+\p{Lu}").unwrap());

fn is_prose(line: &str) -> bool {
    let words = line.split_whitespace().count();
    let has_qty = LEADING_QTY.is_match(line.trim_start_matches(BULLETS).trim());
    line.chars().count() > 160
        || (line.ends_with('.') && words >= 5)
        || words >= if has_qty { 25 } else { 12 }
        || MID_SENTENCE.is_match(line)
}

fn looks_like_ingredient_line(line: &str) -> bool {
    line.chars().count() > 1
        && !is_prose(line)
        && !is_header(line)
        && !JUNK.is_match(line)
        && line.chars().any(|c| c.is_alphabetic())
}

/// Strips every parenthetical (there can be more than one: "500 ml (2 cups)
/// milk (whole)"), then cuts at the first comma — the rest is prep notes.
fn clean_name(rest: &str) -> String {
    let mut s = rest.to_string();
    while PAREN.is_match(&s) {
        s = PAREN.replace_all(&s, " ").into_owned();
    }
    let name = s.split(',').next().unwrap_or("").trim().trim_end_matches('.').trim().to_lowercase();
    for prefix in ["of ", "de ", "d'", "d’"] {
        if let Some(stripped) = name.strip_prefix(prefix) {
            return stripped.trim().to_string();
        }
    }
    name
}

/// If `rest` starts with one of `MULTI_WORD_UNITS` (case-insensitively),
/// returns that unit and whatever follows it.
fn strip_multi_word_unit(rest: &str) -> Option<(String, String)> {
    let lower = rest.to_lowercase();
    for mw in MULTI_WORD_UNITS {
        if lower.starts_with(mw) {
            return Some((mw.to_string(), rest[mw.len()..].trim().to_string()));
        }
    }
    None
}

fn parse_line(raw: &str) -> Option<SourceIngredient> {
    let line = raw.trim().trim_start_matches(BULLETS).trim();
    if !looks_like_ingredient_line(line) {
        return None;
    }
    let (qty, unit, rest) = if let Some(cap) = LEADING_QTY.captures(line) {
        let rest = cap.get(3).map_or("", |m| m.as_str());
        let qty = cap.get(2).and_then(|m| parse_number(m.as_str())).or_else(|| parse_number(&cap[1]));
        if let Some((mw, remainder)) = strip_multi_word_unit(rest) {
            (qty, Some(mw), remainder)
        } else {
            let first = rest.split_whitespace().next().unwrap_or("");
            if UNITS.contains(&first.to_lowercase().as_str()) {
                (qty, Some(first.to_string()), rest[first.len()..].trim().to_string())
            } else {
                (qty, None, rest.to_string())
            }
        }
    } else if let Some(cap) = TRAILING_QTY.captures(line) {
        let qty = cap.name("qhi").and_then(|m| parse_number(m.as_str())).or_else(|| parse_number(&cap["q"]));
        (qty, cap.name("u").map(|m| m.as_str().to_string()), cap["name"].trim_end_matches(':').to_string())
    } else {
        (None, None, line.to_string())
    };
    let name = clean_name(&rest);
    (!name.is_empty()).then(|| SourceIngredient { line: line.to_string(), name, qty, unit })
}

/// Whether a run of candidate lines is dense enough with quantities to
/// accept even below the 3-line minimum (a 2-line run needs at least one
/// quantified line).
fn has_a_quantity(line: &str) -> bool {
    let t = line.trim_start_matches(BULLETS).trim();
    LEADING_QTY.is_match(t) || TRAILING_QTY.is_match(t)
}

pub(super) fn source_ingredients(original: &str) -> Vec<SourceIngredient> {
    let text = original.replace(['\r', '\u{200b}'], "");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();

    // 1. Anchored: everything after a recognized "Ingredients" header, until
    // a "Method" header or a prose line — the common, well-formed case.
    if let Some(h) = lines.iter().position(|l| ING_HEADER.is_match(l) && l.split_whitespace().count() <= 5) {
        let mut out = Vec::new();
        for l in &lines[h + 1..] {
            // Check for the method-header boundary before skipping generic
            // header-shaped lines — "Method:" ends with ':' too, and must
            // stop the read rather than be silently skipped over.
            if METHOD_HEADER.is_match(l) {
                break;
            }
            if l.is_empty() || l.ends_with(':') || l.starts_with('#') {
                continue;
            }
            if is_prose(l) {
                break;
            }
            out.extend(parse_line(l));
        }
        if !out.is_empty() {
            return out;
        }
    }

    // 2. Fallback: runs of list-like lines (blank lines and headers don't
    // break a run — only prose, junk or a non-list line does). A run needs
    // 3+ lines, or 2 lines with at least one quantity, to count.
    let mut out = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    let flush = |run: &mut Vec<&str>, out: &mut Vec<SourceIngredient>| {
        let quantified = run.iter().filter(|l| has_a_quantity(l)).count();
        if run.len() >= 3 || (run.len() == 2 && quantified >= 1) {
            out.extend(run.iter().filter_map(|l| parse_line(l)));
        }
        run.clear();
    };
    for l in &lines {
        if l.is_empty() || is_header(l) {
            continue;
        }
        if looks_like_ingredient_line(l) {
            run.push(l);
        } else {
            flush(&mut run, &mut out);
        }
    }
    flush(&mut run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GERMAN: &str = "500 g kleine Kartoffeln (Drillinge)\n1 Zehe Knoblauch\n\
        1 EL Olivenöl\ngrobes Meersalz\n\nDen Backofen vorheizen. Die Kartoffeln waschen.";

    #[test]
    fn reads_the_ingredient_block_only() {
        let got = source_ingredients(GERMAN);
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].name, "kleine kartoffeln");
        assert_eq!(got[0].qty, Some(500.0));
        assert_eq!(got[0].unit.as_deref(), Some("g"));
        assert_eq!(got[3].name, "grobes meersalz");
        assert_eq!(got[3].qty, None);
    }

    #[test]
    fn parses_fractions_bullets_and_notes() {
        let got = source_ingredients("▢ 1 1/2 cups sugar, divided\n• ½ tsp salt\n\nMix well.");
        assert_eq!(got[0].qty, Some(1.5));
        assert_eq!(got[0].unit.as_deref(), Some("cups"));
        assert_eq!(got[0].name, "sugar");
        assert_eq!(got[1].qty, Some(0.5));
    }

    /// Item 1: a sub-heading ("For the dressing:") is skipped, and the two
    /// quantified lines after it are still extracted (a 2-line run needs a
    /// quantity, which both of these have).
    #[test]
    fn skips_section_headers_and_prose() {
        let got = source_ingredients(
            "For the dressing:\n2 tbsp oil\n1 tbsp vinegar\n\nWhisk everything together until smooth and glossy.",
        );
        assert_eq!(got.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["oil", "vinegar"]);
    }

    /// Item 1: the "Ingredients" anchor reads until the "Method" header,
    /// including a single item (no 2-3 line minimum applies on this path).
    #[test]
    fn anchor_header_extracts_a_single_item_until_method_header() {
        let got = source_ingredients("Ingredients:\n2 tbsp oil\n\nMethod:\n1. Whisk.\n2. Serve.");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "oil");
    }

    /// Item 1: three list-like lines with no header or quantity still form
    /// a run under the fallback path.
    #[test]
    fn fallback_accepts_a_three_line_run_without_quantities() {
        let got = source_ingredients("eggs\nflour\nsugar\n\nMix everything together and bake until golden and firm.");
        assert_eq!(got.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["eggs", "flour", "sugar"]);
    }

    /// Item 1 regression: a short (<5-word) multi-sentence instruction line
    /// must not slip into an extracted "ingredient" run just because it's
    /// short — a mid-line ". Capital" is a new sentence, so it's prose even
    /// when the whole line has fewer than 5 words.
    #[test]
    fn short_multi_sentence_instruction_line_is_not_an_ingredient() {
        let got = source_ingredients("500 g Kartoffeln\n2 Möhren\n\nKartoffeln schälen. Kartoffeln kochen.");
        assert_eq!(got.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["kartoffeln", "möhren"]);
    }

    /// Item 1: navigation/step/yield junk lines mixed into an anchored
    /// ingredient list are dropped rather than extracted as ingredients.
    #[test]
    fn junk_lines_are_filtered_out() {
        let got = source_ingredients("Ingredients:\nPrint\nStep 1\nServes 4\n2 eggs\n1 cup milk\n\nMethod: mix well and bake for a while.");
        assert_eq!(got.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["eggs", "milk"]);
    }

    /// Item 1: an ALL-CAPS sub-header doesn't break the run that follows it.
    #[test]
    fn all_caps_header_is_skipped_without_breaking_the_run() {
        let got = source_ingredients(
            "FOR THE CAKE\n2 eggs\n1 cup flour\n1 cup sugar\n\nBake for a while, please make sure oven is hot enough today.",
        );
        assert_eq!(got.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["eggs", "flour", "sugar"]);
    }

    /// Item 6: every parenthetical is stripped (not just the first), then
    /// the name is cut at the first comma.
    #[test]
    fn clean_name_strips_every_parenthetical_then_cuts_at_comma() {
        let got = source_ingredients("500 ml (2 cups) milk (whole), cold\n1 egg\n1 tsp vanilla\n\nWhisk everything together well.");
        assert_eq!(got[0].name, "milk");
    }

    /// Item 7: an abbreviation ending in "." with few words is not treated
    /// as prose, so it stays a valid ingredient line.
    #[test]
    fn abbreviation_period_does_not_read_as_prose() {
        let got = source_ingredients("Pitas 4 Stk.\nFeta 0.5 Packungen\nPaprika 2 Stk.\n\nAlles klein schneiden und servieren.");
        assert_eq!(got.len(), 3);
    }

    /// Item 8: a dash range stores the upper bound, matching how a tagged
    /// range's `end` is compared in `consistency::numeric`.
    #[test]
    fn dash_range_stores_the_upper_bound() {
        let got = source_ingredients("2-3 tbsp oil\n1 onion\n\nHeat the oil and cook the onion until soft.");
        assert_eq!(got[0].qty, Some(3.0));
    }

    /// Item 9: a trailing quantity ("name qty unit") is recognized, not just
    /// a leading one.
    #[test]
    fn trailing_quantity_is_recognized() {
        let got = source_ingredients("Zucchini 200 g\nWeizenmehl 3 EL\n\nAlles zusammen verrühren und abschmecken.");
        assert_eq!(got[0].name, "zucchini");
        assert_eq!(got[0].qty, Some(200.0));
        assert_eq!(got[0].unit.as_deref(), Some("g"));
        assert_eq!(got[1].name, "weizenmehl");
        assert_eq!(got[1].qty, Some(3.0));
        assert_eq!(got[1].unit.as_deref(), Some("EL"));
    }

    /// Item 13: newly recognized unit abbreviations, including the
    /// multi-word French "c. à s." / "c. à c.".
    #[test]
    fn recognizes_newly_added_units() {
        let got = source_ingredients("2 tbs butter\n1 lbs. sugar\n2 c. à s. huile\n\nMélangez tout ensemble et servez chaud.");
        assert_eq!(got[0].unit.as_deref(), Some("tbs"));
        assert_eq!(got[1].unit.as_deref(), Some("lbs."));
        assert_eq!(got[2].unit.as_deref(), Some("c. à s."));
        assert_eq!(got[2].name, "huile");
    }
}

//! Pulls the ingredient list out of the original recipe text so conversions
//! can be checked against it.

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

static LEADING_QTY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d+\s+\d+/\d+|\d+/\d+|\d+(?:[.,]\d+)?|[½¼¾⅓⅔⅛])(?:\s*[-–]\s*[\d.,/]+)?\s*(.*)$").unwrap()
});
const UNITS: &[&str] = &[
    "g", "gr", "kg", "mg", "ml", "cl", "dl", "l", "tsp", "tbsp", "teaspoon", "teaspoons",
    "tablespoon", "tablespoons", "cup", "cups", "oz", "ounce", "ounces", "lb", "lbs", "pound",
    "pounds", "pinch", "can", "cans", "clove", "cloves", "el", "tl", "msp", "prise", "zehe",
    "zehen", "bund", "c.", "c", "cs", "cc", "càs", "càc", "cuillère", "cuillères", "gousse",
    "gousses", "pincée", "sachet", "tranche", "tranches", "stück", "pkt.", "slice", "slices",
];
const BULLETS: &[char] = &['▢', '•', '-', '*', '·', '–'];

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

fn looks_like_ingredient_line(line: &str) -> bool {
    let n = line.chars().count();
    n > 1 && n <= 80 && !line.ends_with('.') && !line.ends_with(':') && !line.starts_with('#')
}

fn clean_name(rest: &str) -> String {
    let no_paren = rest.split('(').next().unwrap_or(rest);
    let no_note = no_paren.split(',').next().unwrap_or(no_paren);
    let name = no_note.trim().to_lowercase();
    for prefix in ["of ", "de ", "d'", "d’"] {
        if let Some(stripped) = name.strip_prefix(prefix) {
            return stripped.trim().to_string();
        }
    }
    name
}

fn parse_line(raw: &str) -> Option<SourceIngredient> {
    let line = raw.trim().trim_start_matches(BULLETS).trim();
    if !looks_like_ingredient_line(line) {
        return None;
    }
    let (qty, unit, rest) = match LEADING_QTY.captures(line) {
        Some(cap) => {
            let rest = cap.get(2).map_or("", |m| m.as_str());
            let first = rest.split_whitespace().next().unwrap_or("");
            if UNITS.contains(&first.to_lowercase().as_str()) {
                (parse_number(&cap[1]), Some(first.to_string()), rest[first.len()..].trim())
            } else {
                (parse_number(&cap[1]), None, rest)
            }
        }
        None => (None, None, line),
    };
    let name = clean_name(rest);
    (!name.is_empty()).then(|| SourceIngredient { line: line.to_string(), name, qty, unit })
}

/// A section header ("For the dressing:", "## Für das Dressing") — never an
/// ingredient line, and excluded from the block-detection ratio below so a
/// short ingredient block introduced by a header isn't misread as prose.
fn is_section_header(line: &str) -> bool {
    line.starts_with('#') || line.ends_with(':')
}

pub(super) fn source_ingredients(original: &str) -> Vec<SourceIngredient> {
    let text = original.replace('\r', "");
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let headers = lines.iter().filter(|l| is_section_header(l)).count();
        let relevant = lines.len() - headers;
        let listy = lines.iter().filter(|l| looks_like_ingredient_line(l)).count();
        // An ingredient block is mostly short, unpunctuated lines (ignoring
        // any section headers mixed into it).
        if lines.len() >= 2 && relevant > 0 && listy * 10 >= relevant * 6 {
            out.extend(lines.iter().filter_map(|l| parse_line(l)));
        }
    }
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

    #[test]
    fn skips_section_headers_and_prose() {
        let got = source_ingredients("For the dressing:\n2 tbsp oil\n\nWhisk everything together until smooth and glossy.");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "oil");
    }
}

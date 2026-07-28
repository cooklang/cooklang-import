//! Deterministic mention -> ingredient-entry linking (v1: normalized token
//! overlap; a learned linker is the planned upgrade path).

use super::entry::Entry;

/// Unit vocabulary shared with assemble's local-amount detection.
/// Single-letter forms are case-sensitive (T = tablespoon, t = teaspoon).
pub fn is_unit_word(w: &str) -> bool {
    const UNITS: &[&str] = &[
        "cup",
        "cups",
        "tablespoon",
        "tablespoons",
        "tbsp",
        "teaspoon",
        "teaspoons",
        "tsp",
        "ounce",
        "ounces",
        "oz",
        "pound",
        "pounds",
        "lb",
        "lbs",
        "gram",
        "grams",
        "g",
        "kg",
        "ml",
        "l",
        "clove",
        "cloves",
        "slice",
        "slices",
        "piece",
        "pieces",
        "can",
        "cans",
        "pinch",
        "dash",
        "handful",
        "drizzle",
        "splash",
    ];
    matches!(w, "T" | "t") || UNITS.contains(&w.to_lowercase().as_str())
}

fn norm(word: &str) -> String {
    let w = word.to_lowercase();
    if let Some(s) = w.strip_suffix("ies") {
        return format!("{s}y");
    }
    for suf in ["oes", "es", "s"] {
        if let Some(stripped) = w.strip_suffix(suf) {
            if stripped.len() >= 3 {
                return stripped.to_string();
            }
        }
    }
    w
}

fn norm_set(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(norm)
        .collect()
}

/// Returns the index of the best-matching entry, or None below threshold.
pub fn link(mention: &str, entries: &[Entry]) -> Option<usize> {
    let m = norm_set(mention);
    if m.is_empty() {
        return None;
    }
    let mut best: Option<(usize, f32)> = None;
    for (i, e) in entries.iter().enumerate() {
        let n = norm_set(&e.name);
        let overlap = m.iter().filter(|t| n.contains(t)).count() as f32;
        // fraction of the mention covered — "vinegar" fully covered by
        // "Balsamic Vinegar" scores 1.0
        let score = overlap / m.len() as f32;
        if (score >= 0.99 || (score >= 0.5 && m.len() > 1)) && best.is_none_or(|(_, s)| score > s) {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> Entry {
        Entry {
            name: name.to_string(),
            qty: None,
            unit: None,
            prep: None,
        }
    }

    fn entries() -> Vec<Entry> {
        vec![
            entry("Ground Beef"),
            entry("Roma Tomato"),
            entry("Balsamic Vinegar"),
        ]
    }

    #[test]
    fn links_exact_and_partial_mentions() {
        let e = entries();
        assert_eq!(link("ground beef", &e), Some(0));
        assert_eq!(link("tomato", &e), Some(1));
        assert_eq!(link("vinegar", &e), Some(2));
    }

    #[test]
    fn links_plural_mention_to_singular_entry() {
        let e = vec![entry("Brioche Buns")];
        assert_eq!(link("bun", &e), Some(0));
    }

    #[test]
    fn unmatched_mention_returns_none() {
        let e = entries();
        assert_eq!(link("chocolate", &e), None);
    }

    #[test]
    fn is_unit_word_cases() {
        assert!(is_unit_word("TBSP"));
        assert!(is_unit_word("T"));
        assert!(is_unit_word("t"));
        assert!(is_unit_word("Cups"));
        assert!(!is_unit_word("x"));
    }
}

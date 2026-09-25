use super::source::{source_ingredients, SourceIngredient};
use super::{Issue, Kind, Severity};
use cooklang::{Recipe, Value};
use std::collections::BTreeMap;

fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Singular-ish key: strips a trailing "s" from every word, so
/// "asperges vertes" / "asperge verte" and "oranges" / "orange" collide.
/// Used only for `SpellingVariant`, which groups whole ingredient names —
/// word-level matching (`covers`, language check) uses `similar` instead,
/// which also catches "tomatoes"/"tomato" and non-Latin scripts.
fn stem(name: &str) -> String {
    normalize(name)
        .split_whitespace()
        .map(|w| w.strip_suffix('s').unwrap_or(w))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generic descriptor words that don't count as coverage/language-match
/// evidence on their own — two ingredients sharing only "fresh" or "oil"
/// isn't evidence they're the same ingredient.
const STOP: &[&str] = &[
    "and", "the", "with", "for", "fresh", "ground", "large", "small", "medium", "chopped", "minced",
    "sliced", "oil", "sauce", "powder", "whole", "dried", "optional", "taste", "more", "plus",
    "finely", "diced", "extra", "about", "into", "cut", "room", "temperature", "serving",
    // German
    "und", "oder", "der", "die", "das", "mit", "frisch", "öl", "soße", "pulver",
    // French
    "des", "les", "pour", "une", "huile", "poudre",
    // Spanish
    "con", "del", "los", "las",
];

fn words_raw(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_string)
        .collect()
}

/// Same as `words_raw`, but drops generic `STOP` words — they're too weak
/// to count as coverage/language-match evidence by themselves.
fn words(s: &str) -> Vec<String> {
    words_raw(s).into_iter().filter(|w| !STOP.contains(&w.as_str())).collect()
}

/// A CJK ideograph, kana or hangul syllable — scripts with no whitespace
/// between words, where `words`'s alnum-splitting doesn't segment anything
/// meaningful.
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x2E80..=0x2EFF | 0x3040..=0x30FF | 0x3400..=0x4DBF |
        0x4E00..=0x9FFF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF)
}

/// True once more than 30% of the alphabetic characters are CJK — used to
/// exempt CJK-original recipes from the word-overlap language check, which
/// doesn't apply the same way to unspaced scripts.
fn is_mostly_cjk(s: &str) -> bool {
    let alpha = s.chars().filter(|c| c.is_alphabetic()).count();
    if alpha == 0 {
        return false;
    }
    let cjk = s.chars().filter(|c| is_cjk(*c)).count();
    (cjk as f64) / (alpha as f64) > 0.3
}

/// Two words match exactly, or share a common prefix of 3+ characters that
/// covers most of the shorter word — "Zwiebeln"/"Zwiebel", "tomatoes"/
/// "tomato", "кави"/"кава" all collide; unrelated words with a short
/// accidental prefix ("egg"/"eggplant" — p=3, min=3, but only by coincidence
/// of "egg" being a prefix of a much longer, unrelated word) still slip
/// through this rule since it works on curated ingredient words, but the
/// stricter word-boundary check in double-count's own name lookup is what
/// actually guards that case.
fn similar(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let p = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    let m = a.chars().count().min(b.chars().count());
    p >= 3 && p + 1 >= m
}

/// A token covers a source line when: they share a CJK character (unspaced
/// scripts), or they share a `similar` word, or — when the token has no
/// (non-stop) words of its own, e.g. a bare "Öl" that's also generic — the
/// token's whole name appears in the source line.
fn covers(token: &str, src: &SourceIngredient) -> bool {
    if token.chars().any(is_cjk) {
        let src_cjk: std::collections::HashSet<char> = src.line.chars().filter(|c| is_cjk(*c)).collect();
        if token.chars().filter(|c| is_cjk(*c)).any(|c| src_cjk.contains(&c)) {
            return true;
        }
    }
    let tok_words = words(token);
    let src_words = words(&src.line);
    if !tok_words.is_empty() {
        return tok_words.iter().any(|w| src_words.iter().any(|s| similar(w, s)));
    }
    let whole = normalize(token);
    if whole.is_empty() {
        return false;
    }
    src.line.to_lowercase().contains(&whole) || words_raw(&src.line).iter().any(|w| similar(w, &whole))
}

fn numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(n.value()),
        Value::Range { end, .. } => Some(end.value()),
        Value::Text(_) => None,
    }
}

/// Collapses unit spelling/language variants that mean the same measure, so
/// a source line in one language/abbreviation style can be compared against
/// a doc tag in another. Falls back to the (trimmed, lower-cased) input
/// unchanged for anything not in the table, so two identical unknown units
/// still compare equal.
fn norm_unit(u: &str) -> String {
    let lower = u.trim().to_lowercase();
    let trimmed = lower.trim_end_matches('.');
    match trimmed {
        "tbsp" | "tbs" | "tbls" | "tablespoon" | "tablespoons" | "el" | "càs" | "c. à s" | "cs" => "tbsp",
        "tsp" | "teaspoon" | "teaspoons" | "tl" | "càc" | "c. à c" | "cc" => "tsp",
        "cup" | "cups" | "c" => "cup",
        "g" | "gr" | "gram" | "grams" | "gramm" | "г" => "g",
        "kg" | "кг" => "kg",
        "ml" | "мл" => "ml",
        "cl" => "cl",
        "l" | "л" | "liter" | "litre" => "l",
        "oz" | "ounce" | "ounces" => "oz",
        "lb" | "lbs" | "pound" | "pounds" => "lb",
        "clove" | "cloves" => "clove",
        other => return other.to_string(),
    }
    .to_string()
}

/// A source line that explicitly splits its quantity across the recipe
/// ("2 tbsp oil, divided") — tagging it more than once is correct, not a
/// double count.
static DIVIDED: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)divided|plus more|\+|split|reserved|geteilt|en deux|dividido").unwrap());

/// Finds `needle` in `haystack` as a whole word (bounded by non-alphanumeric
/// characters or the string edges) — "salt" matches "salt, pepper" but not
/// "unsalted butter"; "egg" matches "egg" but not "eggplant". Manual scan
/// instead of a per-name regex compile.
fn word_boundary_match(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let idx = start + pos;
        let before_ok = haystack[..idx].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let end_idx = idx + needle.len();
        let after_ok = haystack[end_idx..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        // Advance past exactly one *character* (not byte) so the next slice
        // stays on a UTF-8 boundary — a raw `idx + 1` panics mid-character
        // on any multi-byte script (Cyrillic, accented Latin, CJK).
        start = idx + haystack[idx..].chars().next().map_or(1, char::len_utf8);
        if start > haystack.len() {
            break;
        }
    }
    false
}

pub(super) fn check(original: &str, _body: &str, recipe: &Recipe) -> Vec<Issue> {
    let mut issues = Vec::new();
    let sources = source_ingredients(original);
    let names: Vec<String> = recipe.ingredients.iter().map(|i| normalize(&i.name)).collect();

    // Coverage: every *quantified* source ingredient is tagged somewhere. A
    // source line with no quantity ("salt to taste") is skipped outright —
    // too easy to conflate with something else, not worth the noise.
    for src in &sources {
        if src.qty.is_some() && !names.iter().any(|n| covers(n, src)) {
            issues.push(Issue::new(Kind::MissingIngredient, Severity::Error, &src.line, false));
        }
    }

    // Double count: summed tagged amounts (in the same normalized unit)
    // exceed a single, unambiguous source listing that isn't itself
    // "divided"/"plus more" prep text.
    let mut totals: BTreeMap<(String, String), (f64, usize)> = BTreeMap::new();
    for ing in &recipe.ingredients {
        let Some(q) = &ing.quantity else { continue };
        let Some(v) = numeric(q.value()) else { continue };
        let unit = q.unit().unwrap_or("").to_lowercase();
        let entry = totals.entry((normalize(&ing.name), unit)).or_insert((0.0, 0));
        entry.0 += v;
        entry.1 += 1;
    }
    for ((name, unit), (sum, count)) in &totals {
        if *count < 2 {
            continue;
        }
        let listed: Vec<&SourceIngredient> =
            sources.iter().filter(|s| word_boundary_match(&s.name, name)).collect();
        if let [only] = listed.as_slice() {
            if DIVIDED.is_match(&only.line) {
                continue;
            }
            if norm_unit(only.unit.as_deref().unwrap_or("")) != norm_unit(unit) {
                continue;
            }
            if let Some(q) = only.qty {
                if *sum > q * 1.01 {
                    let evidence = format!("@{name}: tagged {sum} {unit} in {count} places, source lists \"{}\"", only.line);
                    issues.push(Issue::new(Kind::DoubleCount, Severity::Error, evidence, false));
                }
            }
        }
    }

    // Spelling variants: two distinct names with the same stem.
    let mut by_stem: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for n in &names {
        let group = by_stem.entry(stem(n)).or_default();
        if !group.contains(&n) {
            group.push(n);
        }
    }
    for group in by_stem.values().filter(|g| g.len() > 1) {
        let evidence = group.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" / ");
        issues.push(Issue::new(Kind::SpellingVariant, Severity::Warning, evidence, false));
    }

    // Language: with 3+ distinct ingredient names, most of them should share
    // a word with the original text — a wholesale translation instead of a
    // conversion. A mostly-CJK original is exempted: unspaced scripts don't
    // segment into comparable words the same way.
    let mut distinct_names: Vec<String> = names.clone();
    distinct_names.sort();
    distinct_names.dedup();
    if distinct_names.len() >= 3 && !is_mostly_cjk(original) {
        let orig_lower = original.to_lowercase();
        let orig_words = words_raw(original);
        let hits = distinct_names
            .iter()
            .filter(|n| {
                let nw = words(n);
                if nw.is_empty() {
                    orig_words.iter().any(|w| similar(w, n)) || orig_lower.contains(n.as_str())
                } else {
                    nw.iter().any(|w| orig_words.iter().any(|o| similar(w, o)))
                }
            })
            .count();
        if (hits as f64) < 0.5 * distinct_names.len() as f64 {
            let sample: Vec<&str> = distinct_names.iter().take(5).map(String::as_str).collect();
            let evidence = format!("ingredient names not found in source: {}", sample.join(", "));
            issues.push(Issue::new(Kind::LanguageMismatch, Severity::Error, evidence, false));
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_matches_prefix_variants_across_scripts() {
        assert!(similar("zwiebel", "zwiebeln"));
        assert!(similar("tomato", "tomatoes"));
        assert!(similar("кава", "кави"));
        // "salt" vs "unsalted" share no common prefix from position 0 ('s' vs
        // 'u'), so the prefix rule naturally rejects it. Note "egg" is a
        // prefix of "eggplant" and *does* satisfy this rule (p=3, m=3) — the
        // stricter word_boundary_match (tested below) is what keeps
        // egg/eggplant apart in double-count's own name lookup, not this
        // word-level similarity check.
        assert!(!similar("salt", "unsalted"));
    }

    #[test]
    fn covers_allows_two_character_words() {
        let src = SourceIngredient { line: "1 Ei".into(), name: "ei".into(), qty: Some(1.0), unit: None };
        assert!(covers("ei", &src));
        let src = SourceIngredient { line: "1 EL Öl".into(), name: "öl".into(), qty: Some(1.0), unit: Some("EL".into()) };
        assert!(covers("öl", &src));
    }

    #[test]
    fn covers_matches_cjk_by_character_containment() {
        let src = SourceIngredient { line: "洋葱 一个".into(), name: "洋葱".into(), qty: Some(1.0), unit: None };
        assert!(covers("洋葱", &src));
        assert!(!covers("胡萝卜", &src));
    }

    #[test]
    fn word_boundary_match_rejects_substrings_inside_other_words() {
        assert!(word_boundary_match("2 eggs", "eggs"));
        assert!(!word_boundary_match("1 eggplant", "egg"));
        assert!(!word_boundary_match("unsalted butter", "salt"));
        assert!(word_boundary_match("salt, black pepper", "salt"));
    }

    /// Regression (found against real data): a rejected match must advance
    /// the scan by one *character*, not one byte — a raw `idx + 1` panics
    /// mid-character on any multi-byte script when the first candidate
    /// match is rejected and the search has to continue past it.
    #[test]
    fn word_boundary_match_does_not_panic_on_multi_byte_text() {
        // "смл" contains "мл" as a raw substring, rejected by the boundary
        // check ('с' is alphanumeric); advancing past the rejection must not
        // split the 2-byte Cyrillic 'м' — the real match is later in the string.
        assert!(word_boundary_match("смл 100 мл", "мл"));
        // "café" contains 'é' preceded by 'f' (rejected); advancing past it
        // must not split the 2-byte 'é' either.
        assert!(!word_boundary_match("café au lait", "é"));
    }

    #[test]
    fn norm_unit_collapses_spelling_and_language_variants() {
        assert_eq!(norm_unit("EL"), "tbsp");
        assert_eq!(norm_unit("Tbs"), "tbsp");
        assert_eq!(norm_unit("TL"), "tsp");
        assert_eq!(norm_unit("Kg"), "kg");
        assert_ne!(norm_unit("cup"), norm_unit("g"));
    }

    #[test]
    fn is_mostly_cjk_detects_unspaced_scripts() {
        assert!(is_mostly_cjk("洋葱切丝，加盐调味，翻炒均匀即可食用。"));
        assert!(!is_mostly_cjk("Die Zwiebel schälen und fein hacken."));
    }
}

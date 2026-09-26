use super::source::{source_ingredients, SourceIngredient};
use super::{Issue, Kind, Severity};
use cooklang::{Recipe, Value};
use std::collections::BTreeMap;

/// Folds the French ligatures "œ"/"Œ" and "æ"/"Æ" to their two-letter
/// spelling ("oe"/"ae") — real exports mix both spellings of the same word
/// ("œufs" vs "oeufs"), and without this a bare word-prefix comparison sees
/// them as unrelated (the very first character already differs).
fn fold_ligatures(s: &str) -> String {
    s.replace(['œ', 'Œ'], "oe").replace(['æ', 'Æ'], "ae")
}

fn normalize(name: &str) -> String {
    fold_ligatures(name.trim()).to_lowercase()
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
    fold_ligatures(s)
        .to_lowercase()
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
pub(super) fn is_cjk(c: char) -> bool {
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
///
/// `src_words` is `src.line`'s word set, computed once by the caller — with
/// N sources and M tagged names, splitting it fresh on every call here made
/// this O(N × M) string-split operations instead of O(N), which is the
/// difference between milliseconds and seconds on a page-sized dump.
fn covers(token: &str, src: &SourceIngredient, src_words: &[String]) -> bool {
    if token.chars().any(is_cjk) {
        let src_cjk: std::collections::HashSet<char> = src.line.chars().filter(|c| is_cjk(*c)).collect();
        if token.chars().filter(|c| is_cjk(*c)).any(|c| src_cjk.contains(&c)) {
            return true;
        }
    }
    let tok_words = words(token);
    if !tok_words.is_empty() {
        return tok_words.iter().any(|w| src_words.iter().any(|s| similar(w, s)));
    }
    let whole = normalize(token);
    if whole.is_empty() {
        return false;
    }
    fold_ligatures(&src.line).to_lowercase().contains(&whole) || words_raw(&src.line).iter().any(|w| similar(w, &whole))
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
/// double count. Deliberately excludes "split" on its own: the `regex`
/// crate has no lookaround to write `\bsplit\b(?!\s+peas)`, and "split" as a
/// prep verb (rather than "divided") is rare enough that a plain ingredient
/// name like "split peas" isn't worth risking.
static DIVIDED: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)divided|plus more|\+|reserved|geteilt|en deux|dividido").unwrap());

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

/// Above this many extracted "sources", the original isn't an ingredient
/// list at all — it's a page-sized dump (a scraped page, a pasted PDF, raw
/// HTML) that happened to produce thousands of short, list-shaped lines.
/// Coverage checking is O(sources × names) in the worst case, so it's
/// skipped entirely rather than risk turning `lint` quadratic on garbage
/// input that was never going to be a real ingredient list anyway.
const MAX_SOURCES_FOR_COVERAGE: usize = 200;
/// However uncovered a recipe's source ingredients are, one recipe is never
/// worth more than this many `MissingIngredient` issues — past this point
/// every additional one is the same underlying defect (a botched or
/// abandoned conversion), not new information.
const MAX_MISSING_INGREDIENT_ISSUES: usize = 20;
/// Below this many words, the original doesn't carry enough text to judge
/// language overlap from at all (empty, or just a source URL) — every
/// ingredient name would trivially "not be found", firing on every such
/// recipe with 3+ ingredients regardless of the actual conversion quality.
const MIN_WORDS_FOR_LANGUAGE_CHECK: usize = 20;

pub(super) fn check(original: &str, _body: &str, recipe: &Recipe) -> Vec<Issue> {
    let mut issues = Vec::new();
    let sources = source_ingredients(original);
    let names: Vec<String> = recipe.ingredients.iter().map(|i| normalize(&i.name)).collect();

    // Language: with 3+ distinct ingredient names, most of them should share
    // a word with the original text — a wholesale translation instead of a
    // conversion. A mostly-CJK original is exempted: unspaced scripts don't
    // segment into comparable words the same way. Computed before coverage
    // so a fired LanguageMismatch can suppress the (redundant) per-line
    // MissingIngredient reports below. Skipped for a page-sized dump for the
    // same reason coverage is (see `MAX_SOURCES_FOR_COVERAGE`) — this scan
    // is O(names × original word count), and a page dump can make the
    // original word count itself huge.
    let mut distinct_names: Vec<String> = names.clone();
    distinct_names.sort();
    distinct_names.dedup();
    let mut language_mismatch = false;
    if distinct_names.len() >= 3
        && original.split_whitespace().count() >= MIN_WORDS_FOR_LANGUAGE_CHECK
        && sources.len() <= MAX_SOURCES_FOR_COVERAGE
        && !is_mostly_cjk(original)
    {
        let orig_lower = fold_ligatures(original).to_lowercase();
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
            language_mismatch = true;
        }
    }

    // Coverage: every *quantified* source ingredient is tagged somewhere. A
    // source line with no quantity ("salt to taste") is skipped outright —
    // too easy to conflate with something else, not worth the noise. Each
    // source line's words are computed once up front (not once per name, as
    // `covers` used to do internally) — the dominant cost on a page-sized
    // dump. Skipped entirely above `MAX_SOURCES_FOR_COVERAGE`, and once a
    // LanguageMismatch already explains the whole recipe.
    if !language_mismatch && sources.len() <= MAX_SOURCES_FOR_COVERAGE {
        let source_words: Vec<Vec<String>> = sources.iter().map(|s| words(&s.line)).collect();
        let mut missing = 0;
        for (src, src_words) in sources.iter().zip(&source_words) {
            if missing >= MAX_MISSING_INGREDIENT_ISSUES {
                break;
            }
            if src.qty.is_some() && !names.iter().any(|n| covers(n, src, src_words)) {
                issues.push(Issue::new(Kind::MissingIngredient, Severity::Error, &src.line, false));
                missing += 1;
            }
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

    if !language_mismatch && sources.len() <= MAX_SOURCES_FOR_COVERAGE {
        issues.extend(under_count(&sources, recipe));
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

    issues
}

/// Coarse physical dimension of a normalized unit: two tagged/listed units in
/// the same (or an unknown) dimension may be a unit conversion of one another
/// ("1 cup" -> "240 ml", "2 cloves" -> "10 g"); units in two different known
/// dimensions ("550 g" vs "1 tbsp") never are.
fn dimension(unit: &str) -> Option<&'static str> {
    match unit {
        "g" | "kg" | "mg" | "oz" | "lb" => Some("mass"),
        "ml" | "cl" | "dl" | "l" | "tsp" | "tbsp" | "cup" => Some("volume"),
        _ => None,
    }
}

static COMPOUND: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"&|\b(and|und|et|y|i)\b").unwrap());

/// A parenthetical in a source line.
static PAREN_AMOUNT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"\(([^()]*)\)").unwrap()
});
static AMOUNT_UNIT: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"\d(?:[\d.,/ ]*\d)?\s*([\p{L}.]+)").unwrap());

/// Normalized units of every amount a source line gives ("1/3 cup (5 Tbsp;
/// 75g) margarine" -> ["cup", "tbsp", "g"], "3/4 cup rye flour 90 g" -> ["cup",
/// "g"]): tagging an alternative amount instead of the leading one is a
/// faithful choice.
fn alt_units(line: &str) -> Vec<String> {
    AMOUNT_UNIT.captures_iter(line).map(|c| norm_unit(&c[1])).collect()
}

/// A multi-word tagged name whose every (non-stop) word is in the listing:
/// "powdered sugar" in "powdered/icing sugar".
fn all_words_in(name: &str, listing: &str) -> bool {
    let nw = words(name);
    let lw = words_raw(listing);
    nw.len() >= 2 && nw.iter().all(|w| lw.iter().any(|l| similar(w, l)))
}

/// An instruction sentence the extractor took for a listing: a long "name"
/// ("the whisked yolk kept back from the first bowl"), or a line that ends in a period
/// once parentheticals are removed ("1 Rotkohl in dünne Ringe hobeln.").
fn instruction_shaped(src: &SourceIngredient) -> bool {
    let words = src.name.split_whitespace().count();
    let bare = PAREN_AMOUNT.replace_all(&src.line, "");
    words >= 5 || (words >= 3 && bare.trim_end().ends_with('.'))
}

/// Formats an amount without float noise ("0.5", "550", "0.333").
fn amount(x: f64) -> String {
    let s = format!("{:.3}", x);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Under count: the source's total quantity of an ingredient (summed across
/// EVERY source listing — "1/4 cup pine nuts" for the pesto and "1/4 cup pine
/// nuts, toasted" for the topping) is larger than what the tagged tokens
/// carry in that unit, with 1% tolerance — including a quantified listing
/// tagged only with empty braces ("550 g flour" -> `@flour{}`).
///
/// Source listings and tagged names are linked by a word-boundary match in
/// either direction and grouped into connected components, so overlapping
/// names ("butter" / "peanut butter") are compared as one group instead of
/// charging one listing to the wrong token. A group is skipped when a token
/// has a text quantity ("a pinch"), or when a tagged unit that isn't among
/// the source's units could be a conversion of a source unit (same or unknown
/// dimension). A range compares the source's lower bound against the tagged
/// upper bound, so "2-3 tbsp" is satisfied by `{2%tbsp}`.
fn under_count(sources: &[SourceIngredient], recipe: &Recipe) -> Vec<Issue> {
    // A page dump that repeats its whole ingredient list: read each line once.
    // (A few identical lines are kept — "½ tsp salt" in two sub-recipes is real.)
    let mut distinct: Vec<&str> = sources.iter().map(|s| s.line.as_str()).collect();
    distinct.sort_unstable();
    distinct.dedup();
    let repeated = sources.len() >= 4 && (distinct.len() as f64) <= 0.6 * sources.len() as f64;
    let mut seen = std::collections::HashSet::new();
    let srcs: Vec<(&SourceIngredient, String, String, f64)> = sources
        .iter()
        .filter(|s| !repeated || seen.insert(s.line.as_str()))
        .filter(|s| !instruction_shaped(s))
        .filter_map(|s| {
            let lo = s.qty_lo.or(s.qty)?;
            Some((s, normalize(&s.name), norm_unit(s.unit.as_deref().unwrap_or("")), lo))
        })
        .filter(|(_, name, _, lo)| !name.is_empty() && *lo > 0.0)
        // A large unitless number is a product type ("Weizenmehl Type 405"), not a count,
        // and a count of a compound listing ("1 salt & pepper") means nothing.
        .filter(|(_, name, unit, lo)| !(unit.is_empty() && (*lo > 50.0 || COMPOUND.is_match(name))))
        .collect();
    let mut names: Vec<String> = recipe.ingredients.iter().map(|i| normalize(&i.name)).collect();
    names.sort();
    names.dedup();
    if srcs.is_empty() || names.is_empty() {
        return Vec::new();
    }
    // Union-find over [sources..., names...].
    let n = srcs.len();
    let mut parent: Vec<usize> = (0..n + names.len()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    let mut linked = vec![false; n + names.len()];
    for (i, (_, sname, _, _)) in srcs.iter().enumerate() {
        for (j, name) in names.iter().enumerate() {
            if word_boundary_match(sname, name) || word_boundary_match(name, sname) || all_words_in(name, sname) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, n + j));
                parent[a] = b;
                linked[i] = true;
                linked[n + j] = true;
            }
        }
    }
    let mut groups: BTreeMap<usize, (Vec<usize>, Vec<usize>)> = BTreeMap::new();
    for (i, _) in linked.iter().enumerate().filter(|(_, l)| **l) {
        {
            let root = find(&mut parent, i);
            let g = groups.entry(root).or_default();
            if i < n { g.0.push(i) } else { g.1.push(i - n) }
        }
    }
    let mut issues = Vec::new();
    'group: for (src_idx, name_idx) in groups.values() {
        let group_names: Vec<&String> = name_idx.iter().map(|&j| &names[j]).collect();
        let mut source_total: BTreeMap<String, f64> = BTreeMap::new();
        for &i in src_idx {
            *source_total.entry(srcs[i].2.clone()).or_default() += srcs[i].3;
        }
        let mut tagged: BTreeMap<String, f64> = BTreeMap::new();
        for ing in recipe.ingredients.iter().filter(|i| group_names.contains(&&normalize(&i.name))) {
            let Some(q) = &ing.quantity else { continue };
            let Some(v) = numeric(q.value()) else { continue 'group };
            *tagged.entry(norm_unit(q.unit().unwrap_or(""))).or_default() += v;
        }
        let alternatives: Vec<String> = src_idx.iter().flat_map(|&i| alt_units(&srcs[i].0.line)).collect();
        let partly_in_source_unit = source_total.keys().any(|u| tagged.get(u).is_some_and(|v| *v > 0.0));
        for unit in tagged.keys().filter(|u| !source_total.contains_key(*u)) {
            // Another amount the source line itself gives ("(75g)", "... rye flour 90 g"),
            // or part tagged in the source unit and part in a spoon measure: a split.
            // The same amount under another unit (250 ml -> 250 g) is a unit change.
            let same_amount = source_total.values().any(|t| (tagged[unit] - t).abs() <= 0.01 * t);
            if alternatives.contains(unit) || partly_in_source_unit || same_amount {
                continue 'group;
            }
            let convertible = source_total.keys().any(|su| match (dimension(unit), dimension(su)) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            });
            if convertible {
                continue 'group;
            }
        }
        let tagged_any = tagged.values().any(|v| *v > 0.0);
        for (unit, total) in &source_total {
            let have = tagged.get(unit).copied().unwrap_or(0.0);
            // Every listing in this unit also gives another amount ("50g egg (1 egg)")
            // and something is tagged: it may be carried in that other amount.
            let alt_only = src_idx
                .iter()
                .filter(|&&i| &srcs[i].2 == unit)
                .all(|&i| AMOUNT_UNIT.find_iter(&srcs[i].0.line).count() >= 2);
            if have < total * 0.99 && !(alt_only && tagged_any) {
                let names_s = group_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" / ");
                let u = if unit.is_empty() { String::new() } else { format!(" {unit}") };
                let evidence = format!(
                    "@{names_s}: source lists {}{u} over {} line(s), tagged {}{u}",
                    amount(*total),
                    src_idx.len(),
                    amount(have)
                );
                issues.push(Issue::new(Kind::UnderCount, Severity::Error, evidence, false));
                break;
            }
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

    /// Test-only convenience: `covers` takes the caller's precomputed source
    /// words in production (see its doc comment), but tests just want to
    /// assert on a single token/source pair.
    fn cov(token: &str, src: &SourceIngredient) -> bool {
        covers(token, src, &words(&src.line))
    }

    #[test]
    fn covers_allows_two_character_words() {
        let src = SourceIngredient { line: "1 Ei".into(), name: "ei".into(), qty: Some(1.0), qty_lo: Some(1.0), unit: None };
        assert!(cov("ei", &src));
        let src = SourceIngredient { line: "1 EL Öl".into(), name: "öl".into(), qty: Some(1.0), qty_lo: Some(1.0), unit: Some("EL".into()) };
        assert!(cov("öl", &src));
    }

    /// Regression (found against real data, row 470): "œufs" (with the
    /// ligature) and "oeufs" (spelled out) must be recognized as the same
    /// word — a plain prefix comparison sees 'œ' and 'o' as different
    /// characters and fails from the very first letter.
    #[test]
    fn covers_folds_french_ligatures() {
        let src = SourceIngredient { line: "2 œufs".into(), name: "œufs".into(), qty: Some(2.0), qty_lo: Some(2.0), unit: None };
        assert!(cov("oeufs", &src));
    }

    #[test]
    fn covers_matches_cjk_by_character_containment() {
        let src = SourceIngredient { line: "洋葱 一个".into(), name: "洋葱".into(), qty: Some(1.0), qty_lo: Some(1.0), unit: None };
        assert!(cov("洋葱", &src));
        assert!(!cov("胡萝卜", &src));
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

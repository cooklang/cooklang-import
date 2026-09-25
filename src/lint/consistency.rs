use super::source::{source_ingredients, SourceIngredient};
use super::tokens::strip_tokens;
use super::{Issue, Kind, Severity};
use cooklang::{Recipe, Value};
use std::collections::BTreeMap;

fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Singular-ish key: strips a trailing "s" from every word, so
/// "asperges vertes" / "asperge verte" and "oranges" / "orange" collide.
fn stem(name: &str) -> String {
    normalize(name)
        .split_whitespace()
        .map(|w| w.strip_suffix('s').unwrap_or(w))
        .collect::<Vec<_>>()
        .join(" ")
}

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .map(stem)
        .collect()
}

/// A token covers a source line when they share a significant word.
fn covers(token: &str, src: &SourceIngredient) -> bool {
    let src_words = words(&src.line);
    words(token).iter().any(|w| src_words.contains(w))
}

fn numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(n.value()),
        Value::Range { end, .. } => Some(end.value()),
        Value::Text(_) => None,
    }
}

pub(super) fn check(original: &str, body: &str, recipe: &Recipe) -> Vec<Issue> {
    let mut issues = Vec::new();
    let sources = source_ingredients(original);
    let names: Vec<String> = recipe.ingredients.iter().map(|i| normalize(&i.name)).collect();

    // Coverage: every listed source ingredient is tagged somewhere.
    for src in &sources {
        if !names.iter().any(|n| covers(n, src)) {
            issues.push(Issue::new(Kind::MissingIngredient, Severity::Error, &src.line, false));
        }
    }

    // Double count: summed tagged amounts exceed a single source listing.
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
        let listed: Vec<&SourceIngredient> = sources.iter().filter(|s| s.name.contains(name.as_str())).collect();
        if let [only] = listed.as_slice() {
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

    // Language: the converted prose must stay in the source language.
    let prose = strip_tokens(body);
    if let (Some(src), Some(out)) = (whatlang::detect(original), whatlang::detect(&prose)) {
        if src.is_reliable() && out.is_reliable() && src.lang() != out.lang() {
            let evidence = format!("source {} → output {}", src.lang().eng_name(), out.lang().eng_name());
            issues.push(Issue::new(Kind::LanguageMismatch, Severity::Error, evidence, false));
        }
    }
    issues
}

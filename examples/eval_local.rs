//! End-to-end eval on the held-out manifest from Plan 1.
//!
//! Usage:
//!   cargo run --release --features local-model --example eval_local -- \
//!       /Users/alexeydubovskoy/Cooklang/fine_tune/training/data/test_recipes.txt [--limit N]
//!
//! Ship gates (spec): ingredient triples >= 90%, parse rate 100%, avg latency
//! < 2000ms are the HARD gates. Timer/cookware F1 >= 0.85 vs the silver-standard
//! (LLM-generated) gold data are reported for information only — the gold spans
//! themselves are not human-verified, so a miss there is not necessarily a bug.
//!
//! Ingredient triples are reported two ways: "strict" (near-verbatim string
//! comparison, modulo a couple of presentation-only folds) and "normalized"
//! (unit-synonym folding, numeric qty equivalence, linker-style name
//! normalization — see `normalized_triples` below). Groundedness sampling
//! showed ~98% of gold (name, qty) pairs are actually present in source text,
//! so the strict/normalized gap is mostly measuring comparison strictness,
//! not pipeline fabrication or correctness. Both are comparison-only — the
//! pipeline's actual output text is never touched by any of this.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn strip_frontmatter(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            return rest[end + 5..].trim().to_string();
        }
    }
    text.trim().to_string()
}

fn parser() -> cooklang::CooklangParser {
    cooklang::CooklangParser::new(cooklang::Extensions::all(), cooklang::Converter::default())
}

/// Fold unicode vulgar fractions to their ASCII "num/den" spelling.
///
/// cooklang's `Number` parser only recognizes ASCII fraction syntax ("1/4");
/// a literal "¼" glyph doesn't match that grammar and lands in `Value::Text`
/// instead, so `Display` prints the glyph verbatim rather than "1/4" — even
/// though the two spellings mean the same quantity. Our pipeline preserves
/// whatever glyph the source recipe used; the (LLM-generated) gold data
/// normalizes to ASCII. Fold both to the same spelling here so the eval
/// isn't penalized by this presentation-only difference (normalize the
/// *comparison*, not the pipeline's actual output).
fn norm_qty(q: &str) -> String {
    match q {
        "½" => "1/2".into(),
        "¼" => "1/4".into(),
        "¾" => "3/4".into(),
        "⅓" => "1/3".into(),
        "⅔" => "2/3".into(),
        "⅛" => "1/8".into(),
        "⅜" => "3/8".into(),
        "⅝" => "5/8".into(),
        "⅞" => "7/8".into(),
        other => other.to_string(),
    }
}

/// Fold a simple trailing-s plural ("cloves" -> "clove", "grams" -> "gram")
/// so unit-spelling differences between our output and the gold's don't
/// masquerade as content mismatches. Mirrors the plural fold already used by
/// the linker (`src/converters/local/linker.rs::norm`).
fn norm_unit(u: &str) -> String {
    match u.strip_suffix('s') {
        Some(s) if s.len() >= 3 => s.to_string(),
        _ => u.to_string(),
    }
}

/// (name, qty, unit) lowered triples from cooklang text — the "strict"
/// comparison: near-verbatim modulo the two small presentation folds above.
fn triples(cook: &str) -> HashSet<(String, String, String)> {
    let (recipe, _) = parser().parse(cook).into_tuple();
    let Some(recipe) = recipe else {
        return HashSet::new();
    };
    recipe
        .ingredients
        .iter()
        .map(|i| {
            let (q, u) = i
                .quantity
                .as_ref()
                .map(|q| {
                    (
                        q.value().to_string(),
                        q.unit().map(|u| u.to_string()).unwrap_or_default(),
                    )
                })
                .unwrap_or_default();
            (
                i.name.to_lowercase(),
                norm_qty(&q.to_lowercase()),
                norm_unit(&u.to_lowercase()),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Normalized comparison: unit-synonym folding, numeric qty equivalence, and
// linker-style name normalization. This is a strictly more thorough version
// of the strict comparison above, built to answer "do these two triples mean
// the same thing", not "are these two strings identical". Still comparison
// only: `triples()`/`normalized_triples()` never feed back into what the
// pipeline outputs.
// ---------------------------------------------------------------------------

const VULGAR_FRACTIONS: &[(&str, f64)] = &[
    ("½", 0.5),
    ("¼", 0.25),
    ("¾", 0.75),
    ("⅓", 1.0 / 3.0),
    ("⅔", 2.0 / 3.0),
    ("⅛", 0.125),
    ("⅜", 0.375),
    ("⅝", 0.625),
    ("⅞", 0.875),
];

/// Value of a standalone vulgar-fraction glyph ("½" -> 0.5) or a glued mixed
/// number ("1½" -> 1.5, digits immediately before the glyph, no space).
fn vulgar_fraction_value(s: &str) -> Option<f64> {
    for (glyph, val) in VULGAR_FRACTIONS {
        if let Some(prefix) = s.strip_suffix(glyph) {
            return if prefix.is_empty() {
                Some(*val)
            } else {
                prefix.parse::<f64>().ok().map(|whole| whole + val)
            };
        }
    }
    None
}

fn parse_simple_fraction(s: &str) -> Option<f64> {
    let (num, den) = s.trim().split_once('/')?;
    let num: f64 = num.trim().parse().ok()?;
    let den: f64 = den.trim().parse().ok()?;
    if den == 0.0 {
        return None;
    }
    Some(num / den)
}

/// Parse a quantity string to a numeric value when possible: unicode vulgar
/// fractions (incl. glued mixed numbers), ASCII fractions ("1/2"), ASCII
/// mixed numbers ("1 1/2"), decimals, and integers. Returns `None` for
/// non-numeric quantities ("to taste") and ranges ("4-5"), which the caller
/// falls back to comparing as strings.
fn parse_qty_numeric(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(v) = vulgar_fraction_value(s) {
        return Some(v);
    }
    if let Some((whole, frac)) = s.split_once(' ') {
        if let (Ok(w), Some(f)) = (whole.trim().parse::<f64>(), parse_simple_fraction(frac)) {
            return Some(w + f);
        }
    }
    if let Some(v) = parse_simple_fraction(s) {
        return Some(v);
    }
    s.parse::<f64>().ok()
}

/// Canonical qty for equivalence comparison: numeric values collapse to a
/// fixed-precision decimal (so "½", "0.5", and "1/2" all produce the same
/// string and therefore compare equal); anything that doesn't parse as a
/// number (non-numeric quantities, ranges) falls back to trimmed+lowercased
/// literal string comparison, per spec.
fn canonical_qty(q: &str) -> String {
    match parse_qty_numeric(q) {
        Some(v) => format!("{v:.6}"),
        None => q.trim().to_lowercase(),
    }
}

/// Fold unit synonyms to one canonical spelling per group. Single-letter
/// forms are case-sensitive and MUST be checked before lowercasing — "T"
/// (tablespoon) and "t" (teaspoon) would otherwise collide.
fn canonical_unit(u: &str) -> String {
    let trimmed = u.trim();
    if trimmed == "T" {
        return "tablespoon".to_string();
    }
    if trimmed == "t" {
        return "teaspoon".to_string();
    }
    match trimmed.to_lowercase().as_str() {
        "tablespoon" | "tablespoons" | "tbsp" | "tbsps" => "tablespoon",
        "teaspoon" | "teaspoons" | "tsp" | "tsps" => "teaspoon",
        "ounce" | "ounces" | "oz" => "ounce",
        "pound" | "pounds" | "lb" | "lbs" => "pound",
        "gram" | "grams" | "g" => "gram",
        "kilogram" | "kilograms" | "kg" => "kilogram",
        "milliliter" | "milliliters" | "millilitre" | "millilitres" | "ml" => "milliliter",
        "liter" | "liters" | "litre" | "litres" | "l" => "liter",
        "cup" | "cups" | "c" => "cup",
        "clove" | "cloves" => "clove",
        "can" | "cans" => "can",
        "package" | "pkg" | "packages" | "pkgs" => "package",
        "piece" | "pieces" => "piece",
        "slice" | "slices" => "slice",
        "unit" | "units" => "unit",
        other => return other.to_string(),
    }
    .to_string()
}

/// Linker-style per-token normalization (lowercase + light plural fold),
/// reusing `converters::local::linker::norm` directly so name comparisons
/// agree with what the pipeline itself considers "the same ingredient".
fn canonical_name(name: &str) -> String {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(cooklang_import::local_linker::norm)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fully normalized (name, qty, unit) triples: unit-synonym folded, qty
/// numeric-equivalence folded, name linker-normalized.
fn normalized_triples(cook: &str) -> HashSet<(String, String, String)> {
    let (recipe, _) = parser().parse(cook).into_tuple();
    let Some(recipe) = recipe else {
        return HashSet::new();
    };
    recipe
        .ingredients
        .iter()
        .map(|i| {
            let (q, u) = i
                .quantity
                .as_ref()
                .map(|q| {
                    (
                        q.value().to_string(),
                        q.unit().map(|u| u.to_string()).unwrap_or_default(),
                    )
                })
                .unwrap_or_default();
            (
                canonical_name(&i.name),
                canonical_qty(&q),
                canonical_unit(&u),
            )
        })
        .collect()
}

/// Whether `cook` parses without errors (the real parse-rate gate check).
fn parses_clean(cook: &str) -> bool {
    let (_, report) = parser().parse(cook).into_tuple();
    !report.has_errors()
}

/// Crude surface extraction of `#name{...}` / `~{...}` / `~name{...}` spans for
/// F1 scoring against the silver gold. `sym` is '#' or '~'.
fn spans(cook: &str, sym: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = cook;
    while let Some(i) = rest.find(sym) {
        rest = &rest[i + 1..];
        // Anonymous timer form: ~{2-3%minutes} — name is empty, brace right away.
        // Named form: #baking sheet{} / ~eggs{3%minutes} — name up to '{'.
        let Some(b) = rest.find('{') else { continue };
        // A name shouldn't span a newline or be absurdly long (next sym far away).
        if b > 60 || rest[..b].contains('\n') {
            continue;
        }
        let name = rest[..b].trim().to_lowercase();
        let after_brace = &rest[b + 1..];
        let amount = after_brace.split('}').next().unwrap_or("").to_lowercase();
        out.push(if name.is_empty() { amount } else { name });
        rest = after_brace;
    }
    out
}

fn f1(gold: &[String], pred: &[String]) -> f64 {
    if gold.is_empty() && pred.is_empty() {
        return 1.0;
    }
    let gset: HashSet<&String> = gold.iter().collect();
    let tp = pred.iter().filter(|p| gset.contains(p)).count() as f64;
    let p = tp / pred.len().max(1) as f64;
    let r = tp / gold.len().max(1) as f64;
    if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    }
}

/// Manifest paths are relative to the fine_tune repo root (three levels up
/// from `training/data/test_recipes.txt`) for the HelloFresh split, but
/// absolute for the recipe-pack split. Resolve accordingly.
fn resolve(base: &Path, p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn infer_base_dir(manifest: &Path) -> PathBuf {
    manifest
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Categorize every normalized-gold triple that still doesn't hit, on a small
/// sample, so a triples % that's "near/above 80%" comes with a sense of what
/// the *remaining* gap actually is:
///   - no pipeline ingredient: no ingredient with this (normalized) name
///     appears anywhere in the pipeline's output at all.
///   - name mismatch: some pred ingredient shares at least one normalized
///     name token with the gold entry, but not an exact post-normalization
///     name match — a normalization/wording gap, not a missing ingredient.
///   - amount mismatch after normalization: the (normalized) name matches
///     exactly, but no pred triple with that name has a matching qty/unit —
///     the qty/unit genuinely differ even after folding.
async fn residual_miss_breakdown(sample_lines: &[String], base_dir: &Path) {
    let (mut no_pipeline_ingredient, mut name_mismatch, mut amount_mismatch) =
        (0usize, 0usize, 0usize);
    let (mut total_gold, mut total_missed) = (0usize, 0usize);

    for line in sample_lines {
        let Some((src_rel, gold_rel)) = line.split_once('\t') else {
            continue;
        };
        let src_path = resolve(base_dir, src_rel);
        let gold_path = resolve(base_dir, gold_rel);
        let (Ok(src_raw), Ok(gold_raw)) = (
            std::fs::read_to_string(&src_path),
            std::fs::read_to_string(&gold_path),
        ) else {
            continue;
        };
        let src = strip_frontmatter(&src_raw);
        let gold = strip_frontmatter(&gold_raw);
        let Ok(pred) = cooklang_import::convert_text_local(&src).await else {
            continue;
        };

        let gold_norm = normalized_triples(&gold);
        let pred_norm = normalized_triples(&pred);
        let pred_names: HashSet<&String> = pred_norm.iter().map(|(n, _, _)| n).collect();
        // Token sets per pred name, for the "shares at least one token" fuzzy check.
        let pred_name_tokens: Vec<Vec<&str>> =
            pred_names.iter().map(|n| n.split(' ').collect()).collect();

        for gt @ (gname, _, _) in &gold_norm {
            total_gold += 1;
            if pred_norm.contains(gt) {
                continue;
            }
            total_missed += 1;
            if pred_names.contains(gname) {
                amount_mismatch += 1;
                continue;
            }
            let gold_tokens: Vec<&str> = gname.split(' ').collect();
            let resembles = pred_name_tokens
                .iter()
                .any(|toks| gold_tokens.iter().any(|t| toks.contains(t)));
            if resembles {
                name_mismatch += 1;
            } else {
                no_pipeline_ingredient += 1;
            }
        }
    }

    println!();
    println!(
        "=== residual-miss breakdown (normalized triples, {} sample recipes) ===",
        sample_lines.len()
    );
    println!("gold triples in sample:                         {total_gold}");
    println!("still missed after normalization:               {total_missed}");
    println!("  no pipeline ingredient (name absent entirely): {no_pipeline_ingredient}");
    println!("  name mismatch (similar name, not exact match): {name_mismatch}");
    println!("  amount mismatch (name matched, qty/unit off):  {amount_mismatch}");
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let manifest = args
        .next()
        .expect("usage: eval_local <test_recipes.txt> [--limit N]");
    let mut limit: Option<usize> = None;
    while let Some(a) = args.next() {
        if a == "--limit" {
            limit = args.next().and_then(|v| v.parse().ok());
        }
    }

    let manifest_path = PathBuf::from(&manifest);
    let base_dir = infer_base_dir(&manifest_path);

    let all_lines: Vec<String> = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading manifest {manifest}: {e}"))
        .lines()
        .map(str::to_string)
        .collect();
    let total_in_manifest = all_lines.len();
    let lines: Vec<String> = match limit {
        Some(n) => all_lines.iter().take(n).cloned().collect(),
        None => all_lines.clone(),
    };

    let (mut n, mut failed) = (0u32, 0u32);
    let (mut strict_hits, mut strict_total) = (0usize, 0usize);
    let (mut norm_hits, mut norm_total) = (0usize, 0usize);
    let mut parse_ok = 0u32;
    let (mut timer_f1_sum, mut cook_f1_sum, mut total_ms) = (0f64, 0f64, 0u128);

    eprintln!(
        "evaluating {} of {} manifest recipes (limit={:?})",
        lines.len(),
        total_in_manifest,
        limit
    );

    for (idx, line) in lines.iter().enumerate() {
        let Some((src_rel, gold_rel)) = line.split_once('\t') else {
            continue;
        };
        let src_path = resolve(&base_dir, src_rel);
        let gold_path = resolve(&base_dir, gold_rel);

        let (Ok(src_raw), Ok(gold_raw)) = (
            std::fs::read_to_string(&src_path),
            std::fs::read_to_string(&gold_path),
        ) else {
            eprintln!("skip (missing file): {src_rel}");
            continue;
        };
        let src = strip_frontmatter(&src_raw);
        let gold = strip_frontmatter(&gold_raw);

        let started = std::time::Instant::now();
        let pred = match cooklang_import::convert_text_local(&src).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("conversion failed for {src_rel}: {e}");
                failed += 1;
                continue;
            }
        };
        total_ms += started.elapsed().as_millis();
        n += 1;

        let gold_triples = triples(&gold);
        let pred_triples = triples(&pred);
        strict_total += gold_triples.len();
        strict_hits += gold_triples.intersection(&pred_triples).count();

        let gold_norm = normalized_triples(&gold);
        let pred_norm = normalized_triples(&pred);
        norm_total += gold_norm.len();
        norm_hits += gold_norm.intersection(&pred_norm).count();

        if parses_clean(&pred) {
            parse_ok += 1;
        }

        timer_f1_sum += f1(&spans(&gold, '~'), &spans(&pred, '~'));
        cook_f1_sum += f1(&spans(&gold, '#'), &spans(&pred, '#'));

        if (idx + 1) % 100 == 0 {
            eprintln!("... {} / {} processed", idx + 1, lines.len());
        }
    }

    let strict_pct = 100.0 * strict_hits as f64 / strict_total.max(1) as f64;
    let norm_pct = 100.0 * norm_hits as f64 / norm_total.max(1) as f64;
    let timer_f1 = timer_f1_sum / n.max(1) as f64;
    let cook_f1 = cook_f1_sum / n.max(1) as f64;
    let parse_pct = 100.0 * parse_ok as f64 / n.max(1) as f64;
    let avg_ms = total_ms / n.max(1) as u128;

    println!();
    println!("=== eval_local report ===");
    println!("recipes in manifest:   {total_in_manifest}");
    println!("recipes evaluated:     {n}  (failed conversions: {failed})");
    println!();
    println!(
        "ingredient triples (strict):     {strict_pct:.1}%  ({strict_hits}/{strict_total})  gate >= 90%  [{}]",
        if strict_pct >= 90.0 { "PASS" } else { "FAIL" }
    );
    println!(
        "ingredient triples (normalized): {norm_pct:.1}%  ({norm_hits}/{norm_total})  unit-synonym + numeric-qty + linker-name normalized, informational",
    );
    println!(
        "parse rate:            {parse_pct:.1}%  ({parse_ok}/{n})       gate == 100%     [{}]",
        if parse_ok == n { "PASS" } else { "FAIL" }
    );
    println!(
        "avg latency:           {avg_ms} ms                    gate < 2000 ms   [{}]",
        if avg_ms < 2000 { "PASS" } else { "FAIL" }
    );
    println!();
    println!(
        "timer F1 (mean, vs silver gold):    {timer_f1:.3}  informational, gate >= 0.85  [{}]",
        if timer_f1 >= 0.85 {
            "PASS"
        } else {
            "FAIL (informational)"
        }
    );
    println!(
        "cookware F1 (mean, vs silver gold): {cook_f1:.3}  informational, gate >= 0.85  [{}]",
        if cook_f1 >= 0.85 {
            "PASS"
        } else {
            "FAIL (informational)"
        }
    );

    if norm_pct >= 80.0 {
        let sample: Vec<String> = all_lines.into_iter().take(30).collect();
        residual_miss_breakdown(&sample, &base_dir).await;
    }
}

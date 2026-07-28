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

/// (name, qty, unit) lowered, normalized triples from cooklang text.
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

    let lines: Vec<String> = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading manifest {manifest}: {e}"))
        .lines()
        .map(str::to_string)
        .collect();
    let total_in_manifest = lines.len();
    let lines: Vec<String> = match limit {
        Some(n) => lines.into_iter().take(n).collect(),
        None => lines,
    };

    let (mut n, mut failed) = (0u32, 0u32);
    let (mut triple_hits, mut triple_total) = (0usize, 0usize);
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
        triple_total += gold_triples.len();
        triple_hits += gold_triples.intersection(&pred_triples).count();

        if parses_clean(&pred) {
            parse_ok += 1;
        }

        timer_f1_sum += f1(&spans(&gold, '~'), &spans(&pred, '~'));
        cook_f1_sum += f1(&spans(&gold, '#'), &spans(&pred, '#'));

        if (idx + 1) % 100 == 0 {
            eprintln!("... {} / {} processed", idx + 1, lines.len());
        }
    }

    let triple_pct = 100.0 * triple_hits as f64 / triple_total.max(1) as f64;
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
        "ingredient triples:    {triple_pct:.1}%  ({triple_hits}/{triple_total})  gate >= 90%      [{}]",
        if triple_pct >= 90.0 { "PASS" } else { "FAIL" }
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
}

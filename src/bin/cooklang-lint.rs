//! Streams JSONL recipes through the Cooklang checker.
//!
//! Each input line: {"udid", "original_text", "cook", ...}. `original_text`
//! and `cook` may be JSON `null` in real data (treated as ""); unknown extra
//! fields (e.g. `source_type`, `channel`) are ignored.
//!
//! Flags: --fix (also autofix and re-check), --summary (per-kind counts on
//! stderr).

use cooklang_import::lint::{autofix, lint, score, source_language, Kind};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
struct Row {
    udid: String,
    #[serde(default)]
    original_text: Option<String>,
    #[serde(default)]
    cook: Option<String>,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let fix = args.iter().any(|a| a == "--fix");
    let summary = args.iter().any(|a| a == "--summary");
    let mut counts: BTreeMap<Kind, usize> = BTreeMap::new();
    let (mut total, mut clean) = (0usize, 0usize);
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: Row = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("skipping bad line: {e}");
                continue;
            }
        };
        let original_text = row.original_text.as_deref().unwrap_or("");
        let cook = row.cook.as_deref().unwrap_or("");

        let issues = lint(original_text, cook);
        let mut record = json!({
            "udid": row.udid,
            "language": source_language(original_text),
            "score": score(&issues),
            "issues": issues,
        });
        let final_issues = if fix {
            let fixed = autofix(cook);
            let fixed_issues = lint(original_text, &fixed);
            record["fixed_score"] = json!(score(&fixed_issues));
            record["fixed_issues"] = json!(fixed_issues);
            record["fixed"] = json!(fixed);
            fixed_issues
        } else {
            issues
        };
        total += 1;
        if final_issues.is_empty() {
            clean += 1;
        }
        let mut seen: Vec<Kind> = final_issues.iter().map(|i| i.kind).collect();
        seen.sort();
        seen.dedup();
        for kind in seen {
            *counts.entry(kind).or_default() += 1;
        }
        writeln!(out, "{record}")?;
    }

    if summary {
        eprintln!(
            "recipes: {total}, checker-clean: {clean} ({:.1}%)",
            100.0 * clean as f64 / total.max(1) as f64
        );
        for (kind, n) in &counts {
            eprintln!("{:>5} {:>5.1}%  {kind:?}", n, 100.0 * *n as f64 / total.max(1) as f64);
        }
    }
    Ok(())
}

//! Streams JSONL recipes through the Cooklang checker.
//!
//! Each input line: {"udid", "original_text", "cook", ...}. `original_text`
//! and `cook` may be JSON `null` in real data (treated as ""); unknown extra
//! fields (e.g. `source_type`, `channel`) are ignored.
//!
//! Flags: --fix (also autofix and re-check), --summary (per-kind counts on
//! stderr).
//!
//! Robustness: a line that isn't valid UTF-8 or isn't valid JSON is reported
//! to stderr and skipped (counted in the summary), never a panic. An unknown
//! flag is a usage error (exit 2). Writing to a closed stdout (`| head`,
//! a pipeline reader that exits early) exits quietly rather than erroring.

use cooklang_import::lint::{autofix, lint, score, source_language, Kind};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

#[derive(Deserialize)]
struct Row {
    udid: String,
    #[serde(default)]
    original_text: Option<String>,
    #[serde(default)]
    cook: Option<String>,
}

const USAGE: &str = "usage: cooklang-lint [--fix] [--summary] < input.jsonl > output.jsonl";

fn main() -> ExitCode {
    let mut fix = false;
    let mut summary = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--fix" => fix = true,
            "--summary" => summary = true,
            other => {
                eprintln!("cooklang-lint: unknown flag '{other}'\n{USAGE}");
                return ExitCode::from(2);
            }
        }
    }

    let mut counts: BTreeMap<Kind, usize> = BTreeMap::new();
    let (mut total, mut skipped, mut clean) = (0usize, 0usize, 0usize);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut buf: Vec<u8> = Vec::new();

    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("cooklang-lint: stdin read error: {e}");
                break;
            }
        }
        while matches!(buf.last(), Some(b'\n' | b'\r')) {
            buf.pop();
        }
        if buf.is_empty() {
            continue;
        }
        let line = match std::str::from_utf8(&buf) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cooklang-lint: skipping line with invalid UTF-8: {e}");
                skipped += 1;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let row: Row = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("cooklang-lint: skipping bad line: {e}");
                skipped += 1;
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
        if let Err(e) = writeln!(out, "{record}") {
            if e.kind() == io::ErrorKind::BrokenPipe {
                return ExitCode::SUCCESS;
            }
            eprintln!("cooklang-lint: stdout write error: {e}");
            return ExitCode::FAILURE;
        }
    }

    if summary {
        eprintln!(
            "recipes: {total}, skipped: {skipped}, checker-clean: {clean} ({:.1}%)",
            100.0 * clean as f64 / total.max(1) as f64
        );
        for (kind, n) in &counts {
            eprintln!("{:>5} {:>5.1}%  {kind:?}", n, 100.0 * *n as f64 / total.max(1) as f64);
        }
    }
    ExitCode::SUCCESS
}

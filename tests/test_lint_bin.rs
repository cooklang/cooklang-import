#![cfg(feature = "lint")]
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn lints_jsonl_and_applies_fixes() {
    let input = r#"{"udid":"a","original_text":"2 cups water\n1 onion\n\nBoil.","cook":"Boil @water{2 cups} with @onion{1}."}"#;
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .arg("--fix")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    let line: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(line["udid"], "a");
    assert!(line["score"].as_u64().unwrap() > 0);
    assert_eq!(line["fixed"], "Boil @water{2%cups} with @onion{1}.");
    assert_eq!(line["fixed_score"], 0);
}

#[test]
fn tolerates_null_original_text_and_cook() {
    let input = r#"{"udid":"b","original_text":null,"cook":null,"source_type":"url","channel":"web"}"#;
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let line: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(line["udid"], "b");
}

#[test]
fn skips_unparseable_lines_without_panicking() {
    let input = "not json at all\n{\"udid\":\"c\",\"original_text\":\"x\",\"cook\":\"Step.\"}\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1);
    let line: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(line["udid"], "c");
    assert!(!out.stderr.is_empty());
}

#[test]
fn summary_writes_to_stderr() {
    let input = r#"{"udid":"a","original_text":"","cook":"Boil @water{2%cups}."}
{"udid":"b","original_text":"","cook":"Boil @water{2 cups}."}
"#;
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .arg("--summary")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("recipes: 2"), "{stderr}");
    assert!(stderr.contains("checker-clean"), "{stderr}");
}

#[test]
fn skips_invalid_utf8_lines_and_counts_them_as_skipped() {
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(&[0xff, 0xfe, 0xfd, b'\n']);
    input.extend_from_slice(b"{\"udid\":\"c\",\"original_text\":\"x\",\"cook\":\"Step.\"}\n");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .arg("--summary")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1);
    let line: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(line["udid"], "c");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("invalid UTF-8"), "{stderr}");
    assert!(stderr.contains("skipped: 1"), "{stderr}");
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .arg("--bogus")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("unknown flag"), "{stderr}");
    assert!(stderr.contains("usage:"), "{stderr}");
}

#[test]
fn exits_quietly_on_a_closed_output_pipe() {
    // Simulates a downstream reader (e.g. `| head -1`) that closes its end
    // of the pipe early: writing to it must not panic or report an error.
    let mut input = String::new();
    for i in 0..5000 {
        input.push_str(&format!("{{\"udid\":\"{i}\",\"original_text\":\"\",\"cook\":\"Step.\"}}\n"));
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Drop the read end immediately so the pipe closes while we're still
    // writing/the child is still producing output.
    drop(child.stdout.take());
    let mut stdin = child.stdin.take().unwrap();
    let _ = stdin.write_all(input.as_bytes());
    drop(stdin);
    let status = child.wait().unwrap();
    // Must exit cleanly (0), never a panic (which would show as a signal on
    // unix or a non-zero/abort code).
    assert!(status.success() || status.code() == Some(0), "{status:?}");
}

#[test]
fn strips_a_leading_utf8_bom_on_the_first_line() {
    let mut input: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
    input.extend_from_slice(b"{\"udid\":\"a\",\"original_text\":\"\",\"cook\":\"Step.\"}\n");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cooklang-lint"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{:?}", String::from_utf8_lossy(&out.stderr));
    let line: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(line["udid"], "a");
    assert!(out.stderr.is_empty(), "{}", String::from_utf8_lossy(&out.stderr));
}

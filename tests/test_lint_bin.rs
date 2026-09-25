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

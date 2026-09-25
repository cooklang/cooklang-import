#![cfg(feature = "lint")]
use cooklang_import::lint::{autofix, lint, score, split_frontmatter, Issue, Kind, Severity};

fn kinds(issues: &[Issue]) -> Vec<Kind> {
    issues.iter().map(|i| i.kind).collect()
}

const ORIG: &str = "2 eggs\n\nBoil the eggs.";

#[test]
fn clean_recipe_has_no_issues() {
    let doc = "---\ntitle: Eggs\n---\n\nBoil @eggs{2}.\n";
    assert_eq!(lint(ORIG, doc), vec![]);
}

#[test]
fn split_frontmatter_separates_yaml_and_body() {
    let (fm, body) = split_frontmatter("---\ntitle: X\n---\n\nStep.\n");
    assert_eq!(fm, Some("title: X"));
    assert_eq!(body.trim(), "Step.");
    assert_eq!(split_frontmatter("Step."), (None, "Step."));
}

#[test]
fn score_weights_errors_three_warnings_one() {
    let issues = vec![
        Issue::new(Kind::MissingPercent, Severity::Error, "x", true),
        Issue::new(Kind::HtmlEntity, Severity::Warning, "y", true),
    ];
    assert_eq!(score(&issues), 4);
}

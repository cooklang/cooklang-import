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
fn split_frontmatter_tolerates_bom_and_crlf() {
    let (fm, body) = split_frontmatter("\u{FEFF}---\ntitle: X\n---\n\nStep.\n");
    assert_eq!(fm, Some("title: X"));
    assert_eq!(body.trim(), "Step.");

    let (fm, body) = split_frontmatter("---\r\ntitle: X\r\n---\r\nStep");
    assert_eq!(fm, Some("title: X"));
    assert_eq!(body, "Step");
}

#[test]
fn split_frontmatter_closing_fence_must_be_exact() {
    // "---more" and "----" are not a bare "---" line, so no frontmatter is
    // recognized at all rather than mis-splitting on a substring match.
    let doc = "---\ntitle: X\n---more\nStep";
    assert_eq!(split_frontmatter(doc), (None, doc));
    let doc = "---\ntitle: X\n----\nStep";
    assert_eq!(split_frontmatter(doc), (None, doc));
}

#[test]
fn split_frontmatter_closing_fence_at_eof() {
    let (fm, body) = split_frontmatter("---\ntitle: X\n---");
    assert_eq!(fm, Some("title: X"));
    assert_eq!(body, "");
}

#[test]
fn score_weights_errors_three_warnings_one() {
    let issues = vec![
        Issue::new(Kind::MissingPercent, Severity::Error, "x", true),
        Issue::new(Kind::HtmlEntity, Severity::Warning, "y", true),
    ];
    assert_eq!(score(&issues), 4);
}

fn body_kinds(body: &str) -> Vec<Kind> {
    kinds(&lint("", &format!("---\ntitle: T\n---\n\n{body}\n")))
}

#[test]
fn flags_missing_percent() {
    assert!(body_kinds("Add @water{2 cups}.").contains(&Kind::MissingPercent));
    assert!(body_kinds("Add @flour{200g}.").contains(&Kind::MissingPercent));
    assert!(!body_kinds("Add @flour{200%g} and @salt{pinch}.").contains(&Kind::MissingPercent));
}

fn issue_for(issues: &[Issue], kind: Kind) -> Option<&Issue> {
    issues.iter().find(|i| i.kind == kind)
}

#[test]
fn missing_percent_accepts_unicode_fractions_and_dash_ranges() {
    assert!(body_kinds("Add @sugar{½ cup}.").contains(&Kind::MissingPercent));
    assert!(body_kinds("Add @potatoes{2–3 large}.").contains(&Kind::MissingPercent));
    assert!(body_kinds("Add @Butter{2Stück}.").contains(&Kind::MissingPercent));
    assert!(body_kinds("Add @sugar{1 ½ cups}.").contains(&Kind::MissingPercent));
    assert!(!body_kinds("Add @eggs{2x}.").contains(&Kind::MissingPercent));
}

#[test]
fn missing_percent_word_range_is_flagged_but_not_autofixable() {
    let doc = "---\ntitle: T\n---\n\nAdd @water{2 to 3 cups}.\n";
    let issues = lint("", doc);
    let issue = issue_for(&issues, Kind::MissingPercent).expect("MissingPercent issue");
    assert!(!issue.autofixable);
}

#[test]
fn flags_note_inside_braces() {
    assert!(body_kinds("Top with @parsley{(chopped)}.").contains(&Kind::NoteInBraces));
}

#[test]
fn flags_stray_braces_and_optional_colon() {
    assert!(body_kinds("Add @ciboulette{20%brins}{}.").contains(&Kind::StrayBraces));
    assert!(body_kinds("Add @?:Schnittlauch{15%g}.").contains(&Kind::OptionalColon));
}

#[test]
fn stray_braces_autofixable_only_when_the_pair_is_empty() {
    let doc = "---\ntitle: T\n---\n\nAdd @ciboulette{20%brins}{}.\n";
    let issues = lint("", doc);
    let issue = issue_for(&issues, Kind::StrayBraces).expect("StrayBraces issue");
    assert!(issue.autofixable);

    let doc = "---\ntitle: T\n---\n\nAdd @a{1%unit}{2}.\n";
    let issues = lint("", doc);
    let issue = issue_for(&issues, Kind::StrayBraces).expect("StrayBraces issue");
    assert!(!issue.autofixable);
}

#[test]
fn flags_unit_as_note() {
    assert!(body_kinds("Mince @garlic{4}(cloves).").contains(&Kind::UnitAsNote));
    assert!(body_kinds("Beat @eggs{2}(large).").contains(&Kind::UnitAsNote));
    assert!(!body_kinds("Dice @onion{1}(finely).").contains(&Kind::UnitAsNote));
}

#[test]
fn unit_as_note_is_case_insensitive_and_tolerates_a_space_before_the_paren() {
    assert!(body_kinds("Mince @garlic{4}(Cloves).").contains(&Kind::UnitAsNote));
    assert!(body_kinds("Mince @garlic{4} (cloves).").contains(&Kind::UnitAsNote));
}

#[test]
fn unit_as_note_accepts_decimal_comma_and_unicode_fraction_quantities() {
    assert!(body_kinds("Add @Knoblauch{1,5}(Zehen).").contains(&Kind::UnitAsNote));
    assert!(body_kinds("Add @garlic{½}(head).").contains(&Kind::UnitAsNote));
}

#[test]
fn unit_as_note_flags_even_with_trailing_prep_text() {
    assert!(body_kinds("Mince @garlic{4}(cloves, minced).").contains(&Kind::UnitAsNote));
}

#[test]
fn unit_as_note_never_fires_on_cookware() {
    assert!(!body_kinds("Use a #pan{2}(large).").contains(&Kind::UnitAsNote));
}

#[test]
fn flags_cookware_quantity_and_junk() {
    assert!(body_kinds("Use a #slow cooker{5-6 quart}.").contains(&Kind::CookwareQuantity));
    assert!(body_kinds("x #cncpt-mob_incontent > div{margin:40px}").contains(&Kind::CookwareJunk));
    assert!(!body_kinds("Heat a #skillet{}.").contains(&Kind::CookwareQuantity));
}

#[test]
fn cookware_name_cannot_start_with_a_space() {
    let ks = body_kinds("# Step 1 {x}");
    assert!(!ks.contains(&Kind::CookwareJunk));
    assert!(!ks.contains(&Kind::CookwareQuantity));
}

#[test]
fn flags_token_in_note() {
    assert!(body_kinds("> Sprinkle with @cheese{} at the end.").contains(&Kind::TokenInNote));
}

#[test]
fn token_in_note_accepts_leading_whitespace_before_the_note() {
    assert!(body_kinds("  > note with @cheese{}").contains(&Kind::TokenInNote));
}

#[test]
fn missing_percent_and_tilde_skip_comment_and_note_lines() {
    assert!(!body_kinds("-- draft: @x{2 cups}\nMix.").contains(&Kind::MissingPercent));
    assert!(!body_kinds("> Use @flour{200g} more if needed.").contains(&Kind::MissingPercent));
    assert!(!body_kinds("> about ~2 cm pieces").contains(&Kind::TildeApprox));
    // Still flagged on a real step line.
    assert!(body_kinds("Add @water{2 cups}.").contains(&Kind::MissingPercent));
    assert!(body_kinds("Cut into ~2 cm pieces.").contains(&Kind::TildeApprox));
}

#[test]
fn flags_percent_in_text_and_phantom_and_tilde() {
    assert!(body_kinds("Dilute in 200%ml water.").contains(&Kind::PercentInText));
    assert!(!body_kinds("Use 80% dark chocolate.").contains(&Kind::PercentInText));
    assert!(body_kinds("Heat to @180C{}.").contains(&Kind::PhantomToken));
    assert!(body_kinds("Cut into ~2 cm pieces.").contains(&Kind::TildeApprox));
    assert!(!body_kinds("Bake ~{20%minutes}.").contains(&Kind::TildeApprox));
}

#[test]
fn percent_in_text_ignores_german_adjective_suffix() {
    assert!(!body_kinds("70%ige Schokolade.").contains(&Kind::PercentInText));
    assert!(!body_kinds("70%-ige Schokolade.").contains(&Kind::PercentInText));
    assert!(body_kinds("Dilute in 200%ml water.").contains(&Kind::PercentInText));
}

#[test]
fn phantom_token_ignores_digit_led_ingredient_names() {
    assert!(!body_kinds("Add @7-spice mix{1%tsp}.").contains(&Kind::PhantomToken));
    assert!(!body_kinds("Add @5-spice powder{}.").contains(&Kind::PhantomToken));
    assert!(!body_kinds("Add @00 Mehl{500%g}.").contains(&Kind::PhantomToken));
    assert!(body_kinds("Heat to @180C{}.").contains(&Kind::PhantomToken));
    assert!(body_kinds("Add @200g{}.").contains(&Kind::PhantomToken));
    assert!(body_kinds("Heat to @350°F{}.").contains(&Kind::PhantomToken));
}

fn fm_kinds(fm: &str) -> Vec<Kind> {
    kinds(&lint("", &format!("---\n{fm}\n---\n\nBoil @eggs{{2}}.\n")))
}

#[test]
fn flags_empty_metadata() {
    assert!(fm_kinds("title: T\nauthor: 'null'").contains(&Kind::EmptyMetadata));
    assert!(fm_kinds("title: T\nprep time: ''").contains(&Kind::EmptyMetadata));
    assert!(fm_kinds("title: T\nnutrition:").contains(&Kind::EmptyMetadata));
}

#[test]
fn flags_html_entities_and_key_drift() {
    assert!(fm_kinds("title: Cr&egrave;me &amp; co").contains(&Kind::HtmlEntity));
    assert!(fm_kinds("title: T\nsource_url: https://x.y").contains(&Kind::MetadataKeyDrift));
}

#[test]
fn flags_invalid_yaml() {
    assert!(fm_kinds("title: a: b: c").contains(&Kind::InvalidFrontmatter));
}

#[test]
fn invalid_frontmatter_evidence_is_the_yaml_error_not_the_raw_text() {
    let fm = "title: a: b: c";
    let doc = format!("---\n{fm}\n---\n\nBoil @eggs{{2}}.\n");
    let issues = lint("", &doc);
    let issue = issue_for(&issues, Kind::InvalidFrontmatter).expect("InvalidFrontmatter issue");
    assert_ne!(issue.evidence, fm);
}

#[test]
fn parse_errors_do_not_suppress_warnings_or_recipe() {
    // "Timer value is text: abc" is a ParseError; "{~}" (an empty, invalid
    // component) triggers "Invalid single word name..." as a ParseWarning.
    // Both must surface, and the recipe (present despite the error) must
    // still be handed to `consistency::check`.
    let doc = "---\ntitle: T\n---\n\nCook for ~{abc%minutes}. {~}\n";
    let ks = kinds(&lint("", doc));
    assert!(ks.contains(&Kind::ParseError));
    assert!(ks.contains(&Kind::ParseWarning));
}

#[test]
fn nested_metadata_is_not_empty() {
    assert!(!fm_kinds("title: T\nnutrition:\n  calories: 480 kcal").contains(&Kind::EmptyMetadata));
}

#[test]
fn list_children_are_not_empty_metadata() {
    assert!(!fm_kinds("title: T\ntags:\n- a").contains(&Kind::EmptyMetadata));
    assert!(!fm_kinds("title: T\ntags:\n- a\n- b").contains(&Kind::EmptyMetadata));
    assert!(!fm_kinds("title: T\nnutrition:\n\tcalories: 480 kcal").contains(&Kind::EmptyMetadata));
}

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
fn missing_percent_still_flags_slash_fractions() {
    assert!(body_kinds("Add @sugar{1/2 cup}.").contains(&Kind::MissingPercent));
    assert!(body_kinds("Add @sugar{1 1/2 cups}.").contains(&Kind::MissingPercent));
    assert!(!body_kinds("Add @sugar{1/2%cup}.").contains(&Kind::MissingPercent));
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
fn unit_as_note_ignores_prose_in_parens_after_a_space() {
    // A note glued to '}' is a real cooklang note; a '(' preceded by a space
    // is ordinary prose in parentheses, not a note — only flag it when the
    // *entire* parenthetical is a single unit word.
    assert!(!body_kinds("Cook @onion{1} (medium heat) until soft.").contains(&Kind::UnitAsNote));
    assert!(!body_kinds("Add @bread{4} (slices are fine) now.").contains(&Kind::UnitAsNote));
    assert!(body_kinds("Mince @garlic{4} (cloves).").contains(&Kind::UnitAsNote));
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
fn missing_percent_and_tilde_skip_block_comment_lines() {
    assert!(!body_kinds("[- @x{2 cups} -]\nMix.").contains(&Kind::MissingPercent));
    assert!(!body_kinds("[- rest for ~2 minutes -]\nMix.").contains(&Kind::TildeApprox));
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

#[test]
fn flags_missing_source_ingredient() {
    let orig = "2 eggs\n1 tsp salt\n\nBoil the eggs.";
    let doc = "---\ntitle: T\n---\n\nBoil @eggs{2}.\n";
    assert!(kinds(&lint(orig, doc)).contains(&Kind::MissingIngredient));
}

#[test]
fn flags_double_count_against_single_listing() {
    let orig = "500 g Kartoffeln\n2 Möhren\n\nKartoffeln schälen. Kartoffeln kochen.";
    let doc = "---\ntitle: T\n---\n\n@Kartoffeln{500%g} schälen. @Kartoffeln{500%g} kochen. @Möhren{2} schneiden.\n";
    assert!(kinds(&lint(orig, doc)).contains(&Kind::DoubleCount));
}

/// Item 14: the source line says "divided" and the *sum* of tagged amounts
/// (4 tbsp) exceeds the single source listing (2 tbsp) — without the
/// divided-skip this would flag; with it, it must not.
#[test]
fn divided_amounts_are_not_double_counted() {
    let orig = "2 tbsp olive oil, divided\n1 onion\n\nHeat oil. Add onion. Add more oil.";
    let doc = "---\ntitle: T\n---\n\nHeat @olive oil{2%tbsp}. Add @onion{1}. Add @olive oil{2%tbsp}.\n";
    assert!(!kinds(&lint(orig, doc)).contains(&Kind::DoubleCount));
}

/// Item 3: a source line whose unit doesn't match (after normalization) the
/// doc's aggregated unit is skipped — cloves are a count, grams are a mass,
/// and this checker doesn't attempt cross-unit conversion.
#[test]
fn double_count_skipped_when_normalized_units_differ() {
    let orig = "2 cloves garlic\n1 onion\n\nMince the garlic and chop the onion.";
    let doc = "---\ntitle: T\n---\n\nMince @garlic{10%g}. Chop @onion{1}. Add more @garlic{10%g}.\n";
    assert!(!kinds(&lint(orig, doc)).contains(&Kind::DoubleCount));
}

/// Item 3: the same amount expressed in a different (but not obviously
/// equivalent, from this checker's point of view) unit than the source —
/// skipped rather than guessed at.
#[test]
fn double_count_skipped_when_source_uses_a_different_unit_for_an_equivalent_amount() {
    let orig = "1/4 cup olive oil\n1 onion\n\nHeat the oil in a pan and add the onion.";
    let doc = "---\ntitle: T\n---\n\nHeat @olive oil{2%tbsp}, add @onion{1} and more @olive oil{2%tbsp}.\n";
    assert!(!kinds(&lint(orig, doc)).contains(&Kind::DoubleCount));
}

/// Item 10: the word-boundary name lookup in double-count must not let
/// "unsalted butter" contribute to the listing for "salt".
#[test]
fn double_count_word_boundary_does_not_confuse_salt_with_unsalted_butter() {
    let orig = "2 tsp salt\n1 cup unsalted butter\n\nMelt the butter and season with salt.";
    let doc = "---\ntitle: T\n---\n\nMelt the @unsalted butter{1%cup}. Season with @salt{1%tsp}. \
        Add more @salt{1%tsp} to taste. Add a final @salt{1%tsp}.\n";
    assert!(kinds(&lint(orig, doc)).contains(&Kind::DoubleCount));
}

/// Item 2: a source line with no quantity ("salt to taste") is never
/// flagged as missing, even when it's never tagged.
#[test]
fn skips_missing_ingredient_without_a_quantity() {
    let orig = "2 eggs\nsalt to taste\n\nBoil the eggs and season with salt.";
    let doc = "---\ntitle: T\n---\n\nBoil @eggs{2}.\n";
    assert!(!kinds(&lint(orig, doc)).contains(&Kind::MissingIngredient));
}

/// Item 4: a bare two-character ingredient name ("Ei") must still count as
/// coverage evidence.
#[test]
fn coverage_matches_two_character_words() {
    let orig = "1 Ei\n200 g Mehl\n\nEi und Mehl verrühren.";
    let doc = "---\ntitle: T\n---\n\n@Ei{1} und @Mehl{200%g} verrühren.\n";
    assert_eq!(kinds(&lint(orig, doc)), vec![]);
}

/// Item 4 + 5: "Zwiebel" (doc) covers "Zwiebeln" (source) via the
/// common-prefix rule, and "Öl" (2 chars) still counts.
#[test]
fn coverage_matches_german_plural_and_two_character_words() {
    let orig = "2 Zwiebeln\n1 EL Öl\n\nHacken.";
    let doc = "---\ntitle: T\n---\n\n@Zwiebel{2} in @Öl{1%EL} hacken.\n";
    assert_eq!(kinds(&lint(orig, doc)), vec![]);
}

/// Item 5: "tomato" (doc) covers "tomatoes" (source) via the common-prefix
/// rule — the old trailing-"s" stem alone ("tomatoe" vs "tomato") missed
/// this.
#[test]
fn coverage_matches_english_plural_via_prefix_similarity() {
    let orig = "2 tomatoes\n1 onion\n\nChop everything finely before serving.";
    let doc = "---\ntitle: T\n---\n\nChop @tomato{2} and @onion{1}.\n";
    assert_eq!(kinds(&lint(orig, doc)), vec![]);
}

#[test]
fn flags_spelling_variants() {
    let orig = "3 oranges\n1 lemon\n\nZest.";
    let doc = "---\ntitle: T\n---\n\nZest @orange{1} and @lemon{1}, juice @oranges{2}.\n";
    assert!(kinds(&lint(orig, doc)).contains(&Kind::SpellingVariant));
}

/// Item 11: with 3+ distinct ingredient names, most of them should share a
/// word with the original — this doc is a wholesale English rewrite of a
/// German original, not a conversion.
#[test]
fn flags_language_mismatch() {
    let orig = "500 g Kartoffeln\n1 Zwiebel\n1 Karotte\n\nDie Kartoffeln waschen und in Würfel schneiden. \
        Die Zwiebel schälen und fein hacken. Die Karotte raspeln, dann alles in einer Pfanne braten.";
    let doc = "---\ntitle: T\n---\n\nWash the @potatoes{500%g}, peel the @onion{1} and grate the @carrot{1}, \
        then fry everything in a pan.\n";
    assert!(kinds(&lint(orig, doc)).contains(&Kind::LanguageMismatch));
}

/// Item 11: a mostly-CJK original is exempted from the word-overlap
/// language check (unspaced scripts don't segment into comparable words).
#[test]
fn language_mismatch_exempts_mostly_cjk_originals() {
    let orig = "洋葱 一个\n胡萝卜 两根\n土豆 三个\n\n将洋葱、胡萝卜、土豆切块后翻炒均匀即可食用。";
    let doc = "---\ntitle: T\n---\n\nStir-fry the @onion{1}, @carrot{2} and @potato{3} until tender.\n";
    assert!(!kinds(&lint(orig, doc)).contains(&Kind::LanguageMismatch));
}

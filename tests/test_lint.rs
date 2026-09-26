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
    // "Reference not found: salt" (an `@&` reference with no definition) is a
    // ParseError; "{~}" (an empty, invalid component) triggers "Invalid
    // single word name..." as a ParseWarning. Both must surface, and the
    // recipe (present despite the error) must still be handed to
    // `consistency::check`. (The lint parser has ADVANCED_UNITS off, so a
    // text timer value like `~{abc%minutes}` is no longer an error.)
    let doc = "---\ntitle: T\n---\n\nAdd @&salt{1}. {~}\n";
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

/// Coordinator fix 1: an empty or URL-only original has far too few words
/// to judge language overlap from — every name would trivially "not be
/// found", firing LanguageMismatch on every recipe with 3+ ingredients
/// whenever the original text is missing or just a URL.
#[test]
fn language_mismatch_skipped_when_original_has_too_few_words() {
    let doc = "---\ntitle: T\n---\n\n@flour{200%g}, @salt{1%tsp} and @water{100%ml}.\n";
    assert!(!kinds(&lint("", doc)).contains(&Kind::LanguageMismatch));
    assert!(!kinds(&lint("https://example.com/recipe", doc)).contains(&Kind::LanguageMismatch));
}

/// Coordinator fix 3: once LanguageMismatch has fired (a wholesale
/// translation), every source line would otherwise also read as "missing" —
/// don't double-report the same underlying defect.
#[test]
fn missing_ingredient_is_skipped_once_language_mismatch_has_fired() {
    let orig = "500 g Kartoffeln\n1 Zwiebel\n1 Karotte\n\nDie Kartoffeln waschen und in Würfel schneiden. \
        Die Zwiebel schälen und fein hacken. Die Karotte raspeln, dann alles in einer Pfanne braten.";
    let doc = "---\ntitle: T\n---\n\nWash the @potatoes{500%g}, peel the @onion{1} and grate the @carrot{1}, \
        then fry everything in a pan.\n";
    let issues = lint(orig, doc);
    assert!(kinds(&issues).contains(&Kind::LanguageMismatch));
    assert!(!kinds(&issues).contains(&Kind::MissingIngredient));
}

/// Coordinator fix 5: DIVIDED must not treat "split peas" as a divided
/// quantity just because it contains the word "split".
#[test]
fn double_count_still_fires_for_split_peas() {
    let orig = "1 pound split peas\n1 onion\n\nRinse the split peas. Cook the split peas until tender.";
    let doc = "---\ntitle: T\n---\n\nRinse @split peas{1%pound}. Cook @split peas{1%pound} until tender. Add @onion{1}.\n";
    assert!(kinds(&lint(orig, doc)).contains(&Kind::DoubleCount));
}

/// Coordinator fix 2: a page-sized dump (way more "sources" than any real
/// ingredient list) must not make `lint` quadratic — coverage is skipped
/// entirely past a source-count cap, so this must run in well under a
/// second even though the naive approach recomputes source-line words for
/// every source×token pair.
#[test]
fn lint_stays_fast_on_a_page_sized_dump() {
    let original: String = (0..40_000).map(|i| format!("{} item{}\n", i % 9 + 1, i)).collect();
    let body: String = (0..300).map(|i| format!("@item{}{{1}}\n", i * 100)).collect();
    let doc = format!("---\ntitle: T\n---\n\n{body}");

    let start = std::time::Instant::now();
    let issues = lint(&original, &doc);
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 1.0, "lint took {elapsed:?}, expected well under 1s");
    assert!(issues.iter().filter(|i| i.kind == Kind::MissingIngredient).count() <= 20);
}

/// Coordinator fix 2: even below the page-dump cap, MissingIngredient never
/// reports more than 20 issues for a single recipe.
#[test]
fn missing_ingredient_is_capped_at_twenty_per_recipe() {
    let original: String = (0..50).map(|i| format!("1 onlyinsource{i}\n")).collect();
    let doc = "---\ntitle: T\n---\n\n@nothing tagged{1}.\n";
    let issues = lint(&original, doc);
    assert_eq!(issues.iter().filter(|i| i.kind == Kind::MissingIngredient).count(), 20);
}

fn fixed(doc: &str) -> String {
    autofix(doc)
}

#[test]
fn fixes_missing_percent() {
    assert_eq!(fixed("Add @water{2 cups}."), "Add @water{2%cups}.");
    assert_eq!(fixed("Add @flour{200g}."), "Add @flour{200%g}.");
    assert_eq!(fixed("Add @sugar{1 1/2 cups}."), "Add @sugar{1 1/2%cups}.");
}

#[test]
fn fixes_unit_as_note_stray_and_optional() {
    assert_eq!(fixed("Mince @garlic{4}(cloves)."), "Mince @garlic{4%cloves}.");
    assert_eq!(fixed("Add @ciboulette{20%brins}{}."), "Add @ciboulette{20%brins}.");
    assert_eq!(fixed("Add @?:Schnittlauch{15%g}."), "Add @?Schnittlauch{15%g}.");
}

#[test]
fn fixes_tokens_in_notes_and_tilde() {
    // Notes may not contain Cooklang syntax, but the quantity is kept as
    // plain text rather than dropped.
    assert_eq!(
        fixed("> Sprinkle @cheese{50%g}(grated) on top."),
        "> Sprinkle 50 g cheese (grated) on top."
    );
    assert_eq!(fixed("> Rest for ~{10%minutes}."), "> Rest for 10 minutes.");
    assert_eq!(fixed("> Rest for ~rest{5%min}."), "> Rest for rest 5 min.");
    assert_eq!(fixed("Cut into ~2 cm pieces. Bake ~{20%minutes}."), "Cut into about 2 cm pieces. Bake ~{20%minutes}.");
}

#[test]
fn fixes_metadata() {
    let doc = "---\ntitle: Cr&egrave;me\nauthor: 'null'\nsource_url: https://x.y\n---\n\nStep.\n";
    assert_eq!(fixed(doc), "---\ntitle: Crème\nsource: https://x.y\n---\n\nStep.\n");
}

fn digits(s: &str) -> Vec<char> {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

#[test]
fn autofix_clears_fixable_issues() {
    let doc = "---\ntitle: T\nprep_time: ''\nauthor: 'null'\nsource_url: https://x.y\ndescr: Cr&egrave;me\n---\n\n\
        Add @water{2 cups} and @garlic{2}(cloves), @ciboulette{20%brins}{}, @?:Schnittlauch{15%g}.\n\
        Cut into ~2 cm pieces. Bake ~{20%minutes}.\n\
        > Sprinkle @cheese{50%g}(grated) on top.\n";
    let after = autofix(doc);
    let issues = lint("", &after);
    assert!(issues.iter().all(|i| !i.autofixable), "{issues:#?}");

    // Idempotent: a second pass changes nothing further.
    assert_eq!(autofix(&after), after);

    // Never changes a digit anywhere, including a note-line token's
    // quantity (see `fixes_tokens_in_notes_and_tilde`: the syntax is
    // stripped, but the "50" is kept as plain text).
    let mut before = digits(doc);
    let mut post = digits(&after);
    before.sort();
    post.sort();
    assert_eq!(before, post);
}

#[test]
fn autofix_never_touches_something_not_flagged() {
    // A space-preceded parenthetical that is NOT a bare unit word is ordinary
    // prose, not a note — `check` never flags it, so `autofix` must leave it
    // alone (including the space and the parens).
    let doc = "Add @garlic{4} (cloves, minced).";
    assert_eq!(fixed(doc), doc);

    // A word-spelled range is flagged but never autofixable — the number
    // stays exactly as written.
    let doc = "Add @water{2 to 3 cups}.";
    assert_eq!(fixed(doc), doc);
}

#[test]
fn fixes_double_encoded_html_entities_to_a_fixed_point() {
    let doc = "---\ntitle: T\ncourse: Pâtes &amp;amp; nouilles\n---\n\nStep.\n";
    let after = autofix(doc);
    assert!(!after.contains("&amp;"), "{after}");
    assert!(after.contains("Pâtes & nouilles"), "{after}");
    let issues = lint("", &after);
    assert!(issues.iter().all(|i| !i.autofixable), "{issues:#?}");
}

// --- Adversarial review round: CRITICAL (C1 YAML-safe entities, C2 duplicate-key guard) ---

fn fm_doc(fm: &str) -> String {
    format!("---\n{fm}\n---\n\nStep.\n")
}

#[test]
fn entity_fix_never_breaks_yaml_colon_in_value() {
    // "Foo&#58; bar" decoded naively ("Foo: bar") turns one mapping value
    // into what looks like a second key: value pair.
    let doc = fm_doc("title: Foo&#58; bar");
    let after = autofix(&doc);
    let (fm, _) = split_frontmatter(&after);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm.unwrap()).expect("valid YAML");
    assert_eq!(parsed["title"].as_str(), Some("Foo: bar"));
    assert!(lint("", &after).iter().all(|i| !i.autofixable));
}

#[test]
fn entity_fix_never_breaks_yaml_on_apostrophe() {
    let doc = fm_doc("title: 'Grandma&#39;s pie'");
    let after = autofix(&doc);
    let (fm, _) = split_frontmatter(&after);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm.unwrap()).expect("valid YAML");
    assert_eq!(parsed["title"].as_str(), Some("Grandma's pie"));
}

#[test]
fn entity_fix_never_injects_a_key_via_newline() {
    // A naively-decoded "&#10;" (newline) followed by "key: value"-shaped
    // text must never inject a real second key into the frontmatter. The raw
    // value "A&#10;author: evil" is inherently ambiguous — read on its own,
    // per `extract_string_value`, it parses as a *mapping* (not a string) —
    // so the safe outcome is declining the fix entirely (this is exactly the
    // kind of case round_trip_safe's original key-set-only check used to
    // wrongly wave through). Either the document is untouched, or — if a fix
    // was somehow applied — it must still have exactly the one key.
    let doc = fm_doc("title: A&#10;author: evil");
    let after = autofix(&doc);
    let (fm, _) = split_frontmatter(&after);
    match serde_yaml::from_str::<serde_yaml::Value>(fm.unwrap()) {
        Ok(parsed) => {
            assert_eq!(parsed.as_mapping().unwrap().len(), 1, "must not gain an `author` key: {parsed:?}");
        }
        Err(_) => assert_eq!(after, doc, "if it's still invalid, it must be untouched"),
    }
}

#[test]
fn entity_fix_never_injects_a_fence_via_newline() {
    let doc = fm_doc("title: A&#10;&#45;&#45;&#45;&#10;@evil{1%kg}");
    let after = autofix(&doc);
    // The closing fence must still be exactly where it should be: the body
    // must not gain a bogus "@evil" ingredient via an injected extra fence.
    let (_, body) = split_frontmatter(&after);
    assert_eq!(body.trim(), "Step.");
}

#[test]
fn entity_fix_never_truncates_via_hash_comment() {
    // A naively-decoded "&#35;" (a literal '#') mid-value must not truncate
    // the rest of the line as a YAML comment.
    let doc = fm_doc("title: Pie &#35;2 of 3");
    let after = autofix(&doc);
    let (fm, _) = split_frontmatter(&after);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm.unwrap()).expect("valid YAML");
    assert_eq!(parsed["title"].as_str(), Some("Pie #2 of 3"));
}

#[test]
fn entity_fix_never_corrupts_a_flow_list() {
    // A flow sequence must keep its element count and content — a
    // line-based fixer that blindly re-quotes the whole value would turn
    // the list into a single string.
    let doc = fm_doc("tags: [a&#44; b, c]");
    let after = autofix(&doc);
    let (fm, _) = split_frontmatter(&after);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm.unwrap()).expect("valid YAML");
    let before_parsed: serde_yaml::Value =
        serde_yaml::from_str(split_frontmatter(&doc).0.unwrap()).unwrap();
    assert_eq!(parsed["tags"], before_parsed["tags"], "the list must not change");
}

#[test]
fn entity_fix_never_touches_a_block_scalar_continuation_line() {
    let doc = fm_doc("description: |\n  line &#10;x");
    let after = autofix(&doc);
    // Must stay valid YAML with the description key intact — not broken by
    // an injected literal newline inside the block scalar's own syntax.
    let (fm, _) = split_frontmatter(&after);
    assert!(serde_yaml::from_str::<serde_yaml::Value>(fm.unwrap()).is_ok(), "{fm:?}");
}

#[test]
fn entity_fix_leaves_unknown_or_invalid_entities_alone_and_non_autofixable() {
    for value in ["Salt&pepper;", "x&#55296;y", "x&#00;y"] {
        let doc = fm_doc(&format!("title: {value}"));
        let after = autofix(&doc);
        assert_eq!(after, doc, "should be a no-op for {value:?}");
        let issues = lint("", &doc);
        let html_entity = issues.iter().find(|i| i.kind == Kind::HtmlEntity);
        assert!(html_entity.is_some_and(|i| !i.autofixable), "{value:?}: {issues:#?}");
    }
}

#[test]
fn entity_fix_converges_on_deeply_nested_encoding() {
    let doc = fm_doc("title: &amp;amp;amp;amp;amp;amp;amp;amp;amp;amp;amp;amp;lt;");
    let after = autofix(&doc);
    assert_eq!(autofix(&after), after, "must be idempotent");
    let (fm, _) = split_frontmatter(&after);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm.unwrap()).expect("valid YAML");
    assert_eq!(parsed["title"].as_str(), Some("<"));
}

#[test]
fn key_rename_skipped_when_target_key_already_exists() {
    // C2: renaming `source_url` to `source` when `source` already exists
    // would create a duplicate YAML key.
    let doc = fm_doc("title: T\nsource: a\nsource_url: b");
    let after = autofix(&doc);
    assert_eq!(after, doc, "the rename must be skipped entirely");
    let issues = lint("", &doc);
    let drift = issues.iter().find(|i| i.kind == Kind::MetadataKeyDrift);
    assert!(drift.is_some_and(|i| !i.autofixable), "{issues:#?}");
    let (fm, _) = split_frontmatter(&after);
    assert!(serde_yaml::from_str::<serde_yaml::Value>(fm.unwrap()).is_ok());
}

#[test]
fn key_rename_skipped_when_two_spellings_of_the_same_key_collide() {
    let doc = fm_doc("prep time: 5 min\nprep_time: 10 min");
    let after = autofix(&doc);
    assert_eq!(after, doc);
    let (fm, _) = split_frontmatter(&after);
    assert!(serde_yaml::from_str::<serde_yaml::Value>(fm.unwrap()).is_ok());
}

// --- Adversarial review round: IMPORTANT (I1 regex parity, I2 unflagged text
// / byte preservation, I3 note quantity preservation) ---

#[test]
fn fixes_a_run_of_stray_empty_braces_in_one_pass() {
    assert_eq!(fixed("Add @x{2%g}{}{}."), "Add @x{2%g}.");
    assert_eq!(fixed("Add @x{2%g}{}{}{}."), "Add @x{2%g}.");
    let after = autofix("Add @x{2%g}{}{}{}.");
    assert_eq!(autofix(&after), after);
    assert!(lint("", &after).iter().all(|i| !i.autofixable));
}

#[test]
fn fixes_an_optional_marker_with_multiple_colons_in_one_pass() {
    assert_eq!(fixed("Add @?::salt{1%g}."), "Add @?salt{1%g}.");
    let after = autofix("Add @?::salt{1%g}.");
    assert_eq!(autofix(&after), after);
    assert!(lint("", &after).iter().all(|i| !i.autofixable));
}

#[test]
fn note_line_stray_and_optional_colon_are_also_fixed() {
    // StrayBraces/OptionalColon aren't exempted on note lines by `check`, so
    // `autofix` must clear them there too, even with no enclosing token.
    assert_eq!(fixed("> text @?: foo"), "> text @? foo");
    assert_eq!(fixed("> text }{} end"), "> text } end");
    for doc in ["> text @?: foo", "> text }{} end"] {
        let after = autofix(doc);
        assert!(lint("", &after).iter().all(|i| !i.autofixable), "{doc}: {after}");
    }
}

#[test]
fn note_tokens_with_an_empty_name_are_fixed_without_panicking() {
    assert_eq!(fixed("> use @{} here"), "> use  here");
    assert_eq!(fixed("> use #{2} here"), "> use 2 here");
    for doc in ["> use @{} here", "> use #{2} here"] {
        assert!(lint("", &autofix(doc)).iter().all(|i| !i.autofixable));
    }
}

#[test]
fn note_tokens_keep_their_quantity_instead_of_dropping_it() {
    assert_eq!(
        fixed("> Sprinkle @cheese{50%g}(grated) on top."),
        "> Sprinkle 50 g cheese (grated) on top."
    );
    assert_eq!(
        fixed("> Sprinkle @cheese{50%g}(grated, 2 tbsp) on top."),
        "> Sprinkle 50 g cheese (grated, 2 tbsp) on top."
    );
    assert_eq!(fixed("> use #{2} here"), "> use 2 here");
    assert_eq!(fixed("> Rest for ~{10%minutes}."), "> Rest for 10 minutes.");
    assert_eq!(fixed("> Rest for ~rest{5%min}."), "> Rest for rest 5 min.");
    // A space-preceded parenthetical next to a note token is prose, not part
    // of the token's own note, and must survive along with its digit.
    assert_eq!(
        fixed("> Use a #big pan{} and @salt{} (about 2 tsp) now."),
        "> Use a big pan and salt (about 2 tsp) now."
    );
}

#[test]
fn tilde_inside_a_token_is_never_rewritten() {
    // The tilde here is part of the quantity, not a stray approximation
    // marker — nothing ever flags it, so autofix must leave it alone.
    for doc in ["Add @x{~2%g}.", "Use #pan{~2}.", "Bake ~20{minutes}."] {
        assert_eq!(fixed(doc), doc, "{doc}");
    }
}

#[test]
fn unit_as_note_exact_match_required_when_space_preceded() {
    // Trailing text after the unit word (even just a comma) makes this
    // ordinary prose, not a note — neither `lint` nor `autofix` may touch it.
    let doc = "Mince @garlic{4} (cloves,).";
    assert!(!kinds(&lint("", doc)).contains(&Kind::UnitAsNote));
    assert_eq!(fixed(doc), doc);
}

#[test]
fn autofix_is_a_byte_exact_no_op_when_nothing_is_flagged() {
    // No dedicated reconstruction-template code path should ever run when
    // there's nothing to fix — these each round-trip through split/rejoin
    // logic that doesn't preserve BOM/CRLF/blank-line layout on its own.
    let docs = [
        "\u{FEFF}Add @water{2%cups}.",
        "\u{FEFF}---\ntitle: T\n---\n\nStep.",
        "---\ntitle: T\n---\nStep.",
        "---\n---\n\nStep.",
        "---\ntitle: x\n---",
    ];
    for doc in docs {
        assert_eq!(autofix(doc), doc, "{doc:?}");
    }
}

#[test]
fn autofix_preserves_crlf_line_endings_when_fixing_the_frontmatter() {
    let doc = "---\r\ntitle: T\r\nsource_url: x\r\n---\r\n\r\nStep.\r\n";
    let after = autofix(doc);
    assert_eq!(after, "---\r\ntitle: T\r\nsource: x\r\n---\r\n\r\nStep.\r\n");
    // No lone '\n' anywhere once every '\r\n' pair is removed — i.e. never a
    // mix of bare LF and CRLF in the same document.
    assert!(!after.replace("\r\n", "").contains('\n'), "mixed line endings: {after:?}");
}

// --- Adversarial review round: MINOR ---

#[test]
fn unit_as_note_does_not_match_a_compound_hyphenated_word() {
    let doc = "Add @x{1}(heads-up: hot).";
    assert!(!kinds(&lint("", doc)).contains(&Kind::UnitAsNote));
    assert_eq!(fixed(doc), doc);
}

#[test]
fn missing_percent_non_fixable_when_the_unit_phrase_contains_a_digit() {
    let doc = "Add @tomatoes{2 x 400g cans}.";
    let issues = lint("", doc);
    let issue = issues.iter().find(|i| i.kind == Kind::MissingPercent).expect("MissingPercent");
    assert!(!issue.autofixable, "{issues:#?}");
    assert_eq!(fixed(doc), doc);
}

#[test]
fn has_children_skips_a_comment_line_before_the_real_children() {
    let doc = "---\ntitle: T\ntags:\n# c\n  - a\n---\n\nStep.\n";
    assert!(!kinds(&lint("", doc)).contains(&Kind::EmptyMetadata));
    assert_eq!(fixed(doc), doc);
}

// --- Second adversarial review round (root cause: round_trip_safe only
// checked the key set, never the values) ---

fn fm_value(doc: &str, key: &str) -> Option<String> {
    let (fm, _) = split_frontmatter(doc);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm?).ok()?;
    parsed.get(key)?.as_str().map(str::to_string)
}

#[test]
fn entity_fix_resolves_a_unicode_escape_without_corrupting_it() {
    // The raw text has a literal "é" YAML escape (not a literal
    // backslash) — round_trip_safe's old key-set-only check let this
    // through even though our own re-quoting turned the real 'é' character
    // into the literal 6 characters "é".
    let doc = "---\ntitle: \"caf\\u00e9 &amp; bar\"\n---\n\nStep.";
    let after = autofix(doc);
    assert_eq!(fm_value(&after, "title").as_deref(), Some("café & bar"));
    assert!(lint("", &after).iter().all(|i| !i.autofixable));
}

#[test]
fn entity_fix_resolves_a_hex_escape_without_corrupting_it() {
    let doc = "---\ntitle: \"a\\x41 &amp; b\"\n---\n\nStep.";
    let after = autofix(doc);
    assert_eq!(fm_value(&after, "title").as_deref(), Some("aA & b"));
}

#[test]
fn entity_fix_never_produces_a_null_bool_int_or_tilde_type() {
    // Each of these decodes to text that would misparse as a different YAML
    // type if emitted bare — the fix must always keep it a String.
    let cases = [
        ("title: &#126;", "title", "~"),
        ("author: nu&#108;l", "author", "null"),
        ("vegan: &#116;rue", "vegan", "true"),
        ("servings: &#52;", "servings", "4"),
    ];
    for (line, key, expected) in cases {
        let doc = format!("---\n{line}\n---\n\nStep.");
        let after = autofix(&doc);
        // Either the value survived as exactly that string (quoted), or (for
        // "null"/"~", which are themselves placeholder sentinels once
        // quoted) the whole now-empty-looking line was structurally deleted
        // — both are safe: never a real null/bool/int, and never left as a
        // dangling autofixable issue.
        if let Some(v) = fm_value(&after, key) {
            assert_eq!(v, expected, "{line}: {after:?}");
        }
        assert!(lint("", &after).iter().all(|i| !i.autofixable), "{line}: {after:?}");
        assert_eq!(autofix(&after), after, "not idempotent for {line}: {after:?}");
    }
}

#[test]
fn entity_fix_strips_a_trailing_comment_instead_of_absorbing_it() {
    let doc = "---\ntitle: Foo &amp; bar # draft\n---\n\nStep.";
    let after = autofix(doc);
    assert_eq!(fm_value(&after, "title").as_deref(), Some("Foo & bar"));
}

#[test]
fn entity_fix_strips_a_trailing_comment_after_a_single_quoted_value() {
    let doc = "---\ntitle: 'Grandma&#39;s' # c\n---\n\nStep.";
    let after = autofix(doc);
    assert_eq!(fm_value(&after, "title").as_deref(), Some("Grandma's"));
}

#[test]
fn entity_fix_handles_bare_values_starting_with_a_yaml_indicator_char() {
    // "&#45; x" etc. don't parse even in isolation (a bare scalar can't
    // start with these characters unquoted) — the fix must still safely
    // quote and decode them rather than leaving them broken forever.
    let cases = [("&#45; x", "- x"), ("&#63; x", "? x"), ("&#96;x", "`x")];
    for (raw, expected) in cases {
        let doc = format!("---\ntitle: {raw}\n---\n\nStep.");
        let after = autofix(&doc);
        assert_eq!(fm_value(&after, "title").as_deref(), Some(expected), "{raw}: {after:?}");
        assert!(lint("", &after).iter().all(|i| !i.autofixable));
    }
}

#[test]
fn entity_fix_resolves_a_duplicate_key_once_the_placeholder_twin_is_deleted() {
    // The structural (placeholder-deletion) pass runs before entity
    // decoding, so by the time the entity fix sees this, the duplicate
    // "title: ''" is already gone and only one "title" key remains.
    let doc = "---\ntitle: ''\ntitle: A &amp; B\n---\n\nStep.";
    let after = autofix(doc);
    assert_eq!(fm_value(&after, "title").as_deref(), Some("A & B"));
    let (fm, _) = split_frontmatter(&after);
    let parsed: serde_yaml::Value = serde_yaml::from_str(fm.unwrap()).expect("valid YAML");
    assert_eq!(parsed.as_mapping().unwrap().len(), 1);
}

#[test]
fn entity_fix_leaves_a_genuine_duplicate_key_untouched() {
    // Unlike the placeholder case above, neither "title" line is a
    // placeholder, so both survive — fixing either one's value would still
    // leave two "title" keys, so the whole document must stay untouched.
    let doc = "---\ntitle: a &amp; b\ntitle: c\n---\n\nStep.";
    assert_eq!(autofix(doc), doc);
}

#[test]
fn entity_fix_fixes_each_key_independently_even_when_a_rename_is_skipped() {
    let doc = "---\ntitle: a &amp; b\nsource_url: x &amp; y\nsource: z\n---\n\nStep.";
    let after = autofix(doc);
    assert_eq!(fm_value(&after, "title").as_deref(), Some("a & b"));
    assert_eq!(fm_value(&after, "source_url").as_deref(), Some("x & y"));
    assert_eq!(fm_value(&after, "source").as_deref(), Some("z"));
}

#[test]
fn entity_fix_skips_a_scalar_folded_onto_a_continuation_line() {
    let doc = "---\ndescription: foo &amp; bar\n  continued here\n---\n\nStep.";
    assert_eq!(autofix(doc), doc);
}

#[test]
fn stray_braces_after_a_non_empty_pair_are_never_independently_removed() {
    // `check` finds "}{2}" as ONE non-fixable match (its non-overlapping
    // scan consumes the closing brace the trailing "{}" would need); the fix
    // must not independently strip that trailing "{}" using a wider pattern
    // that doesn't respect the same consumption.
    let doc = "Add @x{1%g}{2}{}.";
    let issues = lint("", doc);
    assert!(issues.iter().find(|i| i.kind == Kind::StrayBraces).is_some_and(|i| !i.autofixable));
    assert_eq!(fixed(doc), doc);
}

#[test]
fn mixed_line_endings_are_preserved_per_line_not_normalized() {
    let doc = "Add @water{2 cups}.\r\nMix.\nStir.\n";
    assert_eq!(fixed(doc), "Add @water{2%cups}.\r\nMix.\nStir.\n");
}

#[test]
fn a_lone_cr_is_never_converted_or_touched() {
    let doc = "Add @water{2 cups}.\r\nMix.\r\rStir @salt{}.";
    assert_eq!(fixed(doc), "Add @water{2%cups}.\r\nMix.\r\rStir @salt{}.");
}

#[test]
fn note_line_inserts_a_space_between_adjacent_tokens() {
    assert_eq!(fixed("> see @a{1}@b{2}"), "> see 1 a 2 b");
}

#[test]
fn entity_fix_always_quotes_a_yaml_1_1_boolean_word() {
    // cook.md's Rails app reads this frontmatter with Ruby Psych (YAML 1.1),
    // where a bare yes/no/on/off/y/n (any case) is a boolean — even though
    // serde_yaml (YAML 1.2-ish) would parse it back as our own String, we
    // must still always quote it for the actual downstream consumer.
    for (raw, expected) in [
        ("y&#101;s", "yes"),
        ("n&#111;", "no"),
        ("&#111;n", "on"),
        ("&#111;ff", "off"),
        ("&#121;", "y"),
        ("&#110;", "n"),
    ] {
        let doc = format!("---\nvegan: {raw}\n---\n\nStep.");
        let after = autofix(&doc);
        let (fm, _) = split_frontmatter(&after);
        let value_line = fm.unwrap().lines().find(|l| l.starts_with("vegan:")).unwrap();
        assert_eq!(value_line, &format!("vegan: \"{expected}\""), "{raw}: {after:?}");
        assert_eq!(fm_value(&after, "vegan").as_deref(), Some(expected));
    }
}

#[test]
fn non_english_timer_units_are_not_parse_errors() {
    // cooklang's ADVANCED_UNITS extension only knows English time units, so
    // `~{10%Minuten}` was a hard "Unknown timer unit" error. Timer units stay
    // in the recipe's language; the lint parser runs without that extension.
    for body in ["Backen ~{10%Minuten}.", "Варить ~{5%минут}.", "Cuire ~{2%heures}."] {
        let doc = format!("---\ntitle: T\n---\n\n{body}\n");
        let ks = kinds(&lint("", &doc));
        assert!(!ks.contains(&Kind::ParseError), "{body}: {ks:?}");
    }
}

#[test]
fn range_quantities_still_count_toward_double_count() {
    // The lint parser keeps RANGE_VALUES on, so `{2-3%cloves}` is a numeric
    // range and the double-count check reads its upper bound.
    let orig = "2-3 cloves garlic\n1 onion\n\nFry the garlic, add the onion.";
    let twice = "---\ntitle: T\n---\n\nFry @garlic{2-3%cloves}. Add @garlic{2-3%cloves} and @onion{1}.\n";
    assert!(kinds(&lint(orig, twice)).contains(&Kind::DoubleCount));
    let once = "---\ntitle: T\n---\n\nFry @garlic{2-3%cloves}. Add the @garlic{} and @onion{1}.\n";
    let ks = kinds(&lint(orig, once));
    assert!(!ks.contains(&Kind::DoubleCount), "{ks:?}");
    assert!(!ks.contains(&Kind::ParseError) && !ks.contains(&Kind::ParseWarning), "{ks:?}");
}

// --- Nested frontmatter entities (indented `key: value` inside block mappings) ---

fn entity_fixable(doc: &str) -> Vec<bool> {
    lint("", doc).iter().filter(|i| i.kind == Kind::HtmlEntity).map(|i| i.autofixable).collect()
}

fn fixed_fm(doc: &str) -> serde_yaml::Value {
    let after = autofix(doc);
    serde_yaml::from_str(split_frontmatter(&after).0.expect("frontmatter")).expect("valid YAML")
}

#[test]
fn nested_entity_is_decoded_and_the_rest_of_the_tree_is_untouched() {
    let doc = fm_doc("title: T\nnutrition:\n  calories: 420&nbsp;kcal\n  fat: 22 g\ntags:\n- a");
    assert_eq!(entity_fixable(&doc), vec![true]);
    let v = fixed_fm(&doc);
    assert_eq!(v["nutrition"]["calories"].as_str(), Some("420\u{a0}kcal"));
    assert_eq!(v["nutrition"]["fat"].as_str(), Some("22 g"));
    assert_eq!(v["title"].as_str(), Some("T"));
    assert_eq!(v["tags"][0].as_str(), Some("a"));
    assert!(lint("", &autofix(&doc)).iter().all(|i| i.kind != Kind::HtmlEntity));
}

#[test]
fn deeply_nested_entity_is_decoded() {
    let doc = fm_doc("a:\n  b:\n    c: x &amp; y\n  d: e");
    assert_eq!(entity_fixable(&doc), vec![true]);
    let v = fixed_fm(&doc);
    assert_eq!(v["a"]["b"]["c"].as_str(), Some("x & y"));
    assert_eq!(v["a"]["d"].as_str(), Some("e"));
}

#[test]
fn nested_entity_fix_quotes_when_needed() {
    // YAML 1.1 bool word, a colon, a would-be number, a trailing comment.
    let doc = fm_doc("n:\n  vegan: y&#101;s\n  x: a&#58; b\n  k: 4&#50;\n  fat: 2 &amp; 3 g # c");
    assert_eq!(entity_fixable(&doc), vec![true, true, true, true]);
    let after = autofix(&doc);
    assert!(after.contains("vegan: \"yes\""), "{after}");
    let v = fixed_fm(&doc);
    assert_eq!(v["n"]["vegan"].as_str(), Some("yes"));
    assert_eq!(v["n"]["x"].as_str(), Some("a: b"));
    assert_eq!(v["n"]["k"].as_str(), Some("42"));
    assert_eq!(v["n"]["fat"].as_str(), Some("2 & 3 g"));
}

#[test]
fn nested_entity_skips_sequences_flow_collections_and_block_scalars() {
    for fm in [
        "tags:\n  - a &amp; b",
        "steps:\n- name: a &amp; b",
        "n:\n  list:\n  - a &amp; b",
        "n:\n  x: [a &amp; b, c]",
        "n:\n  x: {k: a &amp; b}",
        "n:\n  note: |\n    a &amp; b",
        "n:\n  note: >\n    a &amp; b",
        "n:\n  d: foo &amp; bar\n    continued here",
    ] {
        let doc = fm_doc(fm);
        assert!(entity_fixable(&doc).iter().all(|f| !f), "{fm:?}");
        assert_eq!(autofix(&doc), doc, "{fm:?}");
    }
}

#[test]
fn nested_entity_under_invalid_frontmatter_is_not_fixed() {
    // A duplicate key, and a value that *starts* with an entity (YAML reads
    // the leading `&` as an anchor, so the whole frontmatter is invalid):
    // there's no original tree to validate a nested rewrite against.
    for fm in ["n:\n  a: x &amp; y\n  a: z", "n:\n  k: &#52;2"] {
        let doc = fm_doc(fm);
        assert!(entity_fixable(&doc).iter().all(|f| !f), "{fm:?}");
        assert_eq!(autofix(&doc), doc, "{fm:?}");
    }
}

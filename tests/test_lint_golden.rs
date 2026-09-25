//! A small corpus of realistic, *valid* conversions that must always lint to
//! `vec![]`. Each one deliberately exercises a fix from the code-quality
//! review round (unicode fractions/dash ranges, German '%ige', accented
//! French units, section headers, comments/notes, cookware without a
//! quantity, quantity-less "to taste" ingredients) so a regression in any of
//! those checks shows up here as a false positive on a clean recipe.
//!
//! Originals are kept short, and every ingredient in the original is
//! represented by a token in the conversion, so these stay clean once batch
//! 2 adds ingredient-coverage checks to `consistency::check`.
#![cfg(feature = "lint")]
use cooklang_import::lint::{autofix, lint};

fn assert_clean(original: &str, cooklang: &str) {
    let issues = lint(original, cooklang);
    assert_eq!(issues, vec![], "expected no issues, got: {issues:#?}");
}

#[test]
fn german_dessert_is_clean() {
    // Unicode fraction ("1 ½"), a dash range ("2-3"), the '%ige' adjective
    // suffix that PCT_TEXT must not mistake for a leaked '%' sigil, a YAML
    // sequence ('tags:') and a nested map ('nutrition:') that must not be
    // read as empty metadata.
    let original = "2-3 EL Mehl\n1 1/2 Tassen Zucker\n70 g Schokolade\n\nMische das Mehl mit dem Zucker. Schmelze die Schokolade und rühre sie unter.";
    let cooklang = "---\ntitle: Schokoladenkuchen\ntags:\n- Dessert\n- Schokolade\nnutrition:\n  calories: 480 kcal\n  fat: 22 g\n---\n\nMische @Mehl{2-3%EL} mit @Zucker{1 ½%Tassen}.\n\nSchmelze @Schokolade{70%g} und rühre sie unter. Die 70%ige Schokolade sollte glänzen.\n";
    assert_clean(original, cooklang);
}

#[test]
fn french_recipe_is_clean() {
    // "c. à s." (cuillère à soupe) as a multi-word, accented, period-bearing
    // unit, and an accented ingredient name ("crème fraîche").
    let original = "2 c. à s. d'huile\n1 c. à s. de vinaigre\n100 g de crème fraîche\n\nMélangez tout avec du sel et du poivre.";
    let cooklang = "---\ntitle: Vinaigrette\n---\n\nMélangez @huile{2%c. à s.} avec @vinaigre{1%c. à s.} et @crème fraîche{100%g}.\n\nAssaisonnez avec du sel et du poivre.\n";
    assert_clean(original, cooklang);
}

#[test]
fn english_recipe_with_sections_and_cookware_is_clean() {
    // A '== Section ==' header, cookware with no quantity ('#large
    // skillet{}'), a dash-range timer ('~{10-15%minutes}'), and a slash
    // fraction quantity ('1 1/2%cups').
    let original = "50 g butter\n1 1/2 cups sugar\n\nHeat a large skillet and melt the butter and sugar. Simmer for 10-15 minutes.";
    let cooklang = "---\ntitle: Simple Sauce\n---\n\n== Sauce ==\n\nHeat a #large skillet{} and melt @butter{50%g} with @sugar{1 1/2%cups}.\n\nSimmer for ~{10-15%minutes}.\n";
    assert_clean(original, cooklang);
}

#[test]
fn english_recipe_with_note_and_comment_is_clean() {
    // A '-- comment' and a '> note' with plain prose — neither is a real
    // step, so a bare number or missing '%' in them isn't a defect.
    let original = "200 g pasta\n\nBoil the pasta in salted water. Reserve a splash of pasta water before draining.";
    let cooklang = "---\ntitle: Weeknight Pasta\n---\n\n-- draft: double-check timing before publishing\nBoil @pasta{200%g} in salted water.\n\n> Tip: reserve a splash of pasta water before draining.\n";
    assert_clean(original, cooklang);
}

#[test]
fn ingredient_only_quantity_less_recipe_is_clean() {
    // "to taste" ingredients carry no quantity at all ('@salt{}') — that's
    // not a MissingPercent defect, it's a legitimate empty amount.
    let original = "salt, to taste\npepper, to taste\n\nAdd salt and pepper to taste.";
    let cooklang = "---\ntitle: Seasoning\n---\n\nAdd @salt{} to taste and @pepper{} to taste.\n";
    assert_clean(original, cooklang);
}

#[test]
fn german_soup_is_clean() {
    // A glued unit ('1%l'), cookware with no quantity ('#Topf{}'), and a
    // plain timer, all in one recipe.
    let original = "500 g Kartoffeln\n1 l Brühe\n\nSchäle die Kartoffeln und schneide sie klein. Koche sie in einem Topf mit Brühe für 20 minutes.";
    let cooklang = "---\ntitle: Kartoffelsuppe\n---\n\nSchäle @Kartoffeln{500%g} und schneide sie klein.\n\nKoche sie in einem #Topf{} mit @Brühe{1%l} für ~{20%minutes}.\n";
    assert_clean(original, cooklang);
}

/// Every fixture above is already valid Cooklang — `autofix` must be a
/// complete no-op on each one. A regression in any fix regex would show up
/// here as an unwanted rewrite of already-correct text.
#[test]
fn autofix_never_rewrites_a_clean_recipe() {
    let fixtures: &[&str] = &[
        "---\ntitle: Schokoladenkuchen\ntags:\n- Dessert\n- Schokolade\nnutrition:\n  calories: 480 kcal\n  fat: 22 g\n---\n\nMische @Mehl{2-3%EL} mit @Zucker{1 ½%Tassen}.\n\nSchmelze @Schokolade{70%g} und rühre sie unter. Die 70%ige Schokolade sollte glänzen.\n",
        "---\ntitle: Vinaigrette\n---\n\nMélangez @huile{2%c. à s.} avec @vinaigre{1%c. à s.} et @crème fraîche{100%g}.\n\nAssaisonnez avec du sel et du poivre.\n",
        "---\ntitle: Simple Sauce\n---\n\n== Sauce ==\n\nHeat a #large skillet{} and melt @butter{50%g} with @sugar{1 1/2%cups}.\n\nSimmer for ~{10-15%minutes}.\n",
        "---\ntitle: Weeknight Pasta\n---\n\n-- draft: double-check timing before publishing\nBoil @pasta{200%g} in salted water.\n\n> Tip: reserve a splash of pasta water before draining.\n",
        "---\ntitle: Seasoning\n---\n\nAdd @salt{} to taste and @pepper{} to taste.\n",
        "---\ntitle: Kartoffelsuppe\n---\n\nSchäle @Kartoffeln{500%g} und schneide sie klein.\n\nKoche sie in einem #Topf{} mit @Brühe{1%l} für ~{20%minutes}.\n",
    ];
    for doc in fixtures {
        assert_eq!(&autofix(doc), doc, "autofix rewrote a clean recipe:\n{doc}");
        // Idempotent trivially follows, but assert it directly too.
        assert_eq!(autofix(&autofix(doc)), autofix(doc));
    }
}

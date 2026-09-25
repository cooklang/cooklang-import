//! Property-style fuzzing for `autofix`, run over the 6 golden fixtures plus
//! ~30 crafted defect snippets, mutated (concatenated/wrapped/duplicated)
//! with a small fixed-seed deterministic generator — no external dependency
//! needed for this. Every generated case is checked against six invariants
//! (see `check_case`).
#![cfg(feature = "lint")]
use cooklang_import::lint::{autofix, lint, split_frontmatter, Kind};

/// A tiny xorshift64* PRNG. Deterministic for a given seed, which is all we
/// need: reproducible fuzzing, not cryptographic quality.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// The 6 golden fixtures from `test_lint_golden.rs` — already-clean, valid
/// Cooklang. Kept as literal copies (small, deliberate duplication) rather
/// than shared code, since integration test binaries don't share items.
const GOLDEN: &[&str] = &[
    "---\ntitle: Schokoladenkuchen\ntags:\n- Dessert\n- Schokolade\nnutrition:\n  calories: 480 kcal\n  fat: 22 g\n---\n\nMische @Mehl{2-3%EL} mit @Zucker{1 ½%Tassen}.\n\nSchmelze @Schokolade{70%g} und rühre sie unter. Die 70%ige Schokolade sollte glänzen.\n",
    "---\ntitle: Vinaigrette\n---\n\nMélangez @huile{2%c. à s.} avec @vinaigre{1%c. à s.} et @crème fraîche{100%g}.\n\nAssaisonnez avec du sel et du poivre.\n",
    "---\ntitle: Simple Sauce\n---\n\n== Sauce ==\n\nHeat a #large skillet{} and melt @butter{50%g} with @sugar{1 1/2%cups}.\n\nSimmer for ~{10-15%minutes}.\n",
    "---\ntitle: Weeknight Pasta\n---\n\n-- draft: double-check timing before publishing\nBoil @pasta{200%g} in salted water.\n\n> Tip: reserve a splash of pasta water before draining.\n",
    "---\ntitle: Seasoning\n---\n\nAdd @salt{} to taste and @pepper{} to taste.\n",
    "---\ntitle: Kartoffelsuppe\n---\n\nSchäle @Kartoffeln{500%g} und schneide sie klein.\n\nKoche sie in einem #Topf{} mit @Brühe{1%l} für ~{20%minutes}.\n",
];

/// ~30 crafted defect snippets, each exercising one or more autofixable (or
/// deliberately non-autofixable) issue kinds. Plain bodies (no frontmatter)
/// unless the defect is itself frontmatter-shaped.
const DEFECTS: &[&str] = &[
    "Add @water{2 cups}.",
    "Add @flour{200g}.",
    "Add @sugar{1 1/2 cups}.",
    "Add @sugar{½ cup}.",
    "Add @potatoes{2–3 large}.",
    "Add @egg{2 - 3 cups}.",
    "Add @water{2 to 3 cups}.",
    "Add @Mehl{2 bis 3 EL}.",
    "Добавьте @мука{200 г} и @молоко{500мл}.",
    "@砂糖{大さじ2}を加える。",
    "Add @tomatoes{2 x 400g cans}.",
    "Mince @garlic{4}(cloves).",
    "Mince @garlic{4} (cloves).",
    "Mince @garlic{4} (cloves,).",
    "Add @x{1}(heads-up: hot).",
    "Add @ciboulette{20%brins}{}.",
    "Add @x{2%g}{}{}.",
    "Add @x{2%g}{}{}{}.",
    "Add @?:Schnittlauch{15%g}.",
    "Add @?::salt{1%g}.",
    "Cut into ~2 cm pieces.",
    "About ~ 2 cm and ~\t3 in.",
    "Add @x{~2%g}.",
    "Use #pan{~2}.",
    "> Sprinkle @cheese{50%g}(grated) on top.",
    "> Sprinkle @cheese{50%g}(grated, 2 tbsp) on top.",
    "> use @{} here",
    "> use #{2} here",
    "> text @?: foo",
    "> text }{} end",
    "Dilute in 200%ml water.",
    "Heat to @180C{}.",
    "Use a #slow cooker{5-6 quart}.",
    "---\ntitle: T\nauthor: 'null'\n---\n\nStep.\n",
    "---\ntitle: T\nprep_time: 10 min\n---\n\nStep.\n",
    "---\ntitle: T\nsource: a\nsource_url: b\n---\n\nStep.\n",
    "---\ntitle: Cr&egrave;me\n---\n\nStep.\n",
    "---\ntitle: Foo&#58; bar\n---\n\nStep.\n",
    "---\ntitle: &#39;Tis the season\n---\n\nStep.\n",
    "---\ntitle: &amp;amp;amp;amp;amp;amp;amp;amp;amp;amp;amp;amp;lt;\n---\n\nStep.\n",
    "---\ntitle: Salt&pepper;\n---\n\nStep.\n",
    "---\ntags: [a&#44; b, c]\n---\n\nStep.\n",
];

/// All the snippets in one pool for mutation, tagged so we can tell a
/// frontmatter-shaped one from a bare body one (concatenating two documents
/// that each start their own `---` frontmatter isn't a meaningful mutation).
fn pool() -> Vec<&'static str> {
    GOLDEN.iter().chain(DEFECTS.iter()).copied().collect()
}

fn has_frontmatter(s: &str) -> bool {
    s.trim_start_matches('\u{FEFF}').starts_with("---")
}

/// Builds one mutated document from the pool: a single snippet, two bare
/// bodies concatenated, a bare body duplicated, or a bare body appended
/// after a frontmatter-shaped document's body (never two frontmatters
/// concatenated — that's not a case autofix promises anything about, since
/// the result has two `---` fences and isn't really "a Cooklang document"
/// per the format's own rules).
fn mutate(rng: &mut Rng, pool: &[&'static str]) -> String {
    match rng.range(4) {
        0 => pool[rng.range(pool.len())].to_string(),
        1 => {
            let bodies: Vec<&&str> = pool.iter().filter(|s| !has_frontmatter(s)).collect();
            let a = bodies[rng.range(bodies.len())];
            let b = bodies[rng.range(bodies.len())];
            format!("{a}\n{b}")
        }
        2 => {
            let bodies: Vec<&&str> = pool.iter().filter(|s| !has_frontmatter(s)).collect();
            let a = bodies[rng.range(bodies.len())];
            format!("{a}\n{a}")
        }
        _ => {
            let fm_docs: Vec<&&str> = pool.iter().filter(|s| has_frontmatter(s)).collect();
            let bodies: Vec<&&str> = pool.iter().filter(|s| !has_frontmatter(s)).collect();
            let doc = fm_docs[rng.range(fm_docs.len())];
            let extra = bodies[rng.range(bodies.len())];
            format!("{doc}\n{extra}")
        }
    }
}

fn digits(s: &str) -> Vec<char> {
    let mut v: Vec<char> = s.chars().filter(|c| c.is_ascii_digit()).collect();
    v.sort_unstable();
    v
}

fn ingredient_names(doc: &str) -> Option<Vec<String>> {
    let (recipe, _) = cooklang::parse(doc).into_tuple();
    let recipe = recipe?;
    let mut names: Vec<String> = recipe.ingredients.iter().map(|i| i.name.to_lowercase()).collect();
    names.sort();
    Some(names)
}

/// Checks all 6 invariants for one mutated document. Panics (via assert)
/// describing exactly which invariant failed and on what input, so a
/// regression here points straight at the offending case.
fn check_case(doc: &str) {
    let before = lint("", doc);
    let fixed = autofix(doc);

    // (6) a no-op when nothing is flagged.
    if !before.iter().any(|i| i.autofixable) {
        assert_eq!(fixed, doc, "autofix(x) != x with no autofixable issue: {doc:?}");
    }

    let after = lint("", &fixed);

    // (1) no autofixable issue remains.
    let still_fixable: Vec<_> = after.iter().filter(|i| i.autofixable).collect();
    assert!(still_fixable.is_empty(), "fixable issues remain after autofix on {doc:?}: {still_fixable:?}");

    // (2) idempotent.
    let twice = autofix(&fixed);
    assert_eq!(twice, fixed, "autofix isn't idempotent on {doc:?}\n  1st: {fixed:?}\n  2nd: {twice:?}");

    // (3) no new issue kinds — in particular never a new InvalidFrontmatter
    // or ParseError, which would mean the fix broke something it shouldn't
    // have touched.
    let before_kinds: std::collections::HashSet<Kind> = before.iter().map(|i| i.kind).collect();
    let after_kinds: std::collections::HashSet<Kind> = after.iter().map(|i| i.kind).collect();
    for kind in after_kinds.difference(&before_kinds) {
        assert!(
            !matches!(kind, Kind::InvalidFrontmatter | Kind::ParseError),
            "autofix introduced a new {kind:?} on {doc:?}\n  fixed: {fixed:?}"
        );
    }
    let new_kinds: Vec<&Kind> = after_kinds.difference(&before_kinds).collect();
    assert!(new_kinds.is_empty(), "autofix introduced new issue kind(s) {new_kinds:?} on {doc:?}\n  fixed: {fixed:?}");

    // (4) + (5): only meaningful when both the original and the fixed text
    // actually parse into a recipe — a mutation that concatenates
    // unrelated snippets isn't guaranteed to parse at all, and there's
    // nothing to compare when it doesn't.
    //
    // (4) compares the ingredient *count*, not the exact name set: the
    // OptionalColon fix (`@?::salt` -> `@?salt`) legitimately changes what
    // cooklang parses as the name (an unfixed "@?:" isn't recognized
    // syntax, so cooklang reads the stray leading colon as literally part
    // of the name — ":salt", not "salt"). That's correcting a malformed
    // name, not adding or dropping an ingredient, which is the actual rule.
    //
    // Skipped entirely when a TokenInNote fix applies: turning `> ...
    // @cheese{50%g}(grated) ...` into plain note text is the whole point of
    // that fix (Cooklang notes may not contain syntax at all) and
    // *correctly* removes "cheese" from the parsed ingredient list — the
    // text and its quantity survive as prose (still covered by the digit
    // check below), they just stop being a machine-parseable ingredient.
    let note_token_fixed = before.iter().any(|i| i.kind == Kind::TokenInNote && i.autofixable);
    if !note_token_fixed {
        if let (Some(before_names), Some(after_names)) = (ingredient_names(doc), ingredient_names(&fixed)) {
            assert_eq!(
                before_names.len(),
                after_names.len(),
                "ingredient count changed on {doc:?}\n  fixed: {fixed:?}\n  before: {before_names:?}\n  after: {after_names:?}"
            );
        }
    }
    // Scoped to the body (ingredient/cookware/timer quantities and step
    // text), not the frontmatter: a metadata value's HTML-entity fix can
    // legitimately drop digits that were only ever part of the entity's own
    // numeric reference syntax (`&#39;` -> "'"), never real recipe content —
    // see the CRITICAL entity tests in `test_lint.rs`. The body is never
    // entity-decoded, so its digits must match with zero exceptions.
    let (_, body_before) = split_frontmatter(doc);
    let (_, body_after) = split_frontmatter(&fixed);
    assert_eq!(
        digits(body_before),
        digits(body_after),
        "digit multiset changed in the body on {doc:?}\n  fixed: {fixed:?}"
    );
}

#[test]
fn golden_and_defect_snippets_pass_all_invariants_unmutated() {
    for doc in pool() {
        check_case(doc);
    }
}

#[test]
fn mutated_documents_pass_all_invariants() {
    let pool = pool();
    let mut rng = Rng::new(0xC00C_1A46_u64);
    for _ in 0..3000 {
        let doc = mutate(&mut rng, &pool);
        check_case(&doc);
    }
}

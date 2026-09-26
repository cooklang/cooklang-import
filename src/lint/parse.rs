use super::{Issue, Kind, Severity};
use cooklang::{Converter, CooklangParser, Extensions, Recipe};
use std::sync::LazyLock;

/// The lint parser: every cooklang extension off except component modifiers
/// (`@?optional`, `@-hidden`, ...) and range values (`{2-3%cloves}`).
/// Notably `ADVANCED_UNITS` is off — it only knows English time units, so
/// `~{10%Minuten}` would be a hard "Unknown timer unit" error even though
/// timer units rightly stay in the recipe's language.
static PARSER: LazyLock<CooklangParser> = LazyLock::new(|| {
    CooklangParser::new(
        Extensions::COMPONENT_MODIFIERS | Extensions::RANGE_VALUES,
        Converter::bundled(),
    )
});

/// Warnings that come from scraper metadata or are cosmetic — out of scope
/// for the converter. Frontmatter validity is checked in `metadata`.
const IGNORED: &[&str] = &[
    // e.g. "Unsupported value for key: 'servings'" — a metadata value shaped
    // in a way cooklang's typed fields can't accept (arrays, nested maps).
    "Unsupported value for key",
    // "Time overriden" — an explicit `time required` conflicts with
    // `prep time`/`cook time` and cooklang keeps one of them.
    "Time overriden",
    // "Unnecessary scaling lock modifier" — a `%` lock on a quantity that
    // didn't need one; cosmetic.
    "Unnecessary scaling lock",
    // Cooklang's own frontmatter diagnostics: "Invalid YAML frontmatter
    // syntax: ...", "The frontmatter will be ignored...", and the deprecated
    // `>>` metadata warning. Frontmatter validity is checked by `metadata`,
    // not here.
    "frontmatter",
];

/// Parses `doc` and returns whatever recipe cooklang could build (even when
/// there were errors — cooklang is a best-effort parser) alongside the
/// diagnostics. Callers should still run consistency checks against the
/// recipe whenever it's `Some`, regardless of parse errors.
pub(super) fn parse(doc: &str) -> (Option<Recipe>, Vec<Issue>) {
    let (recipe, report) = PARSER.parse(doc).into_tuple();
    let mut issues: Vec<Issue> = report
        .errors()
        .map(|d| Issue::new(Kind::ParseError, Severity::Error, d.to_string(), false))
        .collect();
    issues.extend(
        report
            .warnings()
            .map(|d| d.to_string())
            .filter(|m| !IGNORED.iter().any(|s| m.contains(s)))
            .map(|m| Issue::new(Kind::ParseWarning, Severity::Warning, m, false)),
    );
    (recipe, issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_ingredient_still_parses_as_optional() {
        let (recipe, issues) = parse("Add @?x{1}.\n");
        assert!(issues.is_empty(), "{issues:?}");
        let recipe = recipe.expect("recipe");
        assert_eq!(recipe.ingredients[0].name, "x");
        assert!(recipe.ingredients[0].modifiers().is_optional());
    }

    #[test]
    fn range_quantity_is_a_numeric_range() {
        let (recipe, issues) = parse("Fry @garlic{2-3%cloves}.\n");
        assert!(issues.is_empty(), "{issues:?}");
        let recipe = recipe.expect("recipe");
        let q = recipe.ingredients[0].quantity.as_ref().expect("quantity");
        assert!(matches!(q.value(), cooklang::Value::Range { .. }), "{:?}", q.value());
        assert_eq!(q.unit(), Some("cloves"));
    }

    #[test]
    fn non_english_timer_units_parse_cleanly() {
        for body in ["~{10%Minuten}", "~{5%минут}", "~{2%heures}"] {
            let (recipe, issues) = parse(&format!("Cook {body}.\n"));
            assert!(issues.is_empty(), "{body}: {issues:?}");
            assert_eq!(recipe.expect("recipe").timers.len(), 1);
        }
    }
}

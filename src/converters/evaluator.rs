/// The prompt template used for evaluating the quality of a Cooklang conversion.
///
/// It instructs an LLM to compare a Cooklang conversion against the original
/// recipe text across ingredient coverage, quantity fidelity, step fidelity,
/// syntax, metadata, language, and token hygiene, and lists the failure modes
/// observed in production conversions. The model returns a JSON object with a
/// `good`/`bad` verdict, a 1-5 score, concrete issues, and a summary note —
/// matching the review taxonomy used by the cook.md admin quality page.
///
/// The prompt is loaded from `eval_prompt.txt` at compile time and contains
/// `{{ORIGINAL}}` and `{{COOKLANG}}` placeholders to be filled with
/// [`inject_evaluation`].
pub const COOKLANG_EVALUATOR_PROMPT: &str = include_str!("eval_prompt.txt");

/// Injects the original recipe text and its Cooklang conversion into the
/// evaluation prompt template.
pub fn inject_evaluation(original_text: &str, cooklang: &str) -> String {
    COOKLANG_EVALUATOR_PROMPT
        .replace("{{ORIGINAL}}", original_text)
        .replace("{{COOKLANG}}", cooklang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_is_embedded() {
        assert!(!COOKLANG_EVALUATOR_PROMPT.is_empty());
        assert!(COOKLANG_EVALUATOR_PROMPT.contains("INGREDIENT COVERAGE"));
        assert!(COOKLANG_EVALUATOR_PROMPT.contains("COMMON ISSUES"));
        assert!(COOKLANG_EVALUATOR_PROMPT.contains("verdict"));
    }

    #[test]
    fn test_inject_evaluation() {
        let filled = inject_evaluation("2 eggs\n\nBoil them.", "Boil @eggs{2}.");
        assert!(filled.contains("2 eggs\n\nBoil them."));
        assert!(filled.contains("Boil @eggs{2}."));
        assert!(!filled.contains("{{ORIGINAL}}"));
        assert!(!filled.contains("{{COOKLANG}}"));
    }
}

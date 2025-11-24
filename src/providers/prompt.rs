/// The system prompt used for converting recipes to Cooklang format.
///
/// This prompt instructs the AI model on how to properly format recipes
/// using Cooklang's markup syntax for ingredients, cookware, and timers.
///
/// The prompt is loaded from `prompt.txt` at compile time using the
/// `include_str!` macro, making it easy to edit without dealing with
/// Rust string syntax.
pub const COOKLANG_CONVERTER_PROMPT: &str = include_str!("prompt.txt");

/// Build the system prompt, optionally annotating the recipe language.
pub fn build_converter_prompt(language: Option<&str>) -> String {
    match language.and_then(|lang| {
        let trimmed = lang.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        Some(lang) => format!(
            "{}\n\nThe recipe text is written in {lang}. Parse the ingredients, cookware, and instructions in {lang} and keep the wording in that language when producing Cooklang output.",
            COOKLANG_CONVERTER_PROMPT
        ),
        None => COOKLANG_CONVERTER_PROMPT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_is_embedded() {
        // Verify the prompt is not empty
        assert!(!COOKLANG_CONVERTER_PROMPT.is_empty());

        // Verify it contains key Cooklang syntax elements
        assert!(COOKLANG_CONVERTER_PROMPT.contains("Cooklang"));
        assert!(COOKLANG_CONVERTER_PROMPT.contains("@ symbol"));
        assert!(COOKLANG_CONVERTER_PROMPT.contains("# symbol"));
        assert!(COOKLANG_CONVERTER_PROMPT.contains("timer"));
    }

    #[test]
    fn test_prompt_contains_examples() {
        // Verify the prompt includes examples
        assert!(COOKLANG_CONVERTER_PROMPT.contains("Example:"));
        assert!(COOKLANG_CONVERTER_PROMPT.contains("@salt"));
        assert!(COOKLANG_CONVERTER_PROMPT.contains("@potato{2}"));
        assert!(COOKLANG_CONVERTER_PROMPT.contains("#pot"));
    }

    #[test]
    fn test_build_converter_prompt_handles_language() {
        let with_language = build_converter_prompt(Some("italian"));
        assert!(with_language.contains("italian"));

        let trimmed_none = build_converter_prompt(Some("   "));
        assert_eq!(trimmed_none, COOKLANG_CONVERTER_PROMPT);
    }
}

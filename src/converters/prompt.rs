use whatlang::detect;

/// The system prompt template used for converting recipes to Cooklang format.
///
/// This prompt instructs the AI model on how to properly format recipes
/// using Cooklang's markup syntax for ingredients, cookware, and timers.
///
/// The prompt is loaded from `prompt.txt` at compile time using the
/// `include_str!` macro, making it easy to edit without dealing with
/// Rust string syntax.
///
/// Contains `{{RECIPE}}` and `{{LANGUAGE}}` placeholders that should be replaced
/// with the actual recipe content and detected language using the `inject_recipe` function.
pub const COOKLANG_CONVERTER_PROMPT: &str = include_str!("prompt.txt");

/// Detects the language of the given text and returns a human-readable language name.
fn detect_language(text: &str) -> String {
    detect(text)
        .map(|info| info.lang().eng_name().to_string())
        .unwrap_or_else(|| "the original language".to_string())
}

/// Injects the recipe content and detected language into the prompt template.
pub fn inject_recipe(recipe_content: &str) -> String {
    let language = detect_language(recipe_content);
    COOKLANG_CONVERTER_PROMPT
        .replace("{{RECIPE}}", recipe_content)
        .replace("{{LANGUAGE}}", &language)
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
}

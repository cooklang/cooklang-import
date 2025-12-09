/// The system prompt template used for converting recipes to Cooklang format.
///
/// This prompt instructs the AI model on how to properly format recipes
/// using Cooklang's markup syntax for ingredients, cookware, and timers.
///
/// The prompt is loaded from `prompt.txt` at compile time using the
/// `include_str!` macro, making it easy to edit without dealing with
/// Rust string syntax.
///
/// Contains a `{{RECIPE}}` placeholder that should be replaced with the actual
/// recipe content using the `inject_recipe` function.
pub const COOKLANG_CONVERTER_PROMPT: &str = include_str!("prompt.txt");

/// Injects the recipe content into the prompt template by replacing `{{RECIPE}}`.
pub fn inject_recipe(recipe_content: &str) -> String {
    COOKLANG_CONVERTER_PROMPT.replace("{{RECIPE}}", recipe_content)
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

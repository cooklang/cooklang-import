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

/// The user prompt fine-tuned converter models were trained with
/// (see recipe-pack's finetune crate).
pub const FINETUNED_CONVERTER_PREFIX: &str = "Convert recipe to Cooklang:\n\n";

/// Builds the converter prompt for the given model.
///
/// Fine-tuned models (`ft:` prefix) get the short prompt they were trained
/// with — the instruction set is baked into their weights, and sending the
/// full rulebook both mismatches their training distribution and costs
/// ~1.4k extra tokens per call. Base models get the full instruction prompt.
pub fn prompt_for_model(model: &str, recipe_content: &str) -> String {
    if model.starts_with("ft:") {
        format!("{}{}", FINETUNED_CONVERTER_PREFIX, recipe_content)
    } else {
        inject_recipe(recipe_content)
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
    fn test_prompt_for_model_finetuned_uses_short_prompt() {
        let p = prompt_for_model(
            "ft:gpt-4.1-mini-2025-04-14:personal::abc",
            "2 eggs\n\nBoil.",
        );
        assert_eq!(p, "Convert recipe to Cooklang:\n\n2 eggs\n\nBoil.");
    }

    #[test]
    fn test_prompt_for_model_base_uses_full_prompt() {
        let p = prompt_for_model("gpt-4.1-mini", "2 eggs\n\nBoil.");
        assert!(p.contains("Cooklang syntax rules"));
        assert!(p.contains("2 eggs\n\nBoil."));
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

//! Recipe title normalization.
//!
//! Scraped titles are often unusable as-is: image-scanned recipes have no
//! title at all, and web titles carry SEO padding and site suffixes long
//! enough to break slug/file naming downstream. `ensure_title` guarantees a
//! concise single-line title: mechanical cleanup first, then an LLM call
//! (only when the title is missing or too long), with a mechanical fallback
//! when no API key is configured.

use super::RecipeComponents;
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::error::Error;

/// Longest acceptable title. Chosen to match SEO title conventions and keep
/// derived file names comfortably inside filesystem limits.
pub const MAX_TITLE_CHARS: usize = 60;

const MODEL: &str = "gpt-4o-mini";

const PROMPT: &str = r#"You name recipes. Given a recipe (and possibly its current title), reply with only this JSON: {"title": "<TITLE>"}

Rules:
- Maximum 60 characters, a single line of plain text: no surrounding quotes, no emoji, no website or author names, no taglines.
- Write the title in the same language as the recipe.
- If a current title is provided, keep its dish name and drop the padding.
- If there is no current title, derive a natural dish name from the main ingredients and method. Do not invent details the recipe does not support.
"#;

/// Ensure `components.name` is a concise, single-line title.
///
/// - Strips `| Site` style suffixes mechanically.
/// - Leaves good titles untouched (no LLM call).
/// - Generates or shortens via LLM when the title is empty or too long.
/// - Falls back to a word-boundary trim when the LLM is unavailable or fails.
pub async fn ensure_title(components: &mut RecipeComponents) {
    let name = strip_site_suffix(&super::sanitize_name(&components.name));

    if !name.is_empty() && name.chars().count() <= MAX_TITLE_CHARS {
        components.name = name;
        return;
    }

    if env::var("OPENAI_API_KEY").is_ok() {
        if let Ok(title) = generate_title(&name, &components.text).await {
            let title = strip_site_suffix(&super::sanitize_name(&title));
            // Guard against a runaway model answer; anything reasonable wins.
            if !title.is_empty() && title.chars().count() <= MAX_TITLE_CHARS + 20 {
                components.name = title;
                return;
            }
        }
    }

    components.name = trim_to_words(&name, MAX_TITLE_CHARS);
}

/// Drop trailing `| Site Name` segments, and ` - Site Name` style segments
/// when the title is overlong. Dish names rarely contain a pipe, so pipes are
/// always stripped; dash segments can be part of the dish name, so they are
/// only stripped when the full title exceeds the limit and enough remains.
pub fn strip_site_suffix(name: &str) -> String {
    let mut name = name.trim();

    if let Some(idx) = name.find(" | ") {
        name = name[..idx].trim_end();
    }

    if name.chars().count() > MAX_TITLE_CHARS {
        for sep in [" — ", " – ", " - "] {
            if let Some(idx) = name.rfind(sep) {
                let head = name[..idx].trim_end();
                if head.chars().count() >= 15 {
                    name = head;
                    break;
                }
            }
        }
    }

    name.to_string()
}

/// Trim to at most `max` characters without cutting a word in half.
fn trim_to_words(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        return name.to_string();
    }
    let mut out = String::new();
    for word in name.split_whitespace() {
        let candidate_len =
            out.chars().count() + word.chars().count() + usize::from(!out.is_empty());
        if candidate_len > max {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        // Single word longer than the limit — hard cut.
        out = name.chars().take(max).collect();
    }
    out
}

async fn generate_title(
    current: &str,
    recipe_text: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let api_key = env::var("OPENAI_API_KEY")?;

    // For testing environment, return mock data
    if api_key == "test_key" {
        return Ok("Simple Test Recipe".to_string());
    }

    let excerpt: String = recipe_text.chars().take(2000).collect();
    let current_line = if current.is_empty() {
        "Current title: none".to_string()
    } else {
        format!("Current title: {}", current)
    };

    let response = Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "model": MODEL,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": PROMPT },
                { "role": "user", "content": format!("{current_line}\n\nRecipe:\n{excerpt}") }
            ]
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("no content in title response")?;
    let parsed: Value = serde_json::from_str(content)?;
    let title = parsed["title"].as_str().ok_or("no title field")?;
    Ok(title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_pipe_suffix() {
        assert_eq!(
            strip_site_suffix("Black Pepper Mushroom | Veggie Anh"),
            "Black Pepper Mushroom"
        );
    }

    #[test]
    fn keeps_dash_segment_in_short_titles() {
        assert_eq!(
            strip_site_suffix("Sweet & Sour Pork - Slow Cooker"),
            "Sweet & Sour Pork - Slow Cooker"
        );
    }

    #[test]
    fn strips_dash_suffix_when_overlong() {
        let long = "The BEST Easy Chocolate Chip Cookies Ever (Soft and Chewy) - My Baking Blog";
        assert_eq!(
            strip_site_suffix(long),
            "The BEST Easy Chocolate Chip Cookies Ever (Soft and Chewy)"
        );
    }

    #[test]
    fn trims_on_word_boundaries() {
        let t = trim_to_words(
            "one two three four five six seven eight nine ten eleven twelve",
            20,
        );
        assert!(t.chars().count() <= 20);
        assert_eq!(t, "one two three four");
    }

    #[tokio::test]
    async fn good_title_is_untouched() {
        std::env::set_var("OPENAI_API_KEY", "test_key");
        let mut c = RecipeComponents {
            text: "2 eggs\n\nBoil them.".into(),
            metadata: String::new(),
            name: "Boiled Eggs".into(),
        };
        ensure_title(&mut c).await;
        assert_eq!(c.name, "Boiled Eggs");
    }

    #[tokio::test]
    async fn missing_title_is_generated() {
        std::env::set_var("OPENAI_API_KEY", "test_key");
        let mut c = RecipeComponents {
            text: "2 eggs\n\nBoil them.".into(),
            metadata: String::new(),
            name: String::new(),
        };
        ensure_title(&mut c).await;
        assert_eq!(c.name, "Simple Test Recipe");
    }

    #[tokio::test]
    async fn overlong_title_is_shortened() {
        std::env::set_var("OPENAI_API_KEY", "test_key");
        let mut c = RecipeComponents {
            text: "2 eggs\n\nBoil them.".into(),
            metadata: String::new(),
            name: "How to Make the Most Incredible Perfectly Boiled Farm Fresh Eggs Every Single Time Without Fail".into(),
        };
        ensure_title(&mut c).await;
        assert_eq!(c.name, "Simple Test Recipe");
    }
}

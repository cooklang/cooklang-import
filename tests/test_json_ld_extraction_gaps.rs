//! Three extraction gaps found by replaying the 2026-08-31..09-07 cookification
//! failures through Bright Data's unlocker: the pages were obtainable, but the
//! extractors rejected or truncated them.

use cooklang_import::url_to_recipe;
use std::env;

/// A page whose visible body is long enough to clear the blocked/empty-page guard,
/// so these tests exercise the extractor chain rather than the fetch guard.
fn page(json_ld: &str) -> String {
    let body = "Some introductory prose about this dish. ".repeat(20);
    format!(
        r#"<!DOCTYPE html>
        <html><head><title>Recipe Page</title>
        <script type="application/ld+json">{json_ld}</script>
        </head><body><h1>Recipe</h1><p>{body}</p></body></html>"#
    )
}

async fn serve(json_ld: &str) -> (mockito::ServerGuard, String) {
    let mut server = mockito::Server::new_async().await;
    let m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(page(json_ld))
        .create_async()
        .await;
    std::mem::forget(m);
    let url = format!("{}/recipe", server.url());
    (server, url)
}

/// cooking.nytimes.com, 2026-09-06 (twice).
///
/// schema.org allows a single-valued property to be written as the bare object
/// rather than a one-element array, and NYT writes `itemListElement` that way.
/// Typing it as a sequence made serde reject the *whole* recipe — 9 ingredients and
/// 3 instructions discarded — and the untagged `RecipeInstructions` enum reported it
/// only as "did not match any variant".
#[tokio::test]
async fn test_how_to_section_accepts_a_single_item_object() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Creamy Cottage Cheese Basil Pasta",
        "recipeIngredient": ["Salt", "1 lemon", "8 ounces pasta"],
        "recipeInstructions": [
            {"@type": "HowToStep", "text": "Bring a large pot of salted water to a boil."},
            {"@type": "HowToSection",
             "itemListElement": {"@type": "HowToStep", "text": "Add the pasta and cook until al dente."}},
            {"@type": "HowToSection",
             "itemListElement": {"@type": "HowToStep", "text": "Toss the pasta with the sauce."}}
        ]
    }"#;
    let (_server, url) = serve(json_ld).await;

    let result = url_to_recipe(&url).await.unwrap();

    assert_eq!(result.name, "Creamy Cottage Cheese Basil Pasta");
    assert!(
        result.text.contains("8 ounces pasta"),
        "got: {}",
        result.text
    );
    assert!(
        result
            .text
            .contains("Add the pasta and cook until al dente"),
        "single-object itemListElement was dropped: {}",
        result.text
    );
    assert!(
        result.text.contains("Toss the pasta with the sauce"),
        "single-object itemListElement was dropped: {}",
        result.text
    );
}

/// thekitchn.com, 2026-09-05.
///
/// The document root is `@type: Recipe` but carries no ingredients or instructions —
/// an SEO stub — while the real recipe sits in `@graph`. Matching the root on type
/// alone selected the stub and never looked at `@graph`.
#[tokio::test]
async fn test_graph_recipe_wins_over_an_empty_root_recipe() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Tuscan Tomato Salad SEO Stub",
        "@graph": [
            {
                "@type": "Recipe",
                "name": "Tuscan Tomato and Chickpea Salad",
                "recipeIngredient": ["2 tomatoes", "1 tin chickpeas", "olive oil"],
                "recipeInstructions": [
                    {"@type": "HowToStep", "text": "Chop the tomatoes."},
                    {"@type": "HowToStep", "text": "Toss everything together."}
                ]
            }
        ]
    }"#;
    let (_server, url) = serve(json_ld).await;

    let result = url_to_recipe(&url).await.unwrap();

    assert_eq!(result.name, "Tuscan Tomato and Chickpea Salad");
    assert!(
        result.text.contains("1 tin chickpeas"),
        "got: {}",
        result.text
    );
    assert!(
        result.text.contains("Chop the tomatoes"),
        "got: {}",
        result.text
    );
}

/// papillesetpupilles.fr (four failures) and chefkoch.de, 2026-09-01..09-07.
///
/// These pages yield ingredients but no instructions: 8 and 0, 16 and 0. That is
/// non-empty text, so the pipeline returned it as a success and stopped — and the
/// converter, handed a list of ingredients with no method, answered "no recipe".
/// Thirteen of the 55 failures in that window carry that byte-identical message.
/// A hit with no instructions must keep the chain going instead.
#[tokio::test]
async fn test_ingredients_without_instructions_falls_through() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Piperade Basquaise",
        "recipeIngredient": ["4 poivrons", "6 tomates", "2 oignons"]
    }"#;
    let (_server, url) = serve(json_ld).await;

    let result = url_to_recipe(&url).await.unwrap();

    assert_ne!(
        result.name, "Piperade Basquaise",
        "an instruction-less hit must not short-circuit the pipeline"
    );
    assert!(!result.text.trim().is_empty());
}

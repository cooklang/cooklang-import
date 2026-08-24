use cooklang_import::url_to_recipe;
use std::env;

/// A page whose visible body is long enough to look like real content, so tests
/// exercise the extractor chain rather than the blocked/empty-page guard.
fn create_recipe_html_with_body(json_ld: &str) -> String {
    let body = "Some introductory prose about this dish. ".repeat(20);
    format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Recipe Page</title>
            <script type="application/ld+json">
                {json_ld}
            </script>
        </head>
        <body>
            <h1>Recipe</h1>
            <p>{body}</p>
        </body>
        </html>
        "#
    )
}

fn create_recipe_html(json_ld: &str) -> String {
    format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Recipe Page</title>
            <script type="application/ld+json">
                {json_ld}
            </script>
        </head>
        <body>
            <h1>Recipe</h1>
        </body>
        </html>
        "#
    )
}

#[tokio::test]
async fn test_recipe_without_instructions() {
    // Test case where recipe has no recipeInstructions field
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Dishoom's House Black Daal",
        "author": {
            "@type": "Organization",
            "name": "HotCooking"
        },
        "cookTime": "PT5H",
        "prepTime": "PT15M",
        "totalTime": "PT5H30M",
        "description": "A daal like no other. This isn't a quick recipe but if you can spare the time you won't be disappointed.",
        "image": [
            "https://assets.hotcooking.co.uk/landscape/dishoom_house_black_dal_large.jpg",
            "https://assets.hotcooking.co.uk/landscape/dishoom_garam_masala_large.jpg"
        ],
        "recipeIngredient": [
            "300g whole black urad daal",
            "12g garlic paste (roughly 4 cloves)",
            "10g ginger paste (roughly 1 heaped tablespoon)",
            "70g tomato purée",
            "8g fine sea salt",
            "⅔ tsp deggi mirch chilli powder (or ⅓ tsp normal chilli powder)",
            "⅓ tsp garam masala",
            "90g unsalted butter",
            "90ml double cream"
        ],
        "recipeYield": 8
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully (ingredients only, no instructions)
    assert_eq!(result.name, "Dishoom's House Black Daal");

    // Verify ingredients were parsed
    assert!(result.text.contains("300g whole black urad daal"));
    assert!(result.text.contains("12g garlic paste"));
    assert!(result.text.contains("90ml double cream"));

    // Verify metadata
    assert!(result.metadata.contains("author: HotCooking"));
    assert!(result.metadata.contains("cook time: 5 hours"));
    assert!(result.metadata.contains("prep time: 15 minutes"));
    assert!(result
        .metadata
        .contains("time required: 5 hours 30 minutes"));
    assert!(result.metadata.contains("servings: 8"));
}

/// A JSON-LD Recipe carrying neither ingredients nor instructions is an SEO stub,
/// not an extraction.
///
/// This test previously asserted the opposite - that such a block yields a
/// successful result with `text == ""`. That expectation was the bug: in the week of
/// 2026-08-17 joshuaweissman.com, uitpaulineskeuken.nl, saborintenso.com,
/// larecette.net, moribyan.com and laboutiquedeschefs.com all published exactly this
/// shape, the pipeline short-circuited on it, and the user got
/// "No recipe found in the text". The stub must now fall through to the remaining
/// extractors and the LLM fallback instead.
#[tokio::test]
async fn test_stub_recipe_falls_through_to_llm_extraction() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Minimal Recipe",
        "author": "Test Chef",
        "description": "A very minimal recipe"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_body(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // The stub was skipped and the LLM fallback ran (mocked by the "test_key" path).
    assert_ne!(
        result.name, "Minimal Recipe",
        "the SEO stub must not be returned as the extraction result"
    );
    assert!(
        !result.text.trim().is_empty(),
        "fallback extraction should produce recipe text, got {:?}",
        result.text
    );
}

#[tokio::test]
async fn test_long_cook_time() {
    // Test that PT5H correctly converts to "5 hours"
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Slow Cooked Recipe",
        "recipeIngredient": ["test"],
        "prepTime": "PT15M",
        "cookTime": "PT5H",
        "totalTime": "PT5H15M"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    assert!(result.metadata.contains("prep time: 15 minutes"));
    assert!(result.metadata.contains("cook time: 5 hours"));
    assert!(result
        .metadata
        .contains("time required: 5 hours 15 minutes"));
}

/// `"recipeIngredient": []` with no instructions is the same stub case as above -
/// structurally present, substantively empty - and must not short-circuit the
/// pipeline either.
#[tokio::test]
async fn test_empty_ingredients_array_falls_through_to_llm_extraction() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Syltad ingefära",
        "author": {
            "@type": "Organization",
            "name": "Hemköp"
        },
        "description": "1 brk, ca 15 minuter, koktid ca 2,5 timme",
        "keywords": ["Asiatiskt", "Tillbehör", "Grönsaker", "Frukt"],
        "recipeIngredient": [],
        "recipeYield": 4,
        "totalTime": "PT150M"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_body(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // The stub was skipped and the LLM fallback ran (mocked by the "test_key" path).
    assert_ne!(
        result.name, "Syltad ingefära",
        "an empty recipeIngredient array must not be returned as the extraction result"
    );
    assert!(
        !result.text.trim().is_empty(),
        "fallback extraction should produce recipe text, got {:?}",
        result.text
    );
}

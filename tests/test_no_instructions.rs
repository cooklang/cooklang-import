use cooklang_import::fetch_recipe;
use std::env;

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
    let result = fetch_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully without instructions
    assert_eq!(result.name, "Dishoom's House Black Daal");
    assert_eq!(result.instructions, ""); // Should be empty string

    // Verify ingredients were parsed
    assert!(result.ingredients.contains("300g whole black urad daal"));
    assert!(result.ingredients.contains("12g garlic paste"));
    assert!(result.ingredients.contains("90ml double cream"));

    // Verify metadata
    assert_eq!(result.metadata.get("author").unwrap(), "HotCooking");
    assert_eq!(result.metadata.get("cook time").unwrap(), "5 hours");
    assert_eq!(result.metadata.get("prep time").unwrap(), "15 minutes");
    assert_eq!(
        result.metadata.get("time required").unwrap(),
        "5 hours 30 minutes"
    );
    assert_eq!(result.metadata.get("servings").unwrap(), "8");

    // Verify description
    assert_eq!(result.description.unwrap(), "A daal like no other. This isn't a quick recipe but if you can spare the time you won't be disappointed.");
}

#[tokio::test]
async fn test_recipe_with_neither_ingredients_nor_instructions() {
    // Test extreme case where recipe has neither ingredients nor instructions
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
        .with_body(create_recipe_html(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully
    assert_eq!(result.name, "Minimal Recipe");
    assert_eq!(result.instructions, "");
    assert_eq!(result.ingredients, "");
    assert_eq!(result.metadata.get("author").unwrap(), "Test Chef");
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
    let result = fetch_recipe(&url).await.unwrap();

    assert_eq!(result.metadata.get("prep time").unwrap(), "15 minutes");
    assert_eq!(result.metadata.get("cook time").unwrap(), "5 hours");
    assert_eq!(
        result.metadata.get("time required").unwrap(),
        "5 hours 15 minutes"
    );
}

#[tokio::test]
async fn test_recipe_with_empty_ingredients_array() {
    // Test case where recipeIngredient is an empty array
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
        .with_body(create_recipe_html(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully with empty ingredients
    assert_eq!(result.name, "Syltad ingefära");
    assert_eq!(result.ingredients, ""); // Should be empty string
    assert_eq!(result.instructions, ""); // Should be empty string

    // Verify metadata
    assert_eq!(result.metadata.get("author").unwrap(), "Hemköp");
    assert_eq!(
        result.metadata.get("time required").unwrap(),
        "2 hours 30 minutes"
    );
    assert_eq!(result.metadata.get("servings").unwrap(), "4");
    assert_eq!(
        result.metadata.get("tags").unwrap(),
        "Asiatiskt, Tillbehör, Grönsaker, Frukt"
    );
}

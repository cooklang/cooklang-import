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
                {}
            </script>
        </head>
        <body>
            <h1>Recipe</h1>
        </body>
        </html>
        "#,
        json_ld
    )
}

#[tokio::test]
async fn test_lowercase_recipe_type() {
    // Test case where @type is "recipe" instead of "Recipe"
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "recipe",
        "name": "Easy Black Bean Soup",
        "image": "https://example.com/blackbean.jpg",
        "description": "This black bean soup recipe is easy to make and full of flavor.",
        "keywords": ["black bean soup", "vegetarian", "easy"],
        "author": {
            "@type": "Person",
            "name": "Chef Maria"
        },
        "prepTime": "PT10M",
        "cookTime": "PT30M",
        "totalTime": "PT40M",
        "recipeYield": "6",
        "recipeCategory": "Soup",
        "recipeCuisine": "Mexican",
        "recipeIngredient": [
            "2 cans black beans",
            "1 onion, diced",
            "2 cloves garlic, minced",
            "1 tsp cumin",
            "4 cups vegetable broth",
            "Salt and pepper to taste"
        ],
        "recipeInstructions": [
            "Sauté onion and garlic until soft.",
            "Add cumin and cook for 1 minute.",
            "Add beans and broth, simmer for 20 minutes.",
            "Season with salt and pepper."
        ]
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

    // Verify the recipe was parsed successfully despite lowercase @type
    assert_eq!(result.name, "Easy Black Bean Soup");
    assert_eq!(
        result.description,
        Some("This black bean soup recipe is easy to make and full of flavor.".to_string())
    );

    // Verify ingredients
    assert!(result.ingredients.contains("2 cans black beans"));
    assert!(result.ingredients.contains("1 onion, diced"));

    // Verify instructions
    assert!(result.instructions.contains("Sauté onion and garlic"));
    assert!(result.instructions.contains("simmer for 20 minutes"));

    // Verify metadata
    assert_eq!(result.metadata.get("author").unwrap(), "Chef Maria");
    assert_eq!(result.metadata.get("prep time").unwrap(), "10 minutes");
    assert_eq!(result.metadata.get("cook time").unwrap(), "30 minutes");
    assert_eq!(result.metadata.get("time required").unwrap(), "40 minutes");
    assert_eq!(result.metadata.get("servings").unwrap(), "6");
    assert_eq!(result.metadata.get("course").unwrap(), "Soup");
    assert_eq!(result.metadata.get("cuisine").unwrap(), "Mexican");
    assert_eq!(
        result.metadata.get("tags").unwrap(),
        "black bean soup, vegetarian, easy"
    );
}

#[tokio::test]
async fn test_mixed_case_recipe_type() {
    // Test case where @type is "RECIPE" (all caps)
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "RECIPE",
        "name": "Quick Pasta",
        "recipeIngredient": ["pasta", "sauce"],
        "recipeInstructions": "Cook pasta, add sauce."
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

    // Verify the recipe was parsed successfully despite uppercase @type
    assert_eq!(result.name, "Quick Pasta");
    assert!(result.ingredients.contains("pasta"));
    assert!(result.instructions.contains("Cook pasta"));
}

#[tokio::test]
async fn test_graph_with_lowercase_type() {
    // Test case where recipe is in @graph with lowercase type
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@graph": [
            {
                "@type": "WebSite",
                "name": "Recipe Website"
            },
            {
                "@type": "recipe",
                "name": "Grilled Cheese",
                "recipeIngredient": ["bread", "cheese", "butter"],
                "recipeInstructions": "Butter bread, add cheese, grill until golden."
            }
        ]
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

    // Verify the recipe was parsed from @graph despite lowercase type
    assert_eq!(result.name, "Grilled Cheese");
    assert!(result.ingredients.contains("cheese"));
    assert!(result.instructions.contains("grill until golden"));
}

#[tokio::test]
async fn test_array_with_mixed_case_types() {
    // Test case where recipe is in array with various case types
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    [
        {
            "@type": "WEBSITE",
            "name": "Site Name"
        },
        {
            "@type": "ReCiPe",
            "name": "Mixed Case Recipe",
            "recipeIngredient": ["ingredient"],
            "recipeInstructions": "Instructions here"
        }
    ]
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    // Verify the recipe was parsed from array despite mixed case type
    assert_eq!(result.name, "Mixed Case Recipe");
    assert_eq!(result.ingredients, "ingredient");
    assert_eq!(result.instructions, "Instructions here");
}

use cooklang_import::fetch_recipe;
use std::env;

fn create_recipe_html_with_metadata(json_ld: &str) -> String {
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
async fn test_comprehensive_metadata_extraction() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Ultimate Chocolate Cake",
        "description": "The best chocolate cake recipe",
        "image": "https://example.com/cake.jpg",
        "author": {
            "@type": "Person",
            "name": "Jane Baker"
        },
        "datePublished": "2024-01-15",
        "prepTime": "PT30M",
        "cookTime": "PT45M",
        "totalTime": "PT1H15M",
        "recipeYield": "12 servings",
        "recipeCategory": "Dessert",
        "recipeCuisine": "French",
        "keywords": ["chocolate", "cake", "dessert", "baking"],
        "suitableForDiet": ["GlutenFree", "Vegetarian"],
        "recipeIngredient": [
            "2 cups flour",
            "1 cup sugar",
            "1/2 cup cocoa powder"
        ],
        "recipeInstructions": "Mix ingredients and bake."
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    // Test all metadata fields (no duplicates)
    assert_eq!(result.metadata.get("source").unwrap(), &url);
    assert_eq!(result.metadata.get("author").unwrap(), "Jane Baker");
    assert_eq!(result.metadata.get("prep time").unwrap(), "30 minutes");
    assert_eq!(result.metadata.get("cook time").unwrap(), "45 minutes");
    assert_eq!(
        result.metadata.get("time required").unwrap(),
        "1 hour 15 minutes"
    );
    assert_eq!(result.metadata.get("servings").unwrap(), "12 servings");
    assert_eq!(result.metadata.get("course").unwrap(), "Dessert");
    assert_eq!(result.metadata.get("cuisine").unwrap(), "French");
    assert_eq!(
        result.metadata.get("diet").unwrap(),
        "GlutenFree, Vegetarian"
    );
    assert_eq!(
        result.metadata.get("tags").unwrap(),
        "chocolate, cake, dessert, baking"
    );

    // Check that duplicate keys are NOT present
    assert!(result.metadata.get("source.url").is_none());
    assert!(result.metadata.get("source.author").is_none());
    assert!(result.metadata.get("time.prep").is_none());
    assert!(result.metadata.get("time.cook").is_none());
    assert!(result.metadata.get("time").is_none());
    assert!(result.metadata.get("duration").is_none());
    assert!(result.metadata.get("serves").is_none());
    assert!(result.metadata.get("yield").is_none());
    assert!(result.metadata.get("category").is_none());
}

#[tokio::test]
async fn test_metadata_with_numeric_yield() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Simple Pasta",
        "description": "Quick pasta recipe",
        "image": "https://example.com/pasta.jpg",
        "recipeIngredient": ["pasta", "sauce"],
        "recipeInstructions": "Cook and serve",
        "recipeYield": 4
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    assert_eq!(result.metadata.get("servings").unwrap(), "4");
}

#[tokio::test]
async fn test_metadata_with_string_author() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Quick Salad",
        "description": "Healthy salad",
        "image": "https://example.com/salad.jpg",
        "author": "John Chef",
        "recipeIngredient": ["lettuce", "tomatoes"],
        "recipeInstructions": "Mix and serve"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    assert_eq!(result.metadata.get("author").unwrap(), "John Chef");
}

#[tokio::test]
async fn test_metadata_partial_fields() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Basic Recipe",
        "description": "Simple recipe",
        "image": "https://example.com/basic.jpg",
        "recipeIngredient": ["ingredient"],
        "recipeInstructions": "Make it",
        "prepTime": "PT10M",
        "recipeCuisine": "American",
        "keywords": "simple, easy"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = fetch_recipe(&url).await.unwrap();

    // Check present fields
    assert_eq!(result.metadata.get("prep time").unwrap(), "10 minutes");
    assert_eq!(result.metadata.get("cuisine").unwrap(), "American");
    assert_eq!(result.metadata.get("tags").unwrap(), "simple, easy");

    // Check absent fields
    assert!(result.metadata.get("author").is_none());
    assert!(result.metadata.get("cook time").is_none());
    assert!(result.metadata.get("servings").is_none());
}

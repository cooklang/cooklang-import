use cooklang_import::url_to_recipe;
use std::env;

fn create_recipe_html_with_metadata(json_ld: &str) -> String {
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
    let result = url_to_recipe(&url).await.unwrap();

    // Test name
    assert_eq!(result.name, "Ultimate Chocolate Cake");

    // Test metadata fields (check they exist in metadata string)
    assert!(result.metadata.contains("author: Jane Baker"));
    assert!(result.metadata.contains("prep time: 30 minutes"));
    assert!(result.metadata.contains("cook time: 45 minutes"));
    assert!(result.metadata.contains("time required: 1 hour 15 minutes"));
    assert!(result.metadata.contains("servings: 12 servings"));
    assert!(result.metadata.contains("course: Dessert"));
    assert!(result.metadata.contains("cuisine: French"));
    assert!(result.metadata.contains("diet: GlutenFree, Vegetarian"));
    assert!(result
        .metadata
        .contains("tags: chocolate, cake, dessert, baking"));
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
    let result = url_to_recipe(&url).await.unwrap();

    assert!(result.metadata.contains("servings: 4"));
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
    let result = url_to_recipe(&url).await.unwrap();

    assert!(result.metadata.contains("author: John Chef"));
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
    let result = url_to_recipe(&url).await.unwrap();

    // Check present fields
    assert!(result.metadata.contains("prep time: 10 minutes"));
    assert!(result.metadata.contains("cuisine: American"));
    assert!(result.metadata.contains("tags: simple, easy"));

    // Check absent fields
    assert!(!result.metadata.contains("author:"));
    assert!(!result.metadata.contains("cook time:"));
    assert!(!result.metadata.contains("servings:"));
}

#[tokio::test]
async fn test_nutrition_extraction() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Creamy Chicken",
        "description": "A delicious chicken dish",
        "image": "https://example.com/chicken.jpg",
        "recipeIngredient": ["chicken", "cream"],
        "recipeInstructions": "Cook the chicken with cream",
        "nutrition": {
            "@type": "NutritionInformation",
            "calories": "732 kcal",
            "fatContent": "24.1 g",
            "saturatedFatContent": "11.8 g",
            "carbohydrateContent": "84.3 g",
            "sugarContent": "9.3 g",
            "proteinContent": "46.3 g",
            "fiberContent": "0.1 g",
            "sodiumContent": "1.4 g",
            "servingSize": "451"
        }
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // Test nutrition fields nested under nutrition key
    assert!(result.metadata.contains("nutrition:"));
    assert!(result.metadata.contains("  calories: 732 kcal"));
    assert!(result.metadata.contains("  fat: 24.1 g"));
    assert!(result.metadata.contains("  saturated fat: 11.8 g"));
    assert!(result.metadata.contains("  carbohydrates: 84.3 g"));
    assert!(result.metadata.contains("  sugar: 9.3 g"));
    assert!(result.metadata.contains("  protein: 46.3 g"));
    assert!(result.metadata.contains("  fiber: 0.1 g"));
    assert!(result.metadata.contains("  sodium: 1.4 g"));
    assert!(result.metadata.contains("  serving size: 451"));
}

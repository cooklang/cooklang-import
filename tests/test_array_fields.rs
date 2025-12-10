use cooklang_import::url_to_recipe;
use std::env;

fn create_recipe_html_with_arrays(json_ld: &str) -> String {
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
async fn test_recipe_with_array_fields_and_empty_strings() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "author": {
            "@type": "Person",
            "name": "amateurprochef"
        },
        "cookTime": "PT30M",
        "datePublished": "2024-09-07T12:17:45-04:00",
        "description": "",
        "image": [
            "https://example.com/image1.jpg",
            "https://example.com/image2.jpg",
            "https://example.com/image3.jpg"
        ],
        "keywords": "",
        "name": "Shahi Paneer",
        "prepTime": "",
        "recipeCategory": [
            "All",
            "All Things Indian"
        ],
        "recipeCuisine": [],
        "recipeIngredient": [
            "300g paneer",
            "4 roma tomatoes",
            "2 red onion"
        ],
        "recipeInstructions": [
            {
                "@type": "HowToStep",
                "text": "Step 1: Chop vegetables"
            },
            {
                "@type": "HowToStep",
                "text": "Step 2: Cook the dish"
            }
        ],
        "recipeYield": "",
        "totalTime": ""
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_arrays(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully
    assert_eq!(result.name, "Shahi Paneer");

    // Check that array fields were handled correctly
    assert!(result.metadata.contains("course: All, All Things Indian"));

    // Check that empty arrays don't create entries
    assert!(!result.metadata.contains("cuisine:"));

    // Check that empty strings don't create entries
    assert!(!result.metadata.contains("tags:"));
    assert!(!result.metadata.contains("prep time:"));
    assert!(!result.metadata.contains("time required:"));
    assert!(!result.metadata.contains("servings:"));

    // Check that non-empty time fields work
    assert!(result.metadata.contains("cook time: 30 minutes"));

    // Check that author was parsed correctly
    assert!(result.metadata.contains("author: amateurprochef"));

    // Check that ingredients were parsed (in text field)
    assert!(result.text.contains("300g paneer"));
    assert!(result.text.contains("4 roma tomatoes"));

    // Check that instructions were parsed (in text field)
    assert!(result.text.contains("Step 1: Chop vegetables"));
    assert!(result.text.contains("Step 2: Cook the dish"));
}

#[tokio::test]
async fn test_recipe_with_single_string_cuisine() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Italian Pasta",
        "description": "Classic pasta dish",
        "image": "https://example.com/pasta.jpg",
        "recipeIngredient": ["pasta", "sauce"],
        "recipeInstructions": "Cook and serve",
        "recipeCuisine": "Italian",
        "recipeCategory": "Main Course"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_arrays(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // Check that single string values still work
    assert!(result.metadata.contains("cuisine: Italian"));
    assert!(result.metadata.contains("course: Main Course"));
}

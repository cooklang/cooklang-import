use cooklang_import::fetch_recipe;
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
    let result = fetch_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully
    assert_eq!(result.name, "Shahi Paneer");

    // Check that array fields were handled correctly
    assert_eq!(
        result.metadata.get("course").unwrap(),
        "All, All Things Indian"
    );

    // Check that empty arrays don't create entries
    assert!(!result.metadata.contains_key("cuisine"));

    // Check that empty strings don't create entries
    assert!(!result.metadata.contains_key("tags"));
    assert!(!result.metadata.contains_key("prep time"));
    assert!(!result.metadata.contains_key("time required"));
    assert!(!result.metadata.contains_key("servings"));

    // Check that non-empty time fields work
    assert_eq!(result.metadata.get("cook time").unwrap(), "30 minutes");

    // Check that author was parsed correctly
    assert_eq!(result.metadata.get("author").unwrap(), "amateurprochef");

    // Check that ingredients were parsed
    assert!(result.content.contains("300g paneer"));
    assert!(result.content.contains("4 roma tomatoes"));

    // Check that instructions were parsed
    assert!(result.content.contains("Step 1: Chop vegetables"));
    assert!(result.content.contains("Step 2: Cook the dish"));
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
    let result = fetch_recipe(&url).await.unwrap();

    // Check that single string values still work
    assert_eq!(result.metadata.get("cuisine").unwrap(), "Italian");
    assert_eq!(result.metadata.get("course").unwrap(), "Main Course");
}

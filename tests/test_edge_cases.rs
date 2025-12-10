use cooklang_import::url_to_recipe;
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
async fn test_bbc_good_food_author_array() {
    // This tests the case where BBC Good Food has authors as an array
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "BBC Good Food Recipe",
        "description": "A delicious recipe",
        "image": "https://example.com/image.jpg",
        "author": [
            {
                "@type": "Person",
                "name": "Chef One"
            },
            {
                "@type": "Person",
                "name": "Chef Two"
            }
        ],
        "recipeIngredient": ["ingredient 1", "ingredient 2"],
        "recipeInstructions": "Cook the food"
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

    assert_eq!(result.name, "BBC Good Food Recipe");
    assert!(result.metadata.contains("author: Chef One, Chef Two"));
}

#[tokio::test]
async fn test_empty_strings_and_empty_arrays() {
    // Test that empty strings and empty arrays don't create metadata entries
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe with Empty Fields",
        "description": "",
        "image": "https://example.com/image.jpg",
        "author": "",
        "prepTime": "",
        "cookTime": "PT30M",
        "totalTime": "",
        "keywords": "",
        "recipeCuisine": [],
        "recipeCategory": [""],
        "recipeYield": "",
        "recipeIngredient": ["ingredient 1"],
        "recipeInstructions": "Cook it"
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

    // These should not exist due to empty values
    assert!(!result.metadata.contains("author:"));
    assert!(!result.metadata.contains("prep time:"));
    assert!(!result.metadata.contains("time required:"));
    assert!(!result.metadata.contains("tags:"));
    assert!(!result.metadata.contains("cuisine:"));
    assert!(!result.metadata.contains("course:"));
    assert!(!result.metadata.contains("servings:"));

    // This should exist
    assert!(result.metadata.contains("cook time: 30 minutes"));
}

#[tokio::test]
async fn test_recipe_yield_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test 1: Array with descriptive text preferred
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Test Recipe 1",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "recipeYield": ["15", "15 Stück"]
    }
    "#;

    let _m1 = server
        .mock("GET", "/recipe1")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url1 = format!("{}/recipe1", server.url());
    let result1 = url_to_recipe(&url1).await.unwrap();
    assert!(result1.metadata.contains("servings: 15 Stück"));

    // Test 2: Simple string yield
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Test Recipe 3",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "recipeYield": "8 portions"
    }
    "#;

    let _m3 = server
        .mock("GET", "/recipe3")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url3 = format!("{}/recipe3", server.url());
    let result3 = url_to_recipe(&url3).await.unwrap();
    assert!(result3.metadata.contains("servings: 8 portions"));

    // Test 3: Numeric yield
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Test Recipe 4",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "recipeYield": 6
    }
    "#;

    let _m4 = server
        .mock("GET", "/recipe4")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url4 = format!("{}/recipe4", server.url());
    let result4 = url_to_recipe(&url4).await.unwrap();
    assert!(result4.metadata.contains("servings: 6"));
}

#[tokio::test]
async fn test_time_fields_conversion() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Timed Recipe",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "prepTime": "PT15M",
        "cookTime": "PT1H30M",
        "totalTime": "PT1H45M"
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
    assert!(result.metadata.contains("cook time: 1 hour 30 minutes"));
    assert!(result.metadata.contains("time required: 1 hour 45 minutes"));
}

#[tokio::test]
async fn test_diet_restrictions_handling() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test diet with schema.org URLs
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Diet-friendly Recipe",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "suitableForDiet": [
            "https://schema.org/GlutenFreeDiet",
            "http://schema.org/VeganDiet",
            "VegetarianDiet"
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
    let result = url_to_recipe(&url).await.unwrap();

    // Should clean up schema.org URLs and "Diet" suffix
    assert!(result.metadata.contains("diet: GlutenFree, Vegan, Vegetarian"));
}

#[tokio::test]
async fn test_instructions_contain_expected_text() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test HowToSection with HowToSteps
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe with HowTo Instructions",
        "image": "test.jpg",
        "recipeIngredient": ["ingredient"],
        "recipeInstructions": [
            {
                "@type": "HowToSection",
                "name": "Preparation",
                "itemListElement": [
                    {
                        "@type": "HowToStep",
                        "name": "Preheat oven to 180°C"
                    },
                    {
                        "@type": "HowToStep",
                        "text": "Mix all ingredients"
                    }
                ]
            },
            {
                "@type": "HowToStep",
                "text": "Let it cool and serve"
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
    let result = url_to_recipe(&url).await.unwrap();

    // Check that all instruction variations were captured
    assert!(result.text.contains("Preheat oven to 180°C"));
    assert!(result.text.contains("Mix all ingredients"));
    assert!(result.text.contains("Let it cool and serve"));
}

#[tokio::test]
async fn test_author_with_only_id_field() {
    // Test that authors with only @id field (no name) are handled gracefully
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    let json_ld = r#"
    {
        "@type": "Recipe",
        "name": "Recipe with ID-only Author",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "author": {
            "@id": "https://example.com/#/schema/person/123456"
        }
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

    // Author should not be in metadata since it only had an @id
    assert!(!result.metadata.contains("author:"));
}

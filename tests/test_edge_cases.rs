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
    let result = fetch_recipe(&url).await.unwrap();

    assert_eq!(result.name, "BBC Good Food Recipe");
    assert_eq!(result.metadata.get("author").unwrap(), "Chef One, Chef Two");
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
    let result = fetch_recipe(&url).await.unwrap();

    // These should not exist due to empty values
    assert!(result.metadata.get("author").is_none());
    assert!(result.metadata.get("prep time").is_none());
    assert!(result.metadata.get("time required").is_none());
    assert!(result.metadata.get("tags").is_none());
    assert!(result.metadata.get("cuisine").is_none());
    assert!(result.metadata.get("course").is_none());
    assert!(result.metadata.get("servings").is_none());

    // This should exist
    assert_eq!(result.metadata.get("cook time").unwrap(), "30 minutes");

    // Description should be None when empty
    assert!(result.description.is_none());
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
    let result1 = fetch_recipe(&url1).await.unwrap();
    assert_eq!(result1.metadata.get("servings").unwrap(), "15 Stück");

    // Test 2: Array with descriptive text first
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Test Recipe 2",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "recipeYield": ["4 servings", "4"]
    }
    "#;

    let _m2 = server
        .mock("GET", "/recipe2")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url2 = format!("{}/recipe2", server.url());
    let result2 = fetch_recipe(&url2).await.unwrap();
    assert_eq!(result2.metadata.get("servings").unwrap(), "4 servings");

    // Test 3: Simple string yield
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
    let result3 = fetch_recipe(&url3).await.unwrap();
    assert_eq!(result3.metadata.get("servings").unwrap(), "8 portions");

    // Test 4: Numeric yield
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
    let result4 = fetch_recipe(&url4).await.unwrap();
    assert_eq!(result4.metadata.get("servings").unwrap(), "6");
}

#[tokio::test]
async fn test_category_and_cuisine_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test with arrays
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Multi-category Recipe",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "recipeCategory": ["Dessert", "Kuchen", "Snack"],
        "recipeCuisine": ["Italian", "Mediterranean"]
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

    assert_eq!(
        result.metadata.get("course").unwrap(),
        "Dessert, Kuchen, Snack"
    );
    assert_eq!(
        result.metadata.get("cuisine").unwrap(),
        "Italian, Mediterranean"
    );
}

#[tokio::test]
async fn test_howto_instructions_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test HowToSection with HowToSteps that have name but no text
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
                        "text": "Mix all ingredients",
                        "name": "This name should be ignored"
                    },
                    {
                        "@type": "HowToStep",
                        "text": "Bake for 30 minutes",
                        "description": "Until golden brown"
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
    let result = fetch_recipe(&url).await.unwrap();

    // Check that all instruction variations were captured
    assert!(result.instructions.contains("Preheat oven to 180°C"));
    assert!(result.instructions.contains("Mix all ingredients"));
    assert!(result.instructions.contains("Bake for 30 minutes"));
    assert!(result.instructions.contains("Until golden brown"));
    assert!(result.instructions.contains("Let it cool and serve"));
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
    let result = fetch_recipe(&url).await.unwrap();

    // Should clean up schema.org URLs and "Diet" suffix
    assert_eq!(
        result.metadata.get("diet").unwrap(),
        "GlutenFree, Vegan, Vegetarian"
    );
}

#[tokio::test]
async fn test_keywords_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test 1: Keywords as string
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 1",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "keywords": "chocolate, cookies, dessert"
    }
    "#;

    let _m1 = server
        .mock("GET", "/recipe1")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url1 = format!("{}/recipe1", server.url());
    let result1 = fetch_recipe(&url1).await.unwrap();
    assert_eq!(
        result1.metadata.get("tags").unwrap(),
        "chocolate, cookies, dessert"
    );

    // Test 2: Keywords as array
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 2",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "keywords": ["healthy", "quick", "easy", "vegan"]
    }
    "#;

    let _m2 = server
        .mock("GET", "/recipe2")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url2 = format!("{}/recipe2", server.url());
    let result2 = fetch_recipe(&url2).await.unwrap();
    assert_eq!(
        result2.metadata.get("tags").unwrap(),
        "healthy, quick, easy, vegan"
    );
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
    let result = fetch_recipe(&url).await.unwrap();

    assert_eq!(result.metadata.get("prep time").unwrap(), "15 minutes");
    assert_eq!(
        result.metadata.get("cook time").unwrap(),
        "1 hour 30 minutes"
    );
    assert_eq!(
        result.metadata.get("time required").unwrap(),
        "1 hour 45 minutes"
    );
}

#[tokio::test]
async fn test_image_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test 1: Single image as string
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 1",
        "image": "https://example.com/image.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m1 = server
        .mock("GET", "/recipe1")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url1 = format!("{}/recipe1", server.url());
    let result1 = fetch_recipe(&url1).await.unwrap();
    assert_eq!(result1.image, vec!["https://example.com/image.jpg"]);

    // Test 2: Multiple images as array
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 2",
        "image": [
            "https://example.com/image1.jpg",
            "https://example.com/image2.jpg"
        ],
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m2 = server
        .mock("GET", "/recipe2")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url2 = format!("{}/recipe2", server.url());
    let result2 = fetch_recipe(&url2).await.unwrap();
    assert_eq!(result2.image.len(), 2);
    assert_eq!(result2.image[0], "https://example.com/image1.jpg");
    assert_eq!(result2.image[1], "https://example.com/image2.jpg");

    // Test 3: Image as object
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 3",
        "image": {
            "@type": "ImageObject",
            "url": "https://example.com/image-object.jpg"
        },
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m3 = server
        .mock("GET", "/recipe3")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url3 = format!("{}/recipe3", server.url());
    let result3 = fetch_recipe(&url3).await.unwrap();
    assert_eq!(result3.image, vec!["https://example.com/image-object.jpg"]);

    // Test 4: No image
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 4",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m4 = server
        .mock("GET", "/recipe4")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url4 = format!("{}/recipe4", server.url());
    let result4 = fetch_recipe(&url4).await.unwrap();
    assert!(result4.image.is_empty());
}

#[tokio::test]
async fn test_author_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");
    let mut server = mockito::Server::new_async().await;

    // Test 1: Author as string
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 1",
        "image": "test.jpg",
        "author": "Simple Author Name",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m1 = server
        .mock("GET", "/recipe1")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url1 = format!("{}/recipe1", server.url());
    let result1 = fetch_recipe(&url1).await.unwrap();
    assert_eq!(
        result1.metadata.get("author").unwrap(),
        "Simple Author Name"
    );

    // Test 2: Author as object
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 2",
        "image": "test.jpg",
        "author": {
            "@type": "Person",
            "name": "Chef Object"
        },
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m2 = server
        .mock("GET", "/recipe2")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url2 = format!("{}/recipe2", server.url());
    let result2 = fetch_recipe(&url2).await.unwrap();
    assert_eq!(result2.metadata.get("author").unwrap(), "Chef Object");

    // Test 3: Multiple authors
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Recipe 3",
        "image": "test.jpg",
        "author": [
            {
                "@type": "Person",
                "name": "Author One"
            },
            {
                "@type": "Person",
                "name": "Author Two"
            },
            {
                "@type": "Person",
                "name": "Author Three"
            }
        ],
        "recipeIngredient": ["test"],
        "recipeInstructions": "test"
    }
    "#;

    let _m3 = server
        .mock("GET", "/recipe3")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html(json_ld))
        .create();

    let url3 = format!("{}/recipe3", server.url());
    let result3 = fetch_recipe(&url3).await.unwrap();
    assert_eq!(
        result3.metadata.get("author").unwrap(),
        "Author One, Author Two, Author Three"
    );
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
    let result = fetch_recipe(&url).await.unwrap();

    // Author should not be in metadata since it only had an @id
    assert!(result.metadata.get("author").is_none());
}

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
async fn test_author_with_only_id() {
    // Test case where author has only @id field, no name
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@id": "https://princesspinkygirl.com/blt-pasta-salad/#recipe",
        "@type": "Recipe",
        "author": {
            "@id": "https://princesspinkygirl.com/#/schema/person/34b9cf76083e3fcd943f7959bef45b88"
        },
        "datePublished": "2024-04-25T05:00:00+00:00",
        "description": "BLT pasta salad is a clever twist on the classic BLT sandwich!",
        "image": [
            "https://princesspinkygirl.com/wp-content/uploads/2020/07/BLT-pasta-salad-square.jpg",
            "https://princesspinkygirl.com/wp-content/uploads/2020/07/BLT-pasta-salad-square-500x500.jpg"
        ],
        "keywords": "Barbecue, BLT pasta salad, Food for a Crowd, pasta, pasta salad, Potluck",
        "name": "BLT Pasta Salad",
        "prepTime": "PT10M",
        "recipeCategory": [
            "Salad",
            "Side Dish"
        ],
        "recipeCuisine": [
            "American"
        ],
        "recipeIngredient": [
            "1 16-ounce box Rotini Pasta (cooked, drained, and rinsed with cold water)",
            "13 slices bacon (cooked and chopped)",
            "½  large red onion (finely diced)"
        ],
        "recipeInstructions": [
            {
                "@type": "HowToStep",
                "name": "In a medium mixing bowl combine dry Hidden Valley Ranch Dressing Mix, mayonnaise, and sour cream.",
                "text": "In a medium mixing bowl combine dry Hidden Valley Ranch Dressing Mix, mayonnaise, and sour cream. Use a whisk to mix together. Cover with plastic wrap and refrigerate for 30 minutes.",
                "url": "https://princesspinkygirl.com/blt-pasta-salad/#wprm-recipe-30514-step-0-0"
            },
            {
                "@type": "HowToStep",
                "name": "Cook rotini pasta per package directions.",
                "text": "Cook rotini pasta per package directions. Drain, rinse with cold water, and drain again. Place in a large mixing bowl.",
                "url": "https://princesspinkygirl.com/blt-pasta-salad/#wprm-recipe-30514-step-0-1"
            }
        ],
        "recipeYield": [
            "10"
        ],
        "totalTime": "PT40M"
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

    // Verify the recipe was parsed successfully
    assert_eq!(result.name, "BLT Pasta Salad");

    // Author should not be in metadata since it only had an @id
    assert!(!result.metadata.contains("author:"));

    // Other metadata should be present
    assert!(result.metadata.contains("prep time: 10 minutes"));
    assert!(result.metadata.contains("time required: 40 minutes"));
    assert!(result.metadata.contains("course: Salad, Side Dish"));
    assert!(result.metadata.contains("cuisine: American"));
    assert!(result.metadata.contains("servings: 10"));
    assert!(result
        .metadata
        .contains("tags: Barbecue, BLT pasta salad, Food for a Crowd, pasta, pasta salad, Potluck"));
}

#[tokio::test]
async fn test_mixed_authors_with_and_without_names() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@type": "Recipe",
        "name": "Multi-Author Recipe",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "author": [
            {
                "@type": "Person",
                "name": "Chef One"
            },
            {
                "@id": "https://example.com/author/2"
            },
            {
                "@type": "Person",
                "name": "Chef Three"
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

    // Should only include authors with names
    assert!(result.metadata.contains("author: Chef One, Chef Three"));
}

#[tokio::test]
async fn test_single_author_with_id_and_name() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@type": "Recipe",
        "name": "Recipe with Full Author",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "author": {
            "@type": "Person",
            "@id": "https://example.com/author/123",
            "name": "Full Author Name"
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

    // Should use the name when both @id and name are present
    assert!(result.metadata.contains("author: Full Author Name"));
}

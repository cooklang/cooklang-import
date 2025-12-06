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
async fn test_recipe_without_ingredients() {
    // Test case where recipe has no recipeIngredient field
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "http://schema.org/",
        "@type": "Recipe",
        "name": "Home style Bhindi fry",
        "author": {
            "@type": "Person",
            "name": "Ranveer Brar"
        },
        "cookTime": "PT15M",
        "prepTime": "PT10M",
        "totalTime": "PT25M",
        "description": "If you haven't tried this Bhindi/ Okra recipe, you are definitely missing something :)",
        "image": {
            "@type": "ImageObject",
            "url": "https://example.com/bhindi.jpg"
        },
        "recipeCuisine": "Indian",
        "recipeCategory": ["Main Course"],
        "recipeInstructions": [
            {
                "@type": "HowToStep",
                "text": "In an iron kadai, add mustard oil, once it's smoky hot, add onion, potatoes and fry them for a while until golden in color."
            },
            {
                "@type": "HowToStep",
                "text": "Add prepared ginger green chili paste and saute it for a minute."
            },
            {
                "@type": "HowToStep",
                "text": "Add ladyfinger, salt to taste, coriander powder, dry mango powder, turmeric powder, degi red chili powder and mix it well."
            }
        ],
        "recipeYield": "2"
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

    // Verify the recipe was parsed successfully without ingredients (only instructions)
    assert_eq!(result.name, "Home style Bhindi fry");
    assert!(result.instructions.contains("In an iron kadai"));

    // Verify metadata
    assert_eq!(result.metadata.get("author").unwrap(), "Ranveer Brar");
    assert_eq!(result.metadata.get("cook time").unwrap(), "15 minutes");
    assert_eq!(result.metadata.get("prep time").unwrap(), "10 minutes");
    assert_eq!(result.metadata.get("time required").unwrap(), "25 minutes");
    assert_eq!(result.metadata.get("course").unwrap(), "Main Course");
    assert_eq!(result.metadata.get("cuisine").unwrap(), "Indian");
    assert_eq!(result.metadata.get("servings").unwrap(), "2");

    // Verify instructions were parsed
    assert!(result.instructions.contains("iron kadai"));
    assert!(result.instructions.contains("ginger green chili paste"));
}

#[tokio::test]
async fn test_recipe_with_duration_range() {
    // Test case where cookTime has a range like "PT15-20M"
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "http://schema.org/",
        "@type": "Recipe",
        "name": "Variable Cook Time Recipe",
        "cookTime": "PT15-20M",
        "totalTime": "PT25-30M",
        "recipeIngredient": ["test ingredient"],
        "recipeInstructions": "Test instructions"
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

    // The duration converter might not handle ranges perfectly,
    // but at least it should not crash
    assert_eq!(result.name, "Variable Cook Time Recipe");

    // Check that some time value is extracted (even if not perfect)
    let cook_time = result.metadata.get("cook time");
    assert!(cook_time.is_some());
    println!("Cook time extracted: {cook_time:?}");

    let total_time = result.metadata.get("time required");
    assert!(total_time.is_some());
    println!("Total time extracted: {total_time:?}");
}

#[tokio::test]
async fn test_recipe_with_ingredient_objects() {
    // Test case where recipeIngredient contains objects instead of strings
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Nigella Lawson's Basque Burnt Cheesecake",
        "author": {
            "@type": "Person",
            "name": "Nigella Lawson"
        },
        "description": "Inspired by the iconic San Sebastian dessert",
        "image": "https://example.com/cheesecake.jpg",
        "recipeCuisine": "Spanish",
        "recipeCategory": [],
        "recipeIngredient": [
            {
                "@type": "HowToIngredient",
                "amount": "",
                "name": "For the cheesecake:"
            },
            {
                "@type": "HowToIngredient",
                "amount": "600g",
                "name": "full-fat cream cheese, at room temperature"
            },
            {
                "@type": "HowToIngredient",
                "amount": "175g",
                "name": "caster sugar"
            },
            {
                "@type": "HowToIngredient",
                "amount": "3",
                "name": "large eggs, at room temperature"
            },
            {
                "@type": "HowToIngredient",
                "amount": "¼ tsp",
                "name": "fine sea salt"
            }
        ],
        "recipeInstructions": "Heat the oven to 200ºC/180ºC Fan. Beat the cream cheese with sugar until smooth.",
        "recipeYield": "Gives 8-12 slices"
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
    assert_eq!(result.name, "Nigella Lawson's Basque Burnt Cheesecake");

    // Check ingredients formatting
    assert_eq!(result.ingredients[0], "For the cheesecake:"); // No amount
    assert_eq!(
        result.ingredients[1],
        "600g full-fat cream cheese, at room temperature"
    );
    assert_eq!(result.ingredients[2], "175g caster sugar");
    assert_eq!(result.ingredients[3], "3 large eggs, at room temperature");
    assert_eq!(result.ingredients[4], "¼ tsp fine sea salt");

    // Verify metadata
    assert_eq!(result.metadata.get("author").unwrap(), "Nigella Lawson");
    assert_eq!(result.metadata.get("cuisine").unwrap(), "Spanish");
    assert_eq!(
        result.metadata.get("servings").unwrap(),
        "Gives 8-12 slices"
    );

    // Empty category array should not create metadata
    assert!(!result.metadata.contains_key("course"));
}

#[tokio::test]
async fn test_recipe_with_nested_sections() {
    // Test case where recipeInstructions is a nested array with HowToSection
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Sałatka z brokuła",
        "description": "Sałatki nie solimy gdyż ser feta jest sam w sobie słony.",
        "image": {
            "@type": "ImageObject",
            "url": "https://example.com/salatka.jpg"
        },
        "recipeCategory": "Brokuły",
        "recipeCuisine": "Kuchnia polska",
        "recipeIngredient": [
            "jogurt naturalny 2 łyżki",
            "ser feta 100 gram",
            "brokuł 1 mała szt."
        ],
        "recipeInstructions": [
            [
                {
                    "@type": "HowToSection",
                    "name": "Kroki postępowania",
                    "itemListElement": [
                        {
                            "@type": "HowToStep",
                            "text": "Brokuła umyć, osuszyć i podzielić na różyczki."
                        },
                        {
                            "@type": "HowToStep",
                            "text": "Ser feta pokroić w dużą kostkę."
                        },
                        {
                            "@type": "HowToStep",
                            "text": "Słonecznik podprażyć na patelni."
                        }
                    ]
                }
            ]
        ],
        "recipeYield": "1 - 2",
        "totalTime": "PT15M"
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
    assert_eq!(result.name, "Sałatka z brokuła");

    // Check that nested instructions were extracted
    assert!(result.instructions.contains("Brokuła umyć"));
    assert!(result.instructions.contains("Ser feta pokroić"));
    assert!(result.instructions.contains("Słonecznik podprażyć"));

    // Verify metadata
    assert_eq!(result.metadata.get("course").unwrap(), "Brokuły");
    assert_eq!(result.metadata.get("cuisine").unwrap(), "Kuchnia polska");
    assert_eq!(result.metadata.get("servings").unwrap(), "1 - 2");
    assert_eq!(result.metadata.get("time required").unwrap(), "15 minutes");
}

#[tokio::test]
async fn test_recipe_with_seconds_duration() {
    // Test case where totalTime is in seconds like "PT5400.0S"
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Gordon's Curry",
        "recipeIngredient": ["chicken", "spices"],
        "recipeInstructions": "Cook the curry",
        "totalTime": "PT5400.0S"
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

    // PT5400S = 5400 seconds = 90 minutes = 1 hour 30 minutes
    assert_eq!(
        result.metadata.get("time required").unwrap(),
        "1 hour 30 minutes"
    );
}

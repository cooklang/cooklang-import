use cooklang_import::url_to_recipe;
use std::env;

fn create_recipe_html_with_sections(json_ld: &str) -> String {
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
async fn test_german_recipe_with_sections_and_array_yield() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@id": "https://biancazapatka.com/de/brookies-chocolate-chip-cookie-brownies/#recipe",
        "@type": "Recipe",
        "author": {
            "@type": "Person",
            "name": "Bianca Zapatka"
        },
        "cookTime": "PT25M",
        "datePublished": "2022-09-08T15:49:03+00:00",
        "description": "Saftige Schokoladen-Brownies treffen auf knusprige Chocolate Chip Cookies",
        "image": [
            "https://biancazapatka.com/wp-content/uploads/2022/09/cookie-brownies.jpg",
            "https://biancazapatka.com/wp-content/uploads/2022/09/cookie-brownies-500x500.jpg"
        ],
        "keywords": "Brookies, Brownies, Chocolate Chip Cookies, Cookie Bars, Cookies, Kekse",
        "name": "Vegane Brookies - Chocolate Chip Cookie Brownies",
        "prepTime": "PT20M",
        "recipeCategory": [
            "Dessert",
            "Kuchen",
            "Snack"
        ],
        "recipeCuisine": [
            "Amerikanisch"
        ],
        "recipeIngredient": [
            "160 g Mehl (Weizen-, Dinkel oder glutenfreies Mehl, gesiebt)",
            "30 g Kakaopulver",
            "1  Prise Salz"
        ],
        "recipeInstructions": [
            {
                "@type": "HowToSection",
                "name": "Brownie-Teig",
                "itemListElement": [
                    {
                        "@type": "HowToStep",
                        "name": "Den Backofen auf 180 °C Ober-/Unterhitze vorheizen",
                        "text": "Den Backofen auf 180 °C Ober-/Unterhitze vorheizen und eine 18x28 cm Brownieform leicht einfetten und mit Backpapier auslegen.",
                        "url": "https://biancazapatka.com/de/brookies-chocolate-chip-cookie-brownies/#wprm-recipe-63308-step-1-0"
                    },
                    {
                        "@type": "HowToStep",
                        "name": "Vegane Butter mit der Schokolade über einem Wasserbad oder in der Mikrowelle schmelzen.",
                        "text": "Vegane Butter mit der Schokolade über einem Wasserbad oder in der Mikrowelle schmelzen.",
                        "url": "https://biancazapatka.com/de/brookies-chocolate-chip-cookie-brownies/#wprm-recipe-63308-step-1-1"
                    }
                ]
            },
            {
                "@type": "HowToSection",
                "name": "Cookie-Teig",
                "itemListElement": [
                    {
                        "@type": "HowToStep",
                        "name": "Mehl, Salz, Backpulver und Zucker in einer Schüssel vermischen",
                        "text": "Mehl, Salz, Backpulver und Zucker in einer Schüssel vermischen. Vegane Butter und Wasser hinzufügen und mit den Händen kurz zu einem Teig verkneten.",
                        "url": "https://biancazapatka.com/de/brookies-chocolate-chip-cookie-brownies/#wprm-recipe-63308-step-2-0"
                    }
                ]
            }
        ],
        "recipeYield": [
            "15",
            "15 Stück"
        ],
        "totalTime": "PT45M"
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_sections(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // Verify the recipe was parsed successfully
    assert_eq!(
        result.name,
        "Vegane Brookies - Chocolate Chip Cookie Brownies"
    );

    // Check that yield array was handled correctly - should prefer "15 Stück" over "15"
    assert!(result.metadata.contains("servings: 15 Stück"));

    // Check that category array was handled
    assert!(result.metadata.contains("course: Dessert, Kuchen, Snack"));

    // Check cuisine array with single element
    assert!(result.metadata.contains("cuisine: Amerikanisch"));

    // Check time conversions
    assert!(result.metadata.contains("prep time: 20 minutes"));
    assert!(result.metadata.contains("cook time: 25 minutes"));
    assert!(result.metadata.contains("time required: 45 minutes"));

    // Check keywords
    assert!(result.metadata.contains("tags: Brookies, Brownies, Chocolate Chip Cookies, Cookie Bars, Cookies, Kekse"));

    // Check author
    assert!(result.metadata.contains("author: Bianca Zapatka"));

    // Check that instructions from sections were parsed
    assert!(result.text.contains("Den Backofen auf 180 °C"));
    assert!(result.text.contains("Vegane Butter mit der Schokolade"));
    assert!(result.text.contains("Mehl, Salz, Backpulver und Zucker"));

    // Check that section names are preserved as markdown headers
    assert!(
        result.text.contains("## Brownie-Teig"),
        "Section name 'Brownie-Teig' should be preserved. Got: {}",
        result.text
    );
    assert!(
        result.text.contains("## Cookie-Teig"),
        "Section name 'Cookie-Teig' should be preserved. Got: {}",
        result.text
    );
}

#[tokio::test]
async fn test_recipe_yield_variations() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;

    // Test with array where first item is descriptive
    let json_ld = r#"
    {
        "@context": "https://schema.org",
        "@type": "Recipe",
        "name": "Test Recipe",
        "description": "Test",
        "image": "test.jpg",
        "recipeIngredient": ["test"],
        "recipeInstructions": "test",
        "recipeYield": ["4 servings", "4"]
    }
    "#;

    let _m = server
        .mock("GET", "/recipe1")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_sections(json_ld))
        .create();

    let url = format!("{}/recipe1", server.url());
    let result = url_to_recipe(&url).await.unwrap();

    // Should pick "4 servings" because it contains alphabetic characters
    assert!(result.metadata.contains("servings: 4 servings"));
}

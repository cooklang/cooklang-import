use std::env;
use std::process::Command;

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
async fn test_download_mode_with_metadata() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Test Recipe",
        "description": "A test recipe",
        "image": "https://example.com/image.jpg",
        "author": "Test Author",
        "prepTime": "PT15M",
        "cookTime": "PT30M",
        "totalTime": "PT45M",
        "recipeYield": "4 servings",
        "recipeCategory": "Main Course",
        "recipeCuisine": "Italian",
        "keywords": "test, recipe, metadata",
        "recipeIngredient": [
            "1 cup flour",
            "2 eggs",
            "1/2 cup milk"
        ],
        "recipeInstructions": "Mix all ingredients and cook."
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());

    // Run the binary with --download-only flag
    let output = Command::new("cargo")
        .args(&["run", "--", &url, "--download-only"])
        .env("RUST_LOG", "error") // Suppress debug logs
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that frontmatter is included
    assert!(stdout.contains("---\n"));
    assert!(stdout.contains("author: Test Author"));
    assert!(stdout.contains("cook time: 30 minutes"));
    assert!(stdout.contains("prep time: 15 minutes"));
    assert!(stdout.contains("time required: 45 minutes"));
    assert!(stdout.contains("course: Main Course"));
    assert!(stdout.contains("cuisine: Italian"));
    assert!(stdout.contains("servings: 4 servings"));
    assert!(stdout.contains("tags: test, recipe, metadata"));
    assert!(stdout.contains(&format!("source: \"{}\"", url)));
    assert!(stdout.contains("---\n\n# Test Recipe"));

    // Check that ingredients and instructions are included
    assert!(stdout.contains("## Ingredients"));
    assert!(stdout.contains("1 cup flour"));
    assert!(stdout.contains("## Instructions"));
    assert!(stdout.contains("Mix all ingredients and cook."));
}

#[tokio::test]
async fn test_download_mode_without_metadata() {
    env::set_var("OPENAI_API_KEY", "test_key");

    let mut server = mockito::Server::new_async().await;
    // Minimal recipe without optional metadata fields
    let json_ld = r#"
    {
        "@context": "https://schema.org/",
        "@type": "Recipe",
        "name": "Simple Recipe",
        "recipeIngredient": ["ingredient 1", "ingredient 2"],
        "recipeInstructions": "Simple instructions."
    }
    "#;

    let _m = server
        .mock("GET", "/recipe")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body(create_recipe_html_with_metadata(json_ld))
        .create();

    let url = format!("{}/recipe", server.url());

    // Run the binary with --download-only flag
    let output = Command::new("cargo")
        .args(&["run", "--", &url, "--download-only"])
        .env("RUST_LOG", "error") // Suppress debug logs
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should still have frontmatter with at least the source URL
    assert!(stdout.contains("---\n"));
    assert!(stdout.contains(&format!("source: \"{}\"", url)));
    assert!(stdout.contains("---\n\n# Simple Recipe"));

    // Check basic content
    assert!(stdout.contains("## Ingredients"));
    assert!(stdout.contains("ingredient 1"));
    assert!(stdout.contains("## Instructions"));
    assert!(stdout.contains("Simple instructions."));
}

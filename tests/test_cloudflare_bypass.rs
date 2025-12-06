use cooklang_import::fetch_recipe;
use std::env;

#[tokio::test]
#[ignore] // This test requires network access and is subject to external site changes
async fn test_acouplecooks_cloudflare_bypass() {
    // Enable logging to see debug output if needed
    env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();

    let url = "https://www.acouplecooks.com/apple-cranberry-crisp/";

    println!("Attempting to fetch recipe from: {}", url);

    match fetch_recipe(url).await {
        Ok(recipe) => {
            println!("Successfully bypassed Cloudflare and parsed recipe!");
            println!("Name: {}", recipe.name);

            // Verify we got the correct recipe
            assert_eq!(recipe.name, "Apple Cranberry Crisp");
            assert!(!recipe.ingredients.is_empty() || !recipe.instructions.is_empty());

            // Verify some specific content to ensure we didn't just get a title from metadata
            let all_content = format!("{}\n{}", recipe.ingredients.join("\n"), recipe.instructions);
            assert!(all_content.contains("cranberries"));
            assert!(all_content.contains("sugar"));

            // Check metadata
            assert_eq!(recipe.metadata.get("author").unwrap(), "Sonja Overhiser");
            assert!(recipe.metadata.contains_key("cook time"));
        }
        Err(e) => {
            panic!("Failed to fetch recipe (Cloudflare bypass failed?): {}", e);
        }
    }
}

use cooklang_import::url_to_recipe;
use std::env;

#[tokio::test]
#[ignore] // This test requires network access and is subject to external site changes
async fn test_acouplecooks_cloudflare_bypass() {
    // Enable logging to see debug output if needed
    env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();

    let url = "https://www.acouplecooks.com/apple-cranberry-crisp/";

    println!("Attempting to fetch recipe from: {}", url);

    match url_to_recipe(url).await {
        Ok(result) => {
            println!("Successfully bypassed Cloudflare and parsed recipe!");
            println!("Name: {}", result.name);

            // Verify we got the correct recipe
            assert_eq!(result.name, "Apple Cranberry Crisp");
            assert!(!result.text.is_empty());

            // Verify some specific content to ensure we didn't just get a title from metadata
            assert!(result.text.contains("cranberries"));
            assert!(result.text.contains("sugar"));

            // Check metadata
            assert!(result.metadata.contains("author: Sonja Overhiser"));
            assert!(result.metadata.contains("cook time:"));
        }
        Err(e) => {
            panic!("Failed to fetch recipe (Cloudflare bypass failed?): {}", e);
        }
    }
}

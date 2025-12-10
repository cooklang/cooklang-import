use cooklang_import::url_to_recipe;
use std::env;

#[tokio::test]
#[ignore] // This test requires network access
async fn test_shahi_paneer_recipe() {
    env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();

    let url = "https://amateurprochef.com/2024/09/07/shahi-paneer-2/";
    match url_to_recipe(url).await {
        Ok(result) => {
            println!("Recipe parsed successfully!");
            println!("Name: {}", result.name);
            println!("Metadata: {}", result.metadata);

            // Verify the recipe was parsed
            assert!(result.name.contains("Shahi Paneer"));
            assert!(!result.text.is_empty());

            // Check metadata
            assert!(result.metadata.contains("author: amateurprochef"));
            assert!(result.metadata.contains("cook time: 30 minutes"));
            assert!(result.metadata.contains("course: All, All Things Indian"));
        }
        Err(e) => {
            panic!("Failed to fetch recipe: {e}");
        }
    }
}

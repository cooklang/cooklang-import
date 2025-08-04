use cooklang_import::fetch_recipe;
use std::env;

#[tokio::test]
#[ignore] // This test requires network access
async fn test_shahi_paneer_recipe() {
    env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();

    let url = "https://amateurprochef.com/2024/09/07/shahi-paneer-2/";
    match fetch_recipe(url).await {
        Ok(recipe) => {
            println!("Recipe parsed successfully!");
            println!("Name: {}", recipe.name);
            println!("Metadata: {:?}", recipe.metadata);

            // Verify the recipe was parsed
            assert!(recipe.name.contains("Shahi Paneer"));
            assert!(!recipe.ingredients.is_empty());
            assert!(!recipe.instructions.is_empty());

            // Check metadata
            assert_eq!(recipe.metadata.get("author").unwrap(), "amateurprochef");
            assert_eq!(recipe.metadata.get("cook time").unwrap(), "30 minutes");
            assert_eq!(
                recipe.metadata.get("course").unwrap(),
                "All, All Things Indian"
            );
        }
        Err(e) => {
            panic!("Failed to fetch recipe: {}", e);
        }
    }
}

//! Simple API usage with convenience functions
//!
//! This example shows how to use the high-level convenience functions
//! for the most common use cases.

use cooklang_import::{text_to_cooklang, url_to_recipe, RecipeImporter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple import: URL → Cooklang
    println!("=== Simple Import ===");
    let result = RecipeImporter::builder()
        .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
        .build()
        .await?;
    println!("Recipe in Cooklang format:");
    match result {
        cooklang_import::ImportResult::Cooklang { content, .. } => println!("{}", content),
        cooklang_import::ImportResult::Components(components) => println!("{}", components.text),
    }

    // Extract only: URL → RecipeComponents
    println!("\n=== Extract Only ===");
    let components =
        url_to_recipe("https://www.bbcgoodfood.com/recipes/classic-cottage-pie").await?;
    println!("Recipe name: {}", components.name);
    println!("Has text content: {}", !components.text.is_empty());
    println!("Has metadata: {}", !components.metadata.is_empty());

    // Convert text: Text → Cooklang
    println!("\n=== Convert Text ===");
    let ingredients = "2 eggs\n1 cup flour\n1/2 cup milk";
    let instructions = "Mix all ingredients together. Bake at 350°F for 30 minutes.";

    let content = format!("{}\n\n{}", ingredients, instructions);
    let components = cooklang_import::RecipeComponents {
        text: content,
        metadata: String::new(),
        name: "Simple Recipe".to_string(),
    };
    let cooklang = text_to_cooklang(&components).await?;
    println!("Converted to Cooklang:");
    println!("{}", cooklang);

    Ok(())
}

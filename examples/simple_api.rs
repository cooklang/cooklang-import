//! Simple API usage with convenience functions
//!
//! This example shows how to use the high-level convenience functions
//! for the most common use cases.

use cooklang_import::{convert_text_to_cooklang, extract_recipe_from_url, import_from_url};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple import: URL → Cooklang
    println!("=== Simple Import ===");
    let cooklang =
        import_from_url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie").await?;
    println!("Recipe in Cooklang format:");
    println!("{}", cooklang);

    // Extract only: URL → Recipe
    println!("\n=== Extract Only ===");
    let recipe =
        extract_recipe_from_url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie").await?;
    println!("Recipe name: {}", recipe.name);
    println!("Has {} ingredients", recipe.ingredients.len());
    println!("Has instructions: {}", !recipe.instructions.is_empty());

    // Convert markdown: Markdown → Cooklang
    println!("\n=== Convert Markdown ===");
    let ingredients = "2 eggs\n1 cup flour\n1/2 cup milk";
    let instructions = "Mix all ingredients together. Bake at 350°F for 30 minutes.";

    let content = format!("{}\n\n{}", ingredients, instructions);
    let cooklang = convert_text_to_cooklang(&content).await?;
    println!("Converted to Cooklang:");
    println!("{}", cooklang);

    Ok(())
}

//! Basic usage of the RecipeImporter builder API
//!
//! This example demonstrates the three main use cases:
//! 1. URL → Cooklang: Fetch and convert a recipe
//! 2. URL → Recipe: Extract recipe without conversion
//! 3. Markdown → Cooklang: Convert existing markdown

use cooklang_import::{ImportResult, RecipeImporter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use Case 1: URL → Cooklang
    println!("=== Use Case 1: URL → Cooklang ===");
    let result = RecipeImporter::builder()
        .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
        .build()
        .await?;

    match result {
        ImportResult::Cooklang(cooklang) => {
            println!("Recipe in Cooklang format:");
            println!("{}", cooklang);
        }
        ImportResult::Recipe(_) => unreachable!(),
    }

    println!("\n=== Use Case 2: URL → Recipe (extract only) ===");
    let result = RecipeImporter::builder()
        .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
        .extract_only()
        .build()
        .await?;

    match result {
        ImportResult::Recipe(recipe) => {
            println!("Recipe name: {}", recipe.name);
            println!("Ingredients:");
            for ingredient in &recipe.ingredients {
                println!("  - {}", ingredient);
            }
            println!("\nInstructions:\n{}", recipe.instructions);
        }
        ImportResult::Cooklang(_) => unreachable!(),
    }

    // Use Case 3: Markdown → Cooklang
    println!("\n=== Use Case 3: Markdown → Cooklang ===");
    let ingredients = r#"
## Ingredients
- 2 eggs
- 1 cup flour
- 1/2 cup milk
- 1 tsp salt
"#;

    let instructions = r#"
## Instructions
1. Mix all dry ingredients together
2. Add eggs and milk
3. Stir until smooth
4. Bake at 350°F for 30 minutes
"#;

    let result = RecipeImporter::builder()
        .text(&format!("{}\n\n{}", ingredients, instructions))
        .build()
        .await?;

    match result {
        ImportResult::Cooklang(cooklang) => {
            println!("Converted to Cooklang:");
            println!("{}", cooklang);
        }
        ImportResult::Recipe(_) => unreachable!(),
    }

    Ok(())
}

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
        ImportResult::Cooklang {
            content,
            conversion_metadata,
        } => {
            println!("Recipe in Cooklang format:");
            println!("{}", content);
            if let Some(meta) = conversion_metadata {
                println!("\n--- Conversion Metadata ---");
                println!("Model: {:?}", meta.model_version);
                println!("Input tokens: {:?}", meta.tokens_used.input_tokens);
                println!("Output tokens: {:?}", meta.tokens_used.output_tokens);
                println!("Latency: {}ms", meta.latency_ms);
            }
        }
        ImportResult::Components(_) => unreachable!(),
    }

    println!("\n=== Use Case 2: URL → Recipe (extract only) ===");
    let result = RecipeImporter::builder()
        .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
        .extract_only()
        .build()
        .await?;

    match result {
        ImportResult::Components(components) => {
            println!("Recipe name: {}", components.name);
            println!("\nText:\n{}", components.text);
            println!("\nMetadata:\n{}", components.metadata);
        }
        ImportResult::Cooklang { .. } => unreachable!(),
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
        ImportResult::Cooklang { content, .. } => {
            println!("Converted to Cooklang:");
            println!("{}", content);
        }
        ImportResult::Components(_) => unreachable!(),
    }

    Ok(())
}

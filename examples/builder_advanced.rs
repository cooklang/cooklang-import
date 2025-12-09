//! Advanced builder API usage
//!
//! This example demonstrates advanced features like:
//! - Custom LLM provider configuration
//! - Timeout settings
//! - Error handling
//!
//! Note: Custom providers require a config.toml file with provider settings.
//! See config.toml.example in the repository root.

use cooklang_import::{ImportResult, RecipeImporter};
use std::time::Duration;

// LlmProvider is available for use when you have config.toml configured
#[allow(unused_imports)]
use cooklang_import::LlmProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Custom timeout
    println!("=== Builder with Custom Timeout ===");
    let result = RecipeImporter::builder()
        .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
        .timeout(Duration::from_secs(60))
        .build()
        .await?;

    match result {
        ImportResult::Cooklang {
            content,
            conversion_metadata,
        } => {
            println!("Successfully imported recipe with custom timeout:");
            println!("Recipe length: {} bytes", content.len());
            if let Some(meta) = conversion_metadata {
                println!("Model: {:?}", meta.model_version);
                println!("Latency: {}ms", meta.latency_ms);
            }
        }
        ImportResult::Recipe(_) => unreachable!(),
    }

    // Example 2: Custom provider (requires config.toml)
    println!("\n=== Builder with Custom Provider ===");
    println!("Note: This requires a config.toml with Anthropic configuration");

    // Uncomment if you have Anthropic configured:
    // let result = RecipeImporter::builder()
    //     .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
    //     .provider(LlmProvider::Anthropic)
    //     .build()
    //     .await?;
    //
    // match result {
    //     ImportResult::Cooklang(cooklang) => {
    //         println!("Successfully imported recipe with Anthropic:");
    //         println!("{}", cooklang);
    //     }
    //     ImportResult::Recipe(_) => unreachable!(),
    // }

    // Example 2: Error handling
    println!("\n=== Error Handling Example ===");

    // This will fail because no source is specified
    let result = RecipeImporter::builder().build().await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }

    // This will fail because markdown + extract_only is invalid
    let result = RecipeImporter::builder()
        .text("ingredients\n\ninstructions")
        .extract_only()
        .build()
        .await;

    match result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected error: {}", e),
    }

    // Example 3: Method chaining with multiple options
    println!("\n=== Method Chaining ===");
    let result = RecipeImporter::builder()
        .url("https://www.bbcgoodfood.com/recipes/classic-cottage-pie")
        .timeout(Duration::from_secs(90))
        .build()
        .await?;

    println!("Successfully chained methods and imported recipe");

    if let ImportResult::Cooklang { content, .. } = result {
        println!("Recipe length: {} bytes", content.len());
    }

    // Example 4: Using Ollama (local LLM)
    println!("\n=== Using Ollama (Local LLM) ===");
    println!("Note: This requires Ollama running locally with config.toml");

    // Uncomment if you have Ollama configured:
    // let result = RecipeImporter::builder()
    //     .text("2 eggs\n1 cup flour", "Mix and bake at 350°F")
    //     .provider(LlmProvider::Ollama)
    //     .build()
    //     .await?;
    //
    // if let ImportResult::Cooklang(cooklang) = result {
    //     println!("Converted with Ollama:");
    //     println!("{}", cooklang);
    // }

    Ok(())
}

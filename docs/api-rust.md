# Rust Library API

Use `cooklang-import` as a library in your Rust projects.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cooklang-import = "0.8.6"
tokio = { version = "1.0", features = ["full"] }
```

## API Overview

The library provides three API styles:

1. **Builder API** (recommended) - Flexible, type-safe builder pattern
2. **Convenience Functions** - Simple high-level functions
3. **Low-level API** - Direct access to components

## Builder API

### URL to Cooklang

```rust
use cooklang_import::{RecipeImporter, ImportResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = RecipeImporter::builder()
        .url("https://example.com/recipe")
        .build()
        .await?;

    match result {
        ImportResult::Cooklang(cooklang) => println!("{}", cooklang),
        ImportResult::Recipe(_) => unreachable!(),
    }

    Ok(())
}
```

### URL to Recipe (Extract Only)

```rust
let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .extract_only()
    .build()
    .await?;

match result {
    ImportResult::Recipe(recipe) => {
        println!("Title: {}", recipe.name);
        println!("Ingredients: {:?}", recipe.ingredients);
        println!("Instructions: {}", recipe.instructions);
    }
    ImportResult::Cooklang(_) => unreachable!(),
}
```

### Text to Cooklang

```rust
let recipe_text = "Take 2 eggs and 1 cup of flour. Mix and bake at 350F.";
let result = RecipeImporter::builder()
    .text(recipe_text)
    .build()
    .await?;
```

### Image to Cooklang

Requires `GOOGLE_API_KEY` for OCR.

```rust
let result = RecipeImporter::builder()
    .image_path("/path/to/recipe.jpg")
    .build()
    .await?;
```

## Advanced Builder Options

### Custom Provider

Requires `config.toml` with provider settings.

```rust
use cooklang_import::{RecipeImporter, LlmProvider};

let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .provider(LlmProvider::Anthropic)
    .build()
    .await?;
```

### Custom Timeout

```rust
use std::time::Duration;

let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .timeout(Duration::from_secs(60))
    .build()
    .await?;
```

### Combined Options

```rust
let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .provider(LlmProvider::Ollama)
    .timeout(Duration::from_secs(120))
    .build()
    .await?;
```

## Convenience Functions

For simple use cases:

```rust
use cooklang_import::{
    import_from_url,
    extract_recipe_from_url,
    convert_text_to_cooklang,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch and convert to Cooklang
    let cooklang = import_from_url("https://example.com/recipe").await?;

    // Extract without conversion
    let recipe = extract_recipe_from_url("https://example.com/recipe").await?;

    // Convert plain text to Cooklang
    let recipe_text = "Take 2 eggs and 1 cup of flour. Mix and bake at 350F.";
    let cooklang = convert_text_to_cooklang(recipe_text).await?;

    Ok(())
}
```

## Available Providers

- `LlmProvider::OpenAI` - OpenAI GPT models (default if no config)
- `LlmProvider::Anthropic` - Claude models
- `LlmProvider::Google` - Gemini models
- `LlmProvider::AzureOpenAI` - Azure OpenAI service
- `LlmProvider::Ollama` - Local Llama models via Ollama

## Error Handling

```rust
use cooklang_import::{ImportError, RecipeImporter};

match RecipeImporter::builder().url("...").build().await {
    Ok(result) => println!("Success!"),
    Err(ImportError::FetchError(e)) => eprintln!("Network error: {}", e),
    Err(ImportError::NoExtractorMatched) => eprintln!("Could not parse recipe"),
    Err(ImportError::ConversionError(e)) => eprintln!("Conversion failed: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Examples

See the `examples/` directory:

- `builder_basic.rs` - Basic builder usage
- `simple_api.rs` - Using convenience functions
- `builder_advanced.rs` - Advanced features

Run examples:

```sh
cargo run --example builder_basic
cargo run --example simple_api
cargo run --example builder_advanced
```

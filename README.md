# cooklang-import
A command-line tool to import recipes into [Cooklang](https://cooklang.org/) format using AI-powered conversion.

## Features

- **Multi-provider AI support**: OpenAI, Anthropic Claude, Azure OpenAI, Google Gemini, and Ollama (local Llama models)
- **Automatic fallback**: Seamlessly switch between providers on failure
- **Flexible configuration**: TOML-based config with environment variable overrides
- **Smart extraction**: Supports JSON-LD, HTML class-based, and plain-text extraction
- **Metadata preservation**: Automatically extracts and includes recipe metadata
- **Local AI support**: Run completely offline with Ollama

## Getting started

### Prerequisites

1. [Rust](https://www.rust-lang.org/tools/install)
2. An AI provider (choose one or more):
   - **Free & Local**: [Ollama](https://ollama.ai) for running Llama models on your machine
   - **Cloud Options**:
     - [OpenAI API key](https://platform.openai.com/api-keys)
     - [Anthropic API key](https://console.anthropic.com/)
     - [Google AI API key](https://ai.google.dev/)
     - [Azure OpenAI](https://azure.microsoft.com/en-us/products/ai-services/openai-service)

### Installation

```sh
git clone https://github.com/cooklang/cooklang-import
cd cooklang-import
cargo install --path .
```

## Configuration

### Quick Start (Environment Variables Only)

Set your API key as an environment variable:

```sh
export OPENAI_API_KEY="your-api-key-here"
```

The tool will work immediately with OpenAI's GPT-4.1-mini model (October 2025).

### Advanced Configuration (config.toml)

For multi-provider support and advanced features, create a `config.toml` file:

```sh
cp config.toml.example config.toml
```

Edit `config.toml` to configure your preferred providers:

```toml
# Default provider to use
default_provider = "openai"

# OpenAI Configuration
[providers.openai]
enabled = true
model = "gpt-4.1-mini"  # Fast and cost-effective. Use "gpt-4.1-nano" for lowest latency
temperature = 0.7
max_tokens = 2000
# API key loaded from OPENAI_API_KEY environment variable
# or set here: api_key = "sk-..."

# Anthropic Claude Configuration
[providers.anthropic]
enabled = true
model = "claude-sonnet-4.5"  # Use "claude-haiku-4.5" for faster/cheaper option
temperature = 0.7
max_tokens = 4000
# API key loaded from ANTHROPIC_API_KEY environment variable

# Provider Fallback Configuration
[fallback]
enabled = true
order = ["openai", "anthropic"]
retry_attempts = 3
retry_delay_ms = 1000
```

### Configuration Priority

Configuration is loaded with the following priority (highest to lowest):

1. Environment variables (e.g., `OPENAI_API_KEY`, `COOKLANG__PROVIDERS__OPENAI__MODEL`)
2. `config.toml` file in current directory
3. Default values

### Environment Variable Format

For nested configuration, use double underscores:

```sh
export COOKLANG__PROVIDERS__OPENAI__MODEL="gpt-4.1-mini"
export COOKLANG__FALLBACK__ENABLED=true
```

## Usage Examples

### Use Case 1: URL → Cooklang (Default)

Fetch a recipe from a URL and convert to Cooklang format:

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala
```

### Use Case 2: URL → Recipe (Extract Only)

Download and extract recipe data without AI conversion:

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala --extract-only
```

This outputs the raw ingredients and instructions in markdown format without Cooklang markup.

### Use Case 3: Markdown → Cooklang

Convert structured markdown recipes to Cooklang format (when you have pre-separated ingredients and instructions):

```sh
cooklang-import --markdown \
  --ingredients "2 eggs\n1 cup flour\n1/2 cup milk" \
  --instructions "Mix dry ingredients. Add eggs and milk. Bake at 350°F for 30 minutes."
```

### Use Case 4: Text → Cooklang (NEW!)

Convert plain text recipes to Cooklang format (LLM will parse and structure the recipe):

```sh
cooklang-import --text "Take 2 eggs and 1 cup of flour. Mix them together and bake at 350°F for 30 minutes."
```

This is useful for unstructured recipe text where ingredients and instructions are not clearly separated.

### Advanced Options

#### Custom LLM Provider

Use a different LLM provider (requires config.toml):

```sh
cooklang-import https://example.com/recipe --provider anthropic
cooklang-import --markdown --ingredients "..." --instructions "..." --provider ollama
```

Available providers: `openai`, `anthropic`, `google`, `azure_openai`, `ollama`

#### Custom Timeout

Set a custom timeout for HTTP requests:

```sh
cooklang-import https://example.com/recipe --timeout 60
```

#### Combined Options

Combine multiple options:

```sh
cooklang-import https://example.com/recipe --provider anthropic --timeout 90
```

### CLI Help

For complete usage information:

```sh
cooklang-import --help
```

## Supported AI Providers

### OpenAI

- **Models**: gpt-4.1-mini (default, Oct 2025), gpt-4.1-nano (fastest), gpt-4o-mini, gpt-4o
- **Environment Variable**: `OPENAI_API_KEY`
- **Configuration**: See `config.toml.example`

### Anthropic Claude

- **Models**: claude-sonnet-4.5 (Sep 2025), claude-haiku-4.5 (fastest, Oct 2025), claude-opus-4.1
- **Environment Variable**: `ANTHROPIC_API_KEY`
- **Configuration**: See `config.toml.example`

### Azure OpenAI

- **Models**: Your deployed models (e.g., gpt-4, gpt-35-turbo)
- **Environment Variable**: `AZURE_OPENAI_API_KEY`
- **Required Config**: `endpoint`, `deployment_name`, `api_version`

### Google Gemini

- **Models**: gemini-2.5-flash (latest, Sep 2025), gemini-2.0-flash-lite
- **Environment Variable**: `GOOGLE_API_KEY`
- **Configuration**: See `config.toml.example`

### Ollama (Local Llama Models)

- **Models**: llama3, llama2, codellama, mixtral, and more
- **Requirements**: [Ollama](https://ollama.ai) installed locally
- **No API Key Required**: Runs entirely on your machine
- **Base URL**: `http://localhost:11434` (default)
- **Setup**:
  1. Install Ollama: `curl -fsSL https://ollama.ai/install.sh | sh`
  2. Pull a model: `ollama pull llama3`
  3. Configure in `config.toml` or start using immediately

## Provider Fallback

Enable automatic fallback between providers for reliability:

```toml
[fallback]
enabled = true
order = ["openai", "anthropic", "google"]
retry_attempts = 3
retry_delay_ms = 1000
```

When enabled, the tool will:
1. Try the primary provider with exponential backoff retries
2. If all retries fail, automatically switch to the next provider
3. Continue until a provider succeeds or all providers are exhausted

## Migration from Environment Variables Only

If you're upgrading from a version that only used environment variables:

1. **No action required** - Environment variables continue to work
2. **Optional**: Create `config.toml` for advanced features
3. Keep your `OPENAI_API_KEY` in environment variables for security

## Troubleshooting

### "OPENAI_API_KEY must be set"

Set your API key:
```sh
export OPENAI_API_KEY="your-key-here"
```

### "No providers available in fallback configuration"

Ensure at least one provider is:
- Enabled in `config.toml` (`enabled = true`)
- Included in `fallback.order`
- Has a valid API key configured

### Rate Limiting

If you encounter rate limits:
1. Enable fallback to use multiple providers
2. Increase `retry_delay_ms` in config
3. Use a different provider temporarily

## Library Usage

`cooklang-import` can also be used as a Rust library in your own projects.

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cooklang-import = "0.7.0"
tokio = { version = "1.0", features = ["full"] }
```

### API Overview

The library provides four main use cases:

1. **Builder API** (recommended): Flexible, type-safe builder pattern with fluent interface
2. **Convenience Functions**: Simple high-level functions for common use cases
3. **Low-level API**: Direct access to fetching and conversion functions

### Builder API

The builder API provides the most control and flexibility:

```rust
use cooklang_import::{RecipeImporter, ImportResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use Case 1: URL → Cooklang
    let result = RecipeImporter::builder()
        .url("https://example.com/recipe")
        .build()
        .await?;

    match result {
        ImportResult::Cooklang(cooklang) => println!("{}", cooklang),
        ImportResult::Recipe(_) => unreachable!(),
    }

    // Use Case 2: URL → Recipe (extract only, no conversion)
    let result = RecipeImporter::builder()
        .url("https://example.com/recipe")
        .extract_only()
        .build()
        .await?;

    match result {
        ImportResult::Recipe(recipe) => {
            println!("Ingredients: {}", recipe.ingredients);
            println!("Instructions: {}", recipe.instructions);
        }
        ImportResult::Cooklang(_) => unreachable!(),
    }

    // Use Case 3: Markdown → Cooklang (structured)
    let result = RecipeImporter::builder()
        .markdown("2 eggs\n1 cup flour", "Mix and bake")
        .build()
        .await?;

    // Use Case 4: Text → Cooklang (unstructured)
    let recipe_text = "Take 2 eggs and 1 cup of flour. Mix and bake at 350F.";
    let result = RecipeImporter::builder()
        .text(recipe_text)
        .build()
        .await?;

    Ok(())
}
```

### Convenience Functions

For simple use cases, use the convenience functions:

```rust
use cooklang_import::{
    import_from_url,
    extract_recipe_from_url,
    convert_markdown_to_cooklang,
    convert_text_to_cooklang,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch and convert to Cooklang
    let cooklang = import_from_url("https://example.com/recipe").await?;

    // Extract without conversion
    let recipe = extract_recipe_from_url("https://example.com/recipe").await?;

    // Convert markdown to Cooklang (structured)
    let cooklang = convert_markdown_to_cooklang(
        "2 eggs\n1 cup flour",
        "Mix and bake"
    ).await?;

    // Convert plain text to Cooklang (unstructured)
    let recipe_text = "Take 2 eggs and 1 cup of flour. Mix and bake at 350F.";
    let cooklang = convert_text_to_cooklang(recipe_text).await?;

    Ok(())
}
```

### Advanced Builder Options

The builder supports additional configuration including custom providers and timeouts:

```rust
use cooklang_import::{RecipeImporter, LlmProvider};
use std::time::Duration;

// Use a custom LLM provider (requires config.toml with provider settings)
let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .provider(LlmProvider::Anthropic)
    .build()
    .await?;

// Set a custom timeout for network requests
let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .timeout(Duration::from_secs(60))
    .build()
    .await?;

// Combine both options
let result = RecipeImporter::builder()
    .url("https://example.com/recipe")
    .provider(LlmProvider::Ollama)
    .timeout(Duration::from_secs(120))
    .build()
    .await?;
```

**Available Providers:**
- `LlmProvider::OpenAI` - OpenAI GPT models (default if no config)
- `LlmProvider::Anthropic` - Claude models
- `LlmProvider::Google` - Gemini models
- `LlmProvider::AzureOpenAI` - Azure OpenAI service
- `LlmProvider::Ollama` - Local Llama models via Ollama

**Note:** Custom providers require a `config.toml` file with appropriate provider configuration. See the main Configuration section for details.

### Error Handling

The library provides structured error types:

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

### Examples

See the `examples/` directory for complete examples:

- `builder_basic.rs` - Basic builder usage for all three use cases
- `simple_api.rs` - Using convenience functions
- `builder_advanced.rs` - Advanced features like custom providers and error handling

Run examples with:
```sh
cargo run --example builder_basic
cargo run --example simple_api
cargo run --example builder_advanced
```

## Development

Run tests:
```sh
cargo test
```

Run with debug logging:
```sh
RUST_LOG=debug cooklang-import <url>
```


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

The tool will work immediately with OpenAI's GPT-4 model.

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
model = "gpt-4"
temperature = 0.7
max_tokens = 2000
# API key loaded from OPENAI_API_KEY environment variable
# or set here: api_key = "sk-..."

# Anthropic Claude Configuration
[providers.anthropic]
enabled = true
model = "claude-3-5-sonnet-20250929"
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
export COOKLANG__PROVIDERS__OPENAI__MODEL="gpt-4-turbo"
export COOKLANG__FALLBACK__ENABLED=true
```

## Usage Examples

### Basic Usage

Scrape a recipe from a webpage and convert to Cooklang:

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala
```

### Download Only (No Conversion)

Download and extract recipe data without AI conversion:

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala --download-only
```

This outputs the raw ingredients and instructions without Cooklang markup.

## Supported AI Providers

### OpenAI

- **Models**: GPT-4, GPT-4-turbo, GPT-3.5-turbo
- **Environment Variable**: `OPENAI_API_KEY`
- **Configuration**: See `config.toml.example`

### Anthropic Claude

- **Models**: claude-3-5-sonnet, claude-3-opus, claude-3-sonnet
- **Environment Variable**: `ANTHROPIC_API_KEY`
- **Configuration**: See `config.toml.example`

### Azure OpenAI

- **Models**: Your deployed models (e.g., gpt-4, gpt-35-turbo)
- **Environment Variable**: `AZURE_OPENAI_API_KEY`
- **Required Config**: `endpoint`, `deployment_name`, `api_version`

### Google Gemini

- **Models**: gemini-pro, gemini-pro-vision
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

## Development

Run tests:
```sh
cargo test
```

Run with debug logging:
```sh
RUST_LOG=debug cooklang-import <url>
```


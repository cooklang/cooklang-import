# cooklang-import

A tool to import recipes into [Cooklang](https://cooklang.org/) format using AI-powered conversion. Hosted version runs via https://cook.md

## Features

- **Multi-provider AI support**: OpenAI, Anthropic Claude, Azure OpenAI, Google Gemini, and Ollama
- **Automatic fallback**: Seamlessly switch between providers on failure
- **Smart extraction**: JSON-LD, MicroData, HTML class extractors, and LLM fallback
- **Multiple input types**: URLs, plain text, and images (via OCR)
- **Local AI support**: Run completely offline with Ollama

See [architecture.md](architecture.md) for system design.

## Installation

### CLI

```sh
git clone https://github.com/cooklang/cooklang-import
cd cooklang-import
cargo install --path .
```

### Rust Library

```toml
[dependencies]
cooklang-import = "0.8.6"
```

See [docs/api-rust.md](docs/api-rust.md) for library usage.

### Mobile SDKs

- [iOS (Swift)](docs/sdk-ios.md) - Swift Package Manager or manual installation
- [Android (Kotlin)](docs/sdk-android.md) - GitHub Packages (Maven) or manual installation

## Quick Start

Set your API key:

```sh
export OPENAI_API_KEY="your-api-key-here"
```

The tool works immediately with OpenAI's GPT-4.1-mini model.

## Usage

### URL to Cooklang

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala
```

### URL to Recipe (Extract Only)

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala --extract-only
```

### Text to Cooklang

```sh
cooklang-import --text "Take 2 eggs and 1 cup of flour. Mix and bake at 350°F for 30 minutes."
```

### Image to Cooklang

Requires `GOOGLE_API_KEY` for OCR.

```sh
cooklang-import --image /path/to/recipe-photo.jpg
```

### Options

```sh
cooklang-import --help                           # Full usage info
cooklang-import <url> --provider anthropic       # Use specific provider
cooklang-import <url> --timeout 60               # Custom timeout (seconds)
```

## Configuration

### Basic (config.toml)

```sh
cp config.toml.example config.toml
```

```toml
default_provider = "openai"

[providers.openai]
enabled = true
model = "gpt-4.1-mini"

[providers.anthropic]
enabled = true
model = "claude-sonnet-4.5"

[fallback]
enabled = true
order = ["openai", "anthropic"]
```

See [docs/providers.md](docs/providers.md) for all provider options.

### Configuration Priority

1. Environment variables (e.g., `OPENAI_API_KEY`)
2. `config.toml` file
3. Default values

## Documentation

| Document | Description |
|----------|-------------|
| [architecture.md](architecture.md) | System design and project structure |
| [docs/providers.md](docs/providers.md) | AI provider configuration |
| [docs/api-rust.md](docs/api-rust.md) | Rust library API |
| [docs/sdk-ios.md](docs/sdk-ios.md) | iOS/Swift SDK |
| [docs/sdk-android.md](docs/sdk-android.md) | Android/Kotlin SDK |
| [docs/troubleshooting.md](docs/troubleshooting.md) | Common issues and solutions |

## Development

```sh
cargo test                              # Run tests
RUST_LOG=debug cooklang-import <url>    # Debug logging
```

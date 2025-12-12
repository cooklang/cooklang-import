# cooklang-import
A command-line tool to import recipes into [Cooklang](https://cooklang.org/) format using AI-powered conversion.

## Features

- **Multi-provider AI support**: OpenAI, Anthropic Claude, Azure OpenAI, Google Gemini, and Ollama (local Llama models)
- **Automatic fallback**: Seamlessly switch between providers on failure
- **Flexible configuration**: TOML-based config with environment variable overrides
- **Smart extraction**: Modular pipeline with JSON-LD, MicroData, HTML class extractors, and LLM fallback
- **Multiple input types**: URLs, plain text, and images (via OCR)
- **Metadata preservation**: Automatically extracts and includes recipe metadata as YAML frontmatter
- **Local AI support**: Run completely offline with Ollama

See [architecture.md](architecture.md) for detailed system design and project structure.

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

### Use Case 3: Text → Cooklang

Convert plain text recipes to Cooklang format:

```sh
cooklang-import --text "Take 2 eggs and 1 cup of flour. Mix them together and bake at 350°F for 30 minutes."
```

This uses LLM to parse unstructured recipe text into Cooklang format.

### Use Case 4: Image → Cooklang

Convert recipe images to Cooklang format using OCR:

```sh
cooklang-import --image /path/to/recipe-photo.jpg
```

Requires `GOOGLE_API_KEY` for Google Cloud Vision OCR. Multiple images can be processed together.

### Advanced Options

#### Custom LLM Provider

Use a different LLM provider (requires config.toml):

```sh
cooklang-import https://example.com/recipe --provider anthropic
cooklang-import --text "..." --provider ollama
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
            println!("Title: {}", recipe.name);
            println!("Ingredients: {:?}", recipe.ingredients);
            println!("Instructions: {}", recipe.instructions);
        }
        ImportResult::Cooklang(_) => unreachable!(),
    }

    // Use Case 3: Text → Cooklang
    let recipe_text = "Take 2 eggs and 1 cup of flour. Mix and bake at 350F.";
    let result = RecipeImporter::builder()
        .text(recipe_text)
        .build()
        .await?;

    // Use Case 4: Image → Cooklang (requires GOOGLE_API_KEY)
    let result = RecipeImporter::builder()
        .image_path("/path/to/recipe.jpg")
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

## Mobile SDKs

`cooklang-import` provides native SDKs for iOS and Android via UniFFI bindings.

### iOS (Swift)

#### Installation via Swift Package Manager

Add the package to your Xcode project:

1. **Xcode UI**: File → Add Package Dependencies → Enter URL:
   ```
   https://github.com/cooklang/cooklang-import
   ```

2. **Package.swift**: Add to your dependencies:
   ```swift
   dependencies: [
       .package(url: "https://github.com/cooklang/cooklang-import.git", from: "0.8.0")
   ]
   ```
   And add to your target:
   ```swift
   .target(
       name: "YourApp",
       dependencies: [
           .product(name: "CooklangImport", package: "cooklang-import")
       ]
   )
   ```

#### Manual Installation

1. Download `CooklangImport-ios.zip` from the [latest release](https://github.com/cooklang/cooklang-import/releases)
2. Extract and add `CooklangImportFFI.xcframework` to your Xcode project
3. Add the Swift bindings file from `swift/` to your project

#### Usage in Swift

```swift
import CooklangImport

// Simple import from URL (uses structured data extraction, no LLM needed)
func importRecipe() async throws {
    let cooklang = try await simpleImport(url: "https://example.com/recipe")
    print(cooklang)
}

// Import with LLM configuration (for text/image conversion or fallback)
func importWithLlm() async throws {
    let config = FfiImportConfig(
        provider: .anthropic,
        apiKey: "your-api-key",
        model: nil,  // Uses default model
        timeoutSeconds: 30,
        extractOnly: false
    )

    let result = try await importFromUrl(
        url: "https://example.com/recipe",
        config: config
    )
    print(result)
}

// Extract recipe without Cooklang conversion
func extractOnly() async throws {
    let config = FfiImportConfig(
        provider: nil,
        apiKey: nil,
        model: nil,
        timeoutSeconds: 30,
        extractOnly: true
    )

    let recipe = try await importFromUrl(
        url: "https://example.com/recipe",
        config: config
    )
    // Returns structured recipe data
}

// Convert plain text to Cooklang
func convertText() async throws {
    let config = FfiImportConfig(
        provider: .openai,
        apiKey: "your-api-key",
        model: "gpt-4.1-mini",
        timeoutSeconds: 30,
        extractOnly: false
    )

    let text = "Take 2 eggs and 1 cup flour. Mix and bake at 350F for 30 min."
    let cooklang = try await importFromText(text: text, config: config)
    print(cooklang)
}
```

#### SwiftUI Example

```swift
import SwiftUI
import CooklangImport

struct RecipeImportView: View {
    @State private var url = ""
    @State private var result = ""
    @State private var isLoading = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 16) {
            TextField("Recipe URL", text: $url)
                .textFieldStyle(.roundedBorder)

            Button("Import Recipe") {
                Task {
                    await importRecipe()
                }
            }
            .disabled(isLoading || url.isEmpty)

            if isLoading {
                ProgressView()
            }

            if let error = error {
                Text(error)
                    .foregroundColor(.red)
            }

            ScrollView {
                Text(result)
                    .font(.system(.body, design: .monospaced))
            }
        }
        .padding()
    }

    func importRecipe() async {
        isLoading = true
        error = nil

        do {
            result = try await simpleImport(url: url)
        } catch {
            self.error = error.localizedDescription
        }

        isLoading = false
    }
}
```

#### Available LLM Providers (iOS)

```swift
enum FfiLlmProvider {
    case openai      // Requires OPENAI_API_KEY or apiKey parameter
    case anthropic   // Requires ANTHROPIC_API_KEY or apiKey parameter
    case google      // Requires GOOGLE_API_KEY or apiKey parameter
    case azureOpenai // Requires additional Azure configuration
    case ollama      // Local models via Ollama
}
```

---

### Android (Kotlin)

#### Installation via GitHub Packages (Maven)

1. Add the GitHub Packages repository to your `settings.gradle.kts`:

```kotlin
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()

        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/cooklang/cooklang-import")
            credentials {
                // Use gradle.properties or environment variables
                username = project.findProperty("gpr.user") as String?
                    ?: System.getenv("GITHUB_ACTOR")
                password = project.findProperty("gpr.key") as String?
                    ?: System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
```

2. Add the dependency to your app's `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.cooklang:cooklang-import:0.8.0")
}
```

3. Configure credentials in `~/.gradle/gradle.properties`:

```properties
gpr.user=YOUR_GITHUB_USERNAME
gpr.key=YOUR_GITHUB_TOKEN
```

> **Note:** You need a GitHub personal access token with `read:packages` scope.

#### Manual Installation

1. Download `cooklang-import-android.zip` from the [latest release](https://github.com/cooklang/cooklang-import/releases)
2. Extract and copy the module to your project
3. Add to `settings.gradle.kts`:
   ```kotlin
   include(":cooklang-import-android")
   ```
4. Add dependency:
   ```kotlin
   implementation(project(":cooklang-import-android"))
   ```

#### Usage in Kotlin

```kotlin
import org.cooklang.import.*

// Simple import from URL (uses structured data extraction)
suspend fun importRecipe(url: String): String {
    return simpleImport(url)
}

// Import with LLM configuration
suspend fun importWithLlm(url: String, apiKey: String): String {
    val config = FfiImportConfig(
        provider = FfiLlmProvider.ANTHROPIC,
        apiKey = apiKey,
        model = null,  // Uses default model
        timeoutSeconds = 30u,
        extractOnly = false
    )

    return importFromUrl(url, config)
}

// Extract recipe without Cooklang conversion
suspend fun extractOnly(url: String, apiKey: String): String {
    val config = FfiImportConfig(
        provider = null,
        apiKey = null,
        model = null,
        timeoutSeconds = 30u,
        extractOnly = true
    )

    return importFromUrl(url, config)
}

// Convert plain text to Cooklang
suspend fun convertText(text: String, apiKey: String): String {
    val config = FfiImportConfig(
        provider = FfiLlmProvider.OPENAI,
        apiKey = apiKey,
        model = "gpt-4.1-mini",
        timeoutSeconds = 30u,
        extractOnly = false
    )

    return importFromText(text, config)
}
```

#### Jetpack Compose Example

```kotlin
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.cooklang.import.*

@Composable
fun RecipeImportScreen() {
    var url by remember { mutableStateOf("") }
    var result by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        OutlinedTextField(
            value = url,
            onValueChange = { url = it },
            label = { Text("Recipe URL") },
            modifier = Modifier.fillMaxWidth()
        )

        Button(
            onClick = {
                scope.launch {
                    isLoading = true
                    error = null
                    try {
                        result = simpleImport(url)
                    } catch (e: Exception) {
                        error = e.message
                    }
                    isLoading = false
                }
            },
            enabled = !isLoading && url.isNotBlank(),
            modifier = Modifier.fillMaxWidth()
        ) {
            if (isLoading) {
                CircularProgressIndicator(
                    modifier = Modifier.size(20.dp),
                    strokeWidth = 2.dp
                )
            } else {
                Text("Import Recipe")
            }
        }

        error?.let {
            Text(
                text = it,
                color = MaterialTheme.colorScheme.error
            )
        }

        Text(
            text = result,
            style = MaterialTheme.typography.bodyMedium,
            fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
        )
    }
}
```

#### Available LLM Providers (Android)

```kotlin
enum class FfiLlmProvider {
    OPENAI,       // Requires OPENAI_API_KEY or apiKey parameter
    ANTHROPIC,    // Requires ANTHROPIC_API_KEY or apiKey parameter
    GOOGLE,       // Requires GOOGLE_API_KEY or apiKey parameter
    AZURE_OPENAI, // Requires additional Azure configuration
    OLLAMA        // Local models via Ollama
}
```

#### ProGuard Rules

If you're using ProGuard or R8 minification, the library includes consumer rules automatically. If you need to add them manually:

```proguard
-keep class org.cooklang.** { *; }
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
```

---

### API Reference (Both Platforms)

#### Functions

| Function | Description |
|----------|-------------|
| `simpleImport(url)` | Import recipe from URL using structured data extraction (no LLM required) |
| `importFromUrl(url, config)` | Import recipe from URL with LLM configuration |
| `importFromText(text, config)` | Convert plain text to Cooklang format |

#### FfiImportConfig

| Field | Type | Description |
|-------|------|-------------|
| `provider` | `FfiLlmProvider` | LLM provider to use |
| `apiKey` | `String?` | API key (optional if set via environment) |
| `model` | `String?` | Model name (uses provider default if nil) |
| `timeoutSeconds` | `UInt32` | Request timeout in seconds |
| `extractOnly` | `Bool` | If true, returns extracted recipe without Cooklang conversion |

#### Error Handling

Both platforms throw exceptions/errors that should be caught:

**Swift:**
```swift
do {
    let result = try await simpleImport(url: url)
} catch {
    print("Import failed: \(error)")
}
```

**Kotlin:**
```kotlin
try {
    val result = simpleImport(url)
} catch (e: Exception) {
    println("Import failed: ${e.message}")
}
```

---

### Building from Source

#### iOS

```bash
./scripts/build-ios.sh
```

Output in `target/ios/`:
- `CooklangImportFFI.xcframework` - XCFramework for all iOS targets
- `CooklangImport/` - Swift Package ready to use
- `swift/` - Swift binding files

#### Android

```bash
./scripts/build-android.sh
```

Output in `target/android/`:
- `cooklang-import-android/` - Android library module
- `jniLibs/` - Native libraries for all architectures
- `kotlin/` - Kotlin binding files

#### Supported Architectures

**iOS:**
- arm64 (devices)
- arm64 (simulator, Apple Silicon Macs)
- x86_64 (simulator, Intel Macs)

**Android:**
- arm64-v8a (64-bit ARM)
- armeabi-v7a (32-bit ARM)
- x86_64 (64-bit x86)

## Development

Run tests:
```sh
cargo test
```

Run with debug logging:
```sh
RUST_LOG=debug cooklang-import <url>
```


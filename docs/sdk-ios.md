# iOS SDK

Native Swift bindings for `cooklang-import` via UniFFI.

## Installation

### Swift Package Manager (Recommended)

**Xcode UI:** File → Add Package Dependencies → Enter URL:
```
https://github.com/cooklang/cooklang-import
```

**Package.swift:**
```swift
dependencies: [
    .package(url: "https://github.com/cooklang/cooklang-import.git", from: "0.8.6")
]
```

Add to your target:
```swift
.target(
    name: "YourApp",
    dependencies: [
        .product(name: "CooklangImport", package: "cooklang-import")
    ]
)
```

### Manual Installation

1. Download `CooklangImport-ios.zip` from the [latest release](https://github.com/cooklang/cooklang-import/releases)
2. Extract and add `CooklangImportFFI.xcframework` to your Xcode project
3. Add the Swift bindings file from `swift/` to your project

## Usage

### Simple Import (No LLM Required)

```swift
import CooklangImport

func importRecipe() async throws {
    let cooklang = try await simpleImport(url: "https://example.com/recipe")
    print(cooklang)
}
```

### Import with LLM Configuration

```swift
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
```

### Extract Recipe Without Conversion (No LLM Required)

```swift
func extractOnly() async throws {
    let recipe = try await simpleExtract(url: "https://example.com/recipe")
    // Returns structured recipe data without Cooklang conversion
}
```

### Convert Plain Text

```swift
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

## SwiftUI Example

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

## Available Providers

```swift
enum FfiLlmProvider {
    case openai      // Requires OPENAI_API_KEY or apiKey parameter
    case anthropic   // Requires ANTHROPIC_API_KEY or apiKey parameter
    case google      // Requires GOOGLE_API_KEY or apiKey parameter
    case azureOpenai // Requires additional Azure configuration
    case ollama      // Local models via Ollama
}
```

## Error Handling

```swift
do {
    let result = try await simpleImport(url: url)
} catch {
    print("Import failed: \(error)")
}
```

## Building from Source

```bash
./scripts/build-ios.sh
```

Output in `target/ios/`:
- `CooklangImportFFI.xcframework` - XCFramework for all iOS targets
- `CooklangImport/` - Swift Package ready to use
- `swift/` - Swift binding files

### Supported Architectures

- arm64 (devices)
- arm64 (simulator, Apple Silicon Macs)
- x86_64 (simulator, Intel Macs)

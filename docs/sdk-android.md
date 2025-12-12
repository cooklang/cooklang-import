# Android SDK

Native Kotlin bindings for `cooklang-import` via UniFFI.

## Installation

### GitHub Packages (Maven)

1. Add the GitHub Packages repository to `settings.gradle.kts`:

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
                username = project.findProperty("gpr.user") as String?
                    ?: System.getenv("GITHUB_ACTOR")
                password = project.findProperty("gpr.key") as String?
                    ?: System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
```

2. Add the dependency to `build.gradle.kts`:

```kotlin
dependencies {
    implementation("org.cooklang:cooklang-import:0.8.6")
}
```

3. Configure credentials in `~/.gradle/gradle.properties`:

```properties
gpr.user=YOUR_GITHUB_USERNAME
gpr.key=YOUR_GITHUB_TOKEN
```

> **Note:** You need a GitHub personal access token with `read:packages` scope.

### Manual Installation

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

## Usage

### Simple Import (No LLM Required)

```kotlin
import org.cooklang.import.*

suspend fun importRecipe(url: String): String {
    return simpleImport(url)
}
```

### Import with LLM Configuration

```kotlin
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
```

### Extract Recipe Without Conversion (No LLM Required)

```kotlin
suspend fun extractOnly(url: String): String {
    val config = FfiImportConfig(
        provider = null,
        apiKey = null,
        model = null,
        timeoutSeconds = 30u,
        extractOnly = true
    )

    return importFromUrl(url, config)
    // Returns structured recipe data without Cooklang conversion
}
```

### Convert Plain Text

```kotlin
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

## Jetpack Compose Example

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

## Available Providers

```kotlin
enum class FfiLlmProvider {
    OPENAI,       // Requires OPENAI_API_KEY or apiKey parameter
    ANTHROPIC,    // Requires ANTHROPIC_API_KEY or apiKey parameter
    GOOGLE,       // Requires GOOGLE_API_KEY or apiKey parameter
    AZURE_OPENAI, // Requires additional Azure configuration
    OLLAMA        // Local models via Ollama
}
```

## Error Handling

```kotlin
try {
    val result = simpleImport(url)
} catch (e: Exception) {
    println("Import failed: ${e.message}")
}
```

## ProGuard Rules

The library includes consumer rules automatically. If manual configuration is needed:

```proguard
-keep class org.cooklang.** { *; }
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
```

## Building from Source

```bash
./scripts/build-android.sh
```

Output in `target/android/`:
- `cooklang-import-android/` - Android library module
- `jniLibs/` - Native libraries for all architectures
- `kotlin/` - Kotlin binding files

### Supported Architectures

- arm64-v8a (64-bit ARM)
- armeabi-v7a (32-bit ARM)
- x86_64 (64-bit x86)

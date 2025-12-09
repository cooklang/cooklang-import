use cooklang_import::{generate_frontmatter, ImportResult, LlmProvider, RecipeImporter};
use log::info;
use std::env;
use std::time::Duration;

fn print_help() {
    println!(
        r#"cooklang-import - Import recipes into Cooklang format using AI

USAGE:
    cooklang-import [URL] [OPTIONS]
    cooklang-import --markdown [OPTIONS]

USE CASES:
    1. URL → Cooklang (default):
       cooklang-import https://example.com/recipe

    2. URL → Recipe (extract only, no conversion):
       cooklang-import https://example.com/recipe --extract-only

    3. Text → Cooklang (convert plain text):
       cooklang-import --text "Take 2 eggs and 1 cup flour. Mix and bake at 350F."

    4. Image → Cooklang (OCR then convert):
       cooklang-import --image /path/to/recipe-image.jpg

OPTIONS:
    --extract-only      Extract recipe without converting to Cooklang format

    --text TEXT         Convert plain text recipe to Cooklang

    --image PATH        Convert recipe image to Cooklang (uses Google Vision OCR)
                        Requires GOOGLE_API_KEY environment variable

    --provider NAME     LLM provider to use (openai, anthropic, google, azure_openai, ollama)
                        Requires config.toml with provider configuration
    --timeout SECONDS   Timeout for HTTP requests in seconds (default: no timeout)

    --help, -h          Show this help message

EXAMPLES:
    # Basic import
    cooklang-importhttps://www.bbcgoodfood.com/recipes/slow-cooker-chilli-con-carne

    # Extract without conversion
    cooklang-import https://example.com/recipe --extract-only

    # Convert plain text
    cooklang-import --text "2 eggs, 1 cup flour. Mix and bake"

    # Convert recipe image
    cooklang-import --image recipe-photo.jpg

    # Use custom provider (requires config.toml)
    cooklang-import https://example.com/recipe --provider anthropic

    # Set custom timeout
    cooklang-import https://example.com/recipe --timeout 60

ENVIRONMENT VARIABLES:
    OPENAI_API_KEY      OpenAI API key (required for default provider)
    OPENAI_MODEL        OpenAI model to use (default: gpt-4)
    GOOGLE_API_KEY      Google Cloud Vision API key (required for --image)
    RUST_LOG            Set log level (debug, info, warn, error)

For more information, see: https://github.com/cooklang/cooklang-import
"#
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.len() == 1 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return Ok(());
    }

    // Parse flags
    let extract_only = args.contains(&"--extract-only".to_string())
        || args.contains(&"--download-only".to_string());
    let text_mode = args.contains(&"--text".to_string());
    let image_mode = args.contains(&"--image".to_string());

    // Parse provider option
    let provider = if let Some(idx) = args.iter().position(|arg| arg == "--provider") {
        let provider_name = args
            .get(idx + 1)
            .ok_or("--provider requires a provider name")?;
        Some(match provider_name.as_str() {
            "openai" => LlmProvider::OpenAI,
            "anthropic" => LlmProvider::Anthropic,
            "google" => LlmProvider::Google,
            "azure_openai" => LlmProvider::AzureOpenAI,
            "ollama" => LlmProvider::Ollama,
            _ => {
                return Err(format!(
                "Unknown provider: {}. Available: openai, anthropic, google, azure_openai, ollama",
                provider_name
            )
                .into())
            }
        })
    } else {
        None
    };

    // Parse timeout option
    let timeout = if let Some(idx) = args.iter().position(|arg| arg == "--timeout") {
        let timeout_str = args.get(idx + 1).ok_or("--timeout requires a number")?;
        let seconds: u64 = timeout_str
            .parse()
            .map_err(|_| format!("Invalid timeout value: {}", timeout_str))?;
        Some(Duration::from_secs(seconds))
    } else {
        None
    };

    // Build and execute based on use case
    let result = if image_mode {
        // Use Case 5: Image → Cooklang (OCR then convert)
        let image_path = if let Some(idx) = args.iter().position(|arg| arg == "--image") {
            args.get(idx + 1)
                .ok_or("--image requires a file path")?
                .clone()
        } else {
            return Err("--image mode requires a file path".into());
        };

        info!(
            "Converting image to Cooklang (image: {}, provider: {:?})",
            image_path, provider
        );

        let mut builder = RecipeImporter::builder().image_path(&image_path);

        if let Some(p) = provider {
            builder = builder.provider(p);
        }

        builder.build().await?
    } else if text_mode {
        // Use Case 4: Text → Cooklang
        let text = if let Some(idx) = args.iter().position(|arg| arg == "--text") {
            args.get(idx + 1).ok_or("--text requires a value")?.clone()
        } else {
            return Err("--text mode requires a text value".into());
        };

        info!("Converting text to Cooklang (provider: {:?})", provider);

        let mut builder = RecipeImporter::builder().text(&text);

        if let Some(p) = provider {
            builder = builder.provider(p);
        }

        builder.build().await?
    } else {
        // Use Case 1 or 2: URL-based
        let url = args
            .get(1)
            .filter(|arg| !arg.starts_with("--"))
            .ok_or("Please provide a URL as the first argument")?;

        info!(
            "Importing recipe from URL: {}, extract_only: {}, provider: {:?}, timeout: {:?}",
            url, extract_only, provider, timeout
        );

        let mut builder = RecipeImporter::builder().url(url);

        if extract_only {
            builder = builder.extract_only();
        }

        if let Some(p) = provider {
            builder = builder.provider(p);
        }

        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }

        builder.build().await?
    };

    // Format and print output
    match result {
        ImportResult::Cooklang {
            content,
            conversion_metadata,
        } => {
            println!("{}", content);
            // Log conversion metadata if available
            if let Some(meta) = conversion_metadata {
                eprintln!("\n--- Conversion Metadata ---");
                if let Some(model) = &meta.model_version {
                    eprintln!("Model: {}", model);
                }
                if let Some(input) = meta.tokens_used.input_tokens {
                    eprintln!("Input tokens: {}", input);
                }
                if let Some(output) = meta.tokens_used.output_tokens {
                    eprintln!("Output tokens: {}", output);
                }
                eprintln!("Latency: {}ms", meta.latency_ms);
            }
        }
        ImportResult::Recipe(recipe) => {
            // Build metadata including title
            let mut metadata = recipe.metadata.clone();
            if !recipe.name.is_empty() {
                metadata.insert("title".to_string(), recipe.name.clone());
            }

            // Build the output with frontmatter
            let mut output = generate_frontmatter(&metadata);

            // Add ingredients
            if !recipe.ingredients.is_empty() {
                output.push_str(&recipe.ingredients.join("\n"));
                output.push_str("\n\n");
            }

            // Add instructions
            if !recipe.instructions.is_empty() {
                output.push_str(&recipe.instructions);
            }

            println!("{}", output);
        }
    }

    Ok(())
}

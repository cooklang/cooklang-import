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

OPTIONS:
    --extract-only      Extract recipe without converting to Cooklang format

    --text TEXT         Convert plain text recipe to Cooklang

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

    # Use custom provider (requires config.toml)
    cooklang-import https://example.com/recipe --provider anthropic

    # Set custom timeout
    cooklang-import https://example.com/recipe --timeout 60

ENVIRONMENT VARIABLES:
    OPENAI_API_KEY      OpenAI API key (required for default provider)
    OPENAI_MODEL        OpenAI model to use (default: gpt-4)
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
    let result = if text_mode {
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
        ImportResult::Cooklang(cooklang) => {
            println!("{}", cooklang);
        }
        ImportResult::Recipe(recipe) => {
            // Build the output with frontmatter if metadata exists
            let mut output = generate_frontmatter(&recipe.metadata);

            output.push_str(&format!("# {}\n\n{}", recipe.name, recipe.content));

            println!("{}", output);
        }
    }

    Ok(())
}

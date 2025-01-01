use cooklang_import::{fetch_recipe, import_recipe};
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Get the URL and check for download-only flag
    let args: Vec<String> = env::args().collect();
    let url = args.get(1).ok_or("Please provide a URL as an argument")?;
    let download_only = args.contains(&"--download-only".to_string());

    info!(
        "Importing recipe from URL: {}, download_only: {}",
        url, download_only
    );

    // Import the recipe
    let result = if download_only {
        let recipe = fetch_recipe(url).await?;
        Ok(format!(
            "# {}\n\n## Ingredients\n{}\n\n## Instructions\n{}",
            recipe.name, recipe.ingredients, recipe.instructions
        ))
    } else {
        import_recipe(url).await
    };

    println!("{}", result?);

    Ok(())
}

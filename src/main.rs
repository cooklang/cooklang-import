use cooklang_import::{import_recipe, import_recipe_raw};
use std::env;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Get the URL and check for download-only flag
    let args: Vec<String> = env::args().collect();
    let url = args.get(1).ok_or("Please provide a URL as an argument")?;
    let download_only = args.contains(&"--download-only".to_string());

    info!("Importing recipe from URL: {}, download_only: {}", url, download_only);

    // Import the recipe
    let result = if download_only {
        import_recipe_raw(url)
    } else {
        import_recipe(url)
    };

    println!("{}", result?);

    Ok(())
}

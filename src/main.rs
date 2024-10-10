use cooklang_import::import_recipe;
use std::env;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Get the URL from command-line arguments
    let args: Vec<String> = env::args().collect();
    let url = args.get(1).ok_or("Please provide a URL as an argument")?;

    info!("Importing recipe from URL: {}", url);

    // Import the recipe
    let cooklang_recipe = import_recipe(url)?;
    println!("{}", cooklang_recipe);

    Ok(())
}

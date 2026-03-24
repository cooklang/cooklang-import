use cooklang_import::url_to_recipe;

#[tokio::main]
async fn main() {
    env_logger::init();

    let url = std::env::args().nth(1).unwrap_or_else(|| {
        "https://www.seriouseats.com/slow-cooker-lentil-soup-with-spinach-11931055".to_string()
    });
    println!("Fetching: {}", url);

    match url_to_recipe(&url).await {
        Ok(components) => {
            println!("=== NAME ===");
            println!("{:?}", components.name);
            println!("\n=== NAME LENGTH ===");
            println!("{}", components.name.len());
            println!("\n=== METADATA ===");
            println!("{}", components.metadata);
            println!("\n=== TEXT (first 500 chars) ===");
            println!("{}", &components.text[..components.text.len().min(500)]);
        }
        Err(e) => {
            println!("ERROR: {}", e);
        }
    }
}

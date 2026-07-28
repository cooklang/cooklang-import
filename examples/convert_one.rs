#[tokio::main]
async fn main() {
    let path = std::env::args().nth(1).expect("usage: convert_one <file>");
    let raw = std::fs::read_to_string(path).unwrap();
    // strip YAML frontmatter if present
    let body = match raw
        .strip_prefix("---\n")
        .and_then(|r| r.find("\n---\n").map(|i| &r[i + 5..]))
    {
        Some(b) => b.trim(),
        None => raw.trim(),
    };
    match cooklang_import::convert_text_local(body).await {
        Ok(out) => println!("{out}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

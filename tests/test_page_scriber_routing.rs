use cooklang_import::url_to_recipe;

/// This test verifies that the pipeline can fetch from a site that blocks bots
/// when page scriber is configured. Requires:
/// 1. Page scriber running at http://localhost:4000
/// 2. Config with page_scriber.url and page_scriber.domains set
#[tokio::test]
#[ignore] // Requires page scriber running locally
async fn test_seriouseats_via_page_scriber() {
    let url = "https://www.seriouseats.com/macau-pork-chop-sandwich-recipe-8605269";

    match url_to_recipe(url).await {
        Ok(result) => {
            println!("Name: {}", result.name);
            assert!(!result.name.is_empty(), "Recipe name should not be empty");
            assert!(!result.text.is_empty(), "Recipe text should not be empty");
        }
        Err(e) => {
            panic!("Failed to fetch recipe via page scriber: {}", e);
        }
    }
}

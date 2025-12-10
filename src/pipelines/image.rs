use super::RecipeComponents;
use crate::images_to_text::{self, ImageSource};
use std::error::Error;

pub async fn process(
    images: &[ImageSource],
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    let mut all_text = Vec::new();
    let mut sources = Vec::new();

    for image in images {
        let text = images_to_text::extract(image).await?;
        all_text.push(text);

        match image {
            ImageSource::Path(p) => sources.push(p.clone()),
            ImageSource::Base64(_) => sources.push("base64-image".to_string()),
        }
    }

    let combined = all_text.join("\n\n");
    let source = sources.join(", ");

    Ok(RecipeComponents {
        text: combined,
        metadata: format!("source: {}", source),
        name: String::new(), // Images typically don't have a name extracted
    })
}

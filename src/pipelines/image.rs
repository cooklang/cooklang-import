use crate::images_to_text::{self, ImageSource};
use std::error::Error;

pub async fn process(
    images: &[ImageSource],
) -> Result<String, Box<dyn Error + Send + Sync>> {
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

    // Format as text with frontmatter
    Ok(format!("---\nsource: {}\n---\n\n{}", source, combined))
}

use base64::{engine::general_purpose::STANDARD, Engine as _};
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::path::Path;
use tokio::fs;

/// Performs OCR on an image file using Google Cloud Vision API
///
/// # Arguments
/// * `image_path` - Path to the image file
///
/// # Returns
/// The extracted text from the image
///
/// # Errors
/// Returns an error if:
/// - The image file cannot be read
/// - The Google Vision API request fails
/// - The API key is not set
pub async fn ocr_image_file(image_path: &Path) -> Result<String, Box<dyn Error>> {
    // Read the image file
    let image_data = fs::read(image_path).await?;

    // Perform OCR
    ocr_image_data(&image_data).await
}

/// Performs OCR on image data using Google Cloud Vision API
///
/// # Arguments
/// * `image_data` - Raw image bytes
///
/// # Returns
/// The extracted text from the image
///
/// # Errors
/// Returns an error if:
/// - The Google Vision API request fails
/// - The GOOGLE_API_KEY environment variable is not set
pub async fn ocr_image_data(image_data: &[u8]) -> Result<String, Box<dyn Error>> {
    // Get API key from environment
    let api_key = std::env::var("GOOGLE_API_KEY")
        .map_err(|_| "GOOGLE_API_KEY environment variable not set")?;

    // Base64 encode the image
    let base64_image = STANDARD.encode(image_data);

    // Create request to Google Vision API
    let client = Client::new();
    let url = format!(
        "https://vision.googleapis.com/v1/images:annotate?key={}",
        api_key
    );

    let request_body = json!({
        "requests": [{
            "image": {
                "content": base64_image
            },
            "features": [{
                "type": "TEXT_DETECTION"
            }]
        }]
    });

    debug!("Sending OCR request to Google Vision API");

    let response = client.post(&url).json(&request_body).send().await?;

    // Check for HTTP errors
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("Google Vision API error ({}): {}", status, error_text).into());
    }

    let response_body: Value = response.json().await?;
    debug!("Google Vision API response: {:?}", response_body);

    // Extract text from response
    // The API returns all detected text in the first annotation's description
    let text = response_body["responses"][0]["fullTextAnnotation"]["text"]
        .as_str()
        .ok_or("No text found in image")?
        .to_string();

    if text.trim().is_empty() {
        return Err("No text detected in image".into());
    }

    debug!("Extracted text from image: {} characters", text.len());

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encoding() {
        let data = b"test data";
        let encoded = STANDARD.encode(data);
        assert!(!encoded.is_empty());
    }

    #[tokio::test]
    async fn test_ocr_requires_api_key() {
        // Clear the env var if it exists
        let original_key = std::env::var("GOOGLE_API_KEY").ok();
        std::env::remove_var("GOOGLE_API_KEY");

        let result = ocr_image_data(b"fake image data").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("GOOGLE_API_KEY"));

        // Restore original key if it existed
        if let Some(key) = original_key {
            std::env::set_var("GOOGLE_API_KEY", key);
        }
    }
}

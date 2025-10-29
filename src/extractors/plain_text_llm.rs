use crate::extractors::{Extractor, ParsingContext};
use crate::model::Recipe;
use log::info;
use reqwest::Client;
use scraper::{ElementRef, Html, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;

const PROMPT: &str = r#"
You're an expert in finding recipe ingredients and instructions from messy texts.
Sometimes the text is not a recipe, in that case specify that in error field.
Given the text output only this JSON without any other characters:

{
  "ingredients": [<LIST OF INGREDIENTS HERE>],
  "instructions": [<LIST OF INSTRUCTIONS HERE>],
  "error": "<ERROR MESSAGE HERE IF NO RECIPE>"
}
"#;

const MODEL: &str = "gpt-4o-mini";

pub struct PlainTextLlmExtractor;

#[async_trait::async_trait]
impl Extractor for PlainTextLlmExtractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>> {
        info!("Parsing with PlainTextLlmExtractor for {}", context.url);

        let document = &context.document;

        let texts = if env::var("PAGE_SCRIBER_URL").is_ok() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(fetch_inner_text(&context.url))
            })?
        } else {
            extract_inner_texts(document).join("\n")
        };

        let title = document
            .select(&scraper::Selector::parse("title").unwrap())
            .next()
            .map(|el| el.inner_html().trim().to_string())
            .unwrap_or_else(|| "Untitled Recipe".to_string());

        let json = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(fetch_json(texts))
        })?;

        if let Some(error) = json["error"].as_str() {
            if !error.is_empty() {
                return Err(error.into());
            }
        }

        let ingredients = json["ingredients"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join("\n");

        let instructions = json["instructions"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join(" ");

        // Combine into single content field
        let content = if !ingredients.is_empty() && !instructions.is_empty() {
            format!("{}\n\n{}", ingredients, instructions)
        } else if !ingredients.is_empty() {
            ingredients
        } else {
            instructions
        };

        Ok(Recipe {
            name: title,
            description: None,
            image: vec![],
            content,
            metadata: std::collections::HashMap::new(),
        })
    }
}

async fn fetch_inner_text(url: &str) -> Result<String, Box<dyn Error>> {
    let page_scriber_url = env::var("PAGE_SCRIBER_URL")?;
    let client = Client::new();
    let endpoint = format!("{page_scriber_url}/api/fetch-content");

    let response = client
        .post(&endpoint)
        .json(&ContentRequest {
            url: url.to_string(),
        })
        .send()
        .await?;

    // Check status code before attempting to parse JSON
    if !response.status().is_success() {
        return Err(format!(
            "Page scriber request failed with status: {}",
            response.status()
        )
        .into());
    }

    let content: ContentResponse = response.json().await?;

    Ok(content.content)
}

async fn fetch_json(texts: String) -> Result<Value, Box<dyn Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;

    // For testing environment, return mock data
    if api_key == "test_key" {
        return Ok(serde_json::json!({
            "ingredients": "pasta\nsauce",
            "instructions": ["Cook pasta with sauce"],
            "error": ""
        }));
    }

    let response = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "model": MODEL,
            "messages": [
                {
                    "role": "system",
                    "content": PROMPT
                },
                {
                    "role": "user",
                    "content": texts
                }
            ]
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to get response content")?;

    serde_json::from_str(content).map_err(|e| e.into())
}

fn extract_inner_texts(document: &Html) -> Vec<String> {
    let mut result = Vec::new();
    let root = document.root_element();
    extract_text_from_element(&root, &mut result);

    // Process the result to merge text between block markers
    let mut processed = Vec::new();
    let mut current_block = Vec::new();

    for text in result {
        match text.as_str() {
            "<BLOCK>" => {
                // Start a new block
                if !current_block.is_empty() {
                    let merged = current_block.join(" ").trim().to_string();
                    if !merged.is_empty() {
                        processed.push(merged);
                    }
                    current_block.clear();
                }
            }
            "</BLOCK>" => {
                // End the current block
                if !current_block.is_empty() {
                    let merged = current_block.join(" ").trim().to_string();
                    if !merged.is_empty() {
                        processed.push(merged);
                    }
                    current_block.clear();
                }
            }
            text => current_block.push(text.to_string()),
        }
    }

    // Handle any remaining text
    if !current_block.is_empty() {
        let merged = current_block.join(" ").trim().to_string();
        if !merged.is_empty() {
            processed.push(merged);
        }
    }

    // Remove any trailing empty lines
    while processed.last().is_some_and(|s| s.trim().is_empty()) {
        processed.pop();
    }

    processed
}

fn extract_text_from_element(element: &ElementRef, result: &mut Vec<String>) {
    // Skip hidden elements and script/style tags
    if is_hidden(element) || should_skip_element(element) {
        return;
    }

    let tag_name = element.value().name().to_lowercase();

    // Handle special elements
    if tag_name == "br" {
        result.push("<BLOCK>".to_string());
        return;
    }

    for child in element.children() {
        match child.value() {
            Node::Text(text) => {
                let trimmed = normalize_whitespace(text);
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
            }
            Node::Element(_) => {
                if let Some(child_ref) = ElementRef::wrap(child) {
                    extract_text_from_element(&child_ref, result);
                }
            }
            _ => {}
        }
    }

    // Add newline after block elements
    if is_block_element(&tag_name) {
        result.push("</BLOCK>".to_string());
    }
}

fn is_hidden(element: &ElementRef) -> bool {
    element.value().attr("hidden").is_some()
        || element
            .value()
            .attr("style")
            .map(|s| s.contains("display: none") || s.contains("visibility: hidden"))
            .unwrap_or(false)
}

fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "canvas"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "noscript"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tfoot"
            | "tr"
            | "ul"
            | "video"
    )
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn should_skip_element(element: &ElementRef) -> bool {
    let tag_name = element.value().name().to_lowercase();
    matches!(
        tag_name.as_str(),
        "script" | "style" | "noscript" | "iframe" | "canvas" | "svg"
    )
}

#[derive(Serialize)]
struct ContentRequest {
    url: String,
}

#[derive(Deserialize)]
struct ContentResponse {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_success() {
        let html = "<html><body>Test</body></html>";
        let document = Html::parse_document(html);
        let context = ParsingContext {
            url: "http://example.com".to_string(),
            document,
            texts: None,
        };
        let extractor = PlainTextLlmExtractor;
        // Set up environment for test
        env::set_var("OPENAI_API_KEY", "test_key");

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = extractor.parse(&context);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_parse() {
        // Set up environment for test
        env::set_var("OPENAI_API_KEY", "test_key");

        let html = r#"
            <html>
                <head><title>Test Recipe</title></head>
                <body>
                    <h1>Pasta Recipe</h1>
                    <p>Cook pasta with sauce</p>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let context = ParsingContext {
            url: "http://example.com".to_string(),
            document,
            texts: None,
        };
        let extractor = PlainTextLlmExtractor;

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = extractor.parse(&context).unwrap();
            assert_eq!(result.name, "Test Recipe");
            assert!(!result.content.is_empty());
        });
    }

    #[test]
    fn test_extract_inner_text() {
        let html = r#"
            <html>
                <body>
                    <div>Hello</div>
                    <p>World</p>
                    <span>Test</span>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let text = extract_inner_texts(&document);
        let joined = text.join(" ");
        assert_eq!(joined.trim(), "Hello World Test");
    }

    #[test]
    fn test_block_elements() {
        let html = r#"
            <div>Hello</div>
            <p>World</p>
            <span>Test</span>
        "#;
        let document = Html::parse_document(html);
        let texts = extract_inner_texts(&document);
        let joined = texts.join(" ");
        assert_eq!(joined.trim(), "Hello World Test");
    }

    #[test]
    fn test_hidden_elements() {
        let html = r#"
            <div>Visible</div>
            <div hidden>Hidden</div>
            <div style="display: none">Also Hidden</div>
        "#;
        let document = Html::parse_document(html);
        let texts = extract_inner_texts(&document);
        let joined = texts.join(" ");
        assert_eq!(joined.trim(), "Visible");
    }

    #[test]
    fn test_skip_script_elements() {
        let html = r#"
            <div>Visible content</div>
            <script>console.log('Skip this');</script>
            <style>body { color: red; }</style>
            <div>More content</div>
        "#;
        let document = Html::parse_document(html);
        let texts = extract_inner_texts(&document);
        let joined = texts.join(" ");
        assert_eq!(joined.trim(), "Visible content More content");
    }
}

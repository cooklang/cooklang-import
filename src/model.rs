use crate::pipelines::metadata_to_yaml;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone, Default)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    pub image: Vec<String>,
    pub ingredients: Vec<String>,
    pub instructions: String,
    pub metadata: HashMap<String, String>,
}

impl Recipe {
    /// Serialize Recipe to text format with YAML frontmatter
    pub fn to_text_with_metadata(&self) -> String {
        let mut output = String::new();

        // Build metadata entries including name
        let mut entries: Vec<(String, String)> = Vec::new();
        if !self.name.is_empty() {
            entries.push(("title".to_string(), self.name.clone()));
        }
        if let Some(desc) = &self.description {
            entries.push(("description".to_string(), desc.clone()));
        }
        if !self.image.is_empty() {
            entries.push(("__image__".to_string(), self.image.join(", ")));
        }
        for (key, value) in &self.metadata {
            entries.push((key.clone(), value.clone()));
        }

        // YAML frontmatter
        let yaml = metadata_to_yaml(&entries);
        if !yaml.is_empty() {
            output.push_str("---\n");
            output.push_str(&yaml);
            if !yaml.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("---\n\n");
        }

        // Ingredients (one per line)
        for ingredient in &self.ingredients {
            output.push_str(ingredient);
            output.push('\n');
        }

        // Blank line separator
        output.push('\n');

        // Instructions
        output.push_str(&self.instructions);

        output
    }

    /// Extract frontmatter and body from text format
    pub fn parse_text_format(text: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();
        let body;

        if let Some(stripped) = text.strip_prefix("---\n") {
            if let Some(end) = stripped.find("\n---\n") {
                let frontmatter = &stripped[..end];
                for line in frontmatter.lines() {
                    if let Some((key, value)) = line.split_once(": ") {
                        metadata.insert(key.to_string(), value.to_string());
                    }
                }
                body = stripped[end + 5..].to_string();
            } else {
                body = text.to_string();
            }
        } else {
            body = text.to_string();
        }

        (metadata, body)
    }
}

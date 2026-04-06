pub mod image;
pub mod text;
pub mod url;

/// Components extracted from a recipe source.
/// All fields can be empty strings if the data is not available.
#[derive(Debug, Clone, Default)]
pub struct RecipeComponents {
    /// Recipe text containing ingredients and instructions
    pub text: String,
    /// YAML-formatted metadata (without --- delimiters)
    pub metadata: String,
    /// Recipe name/title (always single-line)
    pub name: String,
}

/// Collapse any whitespace (newlines, tabs, multiple spaces) into a single space.
pub fn sanitize_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build a YAML metadata string from a Recipe's fields.
/// Handles nested values (e.g. nutrition) by parsing pre-formatted YAML blocks.
pub fn metadata_to_yaml(entries: &[(String, String)]) -> String {
    use serde_yaml::Value;

    let mut mapping = serde_yaml::Mapping::new();

    for (key, value) in entries {
        if value.starts_with('\n') {
            // Pre-formatted nested YAML (e.g. nutrition) — parse as nested mapping
            let yaml_str = format!("{}:{}", key, value);
            if let Ok(parsed) = serde_yaml::from_str::<serde_yaml::Mapping>(&yaml_str) {
                for (k, v) in parsed {
                    mapping.insert(k, v);
                }
                continue;
            }
        }
        mapping.insert(Value::String(key.clone()), Value::String(value.clone()));
    }

    if mapping.is_empty() {
        String::new()
    } else {
        serde_yaml::to_string(&mapping).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_to_yaml_simple() {
        let entries = vec![
            ("source".to_string(), "http://example.com".to_string()),
            ("servings".to_string(), "4".to_string()),
        ];
        let yaml = metadata_to_yaml(&entries);
        assert!(yaml.contains("source: http://example.com"));
        assert!(yaml.contains("servings: '4'"));
    }

    #[test]
    fn test_metadata_to_yaml_with_colon() {
        let entries = vec![("description".to_string(), "test : sub".to_string())];
        let yaml = metadata_to_yaml(&entries);
        assert!(yaml.contains("description: 'test : sub'"));
    }

    #[test]
    fn test_metadata_to_yaml_nested() {
        let entries = vec![(
            "nutrition".to_string(),
            "\n  calories: 330 calories\n  fat: 18 grams fat".to_string(),
        )];
        let yaml = metadata_to_yaml(&entries);
        assert!(yaml.contains("nutrition:"));
        assert!(yaml.contains("calories: 330 calories"));
        assert!(yaml.contains("fat: 18 grams fat"));
        // Should NOT be quoted as a single string
        assert!(!yaml.contains("\""));
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("hello  world\n test"), "hello world test");
    }
}

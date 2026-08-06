pub mod image;
pub mod text;
pub mod title;
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

/// Extract the first integer from a free-form yield string ("Makes 12", "4 personnes").
/// The Cooklang parser only accepts numeric `servings`, so descriptive yields must be
/// reduced to a number (callers keep the original text under a `yield` key).
pub fn extract_servings_number(raw: &str) -> Option<u32> {
    let digits: String = raw
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Normalize a raw yield value into metadata entries: a numeric `servings` when a
/// number can be extracted, plus the original text as `yield` when it carries more
/// information than the bare number.
pub fn servings_entries(raw: &str) -> Vec<(String, String)> {
    let raw = raw.trim();
    let mut entries = Vec::new();
    match extract_servings_number(raw) {
        Some(n) => {
            entries.push(("servings".to_string(), n.to_string()));
            if raw != n.to_string() {
                entries.push(("yield".to_string(), raw.to_string()));
            }
        }
        None => {
            if !raw.is_empty() {
                entries.push(("yield".to_string(), raw.to_string()));
            }
        }
    }
    entries
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
        // The Cooklang parser rejects quoted numbers for `servings` — emit it as a
        // YAML number so `servings: 4` parses instead of warning on `servings: '4'`.
        if key == "servings" {
            if let Ok(n) = value.trim().parse::<u64>() {
                mapping.insert(Value::String(key.clone()), Value::Number(n.into()));
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
        // servings must be a bare YAML number — the Cooklang parser rejects '4'
        assert!(yaml.contains("servings: 4"));
        assert!(!yaml.contains("servings: '4'"));
    }

    #[test]
    fn test_extract_servings_number() {
        assert_eq!(extract_servings_number("4"), Some(4));
        assert_eq!(extract_servings_number("Makes 12"), Some(12));
        assert_eq!(extract_servings_number("4 personnes"), Some(4));
        assert_eq!(extract_servings_number("4 to 6 servings"), Some(4));
        assert_eq!(extract_servings_number("a few"), None);
    }

    #[test]
    fn test_servings_entries_numeric_only() {
        assert_eq!(
            servings_entries("4"),
            vec![("servings".to_string(), "4".to_string())]
        );
    }

    #[test]
    fn test_servings_entries_descriptive() {
        assert_eq!(
            servings_entries("Makes 12"),
            vec![
                ("servings".to_string(), "12".to_string()),
                ("yield".to_string(), "Makes 12".to_string()),
            ]
        );
    }

    #[test]
    fn test_servings_entries_no_number() {
        assert_eq!(
            servings_entries("one loaf-ish"),
            vec![("yield".to_string(), "one loaf-ish".to_string())]
        );
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

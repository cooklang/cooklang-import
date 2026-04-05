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

/// Escape a YAML value by wrapping it in double quotes if it contains
/// characters that are special in YAML (e.g. `:`, `#`, `[`, `]`, `{`, `}`).
pub fn yaml_escape(value: &str) -> String {
    if value.contains(':')
        || value.contains('#')
        || value.contains('[')
        || value.contains(']')
        || value.contains('{')
        || value.contains('}')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('*')
        || value.contains('&')
        || value.contains('!')
        || value.contains('|')
        || value.contains('>')
        || value.contains('%')
        || value.contains('@')
        || value.contains('`')
        || value.starts_with(' ')
        || value.ends_with(' ')
    {
        // Escape existing double quotes and backslashes, then wrap
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_escape_plain_value() {
        assert_eq!(yaml_escape("hello world"), "hello world");
    }

    #[test]
    fn test_yaml_escape_colon() {
        assert_eq!(yaml_escape("test : sub"), "\"test : sub\"");
    }

    #[test]
    fn test_yaml_escape_url() {
        assert_eq!(
            yaml_escape("http://example.com/recipe"),
            "\"http://example.com/recipe\""
        );
    }

    #[test]
    fn test_yaml_escape_with_quotes() {
        assert_eq!(yaml_escape("say \"hello\""), "\"say \\\"hello\\\"\"");
    }

    #[test]
    fn test_yaml_escape_hash() {
        assert_eq!(yaml_escape("value # comment"), "\"value # comment\"");
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("hello  world\n test"), "hello world test");
    }
}

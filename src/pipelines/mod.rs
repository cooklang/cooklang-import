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

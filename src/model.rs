use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone, Default)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    pub image: Vec<String>,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

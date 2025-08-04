use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    pub image: Vec<String>,
    pub ingredients: String,
    pub instructions: String,
    pub metadata: HashMap<String, String>,
}

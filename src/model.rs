use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Recipe {
    pub name: String,
    pub description: String,
    pub image: Vec<String>,
    pub ingredients: Vec<String>,
    pub steps: String,
}

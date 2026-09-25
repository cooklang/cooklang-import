use super::Issue;
use cooklang::Recipe;

pub(super) fn check(_original: &str, _body: &str, _recipe: &Recipe) -> Vec<Issue> {
    Vec::new()
}

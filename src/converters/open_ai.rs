use crate::converters::ConvertToCooklang;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

const COOKLANG_CONVERTER_PROMPT: &str = "
    As a distinguished Cooklang Converter, your primary task is
    to transform recipes provided by the user into the structured
    Cooklang recipe markup format.

    Ingredients

    To define an ingredient, use the @ symbol. If the ingredient's
    name contains multiple words, indicate the end of the name with {}.

    Example:
        Then add @salt and @ground black pepper{} to taste.

    To indicate the quantity of an item, place the quantity inside {} after the name.

    Example:
        Poke holes in @potato{2}.

    To use a unit of an item, such as weight or volume, add a % between
    the quantity and unit.

    Example:
        Place @bacon strips{1%kg} on a baking sheet and glaze with @syrup{1/2%tbsp}.

    Cookware

    You can define any necessary cookware with # symbol. If the cookware's
    name contains multiple words, indicate the end of the name with {}.

    Example:
        Place the potatoes into a #pot.
        Mash the potatoes with a #potato masher{}.

    Timer

    You can define a timer using ~.

    Example:
        Lay the potatoes on a #baking sheet{} and place into the #oven{}. Bake for ~{25%minutes}.

    Timers can have a name too.

    Example:
        Boil @eggs{2} for ~eggs{3%minutes}.

    User will give you a classical recipe representation when ingredients listed first
    and then method text.

    Final result shouldn't have original ingredient list, you need to
    incorporate each ingredient and quantities into method's text following
    Cooklang conventions.

    Ensure the original recipe's words are preserved, modifying only
    ingredients and cookware according to Cooklang syntax. Don't convert
    temperature.

    Separate each step with two new lines.
";

pub struct OpenAIConverter {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIConverter {
    pub fn new(api_key: String, model: String) -> Self {
        OpenAIConverter {
            client: Client::new(),
            api_key,
            base_url: "https://api.openai.com".to_string(),
            model,
        }
    }

    #[cfg(test)]
    fn with_base_url(api_key: String, base_url: String, model: String) -> Self {
        OpenAIConverter {
            client: Client::new(),
            api_key,
            base_url,
            model,
        }
    }
}

#[async_trait::async_trait]
impl ConvertToCooklang for OpenAIConverter {
    async fn convert(&self, ingredients: &[String], steps: &str) -> Result<String, Box<dyn Error>> {
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": COOKLANG_CONVERTER_PROMPT},
                    {"role": "user", "content": format!("Ingredients: {:?}\nSteps: {}", ingredients, steps)}
                ],
                "temperature": 0.7
            }))
            .send()
            .await?;

        let response_body: Value = response.json().await?;
        debug!("{:?}", response_body);
        let cooklang_recipe = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract content from response")?
            .to_string();

        Ok(cooklang_recipe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_convert() {
        let mut server = mockito::Server::new();

        // Mock the OpenAI API response
        let mock = server.mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"
                {
                    "choices": [
                        {
                            "message": {
                                "content": ">> Converted recipe in Cooklang format:\n\n@Pasta{500%g}\n@Tomato sauce{1%jar}\n@Cheese{200%g}\n\n#Cook the pasta according to package instructions.\n#Heat the tomato sauce in a pan.\n#Drain the pasta and mix with the sauce.\n#Sprinkle grated cheese on top and serve."
                            }
                        }
                    ]
                }
            "#)
            .create();

        let converter = OpenAIConverter::with_base_url(
            "test_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let ingredients = vec![
            "Pasta".to_string(),
            "Tomato sauce".to_string(),
            "Cheese".to_string(),
        ];
        let steps = "Cook pasta, heat sauce, mix, add cheese.";

        let result = converter.convert(&ingredients, steps).await;

        mock.assert();

        assert!(result.is_ok(), "Conversion failed: {:?}", result.err());
        let converted_recipe = result.unwrap();
        assert!(converted_recipe.contains("@Pasta{500%g}"));
        assert!(converted_recipe.contains("#Cook the pasta according to package instructions."));
    }

    #[tokio::test]
    async fn test_convert_api_error() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(500)
            .create();

        let converter = OpenAIConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let ingredients = vec!["ingredient".to_string()];
        let steps = "step";

        let result = converter.convert(&ingredients, steps).await;

        mock.assert();
        assert!(result.is_err());
    }
}

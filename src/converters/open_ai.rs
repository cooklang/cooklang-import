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

    #[doc(hidden)]
    pub fn with_base_url(api_key: String, base_url: String, model: String) -> Self {
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
    async fn convert(
        &self,
        ingredients: &str,
        instructions: &str,
    ) -> Result<String, Box<dyn Error>> {
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": COOKLANG_CONVERTER_PROMPT},
                    {"role": "user", "content": format!("Ingredients: {:?}\nInstructions: {}", ingredients, instructions)}
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
    use mockito::Server;

    #[tokio::test]
    async fn test_convert() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "choices": [{
                        "message": {
                            "content": ">> ingredients\n@pasta{500%g}\n@sauce\n\n>> instructions\n1. Cook pasta\n2. Add sauce"
                        }
                    }]
                }"#,
            )
            .create();

        let converter = OpenAIConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let ingredients = "pasta\nsauce";
        let instructions = "Cook pasta with sauce";

        let result = converter.convert(ingredients, instructions).await.unwrap();
        assert!(result.contains("@pasta"));
        assert!(result.contains("@sauce"));
        mock.assert();
    }

    #[tokio::test]
    async fn test_convert_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid request"}"#)
            .create();

        let converter = OpenAIConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let ingredients = "ingredient";
        let instructions = "step";

        let result = converter.convert(ingredients, instructions).await;
        assert!(result.is_err());
        mock.assert();
    }
}

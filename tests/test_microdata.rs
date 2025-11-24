#[cfg(test)]
mod tests {
    use cooklang_import::extractors::MicroDataExtractor;
    use cooklang_import::extractors::{Extractor, ParsingContext};
    use scraper::Html;

    #[test]
    fn test_microdata_extraction() {
        let html = r#"
        <html>
        <body>
        <div id="easyrecipe-557-0" class="easyrecipe" itemscope itemtype="http://schema.org/Recipe">
            <div itemprop="name" class="ERSName">Mom's Famous Banana Bread</div>
            <div itemprop="description" class="ERSSummary">Mom was kind enough to share her famous banana bread recipe with us!</div>
            <img itemprop="image" src="https://example.com/banana-bread.jpg" />
            <div itemprop="author" itemscope itemtype="http://schema.org/Person">
                <span itemprop="name">Cooking Divine</span>
            </div>
            <div itemprop="recipeCategory">Breakfast</div>
            <div itemprop="recipeCuisine">American</div>
            <div itemprop="keywords">banana, bread, sweet</div>
            <div itemprop="suitableForDiet">Vegetarian</div>
            
            <div class="ERSTimes">
                <div class="ERSTime">
                    <div class="ERSTimeHeading">Prep time</div>
                    <div class="ERSTimeItem">
                        <time itemprop="prepTime" datetime="PT10M">10 mins</time>
                    </div>
                </div>
                <div class="ERSTime ERSTimeRight">
                    <div class="ERSTimeHeading">Cook time</div>
                    <div class="ERSTimeItem">
                        <time itemprop="cookTime" datetime="PT1H">1 hour</time>
                    </div>
                </div>
                <div class="ERSTime ERSTimeRight">
                    <div class="ERSTimeHeading">Total time</div>
                    <div class="ERSTimeItem">
                        <time itemprop="totalTime" datetime="PT1H10M">1 hour 10 mins</time>
                    </div>
                </div>
            </div>

            <div class="divERSHeadItems">
                <div class="ERSServes">Serves: <span itemprop="recipeYield">12 servings</span></div>
            </div>

            <div class="ERSIngredients">
                <div class="ERSIngredientsHeader ERSHeading">Ingredients</div>
                <ul>
                    <li class="ingredient" itemprop="ingredients">5 Tablespoons Butter (room temperature)</li>
                    <li class="ingredient" itemprop="ingredients">1 Cup White Sugar</li>
                    <li class="ingredient" itemprop="ingredients">1 Large Egg</li>
                </ul>
            </div>

            <div class="ERSInstructions">
                <div class="ERSInstructionsHeader ERSHeading">Directions</div>
                <ol>
                    <li class="instruction" itemprop="recipeInstructions">Preheat oven to 350 degrees and heavily grease a 9 inch bread pan.</li>
                    <li class="instruction" itemprop="recipeInstructions">Beat butter and sugar until light, fluffy and well blended.</li>
                </ol>
            </div>
        </div>
        </body>
        </html>
        "#;

        let context = ParsingContext {
            url: "https://www.cookingdivine.com/recipes/banana-bread/".to_string(),
            document: Html::parse_document(html),
            texts: None,
            recipe_language: None,
        };

        let extractor = MicroDataExtractor;
        let result = extractor.parse(&context);

        assert!(result.is_ok(), "Failed to extract recipe");
        let recipe = result.unwrap();

        assert_eq!(recipe.name, "Mom's Famous Banana Bread");
        assert_eq!(
            recipe.description,
            Some(
                "Mom was kind enough to share her famous banana bread recipe with us!".to_string()
            )
        );

        assert!(recipe.content.contains("5 Tablespoons Butter"));
        assert!(recipe.content.contains("1 Cup White Sugar"));
        assert!(recipe.content.contains("Preheat oven to 350 degrees"));

        assert_eq!(
            recipe.metadata.get("prep_time"),
            Some(&"10 mins".to_string())
        );
        assert_eq!(
            recipe.metadata.get("cook_time"),
            Some(&"1 hour".to_string())
        );
        assert_eq!(
            recipe.metadata.get("total_time"),
            Some(&"1 hour 10 mins".to_string())
        );
        assert_eq!(
            recipe.metadata.get("servings"),
            Some(&"12 servings".to_string())
        );

        // New fields
        assert_eq!(
            recipe.metadata.get("author"),
            Some(&"Cooking Divine".to_string())
        );
        assert_eq!(
            recipe.metadata.get("image"),
            Some(&"https://example.com/banana-bread.jpg".to_string())
        );
        assert_eq!(
            recipe.metadata.get("course"),
            Some(&"Breakfast".to_string())
        );
        assert_eq!(
            recipe.metadata.get("cuisine"),
            Some(&"American".to_string())
        );
        assert_eq!(recipe.metadata.get("diet"), Some(&"Vegetarian".to_string()));
        assert_eq!(
            recipe.metadata.get("tags"),
            Some(&"banana, bread, sweet".to_string())
        );
    }
}

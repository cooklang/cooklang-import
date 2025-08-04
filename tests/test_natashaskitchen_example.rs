#[cfg(test)]
mod tests {
    use cooklang_import::extractors::{Extractor, HtmlClassExtractor, ParsingContext};
    use scraper::Html;

    #[test]
    fn test_natashaskitchen_wprm_extraction() {
        // Simplified HTML structure from Natasha's Kitchen using WPRM plugin
        let html = r#"
        <html>
            <body>
                <div class="wprm-recipe-container">
                    <h2 class="wprm-recipe-name">Chickpea Salad Recipe</h2>
                    <div class="wprm-recipe-summary">
                        <span>This Chickpea Salad recipe is fresh, colorful and surprisingly filling. It's loaded with crisp veggies and plant-based protein.</span>
                    </div>

                    <div class="wprm-recipe-times-container">
                        <div class="wprm-recipe-time-container wprm-recipe-prep-time-container">
                            <span class="wprm-recipe-time wprm-recipe-prep-time">15 mins</span>
                        </div>
                        <div class="wprm-recipe-time-container wprm-recipe-total-time-container">
                            <span class="wprm-recipe-time wprm-recipe-total-time">15 mins</span>
                        </div>
                    </div>

                    <div class="wprm-recipe-servings-container">
                        <span class="wprm-recipe-servings">6 servings</span>
                    </div>

                    <div class="wprm-recipe-ingredients-container">
                        <h3>Ingredients</h3>
                        <ul class="wprm-recipe-ingredients">
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">2</span>
                                <span class="wprm-recipe-ingredient-unit">15 oz cans</span>
                                <span class="wprm-recipe-ingredient-name">chickpeas (garbanzo beans), drained and rinsed</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1</span>
                                <span class="wprm-recipe-ingredient-name">English cucumber, diced</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1</span>
                                <span class="wprm-recipe-ingredient-name">bell pepper (any color), diced</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1 1/2 cups</span>
                                <span class="wprm-recipe-ingredient-name">cherry tomatoes, halved</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1/2</span>
                                <span class="wprm-recipe-ingredient-name">medium red onion, thinly sliced</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1/2 cup</span>
                                <span class="wprm-recipe-ingredient-name">crumbled feta cheese</span>
                            </li>
                        </ul>

                        <h3>Lemon Herb Dressing</h3>
                        <ul class="wprm-recipe-ingredients">
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1/4 cup</span>
                                <span class="wprm-recipe-ingredient-name">olive oil</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">3 Tbsp</span>
                                <span class="wprm-recipe-ingredient-name">lemon juice, freshly squeezed</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1</span>
                                <span class="wprm-recipe-ingredient-name">garlic clove, pressed or finely minced</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1/2 tsp</span>
                                <span class="wprm-recipe-ingredient-name">sea salt</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">1/8 tsp</span>
                                <span class="wprm-recipe-ingredient-name">black pepper</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">2 Tbsp</span>
                                <span class="wprm-recipe-ingredient-name">fresh dill, chopped</span>
                            </li>
                            <li class="wprm-recipe-ingredient">
                                <span class="wprm-recipe-ingredient-amount">2 Tbsp</span>
                                <span class="wprm-recipe-ingredient-name">fresh parsley, chopped</span>
                            </li>
                        </ul>
                    </div>

                    <div class="wprm-recipe-instructions-container">
                        <h3>Instructions</h3>
                        <ul class="wprm-recipe-instructions">
                            <li class="wprm-recipe-instruction">
                                <div class="wprm-recipe-instruction-text">
                                    <span>In a large mixing bowl, add all of the chickpea salad ingredients.</span>
                                </div>
                            </li>
                            <li class="wprm-recipe-instruction">
                                <div class="wprm-recipe-instruction-text">
                                    <span>In a small bowl or measuring cup, whisk together all of the lemon dressing ingredients.</span>
                                </div>
                            </li>
                            <li class="wprm-recipe-instruction">
                                <div class="wprm-recipe-instruction-text">
                                    <span>Drizzle the dressing over the salad and toss to combine. Season with more salt and pepper to taste if desired.</span>
                                </div>
                            </li>
                        </ul>
                    </div>

                    <div class="wprm-recipe-notes-container">
                        <h3>Recipe Notes</h3>
                        <div class="wprm-recipe-notes">
                            <span>Make Ahead: This salad can be made up to 2 days in advance. Store covered in the refrigerator.</span>
                        </div>
                    </div>
                </div>
            </body>
        </html>
        "#;

        let context = ParsingContext {
            url: "https://natashaskitchen.com/chickpea-salad-recipe/".to_string(),
            document: Html::parse_document(html),
            texts: None,
        };

        let extractor = HtmlClassExtractor;
        let result = extractor.parse(&context);

        assert!(result.is_ok(), "Failed to extract recipe");
        let recipe = result.unwrap();

        // Verify title extraction
        assert_eq!(recipe.name, "Chickpea Salad Recipe");

        // Verify description extraction
        assert_eq!(
            recipe.description,
            Some("This Chickpea Salad recipe is fresh, colorful and surprisingly filling. It's loaded with crisp veggies and plant-based protein.".to_string())
        );

        // Verify ingredients extraction (checking for key ingredients)
        assert!(recipe.ingredients.contains("chickpeas"));
        assert!(recipe.ingredients.contains("cucumber"));
        assert!(recipe.ingredients.contains("bell pepper"));
        assert!(recipe.ingredients.contains("cherry tomatoes"));
        assert!(recipe.ingredients.contains("red onion"));
        assert!(recipe.ingredients.contains("feta cheese"));
        assert!(recipe.ingredients.contains("olive oil"));
        assert!(recipe.ingredients.contains("lemon juice"));
        assert!(recipe.ingredients.contains("fresh dill"));

        // Verify instructions extraction
        assert!(recipe.instructions.contains("In a large mixing bowl"));
        assert!(recipe
            .instructions
            .contains("whisk together all of the lemon dressing"));
        assert!(recipe
            .instructions
            .contains("Drizzle the dressing over the salad"));

        // Verify metadata extraction
        assert_eq!(
            recipe.metadata.get("prep_time"),
            Some(&"15 mins".to_string())
        );
        assert_eq!(
            recipe.metadata.get("total_time"),
            Some(&"15 mins".to_string())
        );
        assert_eq!(
            recipe.metadata.get("servings"),
            Some(&"6 servings".to_string())
        );
        assert!(recipe.metadata.get("notes").unwrap().contains("Make Ahead"));

        // Verify ingredients are properly formatted (multiple ingredients on separate lines)
        let ingredient_lines: Vec<&str> = recipe.ingredients.lines().collect();
        assert!(
            ingredient_lines.len() > 10,
            "Should have multiple ingredient lines"
        );

        // Verify instructions are numbered
        assert!(recipe.instructions.contains("1."));
        assert!(recipe.instructions.contains("2."));
        assert!(recipe.instructions.contains("3."));
    }

    #[test]
    fn test_wprm_with_ingredient_groups() {
        // Test that we properly handle recipes with ingredient groups (like "For the Salad:" and "For the Dressing:")
        let html = r#"
        <div class="wprm-recipe-container">
            <h2 class="wprm-recipe-name">Test Recipe with Groups</h2>

            <div class="wprm-recipe-ingredients-container">
                <h4 class="wprm-recipe-ingredient-group-name">For the Salad:</h4>
                <ul class="wprm-recipe-ingredients">
                    <li class="wprm-recipe-ingredient">2 cups lettuce</li>
                    <li class="wprm-recipe-ingredient">1 cup tomatoes</li>
                </ul>

                <h4 class="wprm-recipe-ingredient-group-name">For the Dressing:</h4>
                <ul class="wprm-recipe-ingredients">
                    <li class="wprm-recipe-ingredient">1/4 cup olive oil</li>
                    <li class="wprm-recipe-ingredient">2 Tbsp vinegar</li>
                </ul>
            </div>
        </div>
        "#;

        let context = ParsingContext {
            url: "https://example.com/test".to_string(),
            document: Html::parse_document(html),
            texts: None,
        };

        let extractor = HtmlClassExtractor;
        let result = extractor.parse(&context);

        assert!(result.is_ok());
        let recipe = result.unwrap();

        // Should extract ingredients from both groups
        assert!(recipe.ingredients.contains("lettuce"));
        assert!(recipe.ingredients.contains("tomatoes"));
        assert!(recipe.ingredients.contains("olive oil"));
        assert!(recipe.ingredients.contains("vinegar"));
    }
}

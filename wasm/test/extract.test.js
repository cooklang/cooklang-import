import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { initSync, extractRecipe } from "../index.js";

initSync({
  module: readFileSync(new URL("../pkg/cooklang_import_bg.wasm", import.meta.url)),
});

const page = `<html><head><script type="application/ld+json">
{"@context":"https://schema.org","@type":"Recipe","name":"Pancakes","recipeYield":"4",
 "recipeIngredient":["200 g flour","2 eggs"],
 "recipeInstructions":[{"@type":"HowToStep","text":"Whisk the flour and eggs."},
                       {"@type":"HowToStep","text":"Fry in a hot pan."}]}
</script></head><body></body></html>`;

test("extracts a JSON-LD recipe", () => {
  const result = extractRecipe(page, "https://example.com/pancakes");
  assert.equal(result.recipe.name, "Pancakes");
  assert.deepEqual(result.recipe.ingredients, ["200 g flour", "2 eggs"]);
  assert.match(result.recipe.instructions, /Fry in a hot pan\./);
  assert.equal(result.components.name, "Pancakes");
  assert.match(result.components.text, /^200 g flour\n2 eggs\n\n/);
  assert.match(result.components.metadata, /servings: 4/);
});

test("returns null when the page has no recipe", () => {
  assert.equal(extractRecipe("<html><body>Opening hours</body></html>", "https://example.com"), null);
});

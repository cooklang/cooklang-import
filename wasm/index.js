import init, { initSync, extractRecipeJson } from "./pkg/cooklang_import.js";

export { init, initSync };

/**
 * Extract a recipe from a page's HTML. Call `init()` (or `initSync()`) first.
 * Returns `{ recipe, components }`, or `null` when the page has no structured recipe.
 */
export function extractRecipe(html, url) {
  const json = extractRecipeJson(html, url);
  return json === undefined ? null : JSON.parse(json);
}

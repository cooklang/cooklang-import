import type { InitInput, InitOutput, SyncInitInput } from "./pkg/cooklang_import.js";

export type { InitInput, InitOutput, SyncInitInput };

/** Load the wasm module. Pass the URL (or bytes) of `cooklang_import_bg.wasm`. */
export function init(
  module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>,
): Promise<InitOutput>;

/** Synchronous variant for when the bytes are already in hand. */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/** The recipe as the extractor found it, for display. */
export interface Recipe {
  name: string;
  description: string | null;
  image: string[];
  ingredients: string[];
  /** Steps as one string, separated by newlines. */
  instructions: string;
  /** Free-form metadata: servings, times, author, source and so on. */
  metadata: Record<string, string>;
}

/** What the Cooklang converter takes; send these to the conversion API. */
export interface RecipeComponents {
  name: string;
  /** YAML, without `---` delimiters. */
  metadata: string;
  /** Ingredient lines, a blank line, then the instructions. */
  text: string;
}

export interface Extraction {
  recipe: Recipe;
  components: RecipeComponents;
}

/** Returns `null` when the page has no structured recipe. */
export function extractRecipe(html: string, url: string): Extraction | null;

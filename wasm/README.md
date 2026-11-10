# @cooklang/cooklang-import

The [cooklang-import](https://github.com/cooklang/cooklang-import) recipe extractors,
compiled to WebAssembly. Give it a page's HTML and URL; it returns the recipe it finds in
JSON-LD, MicroData or common recipe-plugin HTML classes. It makes no network requests and
does not convert to Cooklang. Conversion needs a language model and happens elsewhere.

```js
import { init, extractRecipe } from "@cooklang/cooklang-import";
import wasmUrl from "@cooklang/cooklang-import/cooklang_import_bg.wasm?url"; // Vite

await init({ module_or_path: wasmUrl });

const result = extractRecipe(document.documentElement.outerHTML, location.href);
if (result) {
  console.log(result.recipe.name, result.recipe.ingredients);
  // result.components is { name, metadata, text }: the converter's input
}
```

In Node, load the bytes yourself:

```js
import { readFileSync } from "node:fs";
import { initSync, extractRecipe } from "@cooklang/cooklang-import";

initSync({ module: readFileSync("node_modules/@cooklang/cooklang-import/pkg/cooklang_import_bg.wasm") });
```

`extractRecipe` returns `null` when the page has no structured recipe.

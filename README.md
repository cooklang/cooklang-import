# cooklang-import
A command-line tool to import recipes into [Cooklang](https://cooklang.org/) format.

## Getting started

1. Make sure you have the following prerequisites:
    * [Rust](https://www.rust-lang.org/tools/install)
    * [OpenAI API key](https://platform.openai.com/api-keys) set in your environment variables as `OPENAI_API_KEY`
2. Clone this repo locally
3. Change directory into this repo root and run `cargo install --path .`

## Usage examples
### See all available flags
```sh
cooklang-import --help
```
### Scrape a recipe from a webpage and output to screen

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala
```

also supports `--download-only` flag to only download the recipe and not convert it to Cooklang

```sh
cooklang-import https://www.bbcgoodfood.com/recipes/next-level-tikka-masala --download-only
```


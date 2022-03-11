# cook-import
A command-line tool to import recipes into [Cooklang](https://cooklang.org/) format.

## Getting started

1. Make sure you have the following prerequisites:
    * [Python 3](https://www.python.org/downloads/)
    * [Poetry](https://pypi.org/project/poetry/)
2. Clone this repo locally
3. Change directory into this repo root and run `poetry install`

## Usage examples
### See all available flags
```
    poetry run ./cook-import --help
```
### Scrape a recipe from a webpage and output to screen
```
    poetry run. /cook-import --link https://www.bbcgoodfood.com/recipes/next-level-tikka-masala
```
### Scrape a recipe from a webpage and output to file
```
    poetry run ./cook-import --link https://www.bbcgoodfood.com/recipes/next-level-tikka-masala --file
```

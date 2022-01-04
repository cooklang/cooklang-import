
poetry run pyinstaller cook-import \
    --collect-data mf2py \
    --collect-data recipe_scrapers \
    --hidden-import  recipe_scrapers.settings.default  \
    --onefile

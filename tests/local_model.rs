#![cfg(feature = "local-model")]

use cooklang_import::local_model::Tagger;

fn model_dir() -> Option<std::path::PathBuf> {
    match std::env::var_os("RECIPE_PARSER_MODEL_DIR") {
        Some(d) => Some(d.into()),
        None => {
            eprintln!("RECIPE_PARSER_MODEL_DIR unset — skipping model test");
            None
        }
    }
}

#[test]
fn tags_an_ingredient_line_and_a_step() {
    let Some(dir) = model_dir() else { return };
    let tagger = Tagger::load(&dir).expect("model loads");

    let line = tagger.tag("0.25 cup unsalted butter, softened").unwrap();
    let labels: Vec<&str> = line.tags.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels[0], "B-QTY");
    assert!(labels.contains(&"B-NAME"));

    let step = tagger.tag("Roast in the oven for 20 minutes.").unwrap();
    assert!(step.tags.iter().any(|t| t.label.ends_with("TIMER")));
}

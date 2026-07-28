#![cfg(feature = "local-model")]

#[tokio::test]
async fn golden_fixtures_are_stable() {
    if std::env::var_os("RECIPE_PARSER_MODEL_DIR").is_none() {
        eprintln!("RECIPE_PARSER_MODEL_DIR unset — skipping");
        return;
    }
    let dir = std::path::Path::new("tests/fixtures/golden");
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "source") {
            let expected = std::fs::read_to_string(path.with_extension("expected.cook")).unwrap();
            let body = std::fs::read_to_string(&path).unwrap();
            let got = cooklang_import::convert_text_local(&body).await.unwrap();
            assert_eq!(got.trim(), expected.trim(), "fixture {path:?} drifted");
            checked += 1;
        }
    }
    assert!(
        checked >= 6,
        "expected at least 6 golden fixtures, found {checked}"
    );
}

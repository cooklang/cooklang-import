#![cfg(feature = "local-model")]

use cooklang_import::local_words::word_tokenize;

#[derive(serde::Deserialize)]
struct Case {
    text: String,
    tokens: Vec<String>,
}

#[test]
fn tokenizer_matches_python_fixture() {
    let data = include_str!("fixtures/tokenizer_parity.json");
    let cases: Vec<Case> = serde_json::from_str(data).unwrap();
    for case in cases {
        let got: Vec<String> = word_tokenize(&case.text)
            .into_iter()
            .map(|t| t.text)
            .collect();
        assert_eq!(got, case.tokens, "mismatch for: {}", case.text);
    }
}

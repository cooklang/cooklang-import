#![cfg(feature = "local-model")]

const BODY: &str = "\
10 ounce Ground Beef
2 clove Garlic

Wrap garlic in foil and roast about 20 minutes.

Divide ground beef in half and shape into patties.
";

#[tokio::test]
async fn converts_body_to_valid_cooklang() {
    if std::env::var_os("RECIPE_PARSER_MODEL_DIR").is_none() {
        eprintln!("RECIPE_PARSER_MODEL_DIR unset — skipping");
        return;
    }
    let out = cooklang_import::convert_text_local(BODY)
        .await
        .expect("conversion succeeds");
    eprintln!("--- converted output ---\n{out}\n--- end ---");
    let low = out.to_lowercase();
    assert!(
        low.contains("@ground beef") || out.contains("@Ground Beef"),
        "{out}"
    );
    assert!(out.contains("~{20%minutes}"), "{out}");
    // whole output must parse without errors
    let parser =
        cooklang::CooklangParser::new(cooklang::Extensions::all(), cooklang::Converter::default());
    let (_, report) = parser.parse(&out).into_tuple();
    assert!(!report.has_errors(), "{out}");
}

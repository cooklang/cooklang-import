//! Local ONNX-model converter: recipe text -> Cooklang without any API calls.

pub mod artifacts;
pub mod assemble;
pub mod bio;
pub mod entry;
pub mod linker;
pub mod model;
pub mod recover;
pub mod timer;
pub mod words;

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::OnceCell;

use super::{ConversionMetadata, ConversionResult, Converter};
use assemble::{assemble_paragraph, validate_or_plain};
use bio::{decode, Span};
use entry::Entry;
use model::Tagger;
use words::Word;

const MIN_CONFIDENCE: f32 = 0.5;

pub struct LocalConverter {
    tagger: OnceCell<Arc<Tagger>>,
}

impl Default for LocalConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalConverter {
    pub fn new() -> Self {
        Self {
            tagger: OnceCell::new(),
        }
    }

    async fn tagger(&self) -> Result<Arc<Tagger>, String> {
        self.tagger
            .get_or_try_init(|| async {
                let dir = artifacts::ensure_artifacts(None, artifacts::DEFAULT_BASE_URL).await?;
                tokio::task::spawn_blocking(move || Tagger::load(&dir).map(Arc::new))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())
            })
            .await
            .cloned()
    }
}

/// Split the body: first blank-line block = ingredient lines, rest = steps.
fn segment(body: &str) -> (Vec<String>, Vec<String>) {
    let blocks: Vec<&str> = body
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .collect();
    match blocks.split_first() {
        Some((first, rest)) if !rest.is_empty() => (
            first
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            rest.iter()
                .map(|b| b.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect(),
        ),
        _ => (
            vec![],
            blocks
                .iter()
                .map(|b| b.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect(),
        ),
    }
}

/// A step paragraph plus its tagged words and decoded spans, tracked
/// together so the recovery pass (below) can search across every step
/// before assembly runs.
struct StepData {
    text: String,
    words: Vec<Word>,
    spans: Vec<Span>,
}

fn convert_sync(tagger: &Tagger, body: &str) -> Result<String, String> {
    let (ing_lines, steps) = segment(body);

    let mut entries: Vec<Entry> = Vec::new();
    for line in &ing_lines {
        let tagged = tagger.tag(line).map_err(|e| e.to_string())?;
        if let Some(mut e) = Entry::from_tagged(&tagged.words, &tagged.tags) {
            entry::repair_qty_unit(&mut e, &tagged.words);
            entries.push(e);
        }
    }
    let mut used = vec![false; entries.len()];

    // Tag every step up front (rather than assembling paragraph-by-paragraph
    // as before) so the recovery pass below can search the whole recipe for
    // a missed mention, not just the paragraph currently being assembled.
    let mut step_data = Vec::with_capacity(steps.len());
    for step in &steps {
        let tagged = tagger.tag(step).map_err(|e| e.to_string())?;
        let spans = decode(&tagged.tags, MIN_CONFIDENCE);
        step_data.push(StepData {
            text: step.clone(),
            words: tagged.words,
            spans,
        });
    }

    // Dry-run link() over every ING span's surface text to find which
    // entries the tagger's own tagging already covers somewhere in the
    // recipe (this mirrors, without mutating anything, what assembly's own
    // linking will do later).
    let mut covered = vec![false; entries.len()];
    for sd in &step_data {
        for span in &sd.spans {
            if span.kind != "ING" {
                continue;
            }
            let surface = &sd.text[sd.words[span.start].start..sd.words[span.end - 1].end];
            if let Some(i) = linker::link(surface, &entries) {
                covered[i] = true;
            }
        }
    }

    // Recover entries the tagger missed but whose name literally appears in
    // some step's text — this never fabricates an amount, it only supplies
    // the missing span so the existing deterministic linking/amount rules
    // apply exactly as they would for a tagger-produced span. Entries whose
    // names genuinely never occur anywhere stay uncovered (and get the
    // "never mentioned" warning below, which is correct).
    for sd in &mut step_data {
        recover::recover_unmentioned(&sd.words, &mut sd.spans, &entries, &mut covered);
        sd.spans.sort_by_key(|s| s.start);
    }

    let mut out_paras = Vec::with_capacity(step_data.len());
    for sd in &step_data {
        let converted = assemble_paragraph(&sd.text, &sd.words, &sd.spans, &mut entries, &mut used);
        out_paras.push(validate_or_plain(&converted, &sd.text));
    }

    for (e, u) in entries.iter().zip(&used) {
        if !u {
            log::warn!("ingredient never mentioned in steps: {}", e.name);
        }
    }
    Ok(out_paras.join("\n\n"))
}

/// Public helper used by tests, examples, and the library API.
pub async fn convert_text(body: &str) -> Result<String, String> {
    static SHARED: OnceCell<LocalConverter> = OnceCell::const_new();
    let conv = SHARED.get_or_init(|| async { LocalConverter::new() }).await;
    let tagger = conv.tagger().await?;
    let body = body.to_string();
    tokio::task::spawn_blocking(move || convert_sync(&tagger, &body))
        .await
        .map_err(|e| e.to_string())?
}

#[async_trait]
impl Converter for LocalConverter {
    fn name(&self) -> &str {
        "local"
    }

    async fn convert(
        &self,
        ingredients_and_instructions: &str,
    ) -> Result<ConversionResult, Box<dyn std::error::Error + Send + Sync>> {
        let started = Instant::now();
        let tagger = self.tagger().await.map_err(std::io::Error::other)?;
        let body = ingredients_and_instructions.to_string();
        let content = tokio::task::spawn_blocking(move || convert_sync(&tagger, &body))
            .await
            .map_err(std::io::Error::other)?
            .map_err(std::io::Error::other)?;
        Ok(ConversionResult {
            content,
            metadata: ConversionMetadata {
                model_version: Some("recipe-parser-v1".to_string()),
                tokens_used: Default::default(),
                latency_ms: started.elapsed().as_millis() as u64,
            },
        })
    }
}

#[cfg(test)]
mod segment_tests {
    use super::segment;

    #[test]
    fn splits_ingredient_block_from_step_paragraphs() {
        let body = "\
10 ounce Ground Beef
2 clove Garlic

Wrap garlic in foil and roast about 20 minutes.

Divide ground beef in half and shape into patties.
";
        let (ingredients, steps) = segment(body);
        assert_eq!(ingredients, vec!["10 ounce Ground Beef", "2 clove Garlic"]);
        assert_eq!(
            steps,
            vec![
                "Wrap garlic in foil and roast about 20 minutes.",
                "Divide ground beef in half and shape into patties.",
            ]
        );
    }

    #[test]
    fn body_with_no_blank_line_is_treated_as_steps_only() {
        let body = "Just one paragraph with no ingredient block.";
        let (ingredients, steps) = segment(body);
        assert!(ingredients.is_empty());
        assert_eq!(steps, vec!["Just one paragraph with no ingredient block."]);
    }
}

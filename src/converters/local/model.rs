//! ONNX inference wrapper around the trained BIO tagger model.
//!
//! Loads `model.onnx` / `tokenizer.json` / `labels.json` from a directory on
//! disk and exposes `Tagger::tag`, which maps a line of text to one BIO tag
//! per word (see `super::words::word_tokenize` and `super::bio`).

use super::bio::WordTag;
use super::words::{word_tokenize, Word};

use ndarray::{Array2, Axis};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
use tokenizers::{EncodeInput, Tokenizer};

/// Max subtokens per line (including [CLS]/[SEP]). Recipe lines and steps
/// are short; this only guards against pathological input.
const MAX_LENGTH: usize = 256;

#[derive(Debug, Error)]
pub enum ModelError {
    /// ONNX Runtime dylib could not be loaded (missing/unloadable/wrong arch).
    #[error("{0}")]
    Runtime(String),
    /// Model/tokenizer/labels artifacts on disk are missing or malformed.
    #[error("{0}")]
    Artifact(String),
    /// Failure during tokenization or the ONNX Runtime forward pass.
    #[error("{0}")]
    Inference(String),
}

/// Words paired with their predicted BIO tags (parallel arrays, same length).
pub struct Tagged {
    pub words: Vec<Word>,
    pub tags: Vec<WordTag>,
}

pub struct Tagger {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    labels: Vec<String>,
}

/// Platform default library name, matching what ort's own loader falls back
/// to when `ORT_DYLIB_PATH` is unset.
const DEFAULT_DYLIB_NAME: &str = if cfg!(target_os = "macos") {
    "libonnxruntime.dylib"
} else {
    "libonnxruntime.so"
};

/// Pre-flight the ONNX Runtime dylib BEFORE any ort API call.
///
/// The `load-dynamic` ort feature resolves the ONNX Runtime library at
/// runtime. If that load fails — env var unset with no system default
/// available, path missing, wrong architecture, corrupt file — ort's
/// `OnceLock` initialization re-enters itself while formatting the load
/// error and the process deadlocks instead of returning an `Err`. Dlopen the
/// same candidate ourselves first so a bad dylib turns into an immediate,
/// actionable `ModelError::Runtime` instead of a hang.
fn preflight_dylib(
    var: Option<std::ffi::OsString>,
    mut load: impl FnMut(&std::ffi::OsStr) -> Result<(), String>,
) -> Result<(), ModelError> {
    let (candidate, from_env) = match var {
        Some(p) => (p, true),
        None => (std::ffi::OsString::from(DEFAULT_DYLIB_NAME), false),
    };
    if let Err(dlerr) = load(&candidate) {
        let source = if from_env {
            format!("ORT_DYLIB_PATH points at {candidate:?}, which cannot be loaded")
        } else {
            format!(
                "ORT_DYLIB_PATH is not set and the system default {candidate:?} cannot be loaded"
            )
        };
        let hint = if cfg!(target_os = "macos") {
            "install ONNX Runtime (e.g. `brew install onnxruntime`) and point ORT_DYLIB_PATH \
             at libonnxruntime.*.dylib"
        } else {
            "install ONNX Runtime and point ORT_DYLIB_PATH at libonnxruntime.so"
        };
        return Err(ModelError::Runtime(format!(
            "cannot load ONNX Runtime: {source} ({dlerr}); {hint}"
        )));
    }
    Ok(())
}

/// Real loader for `preflight_dylib`: attempt a dlopen and report the error.
fn dlopen_check(path: &std::ffi::OsStr) -> Result<(), String> {
    match unsafe { libloading::Library::new(path) } {
        Ok(lib) => {
            // Intentionally leak the handle: ort dlopens the same library
            // again for real right after this, and unloading a lib that's
            // about to be reloaded buys nothing while risking unload-order
            // issues with its C++ statics. The OS dedupes the second open.
            std::mem::forget(lib);
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

impl Tagger {
    pub fn load(dir: &Path) -> Result<Self, ModelError> {
        preflight_dylib(std::env::var_os("ORT_DYLIB_PATH"), dlopen_check)?;

        let session = Session::builder()
            .map_err(|e| ModelError::Inference(e.to_string()))?
            .commit_from_file(dir.join("model.onnx"))
            .map_err(|e| ModelError::Artifact(format!("failed to load model.onnx: {e}")))?;

        let tokenizer = Tokenizer::from_file(dir.join("tokenizer.json"))
            .map_err(|e| ModelError::Artifact(format!("failed to load tokenizer.json: {e}")))?;

        let labels_json = std::fs::read_to_string(dir.join("labels.json"))
            .map_err(|e| ModelError::Artifact(format!("failed to read labels.json: {e}")))?;
        let labels: Vec<String> = serde_json::from_str(&labels_json)
            .map_err(|e| ModelError::Artifact(format!("failed to parse labels.json: {e}")))?;

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            labels,
        })
    }

    /// Tag a line of text: word-split it, run it through the model, and
    /// align each word to its first sub-token's predicted BIO label.
    pub fn tag(&self, text: &str) -> Result<Tagged, ModelError> {
        let words = word_tokenize(text);
        if words.is_empty() {
            return Ok(Tagged {
                words: Vec::new(),
                tags: Vec::new(),
            });
        }

        let word_texts: Vec<String> = words.iter().map(|w| w.text.clone()).collect();
        let enc = self
            .tokenizer
            .encode(EncodeInput::from(word_texts), true)
            .map_err(|e| ModelError::Inference(format!("tokenization failed: {e}")))?;

        debug_assert_eq!(
            enc.get_tokens().first().map(String::as_str),
            Some("[CLS]"),
            "tokenizer must apply the [CLS]/[SEP] post-processor"
        );

        let mut ids: Vec<i64> = enc.get_ids().iter().map(|&i| i as i64).collect();
        let mut mask: Vec<i64> = enc.get_attention_mask().iter().map(|&i| i as i64).collect();
        let mut type_ids: Vec<i64> = enc.get_type_ids().iter().map(|&i| i as i64).collect();
        let mut word_ids: Vec<Option<u32>> = enc.get_word_ids().to_vec();

        let truncated = ids.len() > MAX_LENGTH;
        if truncated {
            ids.truncate(MAX_LENGTH);
            mask.truncate(MAX_LENGTH);
            type_ids.truncate(MAX_LENGTH);
            word_ids.truncate(MAX_LENGTH);
        }
        let n = ids.len();

        let to_arr = |v: Vec<i64>| {
            Array2::from_shape_vec((1, n), v)
                .map_err(|e| ModelError::Inference(format!("bad tensor shape: {e}")))
        };
        let ids_tensor =
            Tensor::from_array(to_arr(ids)?).map_err(|e| ModelError::Inference(e.to_string()))?;
        let mask_tensor =
            Tensor::from_array(to_arr(mask)?).map_err(|e| ModelError::Inference(e.to_string()))?;
        let type_tensor = Tensor::from_array(to_arr(type_ids)?)
            .map_err(|e| ModelError::Inference(e.to_string()))?;

        // Copy the logits out and release the session lock before the
        // per-word softmax loop — SessionOutputs borrows from the session.
        let logits = {
            let mut session = self
                .session
                .lock()
                .map_err(|e| ModelError::Inference(e.to_string()))?;
            let outputs = session
                .run(ort::inputs![
                    "input_ids" => ids_tensor,
                    "attention_mask" => mask_tensor,
                    "token_type_ids" => type_tensor,
                ])
                .map_err(|e| ModelError::Inference(format!("inference failed: {e}")))?;
            outputs["logits"]
                .try_extract_array::<f32>()
                .map_err(|e| ModelError::Inference(e.to_string()))?
                .to_owned()
        };
        let logits = logits.index_axis(Axis(0), 0); // [n, num_labels]

        let mut tags: Vec<WordTag> = vec![
            WordTag {
                label: "O".to_string(),
                confidence: 1.0,
            };
            words.len()
        ];

        let mut prev: Option<u32> = None;
        for (pos, wid) in word_ids.iter().enumerate() {
            let Some(wid) = *wid else { continue };
            if prev == Some(wid) {
                continue;
            }
            prev = Some(wid);

            let row = logits.index_axis(Axis(0), pos);
            let max = row.iter().cloned().fold(f32::MIN, f32::max);
            let exps: Vec<f32> = row.iter().map(|&v| (v - max).exp()).collect();
            let sum: f32 = exps.iter().sum();
            let (best, p) = exps
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, &e)| (i, e / sum))
                .unwrap();

            if let Some(slot) = tags.get_mut(wid as usize) {
                *slot = WordTag {
                    label: self
                        .labels
                        .get(best)
                        .cloned()
                        .unwrap_or_else(|| "O".to_string()),
                    confidence: p,
                };
            }
        }

        Ok(Tagged { words, tags })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn runtime_msg(err: ModelError) -> String {
        let ModelError::Runtime(msg) = err else {
            panic!("expected Runtime error, got {err:?}")
        };
        msg
    }

    #[test]
    fn env_set_and_loadable_passes() {
        preflight_dylib(Some(OsString::from("/ok/lib.dylib")), |_| Ok(())).unwrap();
    }

    #[test]
    fn env_set_but_unloadable_errors_fast() {
        let msg = runtime_msg(
            preflight_dylib(Some(OsString::from("/bad/lib.dylib")), |_| {
                Err("mach-o file, but is an incompatible architecture".into())
            })
            .unwrap_err(),
        );
        assert!(
            msg.contains("ORT_DYLIB_PATH points at \"/bad/lib.dylib\""),
            "{msg}"
        );
        assert!(msg.contains("incompatible architecture"), "{msg}");
    }

    #[test]
    fn env_unset_with_working_default_passes() {
        let mut asked = Vec::new();
        preflight_dylib(None, |p| {
            asked.push(p.to_os_string());
            Ok(())
        })
        .unwrap();
        assert_eq!(asked, vec![OsString::from(DEFAULT_DYLIB_NAME)]);
    }

    #[test]
    fn env_unset_and_default_unloadable_errors_fast() {
        let msg =
            runtime_msg(preflight_dylib(None, |_| Err("image not found".into())).unwrap_err());
        assert!(msg.contains("ORT_DYLIB_PATH is not set"), "{msg}");
        assert!(msg.contains(DEFAULT_DYLIB_NAME), "{msg}");
        assert!(msg.contains("image not found"), "{msg}");
        if cfg!(target_os = "macos") {
            assert!(msg.contains("brew install onnxruntime"), "{msg}");
        }
    }
}

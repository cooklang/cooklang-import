//! Locate or fetch model artifacts.

use std::path::{Path, PathBuf};

pub const REQUIRED_FILES: &[&str] = &["model.onnx", "tokenizer.json", "labels.json"];
pub const DEFAULT_BASE_URL: &str =
    "https://github.com/cooklang/cooklang-import/releases/download/recipe-parser-v1";

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("cooklang-import")
        .join("recipe-parser-v1")
}

fn is_complete(dir: &Path) -> bool {
    REQUIRED_FILES.iter().all(|f| dir.join(f).is_file())
}

/// Resolve the directory containing `model.onnx`/`tokenizer.json`/`labels.json`.
///
/// Resolution order:
/// 1. `override_dir`, if given.
/// 2. `RECIPE_PARSER_MODEL_DIR` env var, if `override_dir` is `None`.
/// 3. The on-disk cache dir (`dirs::cache_dir()/cooklang-import/recipe-parser-v1`),
///    if it already contains all required files.
/// 4. Download each required file from `base_url` into the cache dir.
///
/// An explicit override (1 or 2) that is missing files is a hard error — we
/// never silently fall back to downloading when the caller pointed us
/// somewhere specific.
pub async fn ensure_artifacts(
    override_dir: Option<PathBuf>,
    base_url: &str,
) -> Result<PathBuf, String> {
    if let Some(dir) =
        override_dir.or_else(|| std::env::var_os("RECIPE_PARSER_MODEL_DIR").map(Into::into))
    {
        if is_complete(&dir) {
            return Ok(dir);
        }
        return Err(format!(
            "model dir {dir:?} is missing one of {REQUIRED_FILES:?}; \
             download the files manually or point RECIPE_PARSER_MODEL_DIR elsewhere"
        ));
    }

    let dir = cache_dir();
    if is_complete(&dir) {
        return Ok(dir);
    }

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let client = reqwest::Client::new();
    for f in REQUIRED_FILES {
        let url = format!("{base_url}/{f}");
        let bytes = client
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| {
                format!(
                    "downloading {url}: {e}. You can download the files manually into {dir:?} \
                     or set RECIPE_PARSER_MODEL_DIR."
                )
            })?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
        // PID-unique tmp name: two processes racing on a cold cache must not
        // truncate each other's in-progress write; the final rename is atomic
        // and last-writer-wins is safe once each writer owns its tmp file.
        let tmp = dir.join(format!("{f}.{}.part", std::process::id()));
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, dir.join(f)).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_dir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "cooklang-import-artifacts-test-{}-{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn complete_override_dir_is_returned_without_network() {
        let dir = unique_temp_dir();
        for f in REQUIRED_FILES {
            std::fs::write(dir.join(f), b"dummy").unwrap();
        }

        let result = ensure_artifacts(Some(dir.clone()), "http://bogus.invalid").await;

        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(result.unwrap(), dir);
    }

    #[tokio::test]
    async fn incomplete_override_dir_errors_with_missing_files_message() {
        let dir = unique_temp_dir();
        std::fs::write(dir.join("model.onnx"), b"dummy").unwrap();
        // tokenizer.json and labels.json intentionally missing.

        let result = ensure_artifacts(Some(dir.clone()), "http://bogus.invalid").await;

        std::fs::remove_dir_all(&dir).ok();
        let err = result.unwrap_err();
        assert!(err.contains("tokenizer.json"), "{err}");
        assert!(err.contains("labels.json"), "{err}");
    }
}

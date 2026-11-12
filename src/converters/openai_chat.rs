//! Request parameters shared by every OpenAI chat-completions call in this crate
//! (the converter, the title generator and the text extractor).
//!
//! OpenAI's current models reason by default, take `max_completion_tokens`
//! instead of `max_tokens`, and reject a non-default `temperature` unless
//! reasoning is off. The GPT-4 family took the old parameters and rejects
//! `reasoning_effort`. Both shapes are produced here so one build can talk to
//! either.

use serde_json::{json, Value};

/// Default model for conversion and for the helper calls.
pub(crate) const DEFAULT_MODEL: &str = "gpt-6-luna";

pub(crate) const OPENAI_BASE_URL: &str = "https://api.openai.com";

/// Reasoning effort used when none is configured. Converting a recipe is markup,
/// not problem solving: with reasoning off the answer is as good and several
/// times faster.
pub(crate) const DEFAULT_REASONING_EFFORT: &str = "none";

/// GPT-4-family and GPT-3.5 models, and fine-tunes of them. They take
/// `temperature` and reject `reasoning_effort`.
pub(crate) fn is_pre_reasoning(model: &str) -> bool {
    let base = model.strip_prefix("ft:").unwrap_or(model);
    base.starts_with("gpt-4") || base.starts_with("gpt-3.5")
}

/// The generation settings of one request.
pub(crate) struct Generation<'a> {
    pub model: &'a str,
    /// `false` for an OpenAI-compatible server reached through a custom base URL.
    pub openai_host: bool,
    /// Configured effort; `None` means [`DEFAULT_REASONING_EFFORT`].
    pub reasoning_effort: Option<&'a str>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
}

impl Generation<'_> {
    /// Add the model and its generation parameters to a chat-completions body.
    pub(crate) fn apply(&self, body: &mut Value) {
        body["model"] = json!(self.model);

        // A compatible server gets the long-standing parameter names; whether it
        // knows the newer ones is anyone's guess.
        if !self.openai_host {
            if let Some(max) = self.max_output_tokens {
                body["max_tokens"] = json!(max);
            }
            if let Some(t) = self.temperature {
                body["temperature"] = json!(t);
            }
            if let Some(effort) = self.reasoning_effort {
                body["reasoning_effort"] = json!(effort);
            }
            return;
        }

        if let Some(max) = self.max_output_tokens {
            body["max_completion_tokens"] = json!(max);
        }

        if is_pre_reasoning(self.model) {
            if let Some(t) = self.temperature {
                body["temperature"] = json!(t);
            }
            return;
        }

        let effort = self.reasoning_effort.unwrap_or(DEFAULT_REASONING_EFFORT);
        body["reasoning_effort"] = json!(effort);
        // With reasoning on, only the default temperature is accepted.
        if effort == "none" {
            if let Some(t) = self.temperature {
                body["temperature"] = json!(t);
            }
        }
    }
}

/// OpenAI's input filter sometimes refuses an ordinary recipe and accepts the
/// same request a moment later.
pub(crate) fn is_prompt_flagged(error_message: &str) -> bool {
    error_message.contains("flagged as potentially violating")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(g: Generation) -> Value {
        let mut b = json!({ "messages": [] });
        g.apply(&mut b);
        b
    }

    #[test]
    fn current_model_gets_reasoning_off_and_the_new_token_cap() {
        let b = body(Generation {
            model: "gpt-6-luna",
            openai_host: true,
            reasoning_effort: None,
            temperature: Some(0.2),
            max_output_tokens: Some(4000),
        });
        assert_eq!(b["model"], "gpt-6-luna");
        assert_eq!(b["reasoning_effort"], "none");
        assert_eq!(b["max_completion_tokens"], 4000);
        assert_eq!(b["temperature"], json!(0.2f32));
        assert!(b.get("max_tokens").is_none());
    }

    // "temperature does not support 0.2 with this model. Only the default (1)
    // value is supported" is what gpt-6-luna answers once reasoning is on.
    #[test]
    fn temperature_is_left_out_when_reasoning_is_on() {
        let b = body(Generation {
            model: "gpt-6-luna",
            openai_host: true,
            reasoning_effort: Some("low"),
            temperature: Some(0.2),
            max_output_tokens: Some(4000),
        });
        assert_eq!(b["reasoning_effort"], "low");
        assert!(b.get("temperature").is_none());
    }

    // "Unrecognized request argument supplied: reasoning_effort" from gpt-4o-mini.
    #[test]
    fn gpt4_family_and_its_fine_tunes_keep_temperature_and_get_no_reasoning() {
        for model in [
            "gpt-4o-mini",
            "gpt-4.1-mini",
            "ft:gpt-4.1-mini-2025-04-14:personal::abc",
            "gpt-3.5-turbo",
        ] {
            let b = body(Generation {
                model,
                openai_host: true,
                reasoning_effort: Some("low"),
                temperature: Some(0.2),
                max_output_tokens: Some(2000),
            });
            assert!(b.get("reasoning_effort").is_none(), "{model}");
            assert_eq!(b["temperature"], json!(0.2f32), "{model}");
            assert_eq!(b["max_completion_tokens"], 2000, "{model}");
        }
    }

    #[test]
    fn compatible_server_gets_the_old_parameter_names() {
        let b = body(Generation {
            model: "some-local-model",
            openai_host: false,
            reasoning_effort: None,
            temperature: Some(0.2),
            max_output_tokens: Some(2000),
        });
        assert_eq!(b["max_tokens"], 2000);
        assert_eq!(b["temperature"], json!(0.2f32));
        assert!(b.get("max_completion_tokens").is_none());
        assert!(b.get("reasoning_effort").is_none());
    }

    #[test]
    fn unset_cap_and_temperature_are_not_sent() {
        let b = body(Generation {
            model: "gpt-6-luna",
            openai_host: true,
            reasoning_effort: None,
            temperature: None,
            max_output_tokens: None,
        });
        assert!(b.get("max_completion_tokens").is_none());
        assert!(b.get("temperature").is_none());
    }

    #[test]
    fn recognises_the_input_filter_refusal() {
        assert!(is_prompt_flagged(
            "Invalid prompt: your prompt was flagged as potentially violating our usage policy."
        ));
        assert!(!is_prompt_flagged("Rate limit reached"));
    }
}

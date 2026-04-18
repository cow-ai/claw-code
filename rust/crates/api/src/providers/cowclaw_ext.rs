//! CowClaw provider extensions — isolated from upstream claw-code.
//!
//! This module contains CowClaw-specific provider logic (ZAI, MiniMax)
//! that is designed to be cleanly separable from upstream code.
//!
//! **Extension points:**
//! - `model_token_limit_fallback()` — called by `model_token_limit()` as a
//!   catch-all for models not recognized by upstream.
//!
//! To integrate with upstream, add ONE hook line in `mod.rs`:
//! ```ignore
//! _ => cowclaw_ext::model_token_limit_fallback(match_key),
//! ```

use crate::providers::ModelTokenLimit;

/// Token limits for CowClaw-specific providers (ZAI GLM, MiniMax).
///
/// Called as the fallback arm of `model_token_limit()` so that adding new
/// CowClaw providers never requires editing upstream's match statement.
///
/// # Arguments
/// * `model` — the already-lowercased, prefix-stripped model key
///   (e.g. `"glm-5.1"`, `"m2.7"`) as produced by `model_token_limit()`.
pub fn model_token_limit_fallback(model: &str) -> Option<ModelTokenLimit> {
    match model {
        // ZAI GLM models (200K context)
        // Source: https://open.bigmodel.cn/dev/api
        "glm-5.1" | "glm-5" | "glm-5-turbo" => Some(ModelTokenLimit {
            max_output_tokens: 16_384,
            context_window_tokens: 200_000,
        }),
        "glm-4.7" | "glm-4.7-flashx" | "glm-4.7-flash" => Some(ModelTokenLimit {
            max_output_tokens: 16_384,
            context_window_tokens: 200_000,
        }),
        "glm-4.6" => Some(ModelTokenLimit {
            max_output_tokens: 16_384,
            context_window_tokens: 200_000,
        }),
        "glm-4.5" | "glm-4.5-air" => Some(ModelTokenLimit {
            max_output_tokens: 16_384,
            context_window_tokens: 128_000,
        }),
        // MiniMax models — canonical ("minimax-m2.7") and short forms
        // produced by prefix stripping ("m2.7" from "minimax/M2.7").
        // Source: https://platform.minimaxi.com/document/Models
        "minimax-m2.7" | "minimax-m2.7-highspeed" | "minimax-m2.5"
        | "m2.7" | "m2.7-highspeed" | "m2.5" => Some(ModelTokenLimit {
            max_output_tokens: 16_384,
            context_window_tokens: 200_000,
        }),
        "minimax-m2" | "m2" => Some(ModelTokenLimit {
            max_output_tokens: 128_000,
            context_window_tokens: 200_000,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glm_5_family_200k_context() {
        for model in ["glm-5.1", "glm-5", "glm-5-turbo"] {
            let limit = model_token_limit_fallback(model).unwrap_or_else(|| {
                panic!("expected token limit for {model}")
            });
            assert_eq!(limit.max_output_tokens, 16_384, "{model}");
            assert_eq!(limit.context_window_tokens, 200_000, "{model}");
        }
    }

    #[test]
    fn glm_4_family_contexts() {
        for model in ["glm-4.7", "glm-4.7-flashx", "glm-4.7-flash", "glm-4.6"] {
            let limit = model_token_limit_fallback(model).unwrap_or_else(|| {
                panic!("expected token limit for {model}")
            });
            assert_eq!(limit.max_output_tokens, 16_384, "{model}");
            assert_eq!(limit.context_window_tokens, 200_000, "{model}");
        }

        // GLM-4.5 family has smaller context
        for model in ["glm-4.5", "glm-4.5-air"] {
            let limit = model_token_limit_fallback(model).unwrap_or_else(|| {
                panic!("expected token limit for {model}")
            });
            assert_eq!(limit.max_output_tokens, 16_384, "{model}");
            assert_eq!(limit.context_window_tokens, 128_000, "{model}");
        }
    }

    #[test]
    fn minimax_models() {
        // M2.7 family
        for model in [
            "minimax-m2.7",
            "minimax-m2.7-highspeed",
            "minimax-m2.5",
            "m2.7",
            "m2.7-highspeed",
            "m2.5",
        ] {
            let limit = model_token_limit_fallback(model).unwrap_or_else(|| {
                panic!("expected token limit for {model}")
            });
            assert_eq!(limit.max_output_tokens, 16_384, "{model}");
            assert_eq!(limit.context_window_tokens, 200_000, "{model}");
        }

        // M2 has larger output
        for model in ["minimax-m2", "m2"] {
            let limit = model_token_limit_fallback(model).unwrap_or_else(|| {
                panic!("expected token limit for {model}")
            });
            assert_eq!(limit.max_output_tokens, 128_000, "{model}");
            assert_eq!(limit.context_window_tokens, 200_000, "{model}");
        }
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(model_token_limit_fallback("gpt-4").is_none());
        assert!(model_token_limit_fallback("claude-opus-4-6").is_none());
        assert!(model_token_limit_fallback("nonexistent-model").is_none());
    }
}

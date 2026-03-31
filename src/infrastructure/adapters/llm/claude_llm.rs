use std::time::Duration;

use anthropic_api_key_from_local_file::{anthropic_api_key_from_local_file, AnthropicApiKeyError};
use serde_json::json;

use crate::core::llm::Llm;

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
// Model alias from Anthropic; update if the API returns 404 for `model`.
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const MAX_TOKENS: u32 = 4096;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Calls the [Anthropic Messages API](https://docs.anthropic.com/en/api/messages) using an API key from
/// [`anthropic_api_key_from_local_file`] (default path) or from [`ClaudeLlm::from_api_key`].
pub struct ClaudeLlm {
    api_key: String,
    client: reqwest::blocking::Client,
    model: String,
}

impl ClaudeLlm {
    /// Builds a client using the key returned by [`anthropic_api_key_from_local_file`](`None`).
    pub fn load_from_default_key_file() -> Result<Self, AnthropicApiKeyError> {
        let api_key = anthropic_api_key_from_local_file(None)?;
        Ok(Self::from_api_key(api_key))
    }

    /// Builds a client with an explicit API key (e.g. from tests or another secret source).
    pub fn from_api_key(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            api_key,
            client,
            model: DEFAULT_MODEL.to_string(),
        }
    }

    fn complete_message(&self, user_message: &str) -> Result<String, String> {
        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "messages": [
                { "role": "user", "content": user_message }
            ]
        });

        let response = self
            .client
            .post(ANTHROPIC_MESSAGES_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;

        let status = response.status();
        let bytes = response
            .bytes()
            .map_err(|e| e.to_string())?;

        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes);
            return Err(format!("HTTP {status}: {text}"));
        }

        let v: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON: {e}"))?;

        let mut out = String::new();
        if let Some(blocks) = v["content"].as_array() {
            for block in blocks {
                append_text_block(block, &mut out);
            }
        }

        if out.is_empty() {
            return Err("empty or unrecognized assistant content in response".to_string());
        }

        Ok(out)
    }
}

fn append_text_block(block: &serde_json::Value, out: &mut String) {
    if block["type"].as_str() != Some("text") {
        return;
    }
    if let Some(t) = block["text"].as_str() {
        out.push_str(t);
    }
}

impl Llm for ClaudeLlm {
    fn receive_message(&self, message: &str) -> String {
        match self.complete_message(message) {
            Ok(text) => text,
            Err(e) => format!("(anthropic error) {e}"),
        }
    }
}

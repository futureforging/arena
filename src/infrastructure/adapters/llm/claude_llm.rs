use serde_json::{json, Map, Value};

use crate::{
    core::llm::{ChatMessage, Llm, LlmCompletion},
    infrastructure::adapters::transport::JsonHttp,
};

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
// Model alias from Anthropic; update if the API returns 404 for `model`.
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const MAX_TOKENS: u32 = 4096;

/// Calls the [Anthropic Messages API](https://docs.anthropic.com/en/api/messages).
///
/// Construct with [`ClaudeLlm::new`]. Load the API key elsewhere (e.g. [`crate::infrastructure::adapters::runtime::LocalFileRuntime`] and [`crate::core::runtime::Runtime::get_secret`]), then pass the key and an optional **base** system prompt (merged per request with the session system prompt).
pub struct ClaudeLlm {
    api_key: String,
    http: JsonHttp,
    model: String,
    system_prompt: Option<String>,
}

impl ClaudeLlm {
    /// Creates a client. Pass [`None`] for `system_prompt` to omit a base system block when merging (unusual).
    pub fn new(api_key: impl Into<String>, system_prompt: Option<String>) -> Self {
        let api_key = api_key.into();
        let model = DEFAULT_MODEL.to_string();
        let http = JsonHttp::new();
        Self {
            api_key,
            http,
            model,
            system_prompt,
        }
    }

    fn build_request_body(
        &self,
        system: Option<&str>,
        messages: &[ChatMessage],
    ) -> (Value, Option<String>) {
        let json_messages: Vec<Value> = messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();

        let mut map = Map::new();
        map.insert("model".to_string(), json!(self.model));
        map.insert("max_tokens".to_string(), json!(MAX_TOKENS));
        map.insert("messages".to_string(), Value::Array(json_messages));
        if let Some(s) = system {
            if !s.is_empty() {
                map.insert("system".to_string(), json!(s));
            }
        }
        let body = Value::Object(map);
        let pretty = serde_json::to_string_pretty(&body).ok();
        (body, pretty)
    }

    fn post_messages_request(&self, body: &Value) -> Result<String, String> {
        let headers = [
            (
                "x-api-key",
                self.api_key
                    .as_str(),
            ),
            ("anthropic-version", ANTHROPIC_VERSION),
            ("content-type", "application/json"),
        ];
        let bytes = self
            .http
            .post_json(ANTHROPIC_MESSAGES_URL, &headers, body)?;

        let v: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON: {e}"))?;

        let mut out = String::new();
        if let Some(blocks) = v["content"].as_array() {
            for block in blocks {
                append_text_block(block, &mut out);
            }
        }

        if out.is_empty() {
            return Err("empty or unrecognized text in Messages API response content".to_string());
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
    fn base_system_prompt(&self) -> Option<&str> {
        self.system_prompt
            .as_deref()
    }

    fn complete(&self, system: Option<&str>, messages: &[ChatMessage]) -> LlmCompletion {
        let (body, request_body_json) = self.build_request_body(system, messages);
        match self.post_messages_request(&body) {
            Ok(text) => LlmCompletion {
                reply: text,
                request_body_json,
            },
            Err(e) => LlmCompletion {
                reply: format!("(anthropic error) {e}"),
                request_body_json,
            },
        }
    }
}

use secure_core::llm::{ChatMessage, Llm, LlmCompletion};
use bytes::Bytes;
use http_body_util::Full;
use serde_json::{json, Map, Value};

const VAULT_LOCKER_ID: &str = "secure-anthropic";
const VAULT_SECRET_ID: &str = "anthropic_api_key";
const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const MAX_TOKENS: u32 = 4096;

pub struct WasiLlm {
    api_key: String,
}

impl WasiLlm {
    /// Retrieves the API key from WASI vault.
    pub async fn new() -> anyhow::Result<Self> {
        let locker = omnia_wasi_vault::vault::open(VAULT_LOCKER_ID.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("opening vault locker: {e:?}"))?;
        let bytes = locker
            .get(VAULT_SECRET_ID.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("getting API key: {e:?}"))?
            .ok_or_else(|| anyhow::anyhow!("API key not found in vault"))?;
        let api_key = String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("API key not valid UTF-8: {e}"))?;
        Ok(Self { api_key })
    }
}

impl Llm for WasiLlm {
    fn complete(&self, system: Option<&str>, messages: &[ChatMessage]) -> LlmCompletion {
        let result = wit_bindgen::block_on(self.complete_async(system, messages));
        match result {
            Ok(reply) => LlmCompletion {
                reply,
                request_body_json: None,
            },
            Err(e) => LlmCompletion {
                reply: format!("(anthropic error) {e}"),
                request_body_json: None,
            },
        }
    }
}

impl WasiLlm {
    /// Async Anthropic completion (use this from async guest code; avoid [`Llm::complete`]'s `block_on` there).
    pub async fn complete_async(&self, system: Option<&str>, messages: &[ChatMessage]) -> anyhow::Result<String> {
        let body = self.build_request_body(system, messages);
        let body_bytes = serde_json::to_vec(&body)?;

        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(ANTHROPIC_MESSAGES_URL)
            .header("content-type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .body(Full::new(Bytes::from(body_bytes)))?;

        let response = omnia_wasi_http::handle(request).await?;
        let response_body = response.into_body();
        let v: Value = serde_json::from_slice(&response_body)?;

        let mut out = String::new();
        if let Some(blocks) = v["content"].as_array() {
            for block in blocks {
                if block["type"].as_str() != Some("text") {
                    continue;
                }
                if let Some(t) = block["text"].as_str() {
                    out.push_str(t);
                }
            }
        }

        if out.is_empty() {
            anyhow::bail!("empty or unrecognized text in Anthropic API response");
        }

        Ok(out)
    }

    fn build_request_body(&self, system: Option<&str>, messages: &[ChatMessage]) -> Value {
        let json_messages: Vec<Value> = messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();

        let mut map = Map::new();
        map.insert("model".to_string(), json!(DEFAULT_MODEL));
        map.insert("max_tokens".to_string(), json!(MAX_TOKENS));
        map.insert("messages".to_string(), Value::Array(json_messages));
        if let Some(s) = system {
            if !s.is_empty() {
                map.insert("system".to_string(), json!(s));
            }
        }
        Value::Object(map)
    }
}

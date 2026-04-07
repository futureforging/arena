//! Outbound HTTP POST with JSON body.

use serde_json::{json, Value};
use verity_core::tool::{Tool, ToolDescriptor, ToolError};

const HTTP_CLIENT_NAME: &str = "http_client";
const HTTP_CLIENT_DESCRIPTION: &str =
    "Send an outbound HTTP POST with a JSON body and return the response";

type OwnedHeaderPairs = Vec<(String, String)>;

fn http_client_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "url": { "type": "string" },
            "headers": { "type": "object" },
            "body": { "type": "object" }
        },
        "required": ["url", "body"]
    })
}

/// Performs HTTP POST via a caller-supplied transport closure.
pub struct HttpClientTool<F>
where
    F: Fn(&str, &[(&str, &str)], &Value) -> Result<Vec<u8>, String> + Send + Sync,
{
    post_json: F,
}

impl<F> HttpClientTool<F>
where
    F: Fn(&str, &[(&str, &str)], &Value) -> Result<Vec<u8>, String> + Send + Sync,
{
    /// Wraps the given POST closure as a [`Tool`].
    pub fn new(post_json: F) -> Self {
        Self {
            post_json,
        }
    }

    fn parse_headers(input: &Value) -> Result<OwnedHeaderPairs, ToolError> {
        let Some(obj) = input.get("headers") else {
            return Ok(Vec::new());
        };
        let Some(map) = obj.as_object() else {
            return Err(ToolError::InvalidInput("headers must be a JSON object".to_string()));
        };
        let mut out = Vec::with_capacity(map.len());
        for (k, v) in map {
            let vs = v
                .as_str()
                .ok_or_else(|| {
                    ToolError::InvalidInput(format!("header value for {k:?} must be a string"))
                })?;
            out.push((k.clone(), vs.to_string()));
        }
        Ok(out)
    }
}

impl<F> Tool for HttpClientTool<F>
where
    F: Fn(&str, &[(&str, &str)], &Value) -> Result<Vec<u8>, String> + Send + Sync,
{
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: HTTP_CLIENT_NAME,
            description: HTTP_CLIENT_DESCRIPTION,
            input_schema: http_client_input_schema(),
        }
    }

    fn execute(&self, input: &Value) -> Result<Value, ToolError> {
        let url = input
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("url must be a string".to_string()))?;
        let body = input
            .get("body")
            .ok_or_else(|| ToolError::InvalidInput("body is required".to_string()))?;
        if !body.is_object() {
            return Err(ToolError::InvalidInput("body must be a JSON object".to_string()));
        }
        let owned_headers = Self::parse_headers(input)?;
        let header_refs: Vec<(&str, &str)> = owned_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let bytes =
            (self.post_json)(url, &header_refs, body).map_err(ToolError::ExecutionFailed)?;
        let response_body = match serde_json::from_slice::<Value>(&bytes) {
            Ok(v) => v,
            Err(_) => Value::String(String::from_utf8_lossy(&bytes).into_owned()),
        };
        Ok(json!({ "response_body": response_body }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn execute_with_valid_input_returns_response() {
        let tool = HttpClientTool::new(|_url, _headers, _body| Ok(br#"{"x":1}"#.to_vec()));
        let out = tool
            .execute(&json!({
                "url": "https://example.com",
                "body": { "a": 1 }
            }))
            .unwrap();
        assert_eq!(out, json!({ "response_body": { "x": 1 } }));
    }

    #[test]
    fn execute_with_missing_url_returns_invalid_input() {
        let tool = HttpClientTool::new(|_, _, _| Ok(vec![]));
        let err = tool
            .execute(&json!({ "body": {} }))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn execute_with_missing_body_returns_invalid_input() {
        let tool = HttpClientTool::new(|_, _, _| Ok(vec![]));
        let err = tool
            .execute(&json!({ "url": "https://example.com" }))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn execute_with_closure_error_returns_execution_failed() {
        let tool = HttpClientTool::new(|_, _, _| Err("network down".to_string()));
        let err = tool
            .execute(&json!({
                "url": "https://example.com",
                "body": {}
            }))
            .unwrap_err();
        assert_eq!(err, ToolError::ExecutionFailed("network down".to_string()));
    }

    #[test]
    fn execute_with_non_json_response_returns_raw_string() {
        let tool = HttpClientTool::new(|_, _, _| Ok(b"not json".to_vec()));
        let out = tool
            .execute(&json!({
                "url": "https://example.com",
                "body": {}
            }))
            .unwrap();
        assert_eq!(out, json!({ "response_body": "not json" }));
    }
}

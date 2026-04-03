use serde_json::json;

use crate::core::{
    arena::{Arena, ArenaError},
    transport::{BoxedPostJsonTransport, IntoBoxedPostJsonTransport},
};

/// [`Arena`] backed by an HTTP endpoint (e.g. the `arena-stub` server).
///
/// Sends each message as `POST {base_url}/message` with body `{"message": "..."}` and
/// parses `{"reply": "..."}` from the response.
pub struct ArenaHttpClient {
    base_url: String,
    transport: BoxedPostJsonTransport,
}

impl ArenaHttpClient {
    /// Creates a client targeting the given base URL (e.g. `"http://127.0.0.1:3000"`).
    pub fn new(base_url: impl Into<String>, transport: impl IntoBoxedPostJsonTransport) -> Self {
        Self {
            base_url: base_url.into(),
            transport: transport.into_boxed_post_json_transport(),
        }
    }
}

impl Arena for ArenaHttpClient {
    fn send(&self, message: &str) -> Result<String, ArenaError> {
        let url = format!("{}/message", self.base_url);
        let body = json!({"message": message});
        let headers = [("content-type", "application/json")];

        let bytes = self
            .transport
            .post_json(&url, &headers, &body)
            .map_err(|e| ArenaError::Other(e.to_string()))?;

        let v: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from arena: {e}")))?;

        v["reply"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ArenaError::Other("arena response missing \"reply\" field".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::ArenaHttpClient;
    use crate::{
        core::arena::{Arena, ArenaError},
        test_support::StubPostJsonTransport,
    };

    #[test]
    fn send_returns_reply_when_json_valid() -> Result<(), String> {
        let transport = StubPostJsonTransport::with_response(br#"{"reply":"yes"}"#.to_vec());
        let client = ArenaHttpClient::new("http://example.test", transport);
        let got = client
            .send("Would you like to hear a knock-knock joke?")
            .map_err(|e| format!("{e}"))?;
        assert_eq!(got, "yes");
        Ok(())
    }

    #[test]
    fn send_errors_when_reply_field_missing() -> Result<(), String> {
        let transport = StubPostJsonTransport::with_response(br#"{"something":"else"}"#.to_vec());
        let client = ArenaHttpClient::new("http://example.test", transport);
        let err = match client.send("x") {
            Ok(_) => {
                return Err(String::from("expected Err for missing reply"));
            },
            Err(e) => e,
        };
        match err {
            ArenaError::Other(msg) if msg.contains("reply") => Ok(()),
            other => Err(format!("unexpected error: {other:?}")),
        }
    }

    #[test]
    fn send_errors_on_invalid_json_body() -> Result<(), String> {
        let transport = StubPostJsonTransport::with_response(b"not json".to_vec());
        let client = ArenaHttpClient::new("http://example.test", transport);
        let err = match client.send("x") {
            Ok(_) => {
                return Err(String::from("expected Err for invalid JSON"));
            },
            Err(e) => e,
        };
        match err {
            ArenaError::Other(msg) if msg.contains("invalid JSON") => Ok(()),
            other => Err(format!("unexpected error: {other:?}")),
        }
    }

    #[test]
    fn send_maps_transport_error_to_arena_other() -> Result<(), String> {
        let transport = StubPostJsonTransport::with_error("upstream failed");
        let client = ArenaHttpClient::new("http://example.test", transport);
        let err = match client.send("x") {
            Ok(_) => {
                return Err(String::from("expected Err from transport"));
            },
            Err(e) => e,
        };
        match err {
            ArenaError::Other(msg) if msg == "upstream failed" => Ok(()),
            other => Err(format!("unexpected error: {other:?}")),
        }
    }
}

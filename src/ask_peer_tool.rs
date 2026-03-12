use std::sync::Arc;

use async_trait::async_trait;
use claude_agent::tools::{ExecutionContext, Tool};
use claude_agent::types::ToolResult;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Custom tool that sends a message to a peer over TCP and returns the response.
#[derive(Clone)]
pub struct AskPeerTool {
    pub peer: Arc<str>,
}

impl AskPeerTool {
    pub fn new(peer: impl Into<Arc<str>>) -> Self {
        Self {
            peer: peer.into(),
        }
    }
}

#[async_trait]
impl Tool for AskPeerTool {
    fn name(&self) -> &str {
        "ask_peer"
    }

    fn description(&self) -> &str {
        "Send a message to your peer Agent B and receive a response. Use this to ask your peer questions."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to send to your peer"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ExecutionContext,
    ) -> ToolResult {
        let message = match input.get("message").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => {
                return ToolResult::error("Error: missing 'message' parameter");
            }
        };

        let stream = match TcpStream::connect(self.peer.as_ref()).await {
            Ok(s) => s,
            Err(e) => {
                return ToolResult::error(format!("Error connecting to peer: {e}"));
            }
        };

        let (reader, mut writer) = stream.into_split();
        if let Err(e) = writer.write_all(format!("{}\n", message).as_bytes()).await {
            return ToolResult::error(format!("Error sending message: {e}"));
        }
        if let Err(e) = writer.shutdown().await {
            return ToolResult::error(format!("Error shutting down write: {e}"));
        }

        let mut buf_reader = BufReader::new(reader);
        let mut response = String::new();
        if let Err(e) = buf_reader.read_line(&mut response).await {
            return ToolResult::error(format!("Error reading response: {e}"));
        }

        let response = response.trim_end_matches('\n').trim_end_matches('\r');
        ToolResult::success(response)
    }
}

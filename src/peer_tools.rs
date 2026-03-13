use std::env;
use std::fs;
use std::sync::Arc;

use async_trait::async_trait;
use claude_agent::tools::{ExecutionContext, Tool};
use claude_agent::types::ToolResult;
use rand::seq::IteratorRandom;
use tokio::sync::Mutex;

use crate::peer_protocol::{self, PeerConnection};

/// Logs messages in clean output mode. (direction, message) where direction is "sent" or "received".
pub type MessageLogFn = Arc<dyn Fn(&str, &str) + Send + Sync>;

/// Sends a message to the peer and returns their response.
#[derive(Clone)]
pub struct SendToPeerTool {
    connection: Arc<Mutex<PeerConnection>>,
    message_log: Option<MessageLogFn>,
}

impl SendToPeerTool {
    pub fn new(connection: Arc<Mutex<PeerConnection>>) -> Self {
        Self {
            connection,
            message_log: None,
        }
    }

    pub fn with_message_log(mut self, log: MessageLogFn) -> Self {
        self.message_log = Some(log);
        self
    }
}

#[async_trait]
impl Tool for SendToPeerTool {
    fn name(&self) -> &str {
        "send_to_peer"
    }

    fn description(&self) -> &str {
        "Send a message to your peer and receive their response. Use this to have a turn in the conversation. You may call this multiple times."
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

    async fn execute(&self, input: serde_json::Value, _context: &ExecutionContext) -> ToolResult {
        let message = match input
            .get("message")
            .and_then(|v| v.as_str())
        {
            Some(m) => m.to_string(),
            None => {
                return ToolResult::error("Error: missing 'message' parameter");
            },
        };

        let mut conn = self
            .connection
            .lock()
            .await;
        if let Some(ref log) = self.message_log {
            log("sent", &message);
        }
        if let Err(e) = conn
            .write_message(&message)
            .await
        {
            return ToolResult::error(format!("Error sending message: {e}"));
        }
        match conn
            .read_message()
            .await
        {
            Ok(response) => {
                if peer_protocol::is_end_sentinel(&response) {
                    let _ = conn
                        .write_sentinel()
                        .await;
                    ToolResult::success("[Conversation ended by peer]")
                } else {
                    ToolResult::success(response)
                }
            },
            Err(e) => ToolResult::error(format!("Error reading response: {e}")),
        }
    }
}

/// Waits for a message from the peer and returns it.
#[derive(Clone)]
pub struct ReceiveFromPeerTool {
    connection: Arc<Mutex<PeerConnection>>,
    message_log: Option<MessageLogFn>,
}

impl ReceiveFromPeerTool {
    pub fn new(connection: Arc<Mutex<PeerConnection>>) -> Self {
        Self {
            connection,
            message_log: None,
        }
    }

    pub fn with_message_log(mut self, log: MessageLogFn) -> Self {
        self.message_log = Some(log);
        self
    }
}

#[async_trait]
impl Tool for ReceiveFromPeerTool {
    fn name(&self) -> &str {
        "receive_from_peer"
    }

    fn description(&self) -> &str {
        "Wait for a message from your peer. Use this when it is their turn to speak. Returns the message they sent."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: serde_json::Value, _context: &ExecutionContext) -> ToolResult {
        let mut conn = self
            .connection
            .lock()
            .await;
        match conn
            .read_message()
            .await
        {
            Ok(response) => {
                if peer_protocol::is_end_sentinel(&response) {
                    let _ = conn
                        .write_sentinel()
                        .await;
                    ToolResult::success("[Conversation ended by peer]")
                } else {
                    ToolResult::success(response)
                }
            },
            Err(e) => ToolResult::error(format!("Error reading message: {e}")),
        }
    }
}

/// Selects a random knock-knock joke from the approved list.
#[derive(Clone)]
pub struct JokePickerTool;

#[async_trait]
impl Tool for JokePickerTool {
    fn name(&self) -> &str {
        "joke_picker"
    }

    fn description(&self) -> &str {
        "Select a random knock-knock joke from the approved list. You MUST use this tool to pick your joke before telling it—do not make up or recall jokes from memory."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: serde_json::Value, _context: &ExecutionContext) -> ToolResult {
        let path = env::var("JOKES_FILE").unwrap_or_else(|_| "ref/jokes.txt".to_string());

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Error reading jokes file: {e}")),
        };

        let jokes: Vec<(String, String)> = content
            .split("\n\n")
            .filter_map(|block| {
                let lines: Vec<&str> = block
                    .trim()
                    .lines()
                    .collect();
                if lines.len() >= 2 {
                    let setup = lines[0]
                        .trim()
                        .to_string();
                    let punchline = lines[1]
                        .trim()
                        .to_string();
                    if !setup.is_empty() && !punchline.is_empty() {
                        return Some((setup, punchline));
                    }
                }
                None
            })
            .collect();

        let (setup, punchline) = match jokes
            .iter()
            .choose(&mut rand::thread_rng())
        {
            Some(joke) => joke.clone(),
            None => return ToolResult::error("No jokes found in file"),
        };

        let result = format!("Setup: {setup}\nPunchline: {punchline}");
        ToolResult::success(result)
    }
}

use std::env;
use std::fs;
use std::sync::Arc;

use rand::seq::IteratorRandom;

use async_trait::async_trait;
use claude_agent::tools::{ExecutionContext, Tool};
use claude_agent::types::ToolResult;
use rand::Rng;
use sha2::{Digest, Sha256};
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

/// Cryptographic operations for commitment schemes. Agents must use only this tool for crypto.
#[derive(Clone)]
pub struct CryptoTool;

#[async_trait]
impl Tool for CryptoTool {
    fn name(&self) -> &str {
        "crypto"
    }

    fn description(&self) -> &str {
        "Perform cryptographic operations for commitment schemes. Use sha256_hex to compute SHA-256 hashes (e.g. for commitments). Use generate_nonce to obtain a random nonce for binding."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["sha256_hex", "generate_nonce"],
                    "description": "Operation to perform"
                },
                "data": {
                    "type": "string",
                    "description": "Input data for sha256_hex (required when operation is sha256_hex)"
                },
                "prefix": {
                    "type": "string",
                    "description": "Optional prefix for generate_nonce output"
                }
            },
            "required": ["operation"]
        })
    }

    async fn execute(&self, input: serde_json::Value, _context: &ExecutionContext) -> ToolResult {
        let op = match input.get("operation").and_then(|v| v.as_str()) {
            Some(o) => o,
            None => return ToolResult::error("Error: missing 'operation' parameter"),
        };

        match op {
            "sha256_hex" => {
                let data = input
                    .get("data")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mut hasher = Sha256::new();
                hasher.update(data.as_bytes());
                let result = hasher.finalize();
                let hex = format!("{:x}", result);
                ToolResult::success(hex)
            },
            "generate_nonce" => {
                let prefix = input
                    .get("prefix")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let bytes: [u8; 16] = rand::thread_rng().gen();
                let hex: String = bytes
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();
                let nonce = if prefix.is_empty() {
                    hex
                } else {
                    format!("{}_{}", prefix, hex)
                };
                ToolResult::success(nonce)
            },
            _ => ToolResult::error(format!("Error: unknown operation '{op}'")),
        }
    }
}

/// Selects a random protocol strategy from the approved list.
#[derive(Clone)]
pub struct StrategyPickerTool;

#[async_trait]
impl Tool for StrategyPickerTool {
    fn name(&self) -> &str {
        "strategy_picker"
    }

    fn description(&self) -> &str {
        "Select a random protocol strategy from the approved list. You MUST use this tool to pick your strategy before proposing it to your peer—do not invent or recall strategies from memory."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: serde_json::Value, _context: &ExecutionContext) -> ToolResult {
        let path = env::var("STRATEGIES_FILE").unwrap_or_else(|_| "ref/strategies.txt".to_string());

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => return ToolResult::error(format!("Error reading strategies file: {e}")),
        };

        let strategies: Vec<String> = content
            .split("\n\n")
            .filter_map(|block| {
                let s = block.trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            })
            .collect();

        let strategy = match strategies.iter().choose(&mut rand::thread_rng()) {
            Some(s) => s.clone(),
            None => return ToolResult::error("No strategies found in file"),
        };

        ToolResult::success(strategy)
    }
}

/// Loads the negotiation protocol by name (from NEGOTIATION_PROTOCOL env).
/// Returns protocol content for injection into the prompt at startup.
pub fn load_negotiation_protocol() -> Result<String, String> {
    let name = env::var("NEGOTIATION_PROTOCOL")
        .map_err(|_| "NEGOTIATION_PROTOCOL must be set".to_string())?;
    let name = name.trim();
    let path = env::var("NEGOTIATION_PROTOCOLS_FILE")
        .unwrap_or_else(|_| "ref/negotiation-protocols.txt".to_string());
    let protocols = load_negotiation_protocols(&path)?;
    protocols
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(n, c)| format!("Protocol: {}\n\n{}", n, c))
        .ok_or_else(|| format!("Protocol '{}' not found in {}", name, path))
}

/// Picks a random strategy from the strategies file for connector injection at startup.
pub fn pick_random_strategy() -> Result<String, String> {
    let path = env::var("STRATEGIES_FILE").unwrap_or_else(|_| "ref/strategies.txt".to_string());
    let content = fs::read_to_string(&path).map_err(|e| format!("Error reading strategies: {e}"))?;
    let strategies: Vec<String> = content
        .split("\n\n")
        .filter_map(|block| {
            let s = block.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
        .collect();
    strategies
        .iter()
        .choose(&mut rand::thread_rng())
        .cloned()
        .ok_or_else(|| "No strategies found in file".to_string())
}

/// Loads negotiation protocols from file. Format: `=== name ===` followed by content.
fn load_negotiation_protocols(path: &str) -> Result<Vec<(String, String)>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Error reading file: {e}"))?;
    let mut protocols = Vec::new();
    // Prepend newline so first protocol is parsed correctly.
    let content = format!("\n{}", content);
    for block in content.split("\n=== ") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let first_newline = block.find('\n').unwrap_or(block.len());
        let first_line = block[..first_newline].trim();
        let name = first_line
            .strip_suffix(" ===")
            .or_else(|| first_line.strip_suffix("==="))
            .unwrap_or(first_line)
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let body = if first_newline < block.len() {
            block[first_newline + 1..].trim()
        } else {
            ""
        };
        protocols.push((name, body.to_string()));
    }
    Ok(protocols)
}

/// Load negotiation protocol by name from NEGOTIATION_PROTOCOL env. For startup injection.
pub fn load_negotiation_protocol_for_startup() -> Result<String, String> {
    let name = env::var("NEGOTIATION_PROTOCOL")
        .map_err(|_| "NEGOTIATION_PROTOCOL must be set".to_string())?;
    let path = env::var("NEGOTIATION_PROTOCOLS_FILE")
        .unwrap_or_else(|_| "ref/negotiation-protocols.txt".to_string());
    let protocols = load_negotiation_protocols(&path)?;
    protocols
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name.trim()))
        .map(|(n, c)| format!("Protocol: {}\n\n{}", n, c))
        .ok_or_else(|| format!("Protocol '{}' not found", name.trim()))
}

/// Returns the negotiation protocol by name (from NEGOTIATION_PROTOCOL env) or at random.
/// Used by both connector and listener.
#[derive(Clone)]
pub struct NegotiationProtocolPickerTool;

#[async_trait]
impl Tool for NegotiationProtocolPickerTool {
    fn name(&self) -> &str {
        "negotiation_protocol_picker"
    }

    fn description(&self) -> &str {
        "Get the negotiation protocol to follow. Call this FIRST. Returns step-by-step instructions for your role (connector or listener). Follow it exactly to coordinate with your peer."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _input: serde_json::Value, _context: &ExecutionContext) -> ToolResult {
        let path = env::var("NEGOTIATION_PROTOCOLS_FILE")
            .unwrap_or_else(|_| "ref/negotiation-protocols.txt".to_string());

        let protocols = match load_negotiation_protocols(&path) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(e),
        };

        if protocols.is_empty() {
            return ToolResult::error("No negotiation protocols found in file");
        }

        let (name, content) = if let Ok(env_name) = env::var("NEGOTIATION_PROTOCOL") {
            let env_name = env_name.trim();
            match protocols.iter().find(|(n, _)| n.eq_ignore_ascii_case(env_name)) {
                Some(p) => p.clone(),
                None => {
                    return ToolResult::error(format!(
                        "Protocol '{}' not found. Set NEGOTIATION_PROTOCOL to a valid name.",
                        env_name
                    ));
                },
            }
        } else {
            protocols
                .iter()
                .choose(&mut rand::thread_rng())
                .cloned()
                .expect("protocols not empty")
        };

        ToolResult::success(format!("Protocol: {}\n\n{}", name, content))
    }
}

/// Load negotiation protocol by name. Uses NEGOTIATION_PROTOCOL env and NEGOTIATION_PROTOCOLS_FILE.
/// Returns protocol content for injection into prompt.
pub fn load_protocol_for_injection() -> Result<String, String> {
    let name = env::var("NEGOTIATION_PROTOCOL")
        .map_err(|_| "NEGOTIATION_PROTOCOL must be set".to_string())?;
    let name = name.trim();
    let path = env::var("NEGOTIATION_PROTOCOLS_FILE")
        .unwrap_or_else(|_| "ref/negotiation-protocols.txt".to_string());
    let protocols = load_negotiation_protocols(&path)?;
    protocols
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(n, c)| format!("Protocol: {}\n\n{}", n, c))
        .ok_or_else(|| format!("Protocol '{}' not found in {}", name, path))
}

/// Pick a random strategy from strategies.txt for injection into connector prompt.
pub fn pick_strategy_for_injection() -> Result<String, String> {
    let path = env::var("STRATEGIES_FILE").unwrap_or_else(|_| "ref/strategies.txt".to_string());
    let content = fs::read_to_string(&path).map_err(|e| format!("Error reading strategies: {e}"))?;
    let strategies: Vec<String> = content
        .split("\n\n")
        .filter_map(|block| {
            let s = block.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
        .collect();
    strategies
        .iter()
        .choose(&mut rand::thread_rng())
        .cloned()
        .ok_or_else(|| "No strategies found in file".to_string())
}

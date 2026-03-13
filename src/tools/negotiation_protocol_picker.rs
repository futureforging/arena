use std::env;
use std::fs;

use async_trait::async_trait;
use claude_agent::tools::{ExecutionContext, Tool};
use claude_agent::types::ToolResult;
use rand::seq::IteratorRandom;

/// (name, body) pairs from the negotiation protocols file.
type ProtocolList = Vec<(String, String)>;

/// Loads negotiation protocols from file. Format: `=== name ===` followed by content.
fn load_negotiation_protocols(path: &str) -> Result<ProtocolList, String> {
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
                }
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

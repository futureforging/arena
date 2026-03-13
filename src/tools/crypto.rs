use async_trait::async_trait;
use claude_agent::tools::{ExecutionContext, Tool};
use claude_agent::types::ToolResult;
use rand::Rng;
use sha2::{Digest, Sha256};

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
                let data = input.get("data").and_then(|v| v.as_str()).unwrap_or("");
                let mut hasher = Sha256::new();
                hasher.update(data.as_bytes());
                let result = hasher.finalize();
                let hex = format!("{:x}", result);
                ToolResult::success(hex)
            }
            "generate_nonce" => {
                let prefix = input.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
                let bytes: [u8; 16] = rand::thread_rng().gen();
                let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
                let nonce = if prefix.is_empty() {
                    hex
                } else {
                    format!("{}_{}", prefix, hex)
                };
                ToolResult::success(nonce)
            }
            _ => ToolResult::error(format!("Error: unknown operation '{op}'")),
        }
    }
}

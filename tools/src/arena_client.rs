//! Send messages to an arena peer and receive replies.

use secure_core::tool::{Tool, ToolDescriptor, ToolError};
use serde_json::{json, Value};

const ARENA_NAME: &str = "arena";
const ARENA_DESCRIPTION: &str = "Send a message to the arena peer and receive a reply";

fn arena_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "message": { "type": "string" }
        },
        "required": ["message"]
    })
}

/// Sends arena messages via a caller-supplied transport closure.
pub struct ArenaClientTool<F>
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    send_message: F,
}

impl<F> ArenaClientTool<F>
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    /// Wraps the given send closure as a [`Tool`].
    pub fn new(send_message: F) -> Self {
        Self {
            send_message,
        }
    }
}

impl<F> Tool for ArenaClientTool<F>
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: ARENA_NAME,
            description: ARENA_DESCRIPTION,
            input_schema: arena_input_schema(),
        }
    }

    fn execute(&self, input: &Value) -> Result<Value, ToolError> {
        let message = input
            .get("message")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("message must be a string".to_string()))?;
        let reply = (self.send_message)(message).map_err(ToolError::ExecutionFailed)?;
        Ok(json!({ "reply": reply }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn execute_with_valid_message_returns_reply() {
        let tool = ArenaClientTool::new(|m| Ok(format!("echo:{m}")));
        let out = tool
            .execute(&json!({ "message": "hello" }))
            .unwrap();
        assert_eq!(out, json!({ "reply": "echo:hello" }));
    }

    #[test]
    fn execute_with_missing_message_returns_invalid_input() {
        let tool = ArenaClientTool::new(|_| Ok("x".to_string()));
        let err = tool
            .execute(&json!({}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn execute_with_closure_error_returns_execution_failed() {
        let tool = ArenaClientTool::new(|_| Err("disconnected".to_string()));
        let err = tool
            .execute(&json!({ "message": "hi" }))
            .unwrap_err();
        assert_eq!(err, ToolError::ExecutionFailed("disconnected".to_string()));
    }
}

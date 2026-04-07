//! Retrieve named secrets from the runtime vault.

use secure_core::tool::{Tool, ToolDescriptor, ToolError};
use serde_json::{json, Value};

const SECRETS_NAME: &str = "secrets";
const SECRETS_DESCRIPTION: &str = "Retrieve a named secret from the runtime vault";

fn secrets_input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "secret_name": { "type": "string" }
        },
        "required": ["secret_name"]
    })
}

/// Retrieves a named secret via a caller-supplied vault accessor.
pub struct SecretsTool<F>
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    get_secret: F,
}

impl<F> SecretsTool<F>
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    /// Wraps the given secret lookup closure as a [`Tool`].
    pub fn new(get_secret: F) -> Self {
        Self {
            get_secret,
        }
    }

    fn secret_name_from_input(input: &Value) -> Result<&str, ToolError> {
        let name = input
            .get("secret_name")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidInput("secret_name must be a string".to_string()))?;
        Ok(name)
    }
}

impl<F> Tool for SecretsTool<F>
where
    F: Fn(&str) -> Result<String, String> + Send + Sync,
{
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: SECRETS_NAME,
            description: SECRETS_DESCRIPTION,
            input_schema: secrets_input_schema(),
        }
    }

    fn execute(&self, input: &Value) -> Result<Value, ToolError> {
        let secret_name = Self::secret_name_from_input(input)?;
        let value = (self.get_secret)(secret_name).map_err(ToolError::ExecutionFailed)?;
        Ok(json!({ "value": value }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn execute_with_valid_secret_name_returns_value() {
        let tool = SecretsTool::new(|_| Ok("key123".to_string()));
        let out = tool
            .execute(&json!({ "secret_name": "api_key" }))
            .unwrap();
        assert_eq!(out, json!({ "value": "key123" }));
    }

    #[test]
    fn execute_with_missing_secret_name_returns_invalid_input() {
        let tool = SecretsTool::new(|_| Ok("x".to_string()));
        let err = tool
            .execute(&json!({}))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn execute_with_non_string_secret_name_returns_invalid_input() {
        let tool = SecretsTool::new(|_| Ok("x".to_string()));
        let err = tool
            .execute(&json!({ "secret_name": 42 }))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)));
    }

    #[test]
    fn execute_with_closure_error_returns_execution_failed() {
        let tool = SecretsTool::new(|_| Err("vault locked".to_string()));
        let err = tool
            .execute(&json!({ "secret_name": "k" }))
            .unwrap_err();
        assert_eq!(err, ToolError::ExecutionFailed("vault locked".to_string()));
    }
}

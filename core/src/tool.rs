//! First-class tool abstraction: named, typed capabilities exposed to agents.

use std::collections::HashMap;

use serde_json::Value;

type ToolBox = Box<dyn Tool>;

/// Metadata describing a tool's identity and purpose.
pub struct ToolDescriptor {
    /// Unique name for dispatch (e.g. "secrets", "http_client").
    pub name: &'static str,
    /// Human-readable description of what the tool does.
    pub description: &'static str,
    /// JSON Schema describing the input this tool accepts.
    pub input_schema: Value,
}

/// Errors from tool execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolError {
    /// The input didn't match the tool's expected schema.
    InvalidInput(String),
    /// The tool's backing operation failed.
    ExecutionFailed(String),
    /// No tool with the given name exists in the registry.
    NotFound(String),
}

/// A named, auditable capability the runtime exposes to the agent.
pub trait Tool: Send + Sync {
    /// Returns metadata describing this tool.
    fn descriptor(&self) -> ToolDescriptor;

    /// Executes the tool with structured JSON input, returning
    /// structured JSON output.
    fn execute(&self, input: &Value) -> Result<Value, ToolError>;
}

/// The set of tools an agent is permitted to use.
/// Populated at construction; immutable thereafter.
pub struct ToolRegistry {
    tools: HashMap<String, ToolBox>,
}

impl ToolRegistry {
    /// Creates a registry from a fixed set of tools.
    pub fn new(tools: Vec<ToolBox>) -> Self {
        let mut map = HashMap::new();
        for tool in tools {
            let name = tool
                .descriptor()
                .name
                .to_string();
            map.insert(name, tool);
        }
        Self {
            tools: map,
        }
    }

    /// Returns descriptors for all registered tools (the "menu").
    pub fn list(&self) -> Vec<ToolDescriptor> {
        self.tools
            .values()
            .map(|t| t.descriptor())
            .collect()
    }

    /// Looks up a tool by name and executes it.
    /// Returns `ToolError::NotFound` if the name is not registered.
    pub fn execute(&self, name: &str, input: &Value) -> Result<Value, ToolError> {
        match self
            .tools
            .get(name)
        {
            Some(tool) => tool.execute(input),
            None => Err(ToolError::NotFound(name.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct StubTool {
        name: &'static str,
        response: Value,
    }

    impl Tool for StubTool {
        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: self.name,
                description: "stub tool for tests",
                input_schema: json!({}),
            }
        }

        fn execute(&self, _input: &Value) -> Result<Value, ToolError> {
            Ok(self
                .response
                .clone())
        }
    }

    #[test]
    fn empty_registry_list_returns_empty_vec() {
        let registry = ToolRegistry::new(vec![]);
        assert!(registry
            .list()
            .is_empty());
    }

    #[test]
    fn empty_registry_execute_returns_not_found() {
        let registry = ToolRegistry::new(vec![]);
        let err = registry
            .execute("anything", &json!({}))
            .expect_err("expected NotFound");
        assert_eq!(err, ToolError::NotFound("anything".to_string()));
    }

    #[test]
    fn registry_with_one_tool_list_returns_its_descriptor() {
        let tool = Box::new(StubTool {
            name: "alpha",
            response: json!("ok"),
        }) as Box<dyn Tool>;
        let registry = ToolRegistry::new(vec![tool]);
        let list = registry.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[0].description, "stub tool for tests");
    }

    #[test]
    fn registry_dispatches_to_correct_tool_by_name() {
        let t1 = Box::new(StubTool {
            name: "first",
            response: json!(1),
        }) as Box<dyn Tool>;
        let t2 = Box::new(StubTool {
            name: "second",
            response: json!(2),
        }) as Box<dyn Tool>;
        let registry = ToolRegistry::new(vec![t1, t2]);
        assert_eq!(
            registry
                .execute("first", &json!({}))
                .unwrap(),
            json!(1)
        );
        assert_eq!(
            registry
                .execute("second", &json!({}))
                .unwrap(),
            json!(2)
        );
    }

    #[test]
    fn registry_execute_unknown_name_returns_not_found() {
        let tool = Box::new(StubTool {
            name: "only",
            response: json!({}),
        }) as Box<dyn Tool>;
        let registry = ToolRegistry::new(vec![tool]);
        let err = registry
            .execute("other", &json!({}))
            .expect_err("expected NotFound");
        assert_eq!(err, ToolError::NotFound("other".to_string()));
    }

    #[test]
    fn descriptor_fields_are_non_empty() {
        let tool = StubTool {
            name: "named",
            response: json!({}),
        };
        let d = tool.descriptor();
        assert!(!d
            .name
            .is_empty());
        assert!(!d
            .description
            .is_empty());
    }
}

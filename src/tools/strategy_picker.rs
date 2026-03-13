use std::env;
use std::fs;

use async_trait::async_trait;
use claude_agent::tools::{ExecutionContext, Tool};
use claude_agent::types::ToolResult;
use rand::seq::IteratorRandom;

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

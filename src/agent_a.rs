use std::env;

use claude_agent::agent::AgentEvent;
use claude_agent::permissions::PermissionMode;
use claude_agent::tools::ToolAccess;
use claude_agent::{Agent, Auth};
use futures::StreamExt;

use crate::ask_peer_tool::AskPeerTool;

const PROMPT: &str = "Use the ask_peer tool to ask your peer Agent B for a knock knock joke. \
    When you receive the response, follow the standard knock knock joke format until the punchline, \
    then say thank you and we're done.";

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let peer = env::var("PEER").map_err(|_| "PEER environment variable is required")?;
    let ask_peer = AskPeerTool::new(peer.as_str());

    let agent: Agent = Agent::builder()
        .tools(ToolAccess::None)
        .tool(ask_peer)
        .permission_mode(PermissionMode::BypassPermissions)
        .auth(Auth::from_env())
        .await?
        .build()
        .await?;

    println!("[Agent A] Connecting to peer and asking for knock knock joke...");

    let stream = agent
        .execute_stream(PROMPT)
        .await?;
    let mut stream = std::pin::pin!(stream);

    while let Some(event) = stream
        .next()
        .await
    {
        match event? {
            AgentEvent::Text(text) => {
                print!("[Agent A] {}", text);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            },
            AgentEvent::Thinking(text) => {
                if !text.is_empty() {
                    println!("[Agent A] (thinking) {}", text);
                }
            },
            AgentEvent::ToolComplete {
                name,
                output,
                ..
            } => {
                println!("[Agent A] Tool {} completed: {}", name, output);
            },
            AgentEvent::ToolBlocked {
                name,
                reason,
                ..
            } => {
                println!("[Agent A] Tool {} blocked: {}", name, reason);
            },
            AgentEvent::ContextUpdate {
                ..
            } => {},
            AgentEvent::Complete(result) => {
                println!("[Agent A] Complete. Tokens: {}", result.total_tokens());
            },
        }
    }

    println!("[Agent A] Done.");
    Ok(())
}

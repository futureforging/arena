use std::env;

use claude_agent::agent::AgentEvent;
use claude_agent::permissions::PermissionMode;
use claude_agent::tools::ToolAccess;
use claude_agent::{Agent, Auth};
use futures::StreamExt;

use crate::ask_peer_tool::AskPeerTool;

const PROMPT: &str = "Use the ask_peer tool once to ask your peer Agent B for a knock knock joke. \
    Send exactly: 'Please tell me a knock knock joke.' \
    When you receive the full joke in the response, thank Agent B briefly and we're done.";

fn print_text_with_prefix(text: &str, need_prefix: &mut bool) {
    for line in text.split_inclusive('\n') {
        if *need_prefix {
            print!("[Agent A] ");
            *need_prefix = false;
        }
        print!("{}", line);
        if line.ends_with('\n') {
            *need_prefix = true;
        }
    }
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

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
    let mut need_prefix = true;

    while let Some(event) = stream
        .next()
        .await
    {
        match event? {
            AgentEvent::Text(text) => {
                print_text_with_prefix(&text, &mut need_prefix);
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
                println!("\n[Agent A] → Tool {}: {}", name, output);
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
                println!("\n[Agent A] Complete. Tokens: {}", result.total_tokens());
            },
        }
    }

    println!("\n[Agent A] Done.");
    Ok(())
}

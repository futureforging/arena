use std::env;
use std::fs;
use std::io::Write;
use std::sync::Arc;

use claude_agent::agent::AgentEvent;
use claude_agent::permissions::PermissionMode;
use claude_agent::tools::ToolAccess;
use claude_agent::{Agent, Auth};
use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::peer_protocol::PeerConnection;
use crate::peer_tools::{MessageLogFn, ReceiveFromPeerTool, SendToPeerTool};

const DEFAULT_PORT: u16 = 9001;

/// When set (any value), show thinking, agent text, tool names. Otherwise, only show messages exchanged.
const ENV_VERBOSE: &str = "PEER_AGENT_VERBOSE";

/// When set, write full debug log to this path.
const ENV_LOG: &str = "PEER_AGENT_LOG";

const DEFAULT_PROMPT_CONNECTOR: &str = "You are in a conversation with a peer. Use send_to_peer to send your first message. Use receive_from_peer to wait for their reply, then send_to_peer to respond. Continue until the conversation reaches a natural conclusion.";

const DEFAULT_PROMPT_LISTENER: &str = "You are in a conversation with a peer. Use receive_from_peer first to wait for their message. Use send_to_peer to respond. Continue until the conversation reaches a natural conclusion.";

fn load_prompt(role: &str) -> String {
    if let Ok(prompt) = env::var("AGENT_PROMPT") {
        return prompt;
    }
    if let Ok(path) = env::var("AGENT_PROMPT_FILE") {
        if let Ok(content) = fs::read_to_string(&path) {
            return content
                .trim()
                .to_string();
        }
    }
    if role == "connector" {
        DEFAULT_PROMPT_CONNECTOR.to_string()
    } else {
        DEFAULT_PROMPT_LISTENER.to_string()
    }
}

fn print_text_with_prefix(text: &str, need_prefix: &mut bool) {
    for line in text.split_inclusive('\n') {
        if *need_prefix {
            print!("[PeerAgent] ");
            *need_prefix = false;
        }
        print!("{}", line);
        if line.ends_with('\n') {
            *need_prefix = true;
        }
    }
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

async fn obtain_connection(
    verbose: bool,
) -> Result<(TcpStream, &'static str), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(peer) = env::var("PEER") {
        let stream = TcpStream::connect(peer.as_str()).await?;
        if verbose {
            println!("[PeerAgent] Connected to peer.");
        }
        return Ok((stream, "connector"));
    }
    if let Ok(port_str) = env::var("LISTEN_PORT") {
        let port: u16 = port_str
            .parse()
            .unwrap_or(DEFAULT_PORT);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        if verbose {
            println!("[PeerAgent] Listening on port {}...", port);
        }
        let (stream, _) = listener
            .accept()
            .await?;
        if verbose {
            println!("[PeerAgent] Peer connected.");
        }
        return Ok((stream, "listener"));
    }
    Err("Set PEER (to connect) or LISTEN_PORT (to listen)".into())
}

fn create_message_logger(verbose: bool, role_label: &str) -> Option<MessageLogFn> {
    if verbose {
        return None;
    }
    let role_label = role_label.to_string();
    let log_path = env::var(ENV_LOG).ok();
    Some(Arc::new(move |direction: &str, message: &str| {
        if direction != "sent" {
            return;
        }
        let line = format!("[{}] → {}", role_label, message);
        println!("{}", line);
        let _ = std::io::Write::flush(&mut std::io::stdout());
        if let Some(ref path) = log_path {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(f, "[{}] {}", direction, line);
                let _ = f.flush();
            }
        }
    }))
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let verbose = env::var(ENV_VERBOSE).is_ok();
    let (stream, role) = obtain_connection(verbose).await?;
    let connection = PeerConnection::new(stream);
    let connection = Arc::new(Mutex::new(connection));

    let role_label = env::var("AGENT_ROLE").unwrap_or_else(|_| role.to_string());
    let mut send_tool = SendToPeerTool::new(Arc::clone(&connection));
    let mut receive_tool = ReceiveFromPeerTool::new(Arc::clone(&connection));
    if let Some(log) = create_message_logger(verbose, &role_label) {
        send_tool = send_tool.with_message_log(Arc::clone(&log));
        receive_tool = receive_tool.with_message_log(log);
    }

    let prompt = load_prompt(role);

    let agent: Agent = Agent::builder()
        .tools(ToolAccess::None)
        .tool(send_tool)
        .tool(receive_tool)
        .permission_mode(PermissionMode::BypassPermissions)
        .auth(Auth::from_env())
        .await?
        .build()
        .await?;

    println!("[PeerAgent] role={}", role_label);

    let stream = agent
        .execute_stream(&prompt)
        .await?;
    let mut stream = std::pin::pin!(stream);
    let mut need_prefix = true;

    while let Some(event) = stream
        .next()
        .await
    {
        match event? {
            AgentEvent::Text(text) => {
                if verbose {
                    print_text_with_prefix(&text, &mut need_prefix);
                }
            },
            AgentEvent::Thinking(text) => {
                if verbose && !text.is_empty() {
                    println!("[PeerAgent] (thinking) {}", text);
                }
            },
            AgentEvent::ToolComplete {
                name,
                output,
                ..
            } => {
                if verbose {
                    println!("\n[PeerAgent] → Tool {}: {}", name, output);
                }
            },
            AgentEvent::ToolBlocked {
                name,
                reason,
                ..
            } => {
                eprintln!("[PeerAgent] Tool {} blocked: {}", name, reason);
            },
            AgentEvent::ContextUpdate {
                ..
            } => {},
            AgentEvent::Complete(result) => {
                if verbose {
                    println!("\n[PeerAgent] Complete. Tokens: {}", result.total_tokens());
                }
            },
        }
    }

    {
        let mut conn = connection
            .lock()
            .await;
        if let Err(e) = conn
            .write_sentinel()
            .await
        {
            let msg = e.to_string();
            if !msg.contains("Broken pipe") && !msg.contains("connection reset") {
                eprintln!("[PeerAgent] Failed to send end sentinel: {}", e);
            }
        }
    }

    if verbose {
        println!("\n[PeerAgent] Done.");
    }
    Ok(())
}

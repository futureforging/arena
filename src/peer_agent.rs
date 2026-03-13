use std::env;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use tokio::time::sleep;

use claude_agent::agent::AgentEvent;
use claude_agent::permissions::PermissionMode;
use claude_agent::tools::ToolAccess;
use claude_agent::{Agent, Auth};
use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::peer_protocol::PeerConnection;
use crate::peer_tools::{
    load_negotiation_protocol, pick_random_strategy,
    CryptoTool, MessageLogFn, ReceiveFromPeerTool, SendToPeerTool,
};

const DEFAULT_PORT: u16 = 9001;

/// When set (any value), show thinking, agent text, tool names. Otherwise, only show messages exchanged.
const ENV_VERBOSE: &str = "PEER_AGENT_VERBOSE";

/// When set, write full debug log to this path.
const ENV_LOG: &str = "PEER_AGENT_LOG";

const DEFAULT_PROMPT_CONNECTOR: &str = "Yao's Millionaire: determine who is richer without revealing exact wealth. Follow the protocol above. You are the connector. Use crypto tool for commitments.";

const DEFAULT_PROMPT_LISTENER: &str = "Yao's Millionaire: determine who is richer without revealing exact wealth. Follow the protocol above. You are the listener. Use crypto tool for commitments.";

const MAX_RETRIES: u32 = 3;

fn is_retryable_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("overloaded")
        || m.contains("internal server error")
        || m.contains("rate limit")
        || m.contains("429")
        || m.contains("503")
        || m.contains("502")
        || m.contains("stream error")
}

fn build_prompt(
    base_prompt: String,
    wealth: u32,
    protocol: &str,
    strategy: Option<&str>,
) -> String {
    let wealth_line = format!("Your wealth is ${} million.\n\n", wealth);
    let strategy_section = strategy
        .map(|s| format!("Strategy to propose (send this to your peer after handshake):\n{}\n\n", s))
        .unwrap_or_default();
    format!("{}{}\n---\n\n{}{}", wealth_line, protocol, strategy_section, base_prompt)
}

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
    let wealth: u32 = rand::thread_rng().gen_range(1..=100);
    println!("[{}] Your wealth: ${} million (not revealed to peer)", role_label, wealth);

    let protocol = match load_negotiation_protocol() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[{}] Error: {}", role_label, e);
            return Err(e.into());
        },
    };
    let strategy = if role == "connector" {
        match pick_random_strategy() {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[{}] Warning: {}", role_label, e);
                None
            },
        }
    } else {
        None
    };
    println!("[{}] Starting agent...", role_label);

    let mut send_tool = SendToPeerTool::new(Arc::clone(&connection));
    let mut receive_tool = ReceiveFromPeerTool::new(Arc::clone(&connection));
    if let Some(log) = create_message_logger(verbose, &role_label) {
        send_tool = send_tool.with_message_log(Arc::clone(&log));
        receive_tool = receive_tool.with_message_log(log);
    }

    let base_prompt = load_prompt(role);
    let prompt = build_prompt(base_prompt, wealth, &protocol, strategy.as_deref());

    let builder = Agent::builder()
        .tools(ToolAccess::None)
        .tool(CryptoTool)
        .tool(send_tool)
        .tool(receive_tool);

    let agent: Agent = builder
        .permission_mode(PermissionMode::BypassPermissions)
        .auth(Auth::from_env())
        .await?
        .build()
        .await?;

    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let stream = match agent.execute_stream(&prompt).await {
            Ok(s) => s,
            Err(e) => {
                if attempt < MAX_RETRIES && is_retryable_error(&e.to_string()) {
                    let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                    eprintln!(
                        "[{}] Retryable error (attempt {}): {}. Retrying in {:?}...",
                        role_label, attempt, e, delay
                    );
                    sleep(delay).await;
                    continue;
                }
                return Err(e.into());
            },
        };
        let mut stream = std::pin::pin!(stream);
        let mut need_prefix = true;
        let mut stream_err = None;

        while let Some(event) = stream.next().await {
            match event {
                Ok(ev) => {
                    match ev {
            AgentEvent::Text(text) => {
                if verbose {
                    print_text_with_prefix(&text, &mut need_prefix);
                }
            },
            AgentEvent::Thinking(text) => {
                if !text.is_empty() {
                    println!("[{}] thinking: ({})", role_label, text);
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
                },
                Err(e) => {
                    stream_err = Some(e);
                    break;
                },
            }
        }

        if let Some(e) = stream_err {
            if attempt < MAX_RETRIES && is_retryable_error(&e.to_string()) {
                let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                eprintln!(
                    "[{}] Retryable stream error (attempt {}): {}. Retrying in {:?}...",
                    role_label, attempt, e, delay
                );
                sleep(delay).await;
                continue;
            }
            return Err(e.into());
        }
        break;
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

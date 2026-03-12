mod agent_a;
mod agent_b;
mod ask_peer_tool;

use std::env;
use std::fs;
use std::io;

const SECRET_PATH: &str = "/run/secrets/anthropic_api_key";

/// Load API key from Docker secret if present, otherwise rely on ANTHROPIC_API_KEY env.
fn load_api_key() {
    if env::var("ANTHROPIC_API_KEY").is_ok() {
        return;
    }
    if let Ok(key) = fs::read_to_string(SECRET_PATH) {
        let key = key
            .trim()
            .to_string();
        if !key.is_empty() {
            env::set_var("ANTHROPIC_API_KEY", key);
        }
    }
}

fn report_agent_a_error(e: &dyn std::error::Error) {
    eprintln!("[Agent A] Error: {}", e);
    let mut source = e.source();
    while let Some(s) = source {
        eprintln!("  Caused by: {}", s);
        source = s.source();
    }
}

fn check_api_key() {
    let key = env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if key
        .trim()
        .is_empty()
    {
        eprintln!(
            "ANTHROPIC_API_KEY is not set. Create {} with your API key, or set the environment variable.",
            SECRET_PATH
        );
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    load_api_key();
    check_api_key();

    if env::var("LISTEN_PORT").is_ok() {
        agent_b::run().await
    } else if env::var("PEER").is_ok() {
        match agent_a::run().await {
            Ok(()) => Ok(()),
            Err(e) => {
                report_agent_a_error(&*e);
                std::process::exit(1);
            },
        }
    } else {
        eprintln!("Set LISTEN_PORT (for Agent B) or PEER (for Agent A)");
        std::process::exit(1);
    }
}

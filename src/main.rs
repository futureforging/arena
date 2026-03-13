mod peer_agent;
mod tools;

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
        let key = key.trim().to_string();
        if !key.is_empty() {
            env::set_var("ANTHROPIC_API_KEY", key);
        }
    }
}

fn report_error(e: &dyn std::error::Error) {
    eprintln!("[PeerAgent] Error: {}", e);
    let mut source = e.source();
    while let Some(s) = source {
        eprintln!("  Caused by: {}", s);
        source = s.source();
    }
}

fn check_api_key() {
    let key = env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if key.trim().is_empty() {
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

    match peer_agent::run().await {
        Ok(()) => Ok(()),
        Err(e) => {
            report_error(&*e);
            std::process::exit(1);
        },
    }
}

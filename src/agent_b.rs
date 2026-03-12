use std::env;
use std::io;

use claude_agent::query;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 9001;

pub async fn run() -> io::Result<()> {
    let port = env::var("LISTEN_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("[Agent B] Listening on port {}", port);

    let (stream, _) = listener.accept().await?;
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let mut request = String::new();
    buf_reader.read_line(&mut request).await?;
    let request = request.trim_end_matches('\n').trim_end_matches('\r');
    println!("[Agent B] Received request: {}", request);

    let prompt = format!(
        "Tell me a knock knock joke. The peer requested: {}",
        request
    );
    println!("[Agent B] Calling Claude for joke...");

    match query(&prompt).await {
        Ok(response) => {
            println!("[Agent B] Joke: {}", response);
            writer
                .write_all(format!("{}\n", response).as_bytes())
                .await?;
        }
        Err(e) => {
            let err_msg = format!("Error: {}", e);
            eprintln!("[Agent B] {}", err_msg);
            writer.write_all(format!("{}\n", err_msg).as_bytes()).await?;
        }
    }

    writer.shutdown().await?;
    println!("[Agent B] Response sent. Done.");
    Ok(())
}

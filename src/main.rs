use std::env;
use std::io::{self, BufRead};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    let stream = if let Ok(port) = env::var("LISTEN_PORT") {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let (s, _) = listener.accept().await?;
        s
    } else if let Ok(peer) = env::var("PEER") {
        loop {
            if let Ok(s) = TcpStream::connect(&peer).await {
                break s;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    } else {
        eprintln!("Set LISTEN_PORT or PEER");
        std::process::exit(1);
    };

    let (r, mut w) = tokio::io::split(stream);
    let mut reader = BufReader::new(r).lines();

    tokio::spawn(async move {
        while let Ok(Some(line)) = reader.next_line().await {
            println!("{}", line);
        }
    });

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(msg) = line {
            let _ = w.write_all(format!("{}\n", msg).as_bytes()).await;
        }
    }

    Ok(())
}

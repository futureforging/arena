use std::env;
use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

fn print_prompt() {
    let _ = (print!("> "), io::stdout().flush());
}

async fn connect_with_retry(peer: &str) -> TcpStream {
    loop {
        match TcpStream::connect(peer).await {
            Ok(s) => return s,
            Err(_) => tokio::time::sleep(tokio::time::Duration::from_millis(500)).await,
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let stream = if let Ok(port) = env::var("LISTEN_PORT") {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        let (s, _) = listener
            .accept()
            .await?;
        s
    } else if let Ok(peer) = env::var("PEER") {
        connect_with_retry(&peer).await
    } else {
        eprintln!("Set LISTEN_PORT or PEER");
        std::process::exit(1);
    };

    let (r, mut w) = tokio::io::split(stream);
    let mut reader = BufReader::new(r).lines();

    tokio::spawn(async move {
        while let Ok(Some(line)) = reader
            .next_line()
            .await
        {
            println!("{}", line);
            print_prompt();
        }
    });

    print_prompt();
    for msg in io::stdin().lock().lines().map_while(Result::ok) {
        let _ = w.write_all(format!("{}\n", msg).as_bytes()).await;
        print_prompt();
    }

    Ok(())
}

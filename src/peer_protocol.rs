use std::io;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub const SENTINEL_END: &str = "__CONVERSATION_END__";

/// Line-based protocol over TCP. One message per line (`\n`-delimited).
/// Use `write_message` and `read_message` for normal messages.
/// Use `write_sentinel` / `is_end_sentinel` for conversation termination.
pub async fn write_message(
    writer: &mut (impl AsyncWriteExt + Unpin),
    message: &str,
) -> io::Result<()> {
    writer.write_all(format!("{}\n", message).as_bytes()).await
}

pub async fn write_sentinel(
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> io::Result<()> {
    writer.write_all(format!("{}\n", SENTINEL_END).as_bytes()).await
}

pub async fn read_message(
    reader: &mut BufReader<impl tokio::io::AsyncRead + Unpin>,
) -> io::Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(line
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string())
}

pub fn is_end_sentinel(line: &str) -> bool {
    line.is_empty() || line.trim() == SENTINEL_END
}

/// Shared peer connection for use by both tools.
/// Holds the split TCP stream; access is serialized via the outer Mutex.
pub struct PeerConnection {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl PeerConnection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            reader: BufReader::new(reader),
            writer,
        }
    }

    pub async fn write_message(&mut self, message: &str) -> io::Result<()> {
        write_message(&mut self.writer, message).await
    }

    pub async fn write_sentinel(&mut self) -> io::Result<()> {
        write_sentinel(&mut self.writer).await
    }

    pub async fn read_message(&mut self) -> io::Result<String> {
        read_message(&mut self.reader).await
    }
}

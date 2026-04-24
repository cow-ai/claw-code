use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub mod server;

/// Read one MCP stdio message: `Content-Length: N\r\n\r\n<payload>`.
pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> crate::Result<Value> {
    let mut header_buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        reader.read_exact(&mut byte).await?;
        header_buf.push(byte[0]);
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let headers = String::from_utf8_lossy(&header_buf);
    let content_length: usize = headers
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split_once(':').map(|x| x.1))
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| crate::Error::Other("missing content-length".into()))?;

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(serde_json::from_slice(&body)?)
}

/// Write one MCP stdio message: `Content-Length: N\r\n\r\n<payload>`.
pub async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    msg: &Value,
) -> crate::Result<()> {
    let body = serde_json::to_vec(msg)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

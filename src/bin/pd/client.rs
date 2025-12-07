use anyhow::Result;
use parallel_downloader::config::Settings;
use parallel_downloader::ipc::{Command, Request, Response};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn send_command_raw(cmd: Command) -> Result<Response> {
    let settings = Settings::load().unwrap_or_default();
    let addr = settings
        .daemon_addr
        .unwrap_or_else(|| "127.0.0.1:9090".to_string());
    let secret = settings.daemon_secret;
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|_| anyhow::anyhow!("Could not connect to daemon. Is it running?"))?;

    let req = Request {
        secret,
        command: cmd,
    };

    let json_req = serde_json::to_string(&req)?;
    stream.write_all(json_req.as_bytes()).await?;

    let mut buf = [0; 1024];
    let n = stream.read(&mut buf).await?;
    let json_resp = String::from_utf8_lossy(&buf[..n]);

    let response: Response = serde_json::from_str(&json_resp)?;

    Ok(response)
}

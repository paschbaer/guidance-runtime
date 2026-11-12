use anyhow::Result;
use guidance_protocol::{Request, Response};
use guidance_runtime::Runtime;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod dispatch;
mod error;
mod params;

/// Maximum accepted line length of an inbound request.
const MAX_LINE: usize = 1_048_576;

async fn stream<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    s: S,
    rt: Runtime,
) -> Result<()> {
    let (r, mut w) = tokio::io::split(s);
    let mut l = BufReader::new(r).lines();
    while let Some(x) = l.next_line().await? {
        if x.len() > MAX_LINE {
            break;
        }
        let z = match serde_json::from_str(&x) {
            Ok(q) => dispatch::run(&rt, q).await,
            Err(e) => Response::err("unknown".into(), "invalid_request", e.to_string()),
        };
        w.write_all(serde_json::to_string(&z)?.as_bytes()).await?;
        w.write_all(b"\n").await?;
    }
    Ok(())
}
#[cfg(unix)]
pub async fn serve(rt: Runtime) -> Result<()> {
    let _ = tokio::fs::remove_file(guidance_protocol::SOCKET).await;
    let l = tokio::net::UnixListener::bind(guidance_protocol::SOCKET)?;
    loop {
        let (s, _) = l.accept().await?;
        let r = rt.clone();
        tokio::spawn(async move {
            let _ = stream(s, r).await;
        });
    }
}
#[cfg(windows)]
pub async fn serve(rt: Runtime) -> Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;
    loop {
        let s = ServerOptions::new()
            .first_pipe_instance(false)
            .create(guidance_protocol::PIPE)?;
        s.connect().await?;
        let r = rt.clone();
        tokio::spawn(async move {
            let _ = stream(s, r).await;
        });
    }
}
pub async fn request(q: Request) -> Result<Response> {
    #[cfg(unix)]
    let s = tokio::net::UnixStream::connect(guidance_protocol::SOCKET).await?;
    #[cfg(windows)]
    let s = tokio::net::windows::named_pipe::ClientOptions::new().open(guidance_protocol::PIPE)?;
    let (r, mut w) = tokio::io::split(s);
    w.write_all(serde_json::to_string(&q)?.as_bytes()).await?;
    w.write_all(b"\n").await?;
    let mut x = String::new();
    BufReader::new(r).read_line(&mut x).await?;
    Ok(serde_json::from_str(&x)?)
}

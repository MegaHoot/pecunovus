use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;

/// Start listening TCP on address and return the listener.
/// Consumer should `accept().await` and hand streams to Connection::spawn.
pub async fn bind(addr: &str) -> Result<TcpListener> {
    let l = TcpListener::bind(addr).await?;
    Ok(l)
}

pub async fn connect(addr: &str) -> Result<TcpStream> {
    let s = TcpStream::connect(addr).await?;
    Ok(s)
}

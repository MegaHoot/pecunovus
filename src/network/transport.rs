use tokio::net::{TcpListener, TcpStream};

pub async fn start_listener(addr: &str) -> tokio::io::Result<TcpListener> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}", addr);
    Ok(listener)
}

pub async fn connect(addr: &str) -> tokio::io::Result<TcpStream> {
    TcpStream::connect(addr).await
}

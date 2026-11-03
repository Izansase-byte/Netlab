//! WebSocket transport behind an HTTPS reverse proxy. Keep this backend on the
//! private container network; public clients must use wss:// at the proxy.
use crate::server::{Clients, broadcast};
use futures_util::{SinkExt, StreamExt};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};
use tokio_tungstenite::{
    accept_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};

pub async fn run() -> io::Result<()> {
    let address = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "0.0.0.0:8080".into());
    let listener = TcpListener::bind(&address).await?;
    println!(
        "WebSocket escuchando en {} (publicar mediante HTTPS/WSS)",
        listener.local_addr()?
    );
    let clients: Clients = Arc::new(Mutex::new(Vec::new()));
    crate::admin::start(Arc::clone(&clients));
    let mut next_id = 0;
    loop {
        let (tcp, _) = listener.accept().await?;
        let clients = Arc::clone(&clients);
        let id = next_id;
        next_id += 1;
        tokio::spawn(async move {
            if let Err(error) = connection(id, tcp, &clients).await {
                eprintln!("Cliente {id}: {error}");
            }
            disconnect(id, &clients);
        });
    }
}

fn disconnect(id: usize, clients: &Clients) {
    let name = if let Ok(mut connected) = clients.lock() {
        let name = connected
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone());
        connected.retain(|c| c.id != id);
        name
    } else {
        None
    };
    if let Some(name) = name {
        let _ = broadcast(
            clients,
            id,
            &format!("{name} se ha desconectado del chat.\n"),
        );
    }
}

// Inspect a bounded HTTP header without consuming a WebSocket handshake.
async fn health_request(tcp: &mut TcpStream) -> io::Result<bool> {
    let mut header = [0; 8192];
    let length = loop {
        let count = tcp.peek(&mut header).await?;
        if count == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "HTTP header"));
        }
        if let Some(end) = header[..count].windows(4).position(|s| s == b"\r\n\r\n") {
            break end + 4;
        }
        if count == header.len() {
            return Err(io::Error::other("HTTP header too large"));
        }
        // peek remains readable while a fragmented header is arriving.
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    let request = std::str::from_utf8(&header[..length]).map_err(io::Error::other)?;
    if request.lines().skip(1).any(|line| {
        line.split_once(':')
            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("upgrade"))
    }) {
        return Ok(false);
    }
    let mut first_line = request.lines().next().unwrap_or("").split_whitespace();
    let method = first_line.next().unwrap_or("");
    let path = first_line.next().unwrap_or("");
    let status = match (method, path) {
        ("GET" | "HEAD", "/" | "/health") => "200 OK",
        ("GET" | "HEAD", _) => "404 Not Found",
        _ => "405 Method Not Allowed",
    };
    let body = if status == "200 OK" {
        "Netlab OK\n"
    } else {
        ""
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{}",
        body.len(),
        if method == "HEAD" { "" } else { body },
    );
    tcp.read_exact(&mut header[..length]).await?;
    tcp.write_all(response.as_bytes()).await?;
    tcp.shutdown().await?;
    Ok(true)
}

async fn connection(id: usize, mut tcp: TcpStream, clients: &Clients) -> io::Result<()> {
    if timeout(Duration::from_secs(10), health_request(&mut tcp))
        .await
        .map_err(io::Error::other)??
    {
        return Ok(());
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(4096))
        .max_frame_size(Some(4096));
    let websocket = timeout(
        Duration::from_secs(10),
        accept_async_with_config(tcp, Some(config)),
    )
    .await
    .map_err(io::Error::other)?
    .map_err(io::Error::other)?;
    let (mut sink, mut source) = websocket.split();
    // Adapt the existing line-based chat to text frames, with bounded buffers.
    let (chat, bridge) = tokio::io::duplex(8192);
    let (bridge_reader, mut bridge_writer) = tokio::io::split(bridge);
    let incoming = async {
        while let Some(frame) = source.next().await {
            match frame.map_err(io::Error::other)? {
                Message::Text(text) => {
                    if text.contains(['\n', '\r']) {
                        return Err(io::Error::other("Un mensaje por trama"));
                    }
                    bridge_writer.write_all(text.as_bytes()).await?;
                    bridge_writer.write_all(b"\n").await?;
                }
                Message::Close(_) => break,
                Message::Binary(_) => return Err(io::Error::other("Solo se admite texto")),
                _ => {}
            }
        }
        bridge_writer.shutdown().await
    };
    let outgoing = async {
        let mut lines = BufReader::new(bridge_reader).lines();
        while let Some(line) = lines.next_line().await? {
            timeout(
                Duration::from_secs(10),
                sink.send(Message::Text(line.into())),
            )
            .await
            .map_err(io::Error::other)?
            .map_err(io::Error::other)?;
        }
        sink.close().await.map_err(io::Error::other)?;
        Ok::<(), io::Error>(())
    };
    tokio::pin!(outgoing);
    tokio::select! {
        result = crate::client::handle_client(id, chat, clients) => {
            // Removing the registry sender lets the writer drain before closing.
            disconnect(id, clients);
            let _ = timeout(Duration::from_secs(2), &mut outgoing).await;
            result
        },
        result = incoming => result,
        result = &mut outgoing => result,
    }
}

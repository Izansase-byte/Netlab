//! WebSocket transport behind an HTTPS reverse proxy. Keep this backend on the
//! private container network; public clients must use wss:// at the proxy.
use crate::server::{Clients, broadcast};
use futures_util::{SinkExt, StreamExt};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
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

async fn connection(id: usize, tcp: TcpStream, clients: &Clients) -> io::Result<()> {
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

use futures_util::{SinkExt, StreamExt};
use std::{
    net::TcpListener,
    process::{Command, Stdio},
};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
struct Server(std::process::Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
async fn until(socket: &mut Socket, expected: &str) {
    timeout(Duration::from_secs(5), async {
        while let Some(frame) = socket.next().await {
            let frame = frame.expect("valid websocket frame");
            if let Message::Text(text) = frame {
                if text.contains(expected) {
                    return;
                }
            }
        }
        panic!("closed before receiving {expected}");
    })
    .await
    .expect(expected);
}
async fn send(socket: &mut Socket, text: &str) {
    socket.send(Message::Text(text.into())).await.unwrap();
}

#[tokio::test]
async fn hosted_chat_registration_messages_commands_and_disconnect() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let _server = Server(
        Command::new(env!("CARGO_BIN_EXE_Netlab"))
            .args(["--websocket", &address.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let url = format!("ws://{address}");
    let mut alice = timeout(Duration::from_secs(5), async {
        loop {
            if let Ok((socket, _)) = connect_async(&url).await {
                break socket;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    until(&mut alice, "Introduce tu nombre").await;
    send(&mut alice, "Alice").await;
    until(&mut alice, "Bienvenido/a, Alice").await;
    let (mut bob, _) = connect_async(&url).await.unwrap();
    until(&mut bob, "Introduce tu nombre").await;
    send(&mut bob, "Bob").await;
    until(&mut bob, "Bienvenido/a, Bob").await;
    until(&mut alice, "Bob se ha unido").await;
    send(&mut alice, "mensaje de prueba").await;
    until(&mut bob, "Alice: mensaje de prueba").await;
    send(&mut bob, "/users").await;
    until(&mut bob, "Alice, Bob").await;
    send(&mut bob, "/help").await;
    until(&mut bob, "/users -").await;
    let (mut duplicate, _) = connect_async(&url).await.unwrap();
    until(&mut duplicate, "Introduce tu nombre").await;
    send(&mut duplicate, "alice").await;
    until(&mut duplicate, "Nombre ya en uso").await;
    send(&mut bob, "/quit").await;
    until(&mut alice, "Bob se ha desconectado").await;
    let (mut replacement, _) = connect_async(&url).await.unwrap();
    until(&mut replacement, "Introduce tu nombre").await;
    send(&mut replacement, "Bob").await;
    until(&mut replacement, "Bienvenido/a, Bob").await;
}

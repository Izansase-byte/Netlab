use std::io;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;

use crate::client::handle_client;

pub(crate) struct Client {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) sender: mpsc::UnboundedSender<String>,
}

pub(crate) type Clients = Arc<Mutex<Vec<Client>>>;

pub(crate) fn broadcast(clients: &Clients, sender_id: usize, message: &str) -> io::Result<()> {
    let mut connected = clients.lock().map_err(|_| {
        io::Error::new(
            io::ErrorKind::Other,
            "Lista de clientes bloqueada por un fallo",
        )
    })?;

    connected.retain(|client| {
        if client.id == sender_id {
            return true;
        }
        client.sender.send(message.to_string()).is_ok()
    });

    Ok(())
}

pub async fn run(tls_config: Arc<ServerConfig>) {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());
    let acceptor = TlsAcceptor::from(tls_config);
    let listener = match TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error al enlazar el puerto: {}", e);
            return;
        }
    };

    match listener.local_addr() {
        Ok(address) => println!("Servidor escuchando en {}", address),
        Err(e) => eprintln!("No se puede consultar la direccion: {}", e),
    }

    let clients: Clients = Arc::new(Mutex::new(Vec::new()));
    let local_ip = (|| -> std::io::Result<std::net::IpAddr> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect("1.1.1.1:80")?;
        Ok(socket.local_addr()?.ip())
    })();
    match local_ip {
        Ok(ip) => println!("IP local: {}", ip),
        Err(e) => eprintln!("Error al obtener la IP local: {}", e),
    }
    println!("Versión: {}", env!("CARGO_PKG_VERSION"));
    println!("TLS activo: las conexiones requieren un cliente compatible con TLS.");
    let mut next_id = 0;

    loop {
        let (stream, address) = match listener.accept().await {
            Ok(connection) => connection,
            Err(e) => {
                eprintln!("Error al aceptar la conexión: {}", e);
                continue;
            }
        };

        let id = next_id;
        next_id += 1;
        println!("Conexión desde {}", address);

        let clients = Arc::clone(&clients);
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let stream = match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Error TLS con {}: {}", address, e);
                    return;
                }
            };
            // La limpieza se ejecuta aunque falle el registro o la lectura.
            if let Err(e) = handle_client(id, stream, &clients).await {
                eprintln!("Error del cliente {}: {}", id, e);
            }
            let name = match clients.lock() {
                Ok(mut connected) => {
                    let name = connected
                        .iter()
                        .find(|client| client.id == id)
                        .map(|client| client.name.clone());
                    connected.retain(|client| client.id != id);
                    name
                }
                Err(e) => {
                    eprintln!("Error al retirar al cliente {}: {}", id, e);
                    None
                }
            };

            if let Some(name) = name {
                let notice = format!("{} se ha desconectado del chat.\n", name);
                if let Err(e) = broadcast(&clients, id, &notice) {
                    eprintln!("Error al notificar la desconexión de {}: {}", name, e);
                }
            }
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_reach_other_users_without_echo() {
        let (sender_a, mut receiver_a) = mpsc::unbounded_channel();
        let (sender_b, mut receiver_b) = mpsc::unbounded_channel();
        let clients = Arc::new(Mutex::new(vec![
            Client {
                id: 1,
                name: "Izan".into(),
                sender: sender_a,
            },
            Client {
                id: 2,
                name: "Ana".into(),
                sender: sender_b,
            },
        ]));

        broadcast(&clients, 1, "Izan: hola\n").unwrap();

        assert_eq!(receiver_b.try_recv().unwrap(), "Izan: hola\n");
        assert!(matches!(
            receiver_a.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn closed_recipient_does_not_stop_delivery_to_others() {
        let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
        drop(closed_receiver);
        let (active_sender, mut active_receiver) = mpsc::unbounded_channel();
        let clients = Arc::new(Mutex::new(vec![
            Client {
                id: 2,
                name: "Salido".into(),
                sender: closed_sender,
            },
            Client {
                id: 3,
                name: "Ana".into(),
                sender: active_sender,
            },
        ]));

        broadcast(&clients, 1, "hola\n").unwrap();

        assert_eq!(active_receiver.try_recv().unwrap(), "hola\n");
        assert_eq!(clients.lock().unwrap().len(), 1);
    }
}

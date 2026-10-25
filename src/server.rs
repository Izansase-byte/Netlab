use std::io::{self, Write};
use std::net::UdpSocket;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::net::TcpListener;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;


use crate::client::handle_client;

pub(crate) struct Client {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) stream: TcpStream,
}

pub(crate) type Clients = Arc<Mutex<Vec<Client>>>;

pub(crate) fn broadcast(clients: &Clients, sender_id: usize, message: &str) -> io::Result<()> {
    let mut connected = clients.lock().map_err(|_| {
        io::Error::new(
            io::ErrorKind::Other,
            "Lista de clientes bloqueada por un fallo",
        )
    })?;

    connected.retain_mut(|client| {
        if client.id == sender_id {
            return true;
        }
        match client.stream.write_all(message.as_bytes()) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("Error al enviar a {}: {}", client.name, e);
                let _ = client.stream.shutdown(Shutdown::Both);
                false
            }
        }
    });
    Ok(())
}

pub async fn run(tls_config: Arc<ServerConfig>) {
    let acceptor = TlsAcceptor::from(tls_config);
    let listener = match TcpListener::bind("0.0.0.0:8080").await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error al enlazar el puerto: {}", e);
            return;
        }
    };

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

        // Puente temporal: la atención al cliente todavía usa E/S bloqueante
        // en un hilo independiente. TLS sustituirá este paso.
        let stream = match stream.into_std() {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Error al convertir la conexión: {}", e);
                continue;
            }
        };

        if let Err(e) = stream.set_nonblocking(false) {
            eprintln!("Error al configurar la conexión: {}", e);
            continue;
        }

        let clients = Arc::clone(&clients);
        thread::spawn(move || {
            // La limpieza se ejecuta aunque falle el registro o la lectura.
            if let Err(e) = handle_client(id, &stream, &clients) {
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
            let _ = stream.shutdown(Shutdown::Both);

            if let Some(name) = name {
                let notice = format!("{} se ha desconectado del chat.\n", name);
                if let Err(e) = broadcast(&clients, id, &notice) {
                    eprintln!("Error al notificar la desconexión de {}: {}", name, e);
                }
            }
        });
    }
}
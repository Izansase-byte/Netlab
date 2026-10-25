use std::io::{self, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::client::handle_client;

pub(crate) struct Client {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) stream: TcpStream,
}

pub(crate) type Clients = Arc<Mutex<Vec<Client>>>;

pub(crate) fn broadcast(
    clients: &Clients,
    sender_id: usize,
    message: &str,
) -> io::Result<()> {
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

pub fn run() {
    let listener = match TcpListener::bind("127.0.0.1:8080") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error al enlazar el puerto: {}", e);
            return;
        }
    };

    let clients: Clients = Arc::new(Mutex::new(Vec::new()));
    println!("Servidor de chat iniciado en 127.0.0.1:8080");
    println!("Versión: {}", env!("CARGO_PKG_VERSION"));

    for (id, connection) in listener.incoming().enumerate() {
        let stream = match connection {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Error al aceptar la conexión: {}", e);
                continue;
            }
        };

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
            if let Some(name) = name {
                let notice = format!("{} se ha desconectado del chat.\n", name);
                if let Err(e) = broadcast(&clients, id, &notice) {
                    eprintln!("Error al notificar la desconexión de {}: {}", name, e);
                }
            }
        });
    }}

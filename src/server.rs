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
    println!("Versión: {}c", env!("CARGO_PKG_VERSION"));

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
            match clients.lock() {
                Ok(mut connected) => connected.retain(|client| client.id != id),
                Err(e) => eprintln!("Error al retirar al cliente {}: {}", id, e),
            }
            let _ = stream.shutdown(Shutdown::Both);
            println!("Cliente {} desconectado", id);
        });
    }
}

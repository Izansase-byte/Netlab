use std::io::{self, BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type Clients = Arc<Mutex<Vec<(usize, TcpStream)>>>;

fn main() {
    let listener = match TcpListener::bind("127.0.0.1:8080") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error al enlazar el puerto: {}", e);
            return;
        }
    };

    let clients: Clients = Arc::new(Mutex::new(Vec::new()));
    println!("Servidor de chat iniciado en 127.0.0.1:8080");
    println!("Versión: 0.5.0");

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
                Ok(mut connected) => connected.retain(|(other_id, _)| *other_id != id),
                Err(e) => eprintln!("Error al retirar al cliente {}: {}", id, e),
            }
            let _ = stream.shutdown(Shutdown::Both);
            println!("Cliente {} desconectado", id);
        });
    }
}

fn handle_client(id: usize, mut stream: &TcpStream, clients: &Clients) -> io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut reader = BufReader::new(stream);

    stream.write_all(b"Introduce tu nombre: \n")?;
    let mut name = String::new();
    if reader.read_line(&mut name)? == 0 {
        return Ok(());
    }

    let name = name.trim();
    let name = if name.is_empty() {
        format!("Usuario{}", id)
    } else {
        name.to_string()
    };

    stream.write_all(format!("Bienvenido, {}. Ya puedes escribir.\n", name).as_bytes())?;

    // Solo los clientes que han elegido un nombre reciben mensajes del chat.
    let writer = stream.try_clone()?;
    clients
        .lock()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Lista de clientes bloqueada por un fallo"))?
        .push((id, writer));
    println!("Cliente {} identificado como {}", id, name);

    let mut message = String::new();
    loop {
        message.clear();
        match reader.read_line(&mut message) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                let text = message.trim_end_matches(&['\r', '\n'][..]);
                if text.is_empty() {
                    continue;
                }
                let outgoing = format!("{}: {}\n", name, text);
                println!("{}", outgoing.trim_end());

                let mut connected = clients.lock().map_err(|_| {
                    io::Error::new(io::ErrorKind::Other, "Lista de clientes bloqueada por un fallo")
                })?;

                // Cada mensaje completo se envía a todos excepto al remitente.
                connected.retain_mut(|(other_id, writer)| {
                    if *other_id == id {
                        return true;
                    }
                    match writer.write_all(outgoing.as_bytes()) {
                        Ok(()) => true,
                        Err(e) => {
                            eprintln!("Error al enviar al cliente {}: {}", other_id, e);
                            let _ = writer.shutdown(Shutdown::Both);
                            false
                        }
                    }
                });
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

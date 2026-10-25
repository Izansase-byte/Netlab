use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let version = "0.4.0";
    let listener = match TcpListener::bind("127.0.0.1:8080") {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error al enlazar el puerto: {}", e);
            return;
        }
    };

    // Cada cliente tiene un ID y una copia de su conexión para enviarle datos.
    let clients: Arc<Mutex<Vec<(usize, TcpStream)>>> = Arc::new(Mutex::new(Vec::new()));

    println!("Servidor de chat iniciado en 127.0.0.1:8080");
    println!("Versión: {}", version);

    for (id, connection) in listener.incoming().enumerate() {
        let mut stream = match connection {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Error al aceptar la conexión: {}", e);
                continue;
            }
        };

        // Un cliente que no lee no debe bloquear los envíos indefinidamente.
        if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(2))) {
            eprintln!("Error al configurar la conexión: {}", e);
            continue;
        }

        let writer = match stream.try_clone() {
            Ok(writer) => writer,
            Err(e) => {
                eprintln!("Error al preparar el cliente: {}", e);
                continue;
            }
        };

        match clients.lock() {
            Ok(mut connected) => connected.push((id, writer)),
            Err(e) => {
                eprintln!("Error al acceder a la lista de clientes: {}", e);
                return;
            }
        }

        println!("Cliente {} conectado: {:?}", id, stream.peer_addr());
        let clients = Arc::clone(&clients);

        thread::spawn(move || {
            let mut reader = BufReader::new(&stream);
            let mut message = String::new();

            loop {
                message.clear();

                match reader.read_line(&mut message) {
                    Ok(0) => break,
                    Ok(_) => {
                        println!("Cliente {}: {}", id, message.trim_end());

                        let mut connected = match clients.lock() {
                            Ok(connected) => connected,
                            Err(e) => {
                                eprintln!("Error al acceder a la lista de clientes: {}", e);
                                break;
                            }
                        };

                        // Reenviamos los bytes a todos excepto al remitente.
                        // El mutex evita mezclar envíos simultáneos a un cliente.
                        connected.retain_mut(|(other_id, writer)| {
                            if *other_id == id {
                                return true;
                            }

                            match writer.write_all(&buffer[..bytes_read]) {
                                Ok(()) => true,
                                Err(e) => {
                                    eprintln!("Error al enviar al cliente {}: {}", other_id, e);
                                    let _ = writer.shutdown(Shutdown::Both);
                                    false
                                }
                            }
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        eprintln!("Error al leer del cliente {}: {}", id, e);
                        break;
                    }
                }
            }

            // Retiramos al cliente cuando se desconecta o falla la lectura.
            match clients.lock() {
                Ok(mut connected) => connected.retain(|(other_id, _)| *other_id != id),
                Err(e) => eprintln!("Error al retirar al cliente {}: {}", id, e),
            }
            let _ = reader.get_ref().shutdown(Shutdown::Both);
            println!("Cliente {} desconectado", id);
        });
    }
}

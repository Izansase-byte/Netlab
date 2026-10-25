use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpStream,Shutdown};
use std::time::Duration;

use crate::server::Clients;

pub(crate) fn handle_client(
    id: usize,
    mut stream: &TcpStream,
    clients: &Clients,
    io::result<()> {
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

    }
)
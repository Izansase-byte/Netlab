use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::commands::{self, Commands};
use crate::server::{broadcast, Client, Clients};

pub(crate) fn handle_client(
    id: usize,
    mut stream: &TcpStream,
    clients: &Clients,
) -> io::Result<()> {
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

    // Solo los clientes que han elegido un nombre reciben mensajes del chat.
    let writer = stream.try_clone()?;
    let mut connected = clients.lock().map_err(|_| {
        io::Error::new(
            io::ErrorKind::Other,
            "Lista de clientes bloqueada por un fallo",
        )
    })?;
    
    if connected
    .iter()
    .any(|client| client.name.eq_ignore_ascii_case(&name)) {
        drop(connected);
        stream.write_all(b"Nombre ya en uso. Desconectando...\n")?;
        return Ok(());
    }

    connected.push(Client {
        id,
        name: name.clone(),
        stream: writer,
    });
    drop(connected); // Liberar la lista antes de que broadcast vuelva a bloquearla.

    stream.write_all(format!("Bienvenido, {}. Ya puedes escribir.\n", name).as_bytes())?;

    println!("Cliente {} identificado como {}", id, name);
    let notice = format!("{} se ha unido al chat.\n", name);
    broadcast(clients, id, &notice)?;

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
                match commands::parse_command(text) {
                    Commands::Quit => {
                        stream.write_all(b"Desconectando...\n")?;
                        return Ok(());
                    }
                    Commands::Help => {
                        let help_message = "Comandos disponibles:\n/quit - Salir del chat\n/help - Mostrar esta ayuda\n/users - Listar usuarios conectados\n";
                        stream.write_all(help_message.as_bytes())?;
                        continue;
                    }
                    Commands::Users => {
                        let user_message = {
                            let connected = clients.lock().map_err(|_| {
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    "Lista de clientes bloqueada por un fallo",
                                )
                            })?;
                            let user_list: Vec<&str> =
                                connected.iter().map(|client| client.name.as_str()).collect();
                            format!("Usuarios conectados: {}\n", user_list.join(", "))
                        };
                        stream.write_all(user_message.as_bytes())?;
                        continue;
                    }
                    Commands::Message => {}
                }
                let outgoing = format!("{}: {}\n", name, text);
                println!("{}", outgoing.trim_end());
                broadcast(clients, id, &outgoing)?;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

use std::io::{self, BufRead};
use std::thread;

use crate::server::Clients;

pub(crate) fn start(clients: Clients) {
    thread::spawn(move || {
        println!(
            "Consola del servidor.\n\
             Comandos:\n\
             /users - Mostrar IDs y nombres\n\
             /say mensaje - Enviar un aviso a todos\n\
             /kick ID - Expulsar a un usuario"
        );

        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let command = match line {
                Ok(line) => line,
                Err(e) => {
                    eprintln!("Error al leer la consola: {}", e);
                    break;
                }
            };

            match command.trim() {
                "/users" => match clients.lock() {
                    Ok(connected) => {
                        println!("Usuarios conectados: {}", connected.len());
                        for client in connected.iter() {
                            println!("ID {} - {}", client.id, client.name);
                        }
                    }
                    Err(e) => eprintln!("Error al consultar usuarios: {}", e),
                },
                text if text == "/say" || text.starts_with("/say ") => {
                    let message = text.strip_prefix("/say").unwrap().trim();
                    if message.is_empty() {
                        println!("Uso: /say mensaje");
                        continue;
                    }
                    let notice = format!("[Servidor] {}\n", message);
                    match clients.lock() {
                        Ok(mut connected) => {
                            connected.retain(|client| {
                                client.sender.send(notice.clone()).is_ok()
                            });
                            println!("Aviso enviado a {} usuarios.", connected.len());
                        }
                        Err(e) => eprintln!("Error al enviar el aviso: {}", e),
                    }
                }
                text if text == "/kick" || text.starts_with("/kick ") => {
                    let id = match text.strip_prefix("/kick").unwrap().trim().parse::<usize>() {
                        Ok(id) => id,
                        Err(_) => {
                            println!("Uso: /kick ID");
                            continue;
                        }
                    };
                    match kick_client(&clients, id) {
                        Ok(status) => println!("ID {}: {}", id, status),
                        Err(e) => eprintln!("Error al expulsar al usuario: {}", e),
                    }
                }
                "" => {}
                _ => println!("Comando desconocido. Usa /users, /say mensaje o /kick ID."),
            }
        }
    });
}

// Solo la consola local llama a esta función.
fn kick_client(clients: &Clients, id: usize) -> io::Result<&'static str> {
    let signal = {
        let mut connected = clients.lock().map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "No se pudo acceder a los clientes")
        })?;
        match connected.iter_mut().find(|client| client.id == id) {
            Some(client) => client.kick.take(),
            None => return Ok("No existe un usuario con ese ID."),
        }
    };

    // La lista ya está desbloqueada cuando enviamos la señal.
    match signal {
        Some(signal) => match signal.send(()) {
            Ok(()) => Ok("Expulsión solicitada."),
            Err(_) => Ok("El usuario ya se ha desconectado."),
        },
        None => Ok("La expulsión ya estaba solicitada."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::Client;
    use std::sync::{Arc, Mutex};
    use tokio::sync::{mpsc, oneshot};

    #[test]
    fn kick_only_signals_the_requested_user() {
        let (messages_a, _receiver_a) = mpsc::unbounded_channel();
        let (messages_b, _receiver_b) = mpsc::unbounded_channel();
        let (kick_a, mut signal_a) = oneshot::channel();
        let (kick_b, mut signal_b) = oneshot::channel();
        let clients = Arc::new(Mutex::new(vec![
            Client {
                id: 1,
                name: "Izan".into(),
                sender: messages_a,
                kick: Some(kick_a),
            },
            Client {
                id: 2,
                name: "Ana".into(),
                sender: messages_b,
                kick: Some(kick_b),
            },
        ]));

        kick_client(&clients, 2).unwrap();

        assert!(signal_b.try_recv().is_ok());
        assert!(matches!(
            signal_a.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        assert_eq!(clients.lock().unwrap().len(), 2);
        assert!(clients.lock().unwrap()[1].kick.is_none());
    }
}

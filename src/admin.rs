use std::io::{self, BufRead};
use std::thread;

use crate::server::Clients;

pub(crate) fn start(clients: Clients) {
    thread::spawn(move || {
        println!(
            "Consola del servidor.\n\
             Comandos:\n\
             /users - Mostrar IDs y nombres\n\
             /say mensaje - Enviar un aviso a todos"
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
                    Err(e) => {
                        eprintln!("Error al consultar usuarios: {}", e);
                    }
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
                        Err(e) => {
                            eprintln!("Error al enviar el aviso: {}", e);
                        }
                    }
                }

                text if text == "/kick" || text.starts_with("/Kick") = {
                    let id = match text
                        .strip_prefix("/kick")
                        .unwrap()
                        .trim()
                        .parse::<usize>()
                    {
                        Ok(id) => id,
                        Err(_) => {
                            println!("Uso: /kick ID")
                            continue;
                        }
                    };
                    
                    let signal = {
                        let mut connected = match clients.lock() {
                            Ok(connected) => connected,
                            Err(e) => {
                                eprintln!("Error al consultar usuarios: {}", e);
                                continue;
                            }
                        };
                    }
                        match connected.iter_mut().find()|client| client.id == id) {
                            Some(client) => client.kick.take(),
                            None => {
                                println!("No existe un usuario con el ID {}", id);
                                continue;
                            }
                    }
                };

                match signal {
                    Some(signal) => {
                        if signal.send(()).is_ok() {
                            println!("Expulsion solicitada para el ID {}", id );
                        } else {
                            println!("La expulsion de ese usuario ya estaba solicitada.")
                    }
                    }}


                None => {
                    println!("No existe un usuario con ID{}", id)
                    continue;

                "" => {}
                _ => println!("Comando desconocido. Usa /users o /say mensaje."),
            }
        }
    });
}

use std::io::{self, BufRead};
use std::thread

use crate::server::Clients;

pub(crate) fn start(clients: Clients) {
    thread::spawn(move || {
        println!("Consola del servidor.\n\
        Lista de comandos:\n\
        /users")

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
                "/users" => {
                    match clients.lock() {
                        Ok(connected) => {
                            println!("Usuarios conectados: {}", connected.len())

                            for client in connected.ited() {
                                println!("Id {} - {}" client.id, client.name);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error al consultar usuarios: {}", e);
                        }
                    }
                }
                text if text == /say || text.starts_with("/say") => {
                    let message = text.strip_prefix("/say").unwrap().trim();

                    if message.is_empty() {
                        println!("Uso: /say mensaje");
                        continue;
                    }
                    let notice = format!("[Servidor] {}\n", message)

                    match clients.lock() {
                        Ok(mut connected) => {
                            connected.retain(|client| {
                                client.sender.send(notice.clone()).is_ok()
                            });
                        }
                        Err(e) => {
                            eprintln!("Error al enviar el aviso: {}", e)
                        }
                    }
                }



                "" => {}
                _ => println!("Comando desconocido.")
            }
        }
    });
}
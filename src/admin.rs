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
                "" => {}
                _ => println!("Comando desconocido.")
            }
        }
    });
}
use std::io::{Read,Write};
use std::net::TcpListener;

fn main() {
    let version = "0.3";
    let listener = TcpListener::bind("127.0.0.1:8080");

    match listener {
        Ok(listener) => {
            println!("Servidor iniciado en puerto 8080");
            println!("Versión: {}", version);

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        match stream.peer_addr() {
                            Ok(addr) => println!("Nueva conexión establecida desde: {}", addr),
                            Err(e) => println!("No se pudo obtener la dirección del cliente: {}", e),
                        }

                        let mut buffer = [0u8; 1024];
                    loop {
                        match stream.read(&mut buffer) {
                            
                                Ok(0) => {
                                    println!("El cliente ha cerrado la conexión");
                                    break;
                                }
                                Ok(bytes_read) => {
                                    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                                    println!("Datos recibidos: {} bytes", bytes_read);
                                    println!("Solicitud recibida:\n{}", request);
                                    match stream.write_all(&buffer[..bytes_read]) {
                                        Ok(()) => {println!("Respuesta enviada al cliente")}
                                        Err(e) => {println!("Error al enviar la respuesta: {}", e);
                                    break;}

                                    }
                              }
                                Err(e) => {
                                println!("Error al leer la solicitud: {}", e);
                                break;
                                
                            }}
                        }
                    }
                    Err(e) => {
                        println!("Error al aceptar la conexión: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Error al enlazar el puerto: {}", e);
        }
    }
}

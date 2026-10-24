use std::net::{TcpListener};
use std::io::Read;
fn main() {
    let mut buffer = [0u8; 1024];
    let version = "0.2.1";
    let listener = TcpListener::bind("127.0.0.1:8080");
    match listener {
        Ok(listener) => {
           for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        println!("Nueva conexión establecida desde: {}", stream.peer_addr().unwrap());
                        match stream.read(&mut buffer) {
                            Ok(bytes_read) => {
                                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                                println!("Solicitud recibida:\n{}", request);
                            }
                            Err(e) => {
                                println!("Error al leer la solicitud: {}", e);
                            }}}}
                    Err(e) => {
                        println!("Error al aceptar la conexión: {}", e);
                    }}}}
        Err(e) => {
            println!("Error al enlazar el puerto: {}", e);
        
    
}} 

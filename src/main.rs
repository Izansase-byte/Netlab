use std::net::{TcpListener};
use std::io::Read;
fn main() {
    let mut buffer = [0u8; 1024];
    let version = "0.2.1";
    let listener = TcpListener::bind("127.0.0.1:8080");
    match listener {
        Ok(listener) => {
            println!("Servidor iniciado en puerto 8080");
            println!("Versión: {}", version);
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let peer_addr = stream.peer_addr();
                        println!("Conexión establecida con {:?}", peer_addr);
                        let mut stream = stream;
                       println!(stream.read(&mut buffer))
                    }
                    Err(e) => { println!("Error al aceptar la conexión: {}", e); }}}}


        
        Err(e) => {
            println!("Error al iniciar el servidor: {}", e);
        }}}
        

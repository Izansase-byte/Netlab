use std::net::{TcpListener};
fn main() {
    let version = "0.2.0";
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
                    }
                    Err(e) => { println!("Error al aceptar la conexión: {}", e); }}}}


        
        Err(e) => {
            println!("Error al iniciar el servidor: {}", e);
        }}}
        

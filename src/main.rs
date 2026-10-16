use std::net::TcpListener;
fn main() {
    let version = "0.1.0";
    let listener = TcpListener::bind("127.0.0.1:8080");
    match listener {
        Ok(listener) => {
            println!("Servidor iniciado en puerto 8080");
            println!("Versión: {}", version);
        }
        Err(e) => {
            println!("Error al iniciar el servidor: {}", e);
        }
    }
}

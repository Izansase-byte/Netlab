mod client;
mod commands;
mod server;
mod tls;


fn main() {
    match tls::server_config() {
        Ok(_) => println!("Certificado y clave TLS cargados correctamente")
        Err(e) => {
            eprintln!("Error al preparar TLS: {}", e)
            return;
        }
    }
    server::run();
}

mod client;
mod commands;
mod server;
mod tls;

fn main() {
    match tls::server_config() {
        Ok(_) => println!(
            "Certificado y clave TLS cargados correctamente. El chat todavía usa TCP sin cifrar."
        ),
        Err(e) => {
            eprintln!("Error al preparar TLS: {}", e);
            eprintln!("Ejecuta desde la raíz de Netlab y comprueba los archivos de certs/.");
            std::process::exit(1);
        }
    }

    server::run();
}

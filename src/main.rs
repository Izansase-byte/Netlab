mod admin;
mod client;
mod commands;
mod server;
mod tls;

#[tokio::main]
async fn main() {
    let tls_config = match tls::server_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error al preparar TLS: {}", e);
            std::process::exit(1)
        }
    };

    server::run(tls_config).await;
}

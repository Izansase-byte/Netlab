mod admin;
mod cert_paths;
mod client;
mod commands;
mod server;
mod tls;
mod websocket;

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("--websocket") {
        if let Err(error) = websocket::run().await {
            eprintln!("Error WebSocket: {error}");
            std::process::exit(1);
        }
        return;
    }
    let tls_config = match tls::server_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error al preparar TLS: {}", e);
            std::process::exit(1)
        }
    };

    server::run(tls_config).await;
}

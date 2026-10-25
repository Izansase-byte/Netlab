use std::fs::File;
use std::io::{self, BufRead};
use std::sync::Arc;
use std::thread;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let mut cert_file = std::io::BufReader::new(File::open("certs/server-cert.pem")?);
    let certificates = rustls_pemfile::certs(&mut cert_file)
        .collect::<io::Result<Vec<_>>>()?;

    if certificates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "El archivo no contiene certificados",
        ));
    }

    let mut roots = RootCertStore::empty();
    for certificate in certificates {
        roots
            .add(certificate)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    }

    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let tcp = timeout(Duration::from_secs(10), TcpStream::connect(&address))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Tiempo de conexión agotado"))??;

    // La IP local puede cambiar; la identidad del certificado es netlab.local.
    let server_name = ServerName::try_from("netlab.local")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let stream = timeout(
        Duration::from_secs(10),
        connector.connect(server_name, tcp),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Tiempo de negociación TLS agotado"))??;

    println!("Conexión TLS verificada con {}", address);

    let (reader, mut writer) = tokio::io::split(stream);
    let mut server_messages = BufReader::new(reader).lines();
    let (keyboard_sender, mut keyboard_receiver) = mpsc::channel::<io::Result<String>>(32);

    // Esperar al teclado en un hilo evita bloquear la recepción de mensajes.
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if keyboard_sender.blocking_send(line).is_err() {
                break;
            }
        }
    });

    let mut keyboard_closed = false;
    loop {
        tokio::select! {
            incoming = server_messages.next_line() => {
                match incoming? {
                    Some(message) => println!("{}", message),
                    None => {
                        println!("Conexión cerrada por el servidor.");
                        break;
                    }
                }
            }
            outgoing = keyboard_receiver.recv(), if !keyboard_closed => {
                match outgoing {
                    Some(line) => {
                        let message = line?;
                        timeout(Duration::from_secs(10), async {
                            writer.write_all(message.as_bytes()).await?;
                            writer.write_all(b"\n").await?;
                            writer.flush().await
                        })
                        .await
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::TimedOut, "Tiempo de envío agotado")
                        })??;
                    }
                    None => {
                        keyboard_closed = true;
                        timeout(Duration::from_secs(10), writer.shutdown())
                            .await
                            .map_err(|_| {
                                io::Error::new(io::ErrorKind::TimedOut, "Tiempo de cierre agotado")
                            })??;
                    }
                }
            }
        }
    }

    Ok(())
}

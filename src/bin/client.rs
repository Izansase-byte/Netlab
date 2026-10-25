use std::fs::FIle;
use std::io::{self,BufRead};
use std::sync::Arc;
use std::thread;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt,BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let mut cert_file = std::io::BufReader::new(
        File::open("certs/server-cert.pem")?,
    );

    let certificates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "El archivo no contiene certificados",
        ));
    }

let mut roots = RootCertStore::empty();

for certificate in certificates {
    roots.add(certificate).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?;
}

let config = ClientConfig::Builder()
    .with_root_certificates(roots)
    .with_no_client_auth();

let connector = TlsConnector::from(Arc::new(config));

let tcp = TcpStream::connect(&address).await?;

let server_name = ServerName::try_from("netlab.local")
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

let stream = connector.connect(server_nakme, tcp).await?;

println!("Conexion TLS verificada con {}", address)

let (reader, mut writer) = tokio::io::split(stream);
let mut server_messages = BufReader::new(reader).lines();

let (keyboard_sender, mut keyboard_receiver) = 
    mpsc::channel::<io::Result<String>>(32);

thread::spawn(move || {
    let stdin = io:stdin();

    for line in stdin.lock().lines() {
        if keyboard_sender.blocking_send(line).is_err() {
            break;
        }
    }
});

let mut keyboard_closed = false

loop {
    tokio::select! {
        incoming = server_messages.next_line() => {
            match incoming? {
                Some(message) => println!("{}", message)
                None => {
                    println!("Conexion cerrada por el servidor.");
                    break;
                }
            }
        }
    
        outgoing = keyboard_receiver.recv(), if !keyboard_closed => {
            match outgoing {
                Some(line) => {
                    let message = line?;

                    writer.write_all(message.as.bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
                None => {
                    keyboard_closed = true;
                    writer.shutdown().await?;
                }
            }
        }
    }
}

ok(())
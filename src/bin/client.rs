use std::io::{self, BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::thread;

fn main() -> std::io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let mut stream = TcpStream::connect(&address)?;
    println!("Conectado a {}", address);

    let receiver = stream.try_clone()?;
    thread::spawn(move || {
        let reader = BufReader::new(receiver);

        for line in reader.lines() {
            match line {
                Ok(message) => println!("{}", message),
                Err(e) => {
                    eprintln!("Error al recibir mensaje: {}", e);
                    break;
                }
            }
        }
        println!("Conexion cerrada por el servidor.");
        std::process::exit(0);
    });

    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let message = line?;

        if let Err(e) = writeln!(stream, "{}", message) {
            eprintln!("Error al enviar mensaje: {}", e);
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

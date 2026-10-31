use futures_util::{SinkExt, StreamExt};
use std::{
    io::{self, BufRead},
    thread,
};
use tokio::{
    sync::mpsc,
    time::{Duration, timeout},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn run(address: &str) -> io::Result<()> {
    let (socket, _) = timeout(Duration::from_secs(15), connect_async(address))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Tiempo de conexión agotado"))?
        .map_err(io::Error::other)?;
    println!("Conectado a {address}");
    let (mut writer, mut reader) = socket.split();
    let (sender, mut keyboard) = mpsc::channel(32);
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            if sender.blocking_send(line).is_err() {
                break;
            }
        }
    });
    loop {
        tokio::select! {
            incoming = reader.next() => match incoming {
                Some(Ok(Message::Text(text))) => println!("{text}"),
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(error)) => return Err(io::Error::other(error)),
                _ => {},
            },
            outgoing = keyboard.recv() => match outgoing {
                Some(line) => {
                    let text = line?;
                    if text.len() > 4096 {
                        eprintln!("El mensaje no puede superar 4096 bytes.");
                        continue;
                    }
                    timeout(Duration::from_secs(10), writer.send(Message::Text(text.into())))
                        .await.map_err(io::Error::other)?.map_err(io::Error::other)?;
                }
                None => { let _ = writer.close().await; break; }
            }
        }
    }
    println!("Conexión cerrada.");
    Ok(())
}

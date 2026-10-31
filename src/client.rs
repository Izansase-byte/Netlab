use chrono::Local;
use std::io;

use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

use crate::commands::{self, Commands};
use crate::server::{Client, Clients, broadcast};

pub(crate) async fn handle_client<S>(id: usize, stream: S, clients: &Clients) -> io::Result<()>
where
    S: tokio::io::AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    writer.write_all(b"Introduce tu nombre: \n").await?;
    writer.flush().await?;

    let mut name = String::new();
    if reader.read_line(&mut name).await? == 0 {
        return Ok(());
    }

    let name = name.trim();
    let name = if name.is_empty() {
        format!("Usuario{}", id)
    } else {
        name.to_string()
    };

    let (sender, receiver) = mpsc::unbounded_channel::<String>();
    let (kick_sender, mut kick_receiver) = oneshot::channel::<()>();

    // El bloqueo termina antes de cualquier operación asíncrona.
    let registered = {
        let mut connected = clients.lock().map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                "Lista de clientes bloqueada por un fallo",
            )
        })?;

        if connected
            .iter()
            .any(|client| client.name.eq_ignore_ascii_case(&name))
        {
            false
        } else {
            connected.push(Client {
                id,
                name: name.clone(),
                sender: sender.clone(),
                kick: Some(kick_sender),
            });
            true
        }
    };

    if !registered {
        writer
            .write_all(b"Nombre ya en uso. Desconectando...\n")
            .await?;
        writer.flush().await?;
        writer.shutdown().await?;
        return Ok(());
    }

    writer
        .write_all(format!("Bienvenido/a, {}. Ya puedes escribir.\n", name).as_bytes())
        .await?;
    writer.flush().await?;

    // Esta tarea es la única que escribe en la conexión durante el chat.
    let mut write_task = tokio::spawn(send_messages(writer, receiver));
    println!("Cliente {} identificado como {}", id, name);
    broadcast(clients, id, &format!("{} se ha unido al chat.\n", name))?;

    let send = |text: String| {
        sender
            .send(text)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "La cola de envío está cerrada"))
    };

    let receive = async {
        let mut message = String::new();
        loop {
            message.clear();
            if reader.read_line(&mut message).await? == 0 {
                return Ok::<(), io::Error>(());
            }

            let text = message.trim_end_matches(&['\r', '\n'][..]);
            if text.is_empty() {
                continue;
            }

            match commands::parse_command(text) {
                Commands::Quit => {
                    send("Desconectando...\n".to_string())?;
                    return Ok(());
                }
                Commands::Help => {
                    send(
                        "Comandos disponibles:\n\
                         /quit - Salir del chat\n\
                         /help - Mostrar esta lista\n\
                         /users - Listar los usuarios conectados\n"
                            .to_string(),
                    )?;
                }
                Commands::Users => {
                    let response = {
                        let connected = clients.lock().map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                "No se pudo acceder a los clientes",
                            )
                        })?;
                        let names: Vec<&str> = connected
                            .iter()
                            .map(|client| client.name.as_str())
                            .collect();
                        format!("Usuarios conectados: {}\n", names.join(", "))
                    };
                    send(response)?;
                }
                Commands::Message => {
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    let outgoing = format!("[{}] {}: {}\n", timestamp, name, text);
                    println!("{}", outgoing.trim_end());
                    broadcast(clients, id, &outgoing)?;
                }
            }
        }
    };

    tokio::select! {
        result = receive => result,
        result = &mut write_task => {
            match result {
                Ok(result) => result,
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            }
        }
        signal = &mut kick_receiver => {
            if signal.is_ok() {
                send("Has sido expulsado por el servidor.\n".to_string())?;
            }
            Ok(())
        }
    }
}

async fn send_messages<W>(
    mut writer: W,
    mut messages: mpsc::UnboundedReceiver<String>,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    while let Some(message) = messages.recv().await {
        timeout(Duration::from_secs(10), async {
            writer.write_all(message.as_bytes()).await?;
            writer.flush().await
        })
        .await
        .map_err(|_| {
            io::Error::new(io::ErrorKind::TimedOut, "El cliente no recibe los mensajes")
        })??;
    }

    timeout(Duration::from_secs(10), writer.shutdown())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Tiempo de cierre TLS agotado"))??;
    Ok(())
}

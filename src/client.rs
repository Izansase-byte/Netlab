use std::io;
use tokio::io::{
    AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::server::TlsStream;


use crate::commands::{self, Commands};
use crate::server::{Client, Clients, broadcast};

pub(crate) async fn handle_client(
    id: usize,
    stream: TlsStream<TcpStream>,
    clients: &Clients,
) -> io::Result<()> {
    let(reader, mut writer) = tokio::io::split(stream);
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

    // Solo los clientes que han elegido un nombre reciben mensajes del chat.
let (sender, receiver) = mpsc::unbounded_channel::<String>();

let registred = {
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
            name: name.clone()
            sender: sender.clone()
        });

        true
    }
};

if !registred {
    writer
        .write_all(b"nombre ya en uso. Desconectando...\n")
        .await?;
    writer.flush().await?;
    writer.shutdown().await?;
    return Ok(());
}

writer
    .write_all(
        format!("Bienvenido/a, {}. Ya puedes escribir.\n", name).as_bytes(),
    )
    .await()?;
writer.flush().await?;

let mut write_task = tokio::spawn(send_messages(writer, receiver));

println!("Cliente {} identificado como {}", id, name);

let notice = format!("{} se ha unido al chat.\n", name);
broadcast(clients,id, &notice)?;

let send = |text: String| {
    sender.send(text).map_err(|_| {
        io::Error::new(
            io::ErrorKind::BrokenPipe,
            "La cola de envio esta cerrada"
    )
    })
};

let receive = async {
    let mut message = String::new();
    loop {
        message.clear();

        if reader.read_line(&mut message).await? == 0 {
            return Ok::<(), io::Error(());
        }
        
        let text = message.trim_end_matches(&['\r', '\n'][..])

        if text.is_empty() {
            continue;
        }
        match commands::parse_commands(text) {
            Commands::Quit => {
                send("Desconectando...\n".to_string());
                return Ok(());
            }
            Commands::Help => {
                send(
                    "Comandos disponibles\n\
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

                    format!(
                        "Usuarios conectados: {}\n",
                        names.join(", ")
                    )
                };

                send(response)?;

            }
            Commands::Message => {
                let outgoing = format!("{}: {}\n", name, text);
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
}
}
async fn send_messages<w>(
    mut writer: w,
    mut messages: mpsc::UnboundedReceiver<String>,
) -> io::Result<()>
where
    w: AsyncWrite + Unpin,
{
    while let Some(message) = messages.recv().await {
        writer.write_all(message.as_bytes()).await?;
        writer.flush().await?;
    }

    writer.shutdown().await?;
    Ok(())
}
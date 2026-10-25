use:std::io;
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

    stream.write_all(b"Introduce tu nombre: \n").await?;
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
let (sender, receiver) = mpsc::unbounded_channel::<String>>();

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
    returon Ok(());
}

writer
    .write_all(
        format!("Bienvenido/a, {}. Ya puedes escribir.\n", name).as_bytes(),
    )
    .await()
writer.flush().await?;

let mut write_task = tokio::spawn(send_messages(writer, receiver));

println!("Cliente {} identificado como {}", id, name);

let notice = format!("{} se ha unido al chat.\n", name)
broadcast(clients,id, &notice)?;

    let mut message = String::new();
    loop {
        message.clear();
        match reader.read_line(&mut message) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                let text = message.trim_end_matches(&['\r', '\n'][..]);
                if text.is_empty() {
                    continue;
                }
                match commands::parse_command(text) {
                    Commands::Quit => {
                        stream.write_all(b"Desconectando...\n")?;
                        return Ok(());
                    }
                    Commands::Help => {
                        let help_message = "Comandos disponibles:\n/quit - Salir del chat\n/help - Mostrar esta ayuda\n/users - Listar usuarios conectados\n";
                        stream.write_all(help_message.as_bytes())?;
                        continue;
                    }
                    Commands::Users => {
                        let user_message = {
                            let connected = clients.lock().map_err(|_| {
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    "Lista de clientes bloqueada por un fallo",
                                )
                            })?;
                            let user_list: Vec<&str> = connected
                                .iter()
                                .map(|client| client.name.as_str())
                                .collect();
                            format!("Usuarios conectados: {}\n", user_list.join(", "))
                        };
                        stream.write_all(user_message.as_bytes())?;
                        continue;
                    }
                    Commands::Message => {}
                }
                let outgoing = format!("{}: {}\n", name, text);
                println!("{}", outgoing.trim_end());
                broadcast(clients, id, &outgoing)?;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
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
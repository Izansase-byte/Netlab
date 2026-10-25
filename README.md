# Netlab

A terminal chat app built in Rust to learn about networks,async and encryption.

## Features

- Multiple simultaneous users.
- Usernames with duplicate name detection.
- Messages broadcast to other connected users.
- Join and leave notifications
- A dedicated terminal client
- TLS connections with server certificate verification.
- Colorful time-stamped messages ;D
- Custom Ip and ports from the server part
- 

## Requirements

- An up-to-date Rust toolchain and Cargo
- OpenSSL to generate a test certificate

## Installation

```bash
git clone https://github.com/Izansase-Byte/Netlab.git
cd Netlab
cargo build --bins
```

## Test Certificate

Run tese commands from the project root folder:

```bash
mkdir -p certs

openssl req -x509 -newkey rsa:2048 -sha256 -nodes \
    -keyout certs/server-key.pem \
    -out certs\server-cert.pem \
    -days 30 \
    -config certs/openssl.cnf \
    -extensions extensions
```

This creates two files:

- `server-cert.pem`: The public certificate needed by clients.
- `server-key.pem`: The private key, which must remain on the server and must not be commited to Git.

The certificate identifies the server as `netlab.local` and expires after 30 days

## Running the Server

From the project root:

```bash
cargo run --bin Netlab
```

The server listens on port `8080` and displays a local IP address. A VPN can affect the address detected

## Running the Client

On the same computer:

```bash
cargo run --bin client -- 127.0.0.1:8080
```

From another computer on the same network:

```bash
cargo run --bin client -- SERVER_IP:8080
```
The other computer need a trusted copy of the server's public certificate at `certs/server-cert.pem` It doesn't need the private key

Enter a username when prompted, then type messages and press Enter to send them.

## Commands

|        Command      |                Description                                               |
| ------------------- | ------------------------------------------------------------------------ |
| `src/main.rs`       | Loads TLS settings and starts the server.                                |
| `src/server.rs`     | Accept connections, manage users, and broadcast messages.                |
| `src/client.rs`     | Registers users and process messages and commands.                       |
| `src/commands.rs`   | Parse chat commands.                                                     |
| `src/tls.rs`        | Loads the server certificate and private key.                            |
| `src/bin/client.rs` | Connects to the server and handles keyboard input and incoming messages. |

The server uses Tokio for asynchronous tasks. Each user has an outgoing message queue and a task that sends those messages through their TLS connection.

## Checks

```bash
cargo fmt --check
cargo check --bins
cargo test --bins
```

To check the complete app, connect two clients and test messages, duplicate usernames, commands and disconnections.


# Netlab

A terminal chat app built in Rust to learn about networks,async and encryption.


## Hosted demo (1.1.0)

Download the client for your platform, extract it, and open the launcher
(`Abrir Netlab.command` on macOS or `Abrir Netlab.bat` on Windows). On Linux,
run `./client` in a terminal. Enter a username, then start chatting.
No Rust installation, certificate files, server setup, or address arguments
are required for the hosted demo. macOS builds are not notarized.

Default endpoint: `wss://netlab.izansase-bot.hackclub.app`.
The client verifies the public HTTPS certificate using bundled trusted CA roots.
Messages are encrypted between the client and the hosting reverse proxy;
this is not end-to-end encryption. This is a shared public demo chat.

### Hosting behind the Nest reverse proxy

```bash
cargo build --release --locked --bins
./target/release/Netlab --websocket 0.0.0.0:8080
```

In the Nest dashboard, add `netlab.izansase-bot.hackclub.app` under Domains
with target port `8080`. The proxy handles public HTTPS/WSS; the WebSocket
backend runs on the private container network. Do not expose this unencrypted
backend directly on a public network. No PEM files are used in this mode.

For a local development test:

```bash
./target/release/Netlab --websocket 127.0.0.1:8080
./target/release/client ws://127.0.0.1:8080
```

An alternative hosted server can be supplied as `./client wss://your-domain`.
For the original direct TLS mode, supply `host:port` and follow the certificate
instructions below. Existing 1.0.x clients cannot connect to the WSS endpoint.

### Service example (Nest container, root session)

Keep the previous server directory as a backup. Adapt paths if needed:

```ini
[Unit]
Description=Netlab WebSocket chat
After=network.target

[Service]
WorkingDirectory=/Netlab-websocket
ExecStart=/Netlab-websocket/target/release/Netlab --websocket 0.0.0.0:8080
Restart=always
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
DynamicUser=true

[Install]
WantedBy=multi-user.target
```

Save as `/etc/systemd/system/netlab-websocket.service`, then run
`systemctl daemon-reload` and `systemctl enable --now netlab-websocket`.
Stop the previous process using port 8080 before starting this service.
Use `journalctl -u netlab-websocket -n 50` to inspect errors.

### Validation

`cargo test` includes a real WebSocket integration test with two users,
messages, duplicate names, commands, disconnection and username reuse.
The client-build workflow produces packages for macOS (Apple Silicon/Intel),
Windows x64, and Linux x64. A successful build is not a manual test on every OS.

---

The following sections describe the original direct TLS mode.

## Language

Actually it's only avaible in spanish but I want to make options to change language or another branch in english at least.

## Features

- Multiple simultaneous users.
- Usernames with duplicate name detection.
- Messages broadcast to other connected users.
- Join and leave notifications
- A dedicated terminal client
- TLS connections with server certificate verification.
- Colorful time-stamped messages ;D
- Custom Ip and ports from the server part
- Admin managing terminal

## Requirements

- An up-to-date Rust toolchain and Cargo
- OpenSSL to generate a test certificate

## Installation

```bash
git clone https://github.com/Izansase-Byte/Netlab.git
cd Netlab
cargo build --bins
```

## Certificates and TLS Setup

Generate the certificate **once on the server**, then copy its public certificate to every client. Do not generate separate certificates on clients.

### 1. Prepare the files on the server

Run from the project root or the extracted server folder:

```bash
mkdir -p certs
```

Create `certs/openssl.cnf` with the following content if your download does not include it:

```ini
[req]
prompt = no
distinguished_name = identity
x509_extensions = extensions

[identity]
CN = netlab.local

[extensions]
subjectAltName = DNS:netlab.local,DNS:localhost,IP:127.0.0.1
basicConstraints = critical,CA:FALSE
keyUsage = critical,digitalSignature,keyEncipherment
extendedKeyUsage = serverAuth
```

The setting is `x509_extensions`, with an underscore.

### 2. Generate the certificate and private key

```bash
openssl req -x509 -newkey rsa:2048 -sha256 -nodes \
  -keyout certs/server-key.pem \
  -out certs/server-cert.pem \
  -days 30 \
  -config certs/openssl.cnf \
  -extensions extensions
```

- `certs/server-cert.pem` is the public certificate. Share this with clients.
- `certs/server-key.pem` is the private key. Keep it only on the server. Never commit it to Git or include it in a public download.

The certificate expires after 30 days. Regenerating it requires copying the new public certificate to all clients.

### 3. Verify the certificate

```bash
openssl x509 -in certs/server-cert.pem -noout -text
```

Check for `Version: 3 (0x2)` and `DNS:netlab.local` in the alternative names.

The client connects to the supplied IP but verifies the identity `netlab.local`. You do not need to configure DNS for that name.

### 4. Copy the public certificate to clients

On each client, place a trusted copy of the server's `server-cert.pem` in its `certs/` folder. Do not copy the private key.

You can compare fingerprints on the server and client using:

```bash
openssl x509 -in certs/server-cert.pem -noout -fingerprint -sha256
```

Compare the results through a trusted channel. They must match.

### Certificate locations

With binaries rebuilt from version 1.0.1 or later, Netlab first searches for `certs/` **beside the executable**. If that folder does not exist, it searches in the current working directory.

- Server folder: `Netlab`, `certs/server-cert.pem`, and `certs/server-key.pem`.
- Client folder: `client` and `certs/server-cert.pem`.

An existing `certs/` folder beside the executable takes priority even if it is incomplete. Add the required files there.

For `cargo run`, run from the project root so the fallback finds the project's certificates. Avoid leaving outdated certificates in `target/debug/certs/` or `target/release/certs/`.

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

## Chat Commands

| Command | Description |
| --- | --- |
| `/help` | Show chat commands. |
| `/users` | List connected users. |
| `/quit` | Leave the chat. |

## Server Administration

Enter these in the server terminal:

| Command | Description |
| --- | --- |
| `/users` | Show connected users and IDs. |
| `/say message` | Send a notice to all users. |
| `/kick ID` | Disconnect the selected user. |

## Project Structure

|        File         |                Description                                               |
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

## Custom IP

To use custom IP or ports you have to use commands when oppening the server;

```bash
cargo run --bin Netlab -- 0.0.0.0:9000
```
## Releases

Download the programs from the [Releases page](https://github.com/Izansase-byte/Netlab/releases). Precompiled binaries do not require Rust or Cargo.

### Supported platforms

- macOS on Apple Silicon (M1 and later).

Use binaries rebuilt from version 1.0.1 or later for certificate lookup beside the executable. Updating the source does not update an older executable or ZIP.

### Run the server

1. Extract the server download.
2. Follow [Certificates and TLS Setup](#certificates-and-tls-setup) from that folder.
3. Keep the certificate and key in `certs/` beside `Netlab`.
4. Open Terminal in that folder:

```bash
./Netlab
```

For a custom address and port:

```bash
./Netlab 0.0.0.0:9000
```

`0.0.0.0` makes the server listen on its IPv4 network interfaces. Clients must connect to the server's actual local IP, not to `0.0.0.0`.

### Run the client

1. Extract the client download.
2. Create `certs/` beside `client`.
3. Copy only the server's public `server-cert.pem` into that folder.
4. Open Terminal in that folder.

For a server on the same computer:

```bash
./client 127.0.0.1:8080
```

For another computer on the same network:

```bash
./client SERVER_IP:8080
```

Replace `SERVER_IP` with the server's local IP. Match any custom port; for example:

```bash
./client SERVER_IP:9000
```

Keep the server running in its own terminal. If macOS asks to allow incoming connections for the server, allow them for your intended network. Router port forwarding is not needed for a same-network test.

Use Terminal when troubleshooting so you can read errors and provide address arguments.

## Troubleshooting

### No such file or directory

Check the certificate locations above and the paths printed by the program. The client needs `server-cert.pem`; the server needs both `server-cert.pem` and `server-key.pem`.

Use exact filenames and forward slashes in paths, for example `certs/server-cert.pem`.

### UnsupportedCertVersion

Check that the certificate is X.509 version 3. Regenerate it with the configuration and explicit `-extensions extensions` option above, then copy the new public certificate to clients.

### Certificate verification failed

Check that the client trusts the server's current certificate, the certificate has not expired, the system clock is correct, and the certificate contains `DNS:netlab.local`. Do not disable certificate verification.

### Certificate and private key do not match

Generate a new matching pair on the server and distribute the new public certificate to clients.

### Connection refused or timed out

Check that the server is running, the IP and port are correct, and incoming connections are allowed. Some guest Wi-Fi networks isolate devices from each other.

### Rebuild after source changes

```bash
cargo build --release --bins
```

Replace the old release executables with `target/release/Netlab` and `target/release/client`. Keep the certificates in the correct locations. Never upload a ZIP containing the private key.

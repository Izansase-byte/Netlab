use std::fs::File;
use std::io::{self, BufReader};
use std::sync::Arc;

use tokio_rustls::rustls::ServerConfig;

pub(crate) fn server_config() -> io::Result<Arc<ServerConfig>> {
    let certs = crate::cert_paths::certificate_dir()?;
    let mut cert_file = BufReader::new(open_certificate(&certs.join("server-cert.pem"))?);
    let certificates = rustls_pemfile::certs(&mut cert_file).collect::<io::Result<Vec<_>>>()?;

    if certificates.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "El archivo no contiene certificados",
        ));
    }

    let mut key_file = BufReader::new(open_certificate(&certs.join("server-key.pem"))?);
    let key = rustls_pemfile::private_key(&mut key_file)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "No se encontró una clave privada",
        )
    })?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(Arc::new(config))
}

fn open_certificate(path: &std::path::Path) -> io::Result<File> {
    File::open(path).map_err(|e| {
        io::Error::new(e.kind(), format!("No se pudo abrir {}: {}", path.display(), e))
    })
}

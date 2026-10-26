use std::io;
use std::path::PathBuf;

// Las releases buscan los certificados junto al ejecutable.
// Durante el desarrollo también se admite certs/ en la carpeta de ejecución.
pub(crate) fn certificate_dir() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    let executable_dir = executable.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "No se pudo localizar el ejecutable")
    })?;
    let bundled = executable_dir.join("certs");
    if bundled.is_dir() {
        return Ok(bundled);
    }

    let local = std::env::current_dir()?.join("certs");
    if local.is_dir() {
        return Ok(local);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "No se encontró la carpeta de certificados. Coloca certs/ junto al ejecutable: {}. También se comprobó: {}",
            bundled.display(),
            local.display()
        ),
    ))
}

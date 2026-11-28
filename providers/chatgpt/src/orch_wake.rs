//! Best-effort wake hint to the optional Orch relay. Work state stays in Orch.
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

pub fn start(home: &Path) -> mpsc::Sender<()> {
    let path = std::env::var_os("ORCH_RELAY_HEALTH_URL_FILE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state/codex/services/termux-orch-relay/health.url"));
    let (tx, mut rx) = mpsc::channel(1);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let _ = tokio::time::timeout(Duration::from_millis(750), wake(&path)).await;
        }
    });
    tx
}

fn read_private(path: &Path) -> io::Result<String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    let mut value = String::new();
    file.take(257).read_to_string(&mut value)?;
    if value.len() > 256 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(value)
}

async fn wake(path: &Path) -> io::Result<()> {
    let url = read_private(path)?;
    let addr: SocketAddr = url
        .trim()
        .strip_prefix("http://")
        .and_then(|addr| addr.parse().ok())
        .filter(|addr: &SocketAddr| addr.ip().is_loopback())
        .ok_or(io::ErrorKind::InvalidData)?;
    let token = read_private(&path.with_file_name("wake.token"))?;
    let token = token.trim();
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut stream = TcpStream::connect(addr).await?;
    stream.write_all(format!(
        "POST /wake HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    ).as_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn wake_sends_private_bearer_only_to_loopback() {
        let root = std::env::temp_dir().join(format!("provider-wake-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("health.url");
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let write_private = |path: &Path, value: &str| {
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)
                .unwrap()
                .write_all(value.as_bytes())
                .unwrap();
        };
        let token = "a".repeat(64);
        write_private(&path, &format!("http://{addr}\n"));
        write_private(&root.join("wake.token"), &token);
        wake(&path).await.unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = String::new();
        stream.read_to_string(&mut request).await.unwrap();
        assert!(request.starts_with("POST /wake HTTP/1.1\r\n"));
        assert!(request.contains(&format!("\r\nAuthorization: Bearer {token}\r\n")));
        write_private(&path, "http://192.0.2.1:80");
        assert_eq!(
            wake(&path).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

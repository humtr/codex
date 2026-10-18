use codex_chatgpt_provider::{
    cdp::Process,
    context::digest,
    journal::{random_id, Journal},
    runtime,
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("provider-runtime-{}", random_id().unwrap())))
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn source_binary_serves_private_authenticated_endpoint_without_connecting_to_or_spending_chat() {
    let root = Root::new();
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let descriptor = root.0.join("descriptor.json");
    let process = Process::read(std::process::id()).unwrap();
    std::fs::write(&descriptor,json!({"version":1,"kind":"termux-electron-pipe","endpoint":"http://127.0.0.1:9/","token":"a".repeat(43),"host":process,"browser":process}).to_string()).unwrap();
    std::fs::set_permissions(&descriptor, std::fs::Permissions::from_mode(0o600)).unwrap();
    let state = root.0.join("state");
    let binary = env!("CARGO_BIN_EXE_codex-chatgpt-provider");
    let mut child = Command::new(binary)
        .args(["serve", "--descriptor"])
        .arg(&descriptor)
        .arg("--state")
        .arg(&state)
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", root.0.join("config"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let listener: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(listener["app_connected"], false);
        assert_eq!(listener["production_qualified"], false);
        assert!(listener.get("token").is_none());
        let status = Command::new(binary)
            .arg("status")
            .arg("--state")
            .arg(&state)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "status failed without revealing private state"
        );
        let value: Value = serde_json::from_slice(&status.stdout).unwrap();
        assert_eq!(value["live"], true);
        assert!(value.get("token").is_none());
        let credential = Command::new(binary)
            .arg("token")
            .arg("--state")
            .arg(&state)
            .output()
            .unwrap();
        assert!(credential.status.success());
        assert_eq!(credential.stdout.len(), 49);
        assert!(!String::from_utf8(status.stdout)
            .unwrap()
            .contains(std::str::from_utf8(&credential.stdout[..48]).unwrap()));
        assert!(!root.0.join("config").exists());
        let home = root.0.join("codex-home");
        std::fs::create_dir(&home).unwrap();
        let prepared = Command::new(binary)
            .arg("prepare")
            .arg("--home")
            .arg(&home)
            .arg("--state")
            .arg(&state)
            .env("HOME", &root.0)
            .env("XDG_CONFIG_HOME", root.0.join("config"))
            .output()
            .unwrap();
        assert!(
            prepared.status.success(),
            "{}",
            String::from_utf8_lossy(&prepared.stderr)
        );
        let value: Value = serde_json::from_slice(&prepared.stdout).unwrap();
        assert_eq!(value["prepared"], true);
        assert_eq!(value["app_inference_submitted"], false);
        let profile = std::fs::read_to_string(home.join("chatgpt.config.toml")).unwrap();
        assert!(profile.contains("chatgpt_desktop"));
        assert!(!profile.contains(std::str::from_utf8(&credential.stdout[..48]).unwrap()));
        assert!(!root.0.join("config").exists());
    }));
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let until = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while child.try_wait().unwrap().is_none() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
    }
    child.wait().unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
    let stale = Command::new(binary)
        .arg("status")
        .arg("--state")
        .arg(&state)
        .output()
        .unwrap();
    assert!(!stale.status.success());
}

#[tokio::test]
async fn invalid_local_listener_never_receives_our_bearer() {
    let root = Root::new();
    let journal = Journal::open(&root.0).unwrap();
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    journal.write(&digest(&"termux-codex-responses-service-v1"),&json!({"schema":"termux-codex-responses-service-v1","process":Process::read(std::process::id()).unwrap(),"port":socket.local_addr().unwrap().port(),"token":"b".repeat(48)})).unwrap();
    let fake = tokio::spawn(async move {
        let (mut socket, _) = socket.accept().await.unwrap();
        let mut bytes = [0; 8192];
        let count = socket.read(&mut bytes).await.unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]);
        assert!(!request.to_lowercase().contains("authorization"));
        assert!(!request.contains(&"b".repeat(48)));
        let body = json!({"schema":"termux-codex-provider-v1","proof":"00".repeat(32)}).to_string();
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    assert!(runtime::token(&root.0).await.is_err());
    fake.await.unwrap();
}

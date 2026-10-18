//! Candidate process entrypoint, private local credentials and owned endpoint
//! proof. No official-app bearer or native executor is managed here.
use crate::{
    app::{AppHost, Remote},
    cdp::{Descriptor, Process},
    context::digest,
    journal::{random_id, Journal},
    protocol::Error,
    server::{self, Service},
    store,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    schema: String,
    process: Process,
    port: u16,
    token: String,
}
fn key() -> String {
    digest(&"termux-codex-responses-service-v1")
}
pub fn state_path() -> Result<PathBuf, Error> {
    let root = match std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(std::env::var_os("HOME").ok_or(Error("home_unavailable"))?)
            .join(".local/state"),
    };
    if !root.is_absolute()
        || root
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error("absolute_state_directory_required"));
    }
    Ok(root.join("codex/providers/chatgpt"))
}

fn binding(root: &Path) -> Result<Binding, Error> {
    let value = Journal::inspect(root, &key())?.ok_or(Error("provider_not_running"))?;
    let value: Binding =
        serde_json::from_value(value).map_err(|_| Error("invalid_service_record"))?;
    if value.schema != "termux-codex-responses-service-v1"
        || value.port == 0
        || value.token.len() != 48
        || !value.token.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error("invalid_service_record"));
    }
    value.process.validate()?;
    Ok(value)
}

pub async fn token(root: &Path) -> Result<String, Error> {
    let selected = binding(root)?;
    let nonce = format!("{}{}", random_id()?, &random_id()?[..16]);
    let received=tokio::time::timeout(Duration::from_secs(3),async {
        let mut socket=tokio::net::TcpStream::connect(("127.0.0.1",selected.port)).await.map_err(|_|Error("provider_unavailable"))?;
        socket.write_all(format!("GET /__ready HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nX-Termux-Challenge: {nonce}\r\n\r\n").as_bytes()).await.map_err(|_|Error("provider_unavailable"))?;
        let mut bytes=Vec::new();socket.take(8193).read_to_end(&mut bytes).await.map_err(|_|Error("provider_unavailable"))?;
        if bytes.len()>8192 {return Err(Error("invalid_readiness_proof"));}
        let text=String::from_utf8(bytes).map_err(|_|Error("invalid_readiness_proof"))?;
        let (headers,body)=text.split_once("\r\n\r\n").ok_or(Error("invalid_readiness_proof"))?;
        if !headers.starts_with("HTTP/1.1 200 ") {return Err(Error("invalid_readiness_proof"));}
        let value:serde_json::Value=serde_json::from_str(body).map_err(|_|Error("invalid_readiness_proof"))?;
        if value["schema"]!="termux-codex-provider-v1" {return Err(Error("invalid_readiness_proof"));}
        let proof=value["proof"].as_str().filter(|v|v.len()==64 && v.bytes().all(|b|b.is_ascii_hexdigit())).ok_or(Error("invalid_readiness_proof"))?;
        let bytes=(0..64).step_by(2).map(|i|u8::from_str_radix(&proof[i..i+2],16).map_err(|_|Error("invalid_readiness_proof"))).collect::<Result<Vec<_>,_>>()?;
        Ok(bytes)
    }).await.map_err(|_|Error("readiness_timeout"))??;
    let mut expected = Hmac::<Sha256>::new_from_slice(selected.token.as_bytes()).unwrap();
    expected.update(format!("termux-codex-ready-v1:{nonce}").as_bytes());
    expected
        .verify_slice(&received)
        .map_err(|_| Error("invalid_readiness_proof"))?;
    selected.process.validate()?;
    Ok(selected.token)
}

pub async fn status(root: &Path) -> Result<serde_json::Value, Error> {
    let selected = binding(root)?;
    token(root).await?;
    Ok(
        json!({"schema":"termux-codex-provider-status-v1","live":true,"base_url":format!("http://127.0.0.1:{}/v1",selected.port),"process":selected.process,"meaning":"authenticated local endpoint; model/task progress is reported separately"}),
    )
}

pub async fn serve(descriptor: PathBuf, root: PathBuf, port: u16) -> Result<(), Error> {
    Descriptor::read(&descriptor)?;
    let journal = Arc::new(Journal::open(&root)?);
    let bridge = Arc::new(Remote::new(descriptor, journal.owner.clone()));
    let host = Arc::new(AppHost::new(bridge, journal.clone()));
    let token = random_id()?;
    let config = store::config_path().map_err(|_| Error("invalid_configuration_path"))?;
    let service = Service::configured(config, token.clone(), host)?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|_| Error("provider_port_unavailable"))?;
    let port = listener
        .local_addr()
        .map_err(|_| Error("provider_port_unavailable"))?
        .port();
    journal.write(
        &key(),
        &Binding {
            schema: "termux-codex-responses-service-v1".into(),
            process: Process::read(std::process::id())?,
            port,
            token,
        },
    )?;
    println!(
        "{}",
        json!({"schema":"termux-codex-provider-listener-v1","base_url":format!("http://127.0.0.1:{port}/v1"),"app_connected":false,"production_qualified":false})
    );
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    tokio::spawn(async move {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM registration");
        tokio::select! {_=tokio::signal::ctrl_c()=>(),_=terminate.recv()=>()}
        stop.cancel();
    });
    server::serve(listener, service, shutdown)
        .await
        .map_err(|_| Error("provider_server_failed"))
}

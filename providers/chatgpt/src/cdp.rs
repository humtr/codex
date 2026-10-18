//! One owned relay connection multiplexes app lanes. A lost observer never
//! implies cancellation or permission to submit a replacement model request.
use crate::protocol::Error;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig, Message},
};

const LIMIT: usize = 16 * 1024 * 1024;
pub fn private_read(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error("private_file_unavailable"))?;
    let meta = file
        .metadata()
        .map_err(|_| Error("private_file_unavailable"))?;
    if !path.is_absolute()
        || !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o777 != 0o600
        || meta.len() > limit
    {
        return Err(Error("private_file_required"));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error("private_file_unavailable"))?;
    if bytes.len() as u64 > limit {
        return Err(Error("private_file_too_large"));
    }
    Ok(bytes)
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Process {
    pub pid: u32,
    pub start: String,
    pub image: PathBuf,
}
impl Process {
    pub fn read(pid: u32) -> Result<Self, Error> {
        if pid == 0 {
            return Err(Error("invalid_process_owner"));
        }
        let root = PathBuf::from(format!("/proc/{pid}"));
        let raw =
            std::fs::read_to_string(root.join("stat")).map_err(|_| Error("stale_process_owner"))?;
        let parts = raw
            .rsplit_once(')')
            .ok_or(Error("stale_process_owner"))?
            .1
            .split_whitespace()
            .collect::<Vec<_>>();
        if matches!(parts.first(), Some(&"Z") | Some(&"X")) {
            return Err(Error("stale_process_owner"));
        }
        let start = parts
            .get(19)
            .ok_or(Error("stale_process_owner"))?
            .to_string();
        let image =
            std::fs::read_link(root.join("exe")).map_err(|_| Error("stale_process_owner"))?;
        Ok(Self { pid, start, image })
    }
    pub fn validate(&self) -> Result<(), Error> {
        if Self::read(self.pid)? != *self {
            return Err(Error("stale_process_owner"));
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Descriptor {
    pub version: u32,
    pub kind: String,
    pub endpoint: String,
    pub token: String,
    pub host: Process,
    pub browser: Process,
}
impl Descriptor {
    pub fn read(path: &Path) -> Result<Self, Error> {
        let value: Self = serde_json::from_slice(&private_read(path, 8192)?)
            .map_err(|_| Error("invalid_host_descriptor"))?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Error> {
        let port = self
            .endpoint
            .strip_prefix("http://127.0.0.1:")
            .map(|s| s.strip_suffix('/').unwrap_or(s))
            .and_then(|s| s.parse::<u16>().ok())
            .filter(|p| *p > 0);
        if self.version != 1
            || self.kind != "termux-electron-pipe"
            || port.is_none()
            || self.token.len() != 43
            || !self
                .token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error("owned_host_required"));
        }
        self.host.validate()?;
        self.browser.validate()
    }
    pub fn generation(&self) -> String {
        crate::context::digest(self)
    }
}

struct Command {
    method: String,
    params: Value,
    session: Option<String>,
    reply: oneshot::Sender<Result<Value, Error>>,
}
#[derive(Clone)]
pub struct Client {
    queue: mpsc::Sender<Command>,
    pub generation: Arc<str>,
}
impl Client {
    pub async fn connect(descriptor: &Descriptor) -> Result<Self, Error> {
        descriptor.validate()?;
        let endpoint = descriptor
            .endpoint
            .trim_end_matches('/')
            .replacen("http:", "ws:", 1)
            + "/cdp";
        let mut request = endpoint
            .into_client_request()
            .map_err(|_| Error("invalid_host_endpoint"))?;
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {}", descriptor.token)
                .parse()
                .map_err(|_| Error("invalid_host_token"))?,
        );
        let config = WebSocketConfig::default()
            .max_message_size(Some(LIMIT))
            .max_frame_size(Some(LIMIT));
        let (socket, _) = tokio::time::timeout(
            Duration::from_secs(10),
            connect_async_with_config(request, Some(config), false),
        )
        .await
        .map_err(|_| Error("relay_connect_timeout"))?
        .map_err(|_| Error("relay_connect_failed"))?;
        let (queue, mut input) = mpsc::channel::<Command>(128);
        tokio::spawn(async move {
            let (mut write, mut read) = socket.split();
            let mut sequence = 0_u64;
            let mut pending = BTreeMap::<u64, oneshot::Sender<Result<Value, Error>>>::new();
            loop {
                tokio::select! {
                    command = input.recv() => {
                        let Some(command) = command else { break };
                        pending.retain(|_,sender| !sender.is_closed());
                        if pending.len() >= 128 { let _ = command.reply.send(Err(Error("cdp_queue_full"))); continue; }
                        sequence += 1;
                        let mut value = json!({"id":sequence,"method":command.method,"params":command.params});
                        if let Some(session) = command.session { value["sessionId"] = json!(session); }
                        let data = value.to_string();
                        if data.len() > LIMIT { let _ = command.reply.send(Err(Error("cdp_request_too_large"))); continue; }
                        pending.insert(sequence,command.reply);
                        if write.send(Message::Text(data.into())).await.is_err() { break; }
                    }
                    frame = read.next() => match frame {
                        Some(Ok(Message::Text(data))) => {
                            let Ok(value) = serde_json::from_str::<Value>(&data) else { break };
                            if let Some(sender) = value["id"].as_u64().and_then(|id| pending.remove(&id)) {
                                let _ = sender.send(if value.get("error").is_some() { Err(Error("cdp_operation_rejected")) } else { Ok(value["result"].clone()) });
                            }
                        }
                        Some(Ok(Message::Ping(data))) => { if write.send(Message::Pong(data)).await.is_err() { break; } }
                        Some(Ok(Message::Close(_)))|Some(Err(_))|None => break,
                        _ => (),
                    },
                }
            }
            for (_, sender) in pending {
                let _ = sender.send(Err(Error("cdp_observer_disconnected")));
            }
            let _ = write.close().await;
        });
        Ok(Self {
            queue,
            generation: descriptor.generation().into(),
        })
    }
    /// Chromium's private FD3/FD4 protocol. The caller owns the child and both
    /// endpoints; no debug TCP listener, relay, token extraction or JS host.
    pub fn pipe<R, W>(reader: R, mut writer: W, generation: String) -> Self
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
        let (queue, mut input) = mpsc::channel::<Command>(128);
        let (frames, mut output) = mpsc::channel::<Value>(128);
        let reader = tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            loop {
                let mut bytes = Vec::new();
                let result = (&mut reader)
                    .take((LIMIT + 1) as u64)
                    .read_until(0, &mut bytes)
                    .await;
                if result.is_err()
                    || bytes.is_empty()
                    || bytes.len() > LIMIT
                    || bytes.pop() != Some(0)
                {
                    break;
                }
                if bytes.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_slice(&bytes) else {
                    break;
                };
                if frames.send(value).await.is_err() {
                    break;
                }
            }
        });
        tokio::spawn(async move {
            let mut sequence = 0_u64;
            let mut pending = BTreeMap::<u64, oneshot::Sender<Result<Value, Error>>>::new();
            loop {
                tokio::select! {
                    command=input.recv()=>{
                        let Some(command)=command else {break;};
                        pending.retain(|_,sender|!sender.is_closed());
                        if pending.len()>=128 {let _=command.reply.send(Err(Error("cdp_queue_full")));continue;}
                        sequence+=1;
                        let mut value=json!({"id":sequence,"method":command.method,"params":command.params});
                        if let Some(session)=command.session {value["sessionId"]=json!(session);}
                        let mut bytes=serde_json::to_vec(&value).unwrap();
                        if bytes.len()>=LIMIT {let _=command.reply.send(Err(Error("cdp_request_too_large")));continue;}
                        bytes.push(0);pending.insert(sequence,command.reply);
                        if writer.write_all(&bytes).await.is_err() {break;}
                    },
                    frame=output.recv()=>{
                        let Some(value)=frame else {break;};
                        if let Some(sender)=value["id"].as_u64().and_then(|id|pending.remove(&id)) {
                            let _=sender.send(if value.get("error").is_some(){Err(Error("cdp_operation_rejected"))}else{Ok(value["result"].clone())});
                        }
                    },
                }
            }
            reader.abort();
            for (_, sender) in pending {
                let _ = sender.send(Err(Error("cdp_observer_disconnected")));
            }
            let _ = writer.shutdown().await;
        });
        Self {
            queue,
            generation: generation.into(),
        }
    }
    pub async fn request(
        &self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, Error> {
        let (reply, receive) = oneshot::channel();
        tokio::time::timeout(Duration::from_secs(15), async {
            self.queue
                .send(Command {
                    method: method.into(),
                    params,
                    session: session.map(str::to_owned),
                    reply,
                })
                .await
                .map_err(|_| Error("cdp_observer_disconnected"))?;
            receive
                .await
                .map_err(|_| Error("cdp_observer_disconnected"))?
        })
        .await
        .map_err(|_| Error("cdp_operation_timeout"))?
    }
    pub async fn app(&self) -> Result<App, Error> {
        let result = self.request("Target.getTargets", json!({}), None).await?;
        let targets = result["targetInfos"]
            .as_array()
            .ok_or(Error("app_target_unavailable"))?;
        let roots = targets
            .iter()
            .filter(|t| t["type"] == "page" && t["url"] == "app://-/index.html")
            .collect::<Vec<_>>();
        if roots.is_empty() {
            return Err(Error("app_target_unavailable"));
        }
        if roots.len() != 1 {
            return Err(Error("unique_app_target_required"));
        }
        let result = self
            .request(
                "Target.attachToTarget",
                json!({"targetId":roots[0]["targetId"],"flatten":true}),
                None,
            )
            .await?;
        let session = result["sessionId"]
            .as_str()
            .ok_or(Error("app_session_unavailable"))?
            .to_owned();
        Ok(App {
            client: self.clone(),
            session,
        })
    }
}

#[derive(Clone)]
pub struct App {
    client: Client,
    session: String,
}
impl App {
    pub async fn evaluate(&self, expression: &str) -> Result<Value, Error> {
        let value = self
            .client
            .request(
                "Runtime.evaluate",
                json!({"expression":expression,"awaitPromise":true,"returnByValue":true}),
                Some(&self.session),
            )
            .await?;
        if value.get("exceptionDetails").is_some() {
            return Err(Error("app_bridge_evaluation_failed"));
        }
        Ok(value["result"]["value"].clone())
    }
}

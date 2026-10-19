//! Native control transport. Core prepares its qualified server; upstream owns
//! tools, approvals, turns and history. No shell commands or transcript writes.
use crate::{cdp::Process, config::Config, protocol::Error, registration};
use codex_runtime_binding::Binding;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{broadcast, mpsc, oneshot, Mutex},
};

const LIMIT: usize = 16 * 1024 * 1024;
type Replies = Arc<Mutex<BTreeMap<u64, oneshot::Sender<Result<Value, Error>>>>>;

fn read(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    for parent in path.ancestors().skip(1) {
        if !std::fs::symlink_metadata(parent)?.is_dir() {
            return Err(std::io::Error::other("unsafe native binding parent"));
        }
    }
    let file = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
        || metadata.len() > limit
    {
        return Err(std::io::Error::other("unsafe native binding record"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::other("native record bound"));
    }
    Ok(bytes)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    pub directory: PathBuf,
    pub home: PathBuf,
    pub process: Process,
}
impl Owner {
    pub fn load(directory: &Path, home: &Path, installation: &Path) -> Result<Self, Error> {
        let meta = std::fs::symlink_metadata(directory)
            .map_err(|_| Error("native_binding_unavailable"))?;
        if !meta.is_dir()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.mode() & 0o777 != 0o700
        {
            return Err(Error("native_private_binding_required"));
        }
        let binding =
            Binding::read(directory, read).map_err(|_| Error("native_binding_invalid"))?;
        let generations = installation.join(".local/lib/codex/core/generations");
        if binding.home != home
            || binding.program.file_name().is_none_or(|n| n != "runtime")
            || binding.program.parent().and_then(Path::parent) != Some(generations.as_path())
        {
            return Err(Error("native_execution_home_or_runtime_changed"));
        }
        let pid = String::from_utf8(
            read(&directory.join("pid"), 128).map_err(|_| Error("native_binding_invalid"))?,
        )
        .map_err(|_| Error("native_binding_invalid"))?
        .trim()
        .parse::<u32>()
        .map_err(|_| Error("native_binding_invalid"))?;
        let process = Process::read(pid)?;
        if process.image != binding.program {
            return Err(Error("native_runtime_owner_changed"));
        }
        Ok(Self {
            directory: directory.into(),
            home: home.into(),
            process,
        })
    }
    pub fn validate(&self, installation: &Path) -> Result<(), Error> {
        if Self::load(&self.directory, &self.home, installation)? != *self {
            return Err(Error("native_binding_changed"));
        }
        self.process.validate()
    }
}

pub struct Native {
    pub owner: Owner,
    installation: PathBuf,
    write: mpsc::Sender<Value>,
    replies: Replies,
    sequence: AtomicU64,
    pub events: broadcast::Sender<Value>,
    pub human_requests: Arc<Mutex<BTreeMap<String, Value>>>,
}
impl Native {
    pub async fn public_context(&self, thread: &str) -> Result<Vec<Value>, Error> {
        let row = self
            .call(
                "thread/read",
                json!({"threadId":thread,"includeTurns":false}),
            )
            .await?;
        let path = PathBuf::from(
            row["thread"]["path"]
                .as_str()
                .ok_or(Error("native_context_unavailable"))?,
        );
        if !path.is_absolute()
            || path
                .file_name()
                .and_then(|value| value.to_str())
                .is_none_or(|name| !name.ends_with(&format!("-{thread}.jsonl")))
        {
            return Err(Error("native_context_identity_changed"));
        }
        self.owner.validate(&self.installation)?;
        let expected = thread.to_owned();
        tokio::task::spawn_blocking(move || {
            let bytes = crate::cdp::private_read(&path, 64 * 1024 * 1024)?;
            let mut result = vec![];
            let mut identity = false;
            for line in bytes.split_inclusive(|byte| *byte == b'\n') {
                // A concurrently appended partial record is not committed input.
                if !line.ends_with(b"\n") {
                    break;
                }
                let row: Value =
                    serde_json::from_slice(line).map_err(|_| Error("native_context_invalid"))?;
                if row["type"] == "session_meta" {
                    if row["payload"]["id"] != expected {
                        return Err(Error("native_context_identity_changed"));
                    }
                    identity = true;
                }
                let item = &row["payload"];
                if row["type"] == "response_item"
                    && item["type"] == "message"
                    && matches!(item["role"].as_str(), Some("user" | "developer" | "system"))
                {
                    result.push(json!({"role":item["role"],"text":crate::context::text(item)?}));
                }
            }
            if !identity {
                return Err(Error("native_context_identity_changed"));
            }
            Ok(result)
        })
        .await
        .map_err(|_| Error("native_context_unavailable"))?
    }
    pub async fn answer_question(
        &self,
        thread: &str,
        request_id: &str,
        answers: Value,
    ) -> Result<(), Error> {
        self.owner.validate(&self.installation)?;
        let mut requests = self.human_requests.lock().await;
        let row = requests
            .get(request_id)
            .ok_or(Error("native_question_unavailable"))?;
        if row["method"] != "item/tool/requestUserInput" || row["params"]["threadId"] != thread {
            return Err(Error("native_question_unavailable"));
        }
        let questions = row["params"]["questions"]
            .as_array()
            .ok_or(Error("native_question_unavailable"))?;
        let provided = answers
            .as_object()
            .ok_or(Error("native_question_answer_invalid"))?;
        if provided.len() != questions.len()
            || questions.iter().any(|question| {
                provided
                    .get(question["id"].as_str().unwrap_or(""))
                    .is_none_or(|value| {
                        value["answers"].as_array().is_none_or(|answers| {
                            answers.is_empty()
                                || answers.iter().any(|answer| {
                                    answer.as_str().is_none_or(|text| text.len() > 64 * 1024)
                                })
                        })
                    })
            })
        {
            return Err(Error("native_question_answer_invalid"));
        }
        self.write
            .send(json!({"id":row["id"],"result":{"answers":answers}}))
            .await
            .map_err(|_| Error("native_delivery_unknown"))?;
        requests.remove(request_id);
        Ok(())
    }
    pub async fn connect(owner: Owner, installation: PathBuf) -> Result<Arc<Self>, Error> {
        owner.validate(&installation)?;
        let path = owner.directory.join("s");
        let socket =
            std::fs::canonicalize(&path).map_err(|_| Error("native_socket_unavailable"))?;
        let meta =
            std::fs::symlink_metadata(&socket).map_err(|_| Error("native_socket_unavailable"))?;
        use std::os::unix::fs::FileTypeExt;
        if !meta.file_type().is_socket()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.mode() & 0o077 != 0
        {
            return Err(Error("native_private_socket_required"));
        }
        let stream = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::UnixStream::connect(socket),
        )
        .await
        .map_err(|_| Error("native_connection_timeout"))?
        .map_err(|_| Error("native_socket_unavailable"))?;
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut peer as *mut libc::ucred).cast(),
                &mut length,
            )
        } != 0
            || peer.pid != owner.process.pid as i32
            || peer.uid != unsafe { libc::geteuid() }
        {
            return Err(Error("native_socket_owner_changed"));
        }
        let config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(LIMIT))
            .max_frame_size(Some(LIMIT));
        let (websocket, _) = tokio::time::timeout(
            Duration::from_secs(5),
            tokio_tungstenite::client_async_with_config("ws://localhost/rpc", stream, Some(config)),
        )
        .await
        .map_err(|_| Error("native_connection_timeout"))?
        .map_err(|_| Error("native_protocol_unavailable"))?;
        owner.validate(&installation)?;
        let (mut sink, mut input) = websocket.split();
        let (write, mut output) = mpsc::channel::<Value>(64);
        let replies: Replies = Arc::new(Mutex::new(BTreeMap::new()));
        let (events, _) = broadcast::channel(1024);
        let human_requests = Arc::new(Mutex::new(BTreeMap::new()));
        let client = Arc::new(Self {
            owner,
            installation,
            write,
            replies: replies.clone(),
            sequence: AtomicU64::new(1),
            events: events.clone(),
            human_requests: human_requests.clone(),
        });
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    row = output.recv() => match row {
                        Some(row) => if sink.send(row.to_string().into()).await.is_err() { break; },
                        None => break,
                    },
                    row = input.next() => {
                        let Some(Ok(message)) = row else { break; };
                        if message.is_close() { break; }
                        if !message.is_text() { continue; }
                        let Ok(row) = serde_json::from_str::<Value>(&message.to_string()) else { break; };
                        if row.get("method").is_some() {
                            if row["method"]=="serverRequest/resolved" {
                                human_requests.lock().await.remove(&row["params"]["requestId"].to_string());
                            }
                            if let Some(id) = row.get("id") {
                                human_requests.lock().await.insert(id.to_string(), row.clone());
                            }
                            let _ = events.send(row);
                        } else if let Some(id) = row["id"].as_u64() {
                            if let Some(reply) = replies.lock().await.remove(&id) {
                                let value = if row.get("error").is_some() { Err(Error("native_request_rejected")) }
                                    else { row.get("result").cloned().ok_or(Error("native_reply_invalid")) };
                                let _ = reply.send(value);
                            }
                        }
                    }
                }
            }
            for (_, reply) in std::mem::take(&mut *replies.lock().await) {
                let _ = reply.send(Err(Error("native_delivery_unknown")));
            }
            let _ = events.send(json!({"method":"operator/nativeDisconnected"}));
        });
        client
            .call(
                "initialize",
                json!({"clientInfo":{"name":"termux-chatgpt-operator","version":"1"},
            "capabilities":{"experimentalApi":true}}),
            )
            .await?;
        client
            .write
            .send(json!({"method":"initialized","params":{}}))
            .await
            .map_err(|_| Error("native_delivery_unknown"))?;
        Ok(client)
    }
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, Error> {
        self.owner.validate(&self.installation)?;
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        let (send, receive) = oneshot::channel();
        self.replies.lock().await.insert(id, send);
        if self
            .write
            .send(json!({"id":id,"method":method,"params":params}))
            .await
            .is_err()
        {
            self.replies.lock().await.remove(&id);
            return Err(Error("native_delivery_unknown"));
        }
        let result = tokio::time::timeout(Duration::from_secs(30), receive).await;
        self.replies.lock().await.remove(&id);
        match result {
            Ok(Ok(value)) => value,
            _ => Err(Error("native_delivery_unknown")),
        }
    }
}

pub struct Pool {
    config: Config,
    state: PathBuf,
    installation: PathBuf,
    pub home: PathBuf,
    pub workspace: PathBuf,
    base_url: String,
    binary: PathBuf,
    clients: Mutex<BTreeMap<PathBuf, Arc<Native>>>,
}
impl Pool {
    pub fn new(
        config: Config,
        state: PathBuf,
        base_url: String,
        binary: PathBuf,
    ) -> Result<Arc<Self>, Error> {
        let home = config
            .execution
            .home
            .clone()
            .ok_or(Error("operator_home_required"))?;
        let workspace = config
            .execution
            .workspace
            .clone()
            .ok_or(Error("operator_workspace_required"))?;
        let installation =
            PathBuf::from(std::env::var_os("HOME").ok_or(Error("home_unavailable"))?);
        Ok(Arc::new(Self {
            config,
            state,
            installation,
            home,
            workspace,
            base_url,
            binary,
            clients: Mutex::new(BTreeMap::new()),
        }))
    }
    pub fn routing(&self) -> Result<Value, Error> {
        let registry = self
            .installation
            .join(".local/share/codex/core/config/profiles");
        let result = registration::prepare(
            &self.config,
            &self.state,
            &registry,
            &self.home,
            &self.binary,
            &self.base_url,
        )?;
        let profile = PathBuf::from(
            result["profile"]
                .as_str()
                .ok_or(Error("native_profile_unavailable"))?,
        );
        let value: toml::Value = toml::from_str(
            &String::from_utf8(crate::cdp::private_read(&profile, 4 * 1024 * 1024)?)
                .map_err(|_| Error("native_profile_invalid"))?,
        )
        .map_err(|_| Error("native_profile_invalid"))?;
        serde_json::to_value(value).map_err(|_| Error("native_profile_invalid"))
    }
    async fn client(&self, owner: Owner) -> Result<Arc<Native>, Error> {
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get(&owner.directory) {
            if client.owner != owner {
                if client.owner.process.validate().is_ok() {
                    return Err(Error("native_owner_reconciliation_required"));
                }
                // A dead owner can be observed through the replacement verified
                // socket. Work recovery remains explicitly fenced by Operator.
            } else {
                client.owner.validate(&self.installation)?;
                return Ok(client.clone());
            }
        }
        let client = Native::connect(owner.clone(), self.installation.clone()).await?;
        clients.insert(owner.directory, client.clone());
        Ok(client)
    }
    pub async fn for_thread(&self, thread: &str) -> Result<Arc<Native>, Error> {
        let mut found = None;
        let state = self.installation.join(".local/share/codex/core");
        for family in codex_runtime_binding::FAMILIES {
            let root = state.join(family);
            if !root.exists() {
                continue;
            }
            for (count, entry) in std::fs::read_dir(root)
                .map_err(|_| Error("native_binding_unavailable"))?
                .enumerate()
            {
                if count >= 256 {
                    return Err(Error("native_binding_limit"));
                }
                let directory = entry
                    .map_err(|_| Error("native_binding_unavailable"))?
                    .path();
                let binding =
                    Binding::read(&directory, read).map_err(|_| Error("native_binding_invalid"))?;
                if binding.home != self.home {
                    continue;
                }
                let owner = match Owner::load(&directory, &self.home, &self.installation) {
                    Ok(owner) => owner,
                    Err(Error("stale_process_owner")) => continue,
                    Err(error) => return Err(error),
                };
                let client = self.client(owner).await?;
                let mut cursor = Value::Null;
                for page in 0..32 {
                    let response = client
                        .call("thread/loaded/list", json!({"limit":100,"cursor":cursor}))
                        .await?;
                    let ids = response["data"]
                        .as_array()
                        .ok_or(Error("native_reply_invalid"))?;
                    if ids.iter().any(|v| v.as_str() == Some(thread)) {
                        if found.is_some() {
                            return Err(Error("native_writer_ambiguous"));
                        }
                        found = Some(client.clone());
                    }
                    cursor = response["nextCursor"].clone();
                    if cursor.is_null() {
                        break;
                    }
                    if page == 31 {
                        return Err(Error("native_binding_limit"));
                    }
                }
            }
        }
        found.ok_or(Error("native_thread_owner_unavailable"))
    }
    pub async fn prepare(&self) -> Result<Arc<Native>, Error> {
        self.routing()?;
        let catalog = crate::projection::catalog(&self.config)
            .map_err(|_| Error("native_catalog_invalid"))?;
        let core = self
            .config
            .execution
            .core
            .as_ref()
            .ok_or(Error("operator_core_required"))?;
        let mut child = tokio::process::Command::new(core)
            .arg("__app-server-catalog-v1")
            .env("CODEX_HOME", &self.home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Error("native_core_prepare_failed"))?;
        child
            .stdin
            .take()
            .ok_or(Error("native_core_prepare_failed"))?
            .write_all(&serde_json::to_vec(&catalog).unwrap())
            .await
            .map_err(|_| Error("native_core_prepare_failed"))?;
        let output = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output())
            .await
            .map_err(|_| Error("native_core_prepare_unknown"))?
            .map_err(|_| Error("native_core_prepare_unknown"))?;
        if !output.status.success() || output.stdout.len() > 4096 {
            return Err(Error("native_core_prepare_failed"));
        }
        let socket = PathBuf::from(
            String::from_utf8(output.stdout)
                .map_err(|_| Error("native_core_prepare_failed"))?
                .trim(),
        );
        let directory = socket.parent().ok_or(Error("native_binding_invalid"))?;
        if directory
            .parent()
            .is_none_or(|p| p != self.installation.join(".local/share/codex/core/catalogs"))
        {
            return Err(Error("native_binding_invalid"));
        }
        self.client(Owner::load(directory, &self.home, &self.installation)?)
            .await
    }
}

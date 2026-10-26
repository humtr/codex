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
        unix::ffi::OsStrExt,
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
type PendingReply = (String, oneshot::Sender<Result<Value, Error>>);
type Replies = Arc<Mutex<BTreeMap<u64, PendingReply>>>;

fn native_reply(method: &str, row: &Value) -> Result<Value, Error> {
    if let Some(error) = row.get("error") {
        // Upstream's fresh thread store has no paged history until the first
        // turn is persisted. Keep this exact read rejection distinguishable;
        // no mutation, unrelated error or unsupported API is retried here.
        if method == "thread/items/list"
            && error["code"].as_i64() == Some(-32601)
            && error["message"].as_str() == Some("thread/items/list is not supported yet")
        {
            return Err(Error("native_history_not_ready"));
        }
        return Err(Error("native_request_rejected"));
    }
    row.get("result")
        .cloned()
        .ok_or(Error("native_reply_invalid"))
}

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

fn confirmed_unavailable_process(pid: u32) -> bool {
    // Android may hide a reused foreign PID: /proc/PID reports EACCES while
    // /proc/PID/stat reports ENOENT. Inspect the same process record used by
    // Process::read, rather than requiring the directory itself to disappear.
    let file = match File::open(format!("/proc/{pid}/stat")) {
        Ok(file) => file,
        Err(error) => return error.kind() == std::io::ErrorKind::NotFound,
    };
    let mut stat = String::new();
    file.take(4096).read_to_string(&mut stat).is_ok()
        && stat
            .rsplit_once(')')
            .is_some_and(|(_, fields)| matches!(fields.split_whitespace().next(), Some("Z" | "X")))
}

fn binding_for_home(directory: &Path, home: Option<&Path>) -> Result<Option<Binding>, Error> {
    let owner = read(&directory.join("owner"), codex_runtime_binding::OWNER_LIMIT)
        .map_err(|_| Error("native_binding_invalid"))?;
    // A different execution home may still use an older Core record format.
    // Identify its bounded, private owner before interpreting that format.
    if home.is_some_and(|home| owner.split(|b| *b == 0).next() != Some(home.as_os_str().as_bytes()))
    {
        return Ok(None);
    }
    // The old home/runtime and catalog-v1 shapes predate config-v1. A dead or
    // reused PID cannot serve this rendezvous, even when its old codec cannot
    // be decoded. Do not let it poison lookup of an independently valid thread.
    // Current/unknown codecs and matching live owners retain strict decoding.
    let fields = owner.split(|b| *b == 0).collect::<Vec<_>>();
    let historical_program = fields
        .get(1)
        .map(|value| Path::new(std::ffi::OsStr::from_bytes(value)));
    if matches!(fields.as_slice(), [_, _] | [_, _, b"catalog-v1"])
        && historical_program.is_some_and(|program| {
            program.is_absolute() && program.file_name().is_some_and(|name| name == "runtime")
        })
    {
        let pid = String::from_utf8(
            read(&directory.join("pid"), 128).map_err(|_| Error("native_binding_invalid"))?,
        )
        .map_err(|_| Error("native_binding_invalid"))?
        .trim()
        .parse::<u32>()
        .map_err(|_| Error("native_binding_invalid"))?;
        match Process::read(pid) {
            Err(Error("stale_process_owner")) if confirmed_unavailable_process(pid) => {
                return Ok(None);
            }
            Ok(process) if process.image.as_os_str().as_bytes() != fields[1] => return Ok(None),
            Ok(_) => (),
            Err(error) => return Err(error),
        }
    }
    let binding = Binding::read(directory, read).map_err(|_| Error("native_binding_invalid"))?;
    Ok(Some(binding))
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
    pub async fn workspace(&self, thread: &str) -> Result<PathBuf, Error> {
        let row = self
            .call(
                "thread/read",
                json!({"threadId":thread,"includeTurns":false}),
            )
            .await?;
        let cwd = row["thread"]["cwd"]
            .as_str()
            .ok_or(Error("native_workspace_unavailable"))?;
        let actual =
            std::fs::canonicalize(cwd).map_err(|_| Error("native_workspace_unavailable"))?;
        if row["thread"]["id"] != thread || row["thread"]["modelProvider"] != "chatgpt_desktop" {
            return Err(Error("native_workspace_or_route_changed"));
        }
        Ok(actual)
    }
    pub async fn verify_scope(&self, thread: &str, workspace: &Path) -> Result<(), Error> {
        if self.workspace(thread).await? != workspace {
            return Err(Error("native_workspace_or_route_changed"));
        }
        Ok(())
    }
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
        before_send: impl FnOnce() -> Result<(), Error> + Send,
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
        // Validation above is a known rejection with no dispatch. Record intent
        // only at the actual native-send boundary, while the question is locked.
        before_send()?;
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
                            if let Some((method, reply)) = replies.lock().await.remove(&id) {
                                let _ = reply.send(native_reply(&method, &row));
                            }
                        }
                    }
                }
            }
            output.close();
            for (_, (_, reply)) in std::mem::take(&mut *replies.lock().await) {
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
        self.replies
            .lock()
            .await
            .insert(id, (method.to_owned(), send));
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
    pub(crate) fn scoped(&self, config: Config) -> Result<Arc<Self>, Error> {
        Self::new(
            config,
            self.state.clone(),
            self.base_url.clone(),
            self.binary.clone(),
        )
    }
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
        registration::native_routing(&self.config, &self.state, &self.binary, &self.base_url)
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
                if !client.write.is_closed() {
                    return Ok(client.clone());
                }
            }
        }
        let client = Native::connect(owner.clone(), self.installation.clone()).await?;
        clients.insert(owner.directory, client.clone());
        Ok(client)
    }
    pub async fn for_thread(&self, thread: &str) -> Result<Arc<Native>, Error> {
        self.find_thread(thread, Some(&self.home)).await
    }
    /// Observe a recorded rendezvous without starting Core or loading a thread.
    /// Reload the kernel owner so a dead process cannot remain an authority.
    pub(crate) async fn observe_owner(&self, recorded: &Owner) -> Result<Arc<Native>, Error> {
        let owner = Owner::load(&recorded.directory, &recorded.home, &self.installation)?;
        self.client(owner).await
    }
    /// Incoming native requests carry a thread identity, not a Provider-default
    /// account. Discover its unique live owner before selecting an execution scope.
    pub async fn discover_thread(&self, thread: &str) -> Result<Arc<Native>, Error> {
        self.find_thread(thread, None).await
    }
    async fn find_thread(&self, thread: &str, home: Option<&Path>) -> Result<Arc<Native>, Error> {
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
                let Some(binding) = binding_for_home(&directory, home)? else {
                    continue;
                };
                let owner = match Owner::load(&directory, &binding.home, &self.installation) {
                    Ok(owner) => owner,
                    // Historical PID files can name a different live executable
                    // after PID reuse. It cannot own this native rendezvous. Do
                    // not connect to it or let it block another verified thread;
                    // selected Owner validation retains the strict rejection.
                    Err(Error("stale_process_owner" | "native_runtime_owner_changed")) => continue,
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
            .current_dir(&self.workspace)
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

#[cfg(test)]
mod reply_tests {
    use super::*;

    #[test]
    #[ignore = "subprocess peer for the owned transport regression"]
    fn transport_peer() {
        let root = PathBuf::from(std::env::var_os("CODEX_NATIVE_TRANSPORT_FIXTURE").unwrap());
        assert!(root.join("owned").is_file());
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let listener =
                tokio::net::UnixListener::bind(root.join("socket-path").read_link().unwrap())
                    .unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                root.join("socket-path").read_link().unwrap(),
                std::fs::Permissions::from_mode(0o600),
            )
            .unwrap();
            std::fs::write(root.join("ready"), b"ready").unwrap();
            for connection in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(message)) = socket.next().await {
                    if !message.is_text() {
                        continue;
                    }
                    let row: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                    let Some(id) = row.get("id") else {
                        continue;
                    };
                    if row["method"] == "turn/start" {
                        assert_eq!(connection, 0);
                        // The effect commits before its response is lost.
                        std::fs::write(root.join("effect"), b"once").unwrap();
                        socket.close(None).await.unwrap();
                        break;
                    }
                    let result = if row["method"] == "initialize" {
                        json!({})
                    } else {
                        assert_eq!(row["method"], "thread/read");
                        json!({"thread":{"id":"original-thread"}})
                    };
                    socket
                        .send(json!({"id":id,"result":result}).to_string().into())
                        .await
                        .unwrap();
                    if row["method"] == "thread/read" {
                        break;
                    }
                }
            }
        });
    }

    #[tokio::test]
    async fn closed_transport_recovers_the_next_observation_without_replaying_a_mutation() {
        struct Fixture {
            root: PathBuf,
            child: Option<std::process::Child>,
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                if let Some(child) = &mut self.child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let _ = std::fs::remove_dir_all(&self.root);
            }
        }
        let root =
            std::env::temp_dir().join(format!("nt.{}", &crate::journal::random_id().unwrap()[..8]));
        crate::journal::private_directory(&root).unwrap();
        let mut fixture = Fixture {
            root: root.clone(),
            child: None,
        };
        std::fs::write(root.join("owned"), b"owned native transport fixture").unwrap();
        let installation = root.join("i");
        let home = root.join("h");
        let program = installation.join(".local/lib/codex/core/generations/f/runtime");
        crate::journal::private_directory(program.parent().unwrap()).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), &program).unwrap();
        let name = codex_runtime_binding::configuration_namespace(
            &home,
            program.as_os_str(),
            None,
            b"",
            None,
        );
        let directory = installation
            .join(".local/share/codex/core/servers")
            .join(name);
        crate::journal::private_directory(&directory.join("config")).unwrap();
        std::os::unix::fs::symlink(directory.join("s"), root.join("socket-path")).unwrap();
        fixture.child = Some(
            std::process::Command::new(&program)
                .args([
                    "--ignored",
                    "--exact",
                    "operator_native::reply_tests::transport_peer",
                ])
                .env("CODEX_NATIVE_TRANSPORT_FIXTURE", &root)
                .spawn()
                .unwrap(),
        );
        let pid = fixture.child.as_ref().unwrap().id();
        for (name, bytes) in [
            (
                "owner",
                codex_runtime_binding::configuration_owner_bytes(&home, program.as_os_str(), false),
            ),
            ("config/config.toml", vec![]),
            ("pid", pid.to_string().into_bytes()),
        ] {
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(directory.join(name))
                .unwrap();
            use std::io::Write;
            file.write_all(&bytes).unwrap();
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while !root.join("ready").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let owner = Owner::load(&directory, &home, &installation).unwrap();
        let pool = Pool {
            config: Config::default(),
            state: root.join("state"),
            installation,
            home,
            workspace: root.clone(),
            base_url: "http://127.0.0.1:9".into(),
            binary: program,
            clients: Mutex::new(BTreeMap::new()),
        };
        let native = pool.client(owner.clone()).await.unwrap();
        let mut events = native.events.subscribe();
        assert_eq!(
            native
                .call("turn/start", json!({"threadId":"original-thread"}))
                .await,
            Err(Error("native_delivery_unknown"))
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            while events.recv().await.unwrap()["method"] != "operator/nativeDisconnected" {}
        })
        .await
        .unwrap();
        let next = pool.client(owner).await.unwrap();
        assert_eq!(
            next.call("thread/read", json!({"threadId":"original-thread"}))
                .await
                .unwrap(),
            json!({"thread":{"id":"original-thread"}})
        );
        assert_eq!(std::fs::read(root.join("effect")).unwrap(), b"once");
        assert!(fixture.child.as_mut().unwrap().wait().unwrap().success());
    }

    #[test]
    fn request_discovery_accepts_other_homes_without_relaxing_binding_integrity() {
        let root = std::env::temp_dir().join(format!(
            "native-scope-{}",
            crate::journal::random_id().unwrap()
        ));
        let home = root.join("account");
        let program = root.join("runtime");
        let config = b"opaque native settings";
        let directory = root
            .join("servers")
            .join(codex_runtime_binding::configuration_namespace(
                &home,
                program.as_os_str(),
                None,
                config,
                None,
            ));
        crate::journal::private_directory(&directory.join("config")).unwrap();
        use std::io::Write;
        for (name, bytes) in [
            (
                "owner",
                codex_runtime_binding::configuration_owner_bytes(&home, program.as_os_str(), false),
            ),
            ("config/config.toml", config.to_vec()),
        ] {
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(directory.join(name))
                .unwrap();
            file.write_all(&bytes).unwrap();
        }
        assert!(binding_for_home(&directory, Some(&root.join("default")))
            .unwrap()
            .is_none());
        assert_eq!(
            binding_for_home(&directory, None).unwrap().unwrap().home,
            home
        );
        std::fs::write(directory.join("config/config.toml"), b"changed").unwrap();
        assert_eq!(
            binding_for_home(&directory, None).unwrap_err(),
            Error("native_binding_invalid")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn discovery_excludes_reused_nonruntime_pid_without_weakening_selected_owner() {
        struct Root(PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Root(std::env::temp_dir().join(format!(
            "native-discovery-{}",
            crate::journal::random_id().unwrap()
        )));
        let installation = root.0.join("installation");
        let home = installation.join("execution-home");
        let workspace = root.0.join("work");
        for path in [&home, &workspace] {
            crate::journal::private_directory(path).unwrap();
        }
        let program = installation.join(".local/lib/codex/core/generations/fixture/runtime");
        let config = b"";
        let name = codex_runtime_binding::configuration_namespace(
            &home,
            program.as_os_str(),
            None,
            config,
            None,
        );
        let directory = installation
            .join(".local/share/codex/core/servers")
            .join(name);
        crate::journal::private_directory(&directory.join("config")).unwrap();
        let owner =
            codex_runtime_binding::configuration_owner_bytes(&home, program.as_os_str(), false);
        let process = Process::read(std::process::id()).unwrap();
        assert_ne!(process.image, program);
        use std::io::Write;
        for (name, bytes) in [
            ("owner", owner),
            ("config/config.toml", config.to_vec()),
            ("pid", process.pid.to_string().into_bytes()),
        ] {
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(directory.join(name))
                .unwrap();
            file.write_all(&bytes).unwrap();
        }
        // A selected binding still rejects this live process identity. Discovery
        // must not connect to it, or let it poison another thread's lookup.
        assert_eq!(
            Owner::load(&directory, &home, &installation).unwrap_err(),
            Error("native_runtime_owner_changed")
        );
        let pool = Pool {
            config: Config::default(),
            state: root.0.join("state"),
            installation,
            home,
            workspace,
            base_url: "http://127.0.0.1:9".into(),
            binary: root.0.join("provider"),
            clients: Mutex::new(BTreeMap::new()),
        };
        assert_eq!(
            pool.for_thread("unrelated-native-thread")
                .await
                .err()
                .unwrap(),
            Error("native_thread_owner_unavailable")
        );
        assert_eq!(Process::read(process.pid).unwrap(), process);
        // Do not turn discovery into a blanket ignore of invalid private state.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            directory.join("pid"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert_eq!(
            pool.for_thread("unrelated-native-thread")
                .await
                .err()
                .unwrap(),
            Error("native_binding_invalid")
        );
    }

    #[test]
    fn known_legacy_lookup_excludes_only_confirmed_nonowners_and_preserves_records() {
        struct Root(PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Root(std::env::temp_dir().join(format!(
            "native-legacy-{}",
            crate::journal::random_id().unwrap()
        )));
        crate::journal::private_directory(&root.0).unwrap();
        let home = root.0.join("home");
        let process = Process::read(std::process::id()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        for suffix in [b"".as_slice(), b"\0catalog-v1"] {
            let owner = [
                home.as_os_str().as_bytes(),
                b"\0/retained/old/runtime",
                suffix,
            ]
            .concat();
            std::fs::write(root.0.join("owner"), &owner).unwrap();
            std::fs::write(root.0.join("pid"), process.pid.to_string()).unwrap();
            for name in ["owner", "pid"] {
                std::fs::set_permissions(root.0.join(name), std::fs::Permissions::from_mode(0o600))
                    .unwrap();
            }
            // This live PID belongs to a different executable; no socket or
            // historical config exists, and neither should be read or repaired.
            assert!(binding_for_home(&root.0, Some(&home)).unwrap().is_none());
            assert_eq!(std::fs::read(root.0.join("owner")).unwrap(), owner);
            assert_eq!(
                std::fs::read_to_string(root.0.join("pid")).unwrap(),
                process.pid.to_string()
            );

            std::fs::write(root.0.join("pid"), u32::MAX.to_string()).unwrap();
            assert!(binding_for_home(&root.0, Some(&home)).unwrap().is_none());

            // An unreaped completed child still has a /proc directory, but
            // cannot own the old socket. Preserve the record and reap our child.
            let mut child = std::process::Command::new("sh")
                .args(["-c", "exit 0"])
                .spawn()
                .unwrap();
            let until = std::time::Instant::now() + Duration::from_secs(5);
            while !confirmed_unavailable_process(child.id()) {
                assert!(std::time::Instant::now() < until);
                std::thread::sleep(Duration::from_millis(5));
            }
            std::fs::write(root.0.join("pid"), child.id().to_string()).unwrap();
            let excluded = binding_for_home(&root.0, None);
            assert!(child.wait().unwrap().success());
            assert!(excluded.unwrap().is_none());
            assert_eq!(std::fs::read(root.0.join("owner")).unwrap(), owner);

            // A private PID read is mandatory even for an inactive old codec.
            std::fs::set_permissions(root.0.join("pid"), std::fs::Permissions::from_mode(0o644))
                .unwrap();
            assert_eq!(
                binding_for_home(&root.0, Some(&home)).unwrap_err(),
                Error("native_binding_invalid")
            );
            std::fs::set_permissions(root.0.join("pid"), std::fs::Permissions::from_mode(0o600))
                .unwrap();

            // The exact matching live executable remains unsupported, never
            // silently ignored or admitted through its historical format.
            std::fs::write(
                root.0.join("owner"),
                [
                    home.as_os_str().as_bytes(),
                    b"\0",
                    process.image.as_os_str().as_bytes(),
                    suffix,
                ]
                .concat(),
            )
            .unwrap();
            std::fs::write(root.0.join("pid"), process.pid.to_string()).unwrap();
            assert_eq!(
                binding_for_home(&root.0, Some(&home)).unwrap_err(),
                Error("native_binding_invalid")
            );
        }
    }

    #[test]
    fn foreign_legacy_records_do_not_block_current_home_but_own_invalid_records_do() {
        struct Root(PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Root(std::env::temp_dir().join(format!(
            "native-record-{}",
            crate::journal::random_id().unwrap()
        )));
        crate::journal::private_directory(&root.0).unwrap();
        let home = root.0.join("selected-home");
        let owner = root.0.join("owner");
        use std::io::Write;
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&owner)
            .unwrap();
        file.write_all(b"/another/home\0/old/runtime\0legacy-v0")
            .unwrap();
        drop(file);
        assert!(binding_for_home(&root.0, Some(&home)).unwrap().is_none());
        std::fs::write(
            &owner,
            [home.as_os_str().as_bytes(), b"\0/runtime\0legacy-v0"].concat(),
        )
        .unwrap();
        assert_eq!(
            binding_for_home(&root.0, Some(&home)).unwrap_err(),
            Error("native_binding_invalid")
        );
        std::fs::remove_file(&owner).unwrap();
        std::os::unix::fs::symlink(root.0.join("missing"), &owner).unwrap();
        assert_eq!(
            binding_for_home(&root.0, Some(&home)).unwrap_err(),
            Error("native_binding_invalid")
        );
    }

    #[test]
    fn only_exact_paged_history_rejection_is_temporarily_unavailable() {
        let reply =
            json!({"error":{"code":-32601,"message":"thread/items/list is not supported yet"}});
        assert_eq!(
            native_reply("thread/items/list", &reply).unwrap_err().0,
            "native_history_not_ready"
        );
        for method in ["turn/start", "thread/read", "thread/resume"] {
            assert_eq!(
                native_reply(method, &reply).unwrap_err().0,
                "native_request_rejected"
            );
        }
    }

    #[test]
    fn other_history_errors_remain_errors_without_disclosing_remote_text() {
        for reply in [
            json!({"error":{"code":-32600,"message":"thread/items/list is not supported yet"}}),
            json!({"error":{"code":-32601,"message":"another unsupported method"}}),
            json!({"error":{"code":-32603,"message":"private remote details"}}),
        ] {
            assert_eq!(
                native_reply("thread/items/list", &reply).unwrap_err().0,
                "native_request_rejected"
            );
        }
    }

    #[test]
    fn successful_empty_history_and_malformed_reply_remain_distinct() {
        let value = json!({"data":[],"nextCursor":null});
        assert_eq!(
            native_reply("thread/items/list", &json!({"result":value})).unwrap(),
            value
        );
        assert_eq!(
            native_reply("thread/items/list", &json!({})).unwrap_err().0,
            "native_reply_invalid"
        );
    }
}

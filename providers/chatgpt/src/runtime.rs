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
    fs::File,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

/// Chat-first work needs no app host. Only a Codex-origin start or independent
/// native reviewer can acquire this explicitly provisioned host on first use.
type LazyHost = Arc<tokio::sync::Mutex<Option<(crate::host::OwnedHost, Arc<Remote>)>>>;
struct LazyRemote {
    config: crate::config::HostConfig,
    mcp: bool,
    root: PathBuf,
    owner: String,
    host: LazyHost,
    viewer: crate::viewer::Slot,
    journal: Arc<Journal>,
    port: u16,
}
impl crate::app::Bridge for LazyRemote {
    fn operation(
        &self,
        command: serde_json::Value,
    ) -> futures_util::future::BoxFuture<'static, Result<serde_json::Value, Error>> {
        let config = self.config.clone();
        let mcp = self.mcp;
        let root = self.root.clone();
        let owner = self.owner.clone();
        let host = self.host.clone();
        let viewer_slot = self.viewer.clone();
        let journal = self.journal.clone();
        let port = self.port;
        Box::pin(async move {
            if matches!(
                command["op"].as_str(),
                Some("host_status" | "host_open" | "host_stop")
            ) {
                let mut guard = host.lock().await;
                if guard.is_none() {
                    return if command["op"] == "host_open" {
                        Err(Error("host_not_acquired"))
                    } else {
                        Ok(
                            json!({"present":false,"ready":false,"host_started":false,"inference_submitted":false,"stopped":command["op"]=="host_stop"}),
                        )
                    };
                }
                if command["op"] == "host_stop" {
                    let remote = guard.as_ref().unwrap().1.clone();
                    let observed = remote.operation(json!({"op":"host_status"})).await?;
                    if !observed["active_lanes"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
                    {
                        return Err(Error("host_active_work"));
                    }
                    // Retain the owner guard throughout teardown; a concurrent
                    // ensure cannot acquire a replacement before Stop settles.
                    guard.as_mut().unwrap().0.shutdown().await;
                    *guard = None;
                    *viewer_slot.write().unwrap() = None;
                    if let Some(mut binding) = journal.read::<Binding>(&key())? {
                        binding.viewer_capability = None;
                        journal.write(&key(), &binding)?;
                    }
                    return Ok(
                        json!({"stopped":true,"scope":"provider_owned_host","inference_submitted":false}),
                    );
                }
                let remote = guard.as_ref().unwrap().1.clone();
                drop(guard);
                return remote.operation(command).await;
            }
            let remote = {
                let mut host = host.lock().await;
                if host.is_none() {
                    if command["op"] == "release" {
                        return Ok(json!({"released":true}));
                    }
                    if matches!(
                        command["op"].as_str(),
                        Some("stop" | "snapshot" | "dispose")
                    ) {
                        return Err(Error("owned_app_round_unavailable"));
                    }
                    let owned = crate::host::OwnedHost::start(&config, &root).await?;
                    let viewer = config.viewer_assets.as_ref().and_then(|assets| {
                        match crate::viewer::Viewer::new(
                            owned.viewer_socket(),
                            assets.clone(),
                            port,
                        ) {
                            Ok(viewer) => Some(viewer),
                            Err(error) => {
                                eprintln!("optional viewer unavailable: {}", error.0);
                                None
                            }
                        }
                    });
                    if let Some(viewer) = &viewer {
                        let mut binding: Binding =
                            journal.read(&key())?.ok_or(Error("provider_not_running"))?;
                        binding.viewer_capability = Some(viewer.capability.clone());
                        journal.write(&key(), &binding)?;
                        *viewer_slot.write().unwrap() = Some(viewer.clone());
                    }
                    let remote = Remote::pipe(owned.client.clone(), owner).with_viewer(viewer);
                    let remote = Arc::new(if mcp { remote.with_mcp() } else { remote });
                    *host = Some((owned, remote));
                }
                host.as_ref().unwrap().1.clone()
            };
            remote.operation(command).await
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    schema: String,
    process: Process,
    port: u16,
    token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    viewer_capability: Option<String>,
}
fn key() -> String {
    digest(&"termux-codex-responses-service-v1")
}
/// Publish the current authenticated listener for local CLI discovery. The
/// listener owner supplies the same token used by its service readiness proof.
pub fn publish_listener(
    journal: &Journal,
    port: u16,
    token: String,
    viewer_capability: Option<String>,
) -> Result<(), Error> {
    let binding = Binding {
        schema: "termux-codex-responses-service-v1".into(),
        process: Process::read(std::process::id())?,
        port,
        token,
        viewer_capability,
    };
    validate_binding(&binding)?;
    journal.write(&key(), &binding)
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
    validate_binding(&value)?;
    value.process.validate()?;
    Ok(value)
}

fn validate_binding(value: &Binding) -> Result<(), Error> {
    if value.schema != "termux-codex-responses-service-v1"
        || value.port == 0
        || value.token.len() != 48
        || !value.token.bytes().all(|b| b.is_ascii_hexdigit())
        || value
            .viewer_capability
            .as_ref()
            .is_some_and(|v| v.len() != 48 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(Error("invalid_service_record"));
    }
    Ok(())
}

pub async fn token(root: &Path) -> Result<String, Error> {
    Ok(verified_binding(root).await?.token)
}

async fn verified_binding(root: &Path) -> Result<Binding, Error> {
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
    Ok(selected)
}

/// Authentication owns lazy process startup, never inference or request replay.
/// Keep `token` itself read-only for readiness/status and internal proof.
pub async fn ensure_token(root: &Path) -> Result<String, Error> {
    Ok(ensure_binding(root).await?.token)
}

async fn ensure_binding(root: &Path) -> Result<Binding, Error> {
    if !root.is_absolute() {
        return Err(Error("absolute_state_directory_required"));
    }
    if let Ok(value) = verified_binding(root).await {
        return Ok(value);
    }
    let path = store::config_path().map_err(|_| Error("invalid_configuration_path"))?;
    let settings = store::Snapshot::open(&path)
        .and_then(|v| v.config())
        .map_err(|_| Error("invalid_provider_configuration"))?;
    let until = tokio::time::Instant::now() + Duration::from_secs(45);
    if settings.service.startup == crate::config::ServiceStartup::Supervised {
        loop {
            match verified_binding(root).await {
                Ok(value) => return Ok(value),
                Err(Error(
                    "provider_not_running"
                    | "stale_process_owner"
                    | "provider_unavailable"
                    | "readiness_timeout",
                )) => (),
                Err(Error("state_directory_unavailable"))
                    if std::fs::symlink_metadata(root)
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) => {}
                Err(Error("private_file_unavailable"))
                    if std::fs::symlink_metadata(root.join("owner-v1"))
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound) => {}
                Err(error) => return Err(error),
            }
            if tokio::time::Instant::now() >= until {
                return Err(Error("provider_supervised_start_timeout"));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    crate::journal::private_directory(root)?;
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join("startup.lock"))
        .map_err(|_| Error("provider_start_lock_unavailable"))?;
    private_regular(&lock)?;
    loop {
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            break;
        }
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EWOULDBLOCK) {
            return Err(Error("provider_start_lock_unavailable"));
        }
        if tokio::time::Instant::now() >= until {
            return Err(Error("provider_start_timeout"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if let Ok(value) = verified_binding(root).await {
        return Ok(value);
    }
    {
        // A failed readiness check is never authority to restart a live owner.
        // The normal writer lease also covers startup before publication.
        let journal = Journal::open(root)?;
        if let Some(previous) = journal.read::<Binding>(&key())? {
            validate_binding(&previous)?;
            if previous.process.validate().is_ok() {
                return Err(Error("provider_live_but_unavailable"));
            }
        }
        if settings.host.rootfs.is_none()
            && !(settings.execution.core.is_some()
                && settings.execution.home.is_some()
                && settings.execution.workspace.is_some())
        {
            return Err(Error("host_rootfs_required"));
        }
    }
    let log = File::options()
        .append(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join("startup.log"))
        .map_err(|_| Error("provider_start_log_unavailable"))?;
    private_regular(&log)?;
    let mut command = Command::new(
        std::env::current_exe().map_err(|_| Error("provider_executable_unavailable"))?,
    );
    command
        .arg("serve")
        .arg("--state")
        .arg(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log));
    use std::os::unix::process::CommandExt;
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = StartupChild(Some(
        command
            .spawn()
            .map_err(|_| Error("provider_start_failed"))?,
    ));
    loop {
        if let Ok(value) = verified_binding(root).await {
            // The service outlives this short-lived native authentication command.
            // No separate long-lived launcher or automatic inference is needed.
            child.0.take();
            return Ok(value);
        }
        if child
            .0
            .as_mut()
            .unwrap()
            .try_wait()
            .ok()
            .flatten()
            .is_some()
        {
            return Err(Error("provider_start_failed"));
        }
        if tokio::time::Instant::now() >= until {
            return Err(Error("provider_start_timeout"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn private_regular(file: &File) -> Result<(), Error> {
    let m = file
        .metadata()
        .map_err(|_| Error("private_file_required"))?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o600 {
        return Err(Error("private_file_required"));
    }
    Ok(())
}
struct StartupChild(Option<std::process::Child>);
impl Drop for StartupChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            // Prefer owned graceful teardown; only this unreaped child may be
            // escalated on a failed bootstrap, never an existing service.
            match child.try_wait() {
                Ok(None) => (),
                _ => return,
            }
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
            let until = std::time::Instant::now() + Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(None) => (),
                    _ => return,
                }
                if std::time::Instant::now() >= until {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub async fn status(root: &Path) -> Result<serde_json::Value, Error> {
    let selected = verified_binding(root).await?;

    Ok(
        json!({"schema":"termux-codex-provider-status-v1","live":true,"base_url":format!("http://127.0.0.1:{}/v1",selected.port),"process":selected.process,"meaning":"authenticated local endpoint; model/task progress is reported separately"}),
    )
}

pub async fn thread_status(root: &Path, thread: &str) -> Result<serde_json::Value, Error> {
    let lane = crate::app::inspect_lane(root, thread)?;
    let endpoint = status(root).await;
    Ok(json!({"schema":"termux-codex-provider-thread-status-v1",
        "endpoint_live":endpoint.is_ok(),"endpoint_error":endpoint.err().map(|e|e.0),
        "lane":lane,"remote_current_state":"not_queried",
        "inference_submitted":false,"service_started":false}))
}

pub async fn stop(root: &Path) -> Result<serde_json::Value, Error> {
    use std::os::fd::FromRawFd;
    let selected = binding(root)?;
    // Pin the process first, then repeat the full start-time/executable check.
    // No numeric-PID signal fallback on kernels without this capability.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, selected.process.pid, 0) };
    if fd < 0 {
        return Err(Error("owned_process_signal_unavailable"));
    }
    let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd as i32) };
    selected.process.validate()?;
    let sent = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            libc::SIGTERM,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    if sent != 0 {
        return Err(Error("provider_stop_request_failed"));
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        // Losing /proc or executable-path identity after signalling is not an
        // exit acknowledgment. Only the pinned kernel handle proves this owner
        // exited, including if its image was replaced during graceful teardown.
        let mut poll = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut poll, 1, 0) };
        if result > 0 && poll.revents & libc::POLLIN != 0 {
            break;
        }
        if (result < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted)
            || (result > 0 && poll.revents != 0)
        {
            return Err(Error("provider_stop_unconfirmed"));
        }
        if tokio::time::Instant::now() >= until {
            return Err(Error("provider_stop_unconfirmed"));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(
        json!({"schema":"termux-codex-provider-stop-v1","stopped":true,
        "forced_service_termination":false,"service_signal":"SIGTERM","journal_and_profile_preserved":true,
        "native_tool_effects_undone":false,"inference_submitted":false}),
    )
}

pub async fn reveal(root: &Path, thread: &str) -> Result<serde_json::Value, Error> {
    app_control(root, thread, "reveal").await
}

pub async fn approve(root: &Path, thread: &str, ui: bool) -> Result<serde_json::Value, Error> {
    app_control(root, thread, if ui { "approve_ui" } else { "approve" }).await
}

async fn app_control(
    root: &Path,
    thread: &str,
    operation: &str,
) -> Result<serde_json::Value, Error> {
    let approval = operation != "reveal";
    let unconfirmed = if approval {
        "app_approval_unconfirmed"
    } else {
        "app_reveal_unconfirmed"
    };
    if thread.is_empty() || thread.len() > 256 || !thread.bytes().all(|c| c.is_ascii_graphic()) {
        return Err(Error("invalid_thread_identity"));
    }
    let selected = verified_binding(root).await?; // never bootstrap for app controls
    tokio::time::timeout(Duration::from_secs(if operation == "approve_ui" {55} else if approval {40} else {12}),async {
        let mut socket=tokio::net::TcpStream::connect(("127.0.0.1",selected.port)).await.map_err(|_|Error("provider_unavailable"))?;
        socket.write_all(format!("POST /__{operation} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\nAuthorization: Bearer {}\r\nX-Codex-Thread-Id: {thread}\r\n\r\n",selected.token).as_bytes()).await.map_err(|_|Error(unconfirmed))?;
        let mut bytes=Vec::new();socket.take(8193).read_to_end(&mut bytes).await.map_err(|_|Error(unconfirmed))?;
        if bytes.len()>8192 {return Err(Error(unconfirmed));}
        let text=String::from_utf8(bytes).map_err(|_|Error(unconfirmed))?;
        let (headers,body)=text.split_once("\r\n\r\n").ok_or(Error(unconfirmed))?;
        let value:serde_json::Value=serde_json::from_str(body).map_err(|_|Error(unconfirmed))?;
        if !headers.starts_with("HTTP/1.1 200 ") {
            return Err(Error(match value["error"]["code"].as_str() {
                Some("app_approval_ui_required")=>"app_approval_ui_required",
                Some("app_approval_ui_unavailable")=>"app_approval_ui_unavailable",
                Some("app_approval_viewer_unavailable")=>"app_approval_viewer_unavailable",
                Some("app_approval_manual_required")=>"app_approval_manual_required",
                Some("app_approval_ambiguous")=>"app_approval_ambiguous",
                Some("app_approval_unavailable")=>"app_approval_unavailable",
                Some("app_account_changed")=>"app_account_changed",
                Some("operator_work_not_running")=>"operator_work_not_running",
                _=>unconfirmed,
            }));
        }
        if value[if approval { "app_approval_requested" } else { "app_reveal_requested" }]!=true {return Err(Error(unconfirmed));}
        Ok(value)
    }).await.map_err(|_|Error(unconfirmed))?
}

pub async fn view(root: &Path) -> Result<serde_json::Value, Error> {
    if binding(root).is_err() {
        let path = store::config_path().map_err(|_| Error("invalid_configuration_path"))?;
        let settings = store::Snapshot::open(&path)
            .and_then(|v| v.config())
            .map_err(|_| Error("invalid_provider_configuration"))?;
        if settings.host.viewer_assets.is_none() {
            return Err(Error("viewer_assets_required"));
        }
    }
    let selected = ensure_binding(root).await?;
    let capability = selected
        .viewer_capability
        .ok_or(Error("provider_viewer_unavailable"))?;
    let url=format!("http://127.0.0.1:{}/viewer/{capability}/vnc.html?autoconnect=1&resize=remote&path=viewer%2F{capability}%2Fwebsock",selected.port);
    let prefix = PathBuf::from(std::env::var_os("PREFIX").ok_or(Error("termux_prefix_required"))?);
    request_viewer_activity(&prefix.join("bin/termux-am-socket"), &url).await?;
    Ok(
        json!({"viewer_open_requested":true,"physical_ui_confirmed":false,
        "url_exported":false,"inference_submitted":false}),
    )
}
pub async fn mcp_stdio(root: &Path) -> Result<(), Error> {
    if !root.is_absolute() {
        return Err(Error("absolute_state_directory_required"));
    }
    let root = root.to_path_buf();
    crate::mcp_stdio::run(tokio::io::stdin(), tokio::io::stdout(), move |row| {
        let root = root.clone();
        async move {
            let selected = verified_binding(&root).await?;
            crate::mcp_stdio::forward(selected.port, &row, Some(&selected.token)).await
        }
    })
    .await
}
/// CLI and remote MCP use the same admitted tool call. This only observes an
/// existing service and never bootstraps it or retries an uncertain mutation.
pub async fn control(
    root: &Path,
    tool: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let selected = verified_binding(root).await?;
    if tool == "codex_sessions" && args["action"] == "synchronize" {
        let list = json!({"jsonrpc":"2.0","id":random_id()?,"method":"tools/list","params":{}});
        let listed = tokio::time::timeout(
            Duration::from_secs(3),
            crate::mcp_stdio::forward(selected.port, &list, Some(&selected.token)),
        )
        .await
        .map_err(|_| Error("mcp_observer_timeout"))??
        .ok_or(Error("mcp_response_invalid"))?;
        let tools = listed["result"]["tools"]
            .as_array()
            .ok_or(Error("mcp_response_invalid"))?;
        let sessions = tools
            .iter()
            .find(|tool| tool["name"] == "codex_sessions")
            .ok_or(Error("mcp_response_invalid"))?;
        let actions = sessions["inputSchema"]["properties"]["action"]["enum"]
            .as_array()
            .ok_or(Error("mcp_response_invalid"))?;
        if !actions.iter().any(|action| action == "synchronize") {
            return Ok(
                json!({"is_error":false,"result":{"thread_id":args["thread_id"],"status":"legacy","inference_submitted":false}}),
            );
        }
    }
    let row = json!({"jsonrpc":"2.0","id":random_id()?,"method":"tools/call","params":{"name":tool,"arguments":args}});
    let response = tokio::time::timeout(
        Duration::from_secs(610),
        crate::mcp_stdio::forward(selected.port, &row, Some(&selected.token)),
    )
    .await
    .map_err(|_| Error("mcp_observer_timeout"))??
    .ok_or(Error("mcp_response_invalid"))?;
    if response.get("error").is_some() {
        return Err(Error("mcp_request_rejected"));
    }
    let value: serde_json::Value = serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or(Error("mcp_response_invalid"))?,
    )
    .map_err(|_| Error("mcp_response_invalid"))?;
    Ok(json!({"is_error":response["result"]["isError"],"result":value}))
}
/// A local filesystem owner can use a stable work handle without displaying its
/// generation capability. Remote readers never gain this filesystem facade.
pub fn work_capability(root: &Path, work: &str) -> Result<String, Error> {
    if work.len() != 48 || !work.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error("operator_work_unavailable"));
    }
    let record = Journal::inspect(root, &digest(&("operator-work-v1", work)))?
        .ok_or(Error("operator_work_unavailable"))?;
    if record["id"] != work {
        return Err(Error("operator_record_invalid"));
    }
    record["capability"]
        .as_str()
        .filter(|value| value.len() == 48 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_owned)
        .ok_or(Error("operator_record_invalid"))
}
pub(crate) async fn request_viewer_activity(program: &Path, url: &str) -> Result<(), Error> {
    let mut command = tokio::process::Command::new(program);
    command
        .arg(format!("start -a android.intent.action.VIEW -d {url}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| Error("viewer_open_unavailable"))?;
    async fn read(pipe: impl AsyncReadExt + Unpin) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        pipe.take(8193).read_to_end(&mut bytes).await?;
        if bytes.len() > 8192 {
            return Err(std::io::Error::other("activity response too large"));
        }
        Ok(bytes)
    }
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let output = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::try_join!(read(stdout), read(stderr), child.wait())
    })
    .await
    .map_err(|_| Error("viewer_open_unconfirmed"))?
    .map_err(|_| Error("viewer_open_unconfirmed"))?;
    if !output.2.success()
        || [&output.0, &output.1]
            .iter()
            .any(|v| String::from_utf8_lossy(v).contains("Error:"))
    {
        return Err(Error("viewer_open_unconfirmed"));
    }
    Ok(())
}

pub async fn serve(descriptor: Option<PathBuf>, root: PathBuf, port: u16) -> Result<(), Error> {
    let config = store::config_path().map_err(|_| Error("invalid_configuration_path"))?;
    let settings = store::Snapshot::open(&config)
        .and_then(|v| v.config())
        .map_err(|_| Error("invalid_provider_configuration"))?;
    if let Some(path) = &descriptor {
        Descriptor::read(path)?;
    }
    let journal = Arc::new(Journal::open(&root)?);
    // A generated upstream profile retains its endpoint across service restarts.
    // Reclaim only our journal's previously published port; never select a new
    // random endpoint when that address is now occupied by another listener.
    let previous = journal.read::<Binding>(&key())?;
    if let Some(value) = &previous {
        validate_binding(value)?;
    }
    let selected_port = if port == 0 {
        previous.as_ref().map_or(0, |value| value.port)
    } else {
        port
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", selected_port))
        .await
        .map_err(|_| Error("provider_port_unavailable"))?;
    let port = listener
        .local_addr()
        .map_err(|_| Error("provider_port_unavailable"))?
        .port();
    let native_controls = settings.execution.core.is_some()
        && settings.execution.home.is_some()
        && settings.execution.workspace.is_some();
    let lazy_host = Arc::new(tokio::sync::Mutex::new(None));
    let viewer_slot: crate::viewer::Slot = Arc::new(std::sync::RwLock::new(None));
    let bridge: Arc<dyn crate::app::Bridge> = if let Some(path) = descriptor {
        if let (Some(socket), Some(assets)) =
            (&settings.host.viewer_socket, &settings.host.viewer_assets)
        {
            match crate::viewer::Viewer::new(socket.clone(), assets.clone(), port) {
                Ok(viewer) => *viewer_slot.write().unwrap() = Some(viewer),
                Err(error) => eprintln!("optional viewer unavailable: {}", error.0),
            }
        }
        let remote = Remote::new(path, journal.owner.clone())
            .with_viewer(viewer_slot.read().unwrap().clone());
        Arc::new(
            if settings.transport.return_path == crate::config::ReturnPath::Mcp {
                remote.with_mcp()
            } else {
                remote
            },
        )
    } else {
        Arc::new(LazyRemote {
            config: settings.host.clone(),
            mcp: settings.transport.return_path == crate::config::ReturnPath::Mcp,
            root: root.clone(),
            owner: journal.owner.clone(),
            host: lazy_host.clone(),
            viewer: viewer_slot.clone(),
            journal: journal.clone(),
            port,
        })
    };
    let mcp = (!native_controls).then(|| {
        crate::mcp::Broker::with_controls(journal.clone(), bridge.clone(), settings.clone())
    });
    let bridge: Arc<dyn crate::app::Bridge> = match &mcp {
        Some(broker) if settings.transport.return_path == crate::config::ReturnPath::Mcp => {
            Arc::new(crate::mcp_bridge::McpBridge::new(
                bridge,
                broker.clone(),
                journal.clone(),
                settings.transport.connector_name.clone(),
            )?)
        }
        _ => bridge,
    };
    let model_bridge = bridge.clone();
    let token = random_id()?;
    let host = Arc::new(
        AppHost::new(bridge.clone(), journal.clone())
            .with_connector(settings.transport.connector_name.clone()),
    );
    let mut service = if native_controls {
        let pool = crate::operator_native::Pool::new(
            settings.clone(),
            root.clone(),
            format!("http://127.0.0.1:{port}/v1"),
            std::env::current_exe().map_err(|_| Error("provider_program_unavailable"))?,
        )?;
        let operator = crate::operator::Operator::new(
            pool,
            crate::mcp::Broker::new(journal.clone()),
            journal.clone(),
            settings.clone(),
            bridge,
            token.clone(),
        )?;
        Service::configured(config, token.clone(), operator.clone())?.with_operator(operator)?
    } else {
        Service::configured(config, token.clone(), host.clone())?
    };
    service = service.with_app_models(model_bridge, journal.clone())?;
    service = service.with_handoff_issuer(crate::handoff::Issuer::new(journal.owner.clone()))?;
    if let Some(mcp) = mcp {
        service = service.with_mcp(mcp)?;
    }
    let viewer_capability = viewer_slot
        .read()
        .unwrap()
        .as_ref()
        .map(|v| v.capability.clone());
    service = service.with_viewer_slot(viewer_slot)?;
    publish_listener(&journal, port, token, viewer_capability)?;
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
    let result = server::serve(listener, service, shutdown)
        .await
        .map_err(|_| Error("provider_server_failed"));
    host.shutdown().await;
    if let Some((host, _)) = &mut *lazy_host.lock().await {
        host.shutdown().await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn missing_lane_observation_and_stop_never_acquire_a_host() {
        use crate::app::Bridge;
        struct Root(PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Root(std::env::temp_dir().join(format!("ls-{}", random_id().unwrap())));
        let journal = Arc::new(Journal::open(&root.0).unwrap());
        let host = Arc::new(tokio::sync::Mutex::new(None));
        let remote = LazyRemote {
            config: Default::default(),
            mcp: true,
            root: root.0.clone(),
            owner: journal.owner.clone(),
            host: host.clone(),
            viewer: Arc::new(std::sync::RwLock::new(None)),
            journal,
            port: 0,
        };
        assert_eq!(
            remote.operation(json!({"op":"host_status"})).await.unwrap()["host_started"],
            false
        );
        assert_eq!(
            remote
                .operation(json!({"op":"release","lane":"unavailable-owned-lane"}))
                .await
                .unwrap(),
            json!({"released":true})
        );
        for op in ["snapshot", "stop", "dispose"] {
            assert_eq!(
                remote
                    .operation(json!({"op":op,"lane":"unavailable-owned-lane"}))
                    .await,
                Err(Error("owned_app_round_unavailable"))
            );
            assert!(host.lock().await.is_none());
            assert!(!root.0.join("desktop").exists());
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "explicit Android rootfs/viewer assets; empty disposable profile, read-only identity probe, zero Chat"]
    async fn lazy_operator_viewer_registers_after_host_start_without_chat() {
        use crate::app::Bridge;
        use std::future::IntoFuture;
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = Cleanup(std::env::temp_dir().join(format!("lv-{}", &random_id().unwrap()[..8])));
        let journal = Arc::new(Journal::open(&root.0).unwrap());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        journal
            .write(
                &key(),
                &Binding {
                    schema: "termux-codex-responses-service-v1".into(),
                    process: Process::read(std::process::id()).unwrap(),
                    port,
                    token: "a".repeat(48),
                    viewer_capability: None,
                },
            )
            .unwrap();
        let slot: crate::viewer::Slot = Arc::new(std::sync::RwLock::new(None));
        let host = Arc::new(tokio::sync::Mutex::new(None));
        let remote = LazyRemote {
            mcp: true,
            config: crate::config::HostConfig {
                rootfs: Some(
                    std::env::var_os("CHATGPT_PROVIDER_ROOTFS")
                        .expect("qualified rootfs")
                        .into(),
                ),
                viewer_assets: Some(
                    std::env::var_os("CHATGPT_PROVIDER_VIEWER_ASSETS")
                        .expect("qualified assets")
                        .into(),
                ),
                ..Default::default()
            },
            root: root.0.clone(),
            owner: journal.owner.clone(),
            host: host.clone(),
            viewer: slot.clone(),
            journal: journal.clone(),
            port,
        };
        let server = tokio::spawn(
            axum::serve(listener, crate::viewer::router_slot(slot.clone())).into_future(),
        );
        assert!(slot.read().unwrap().is_none());
        assert!(!root.0.join("desktop").exists());
        let probe = tokio::spawn(async move {
            remote
                .operation(json!({"op":"identity","lane":"readonly-probe"}))
                .await
        });
        let until = tokio::time::Instant::now() + Duration::from_secs(45);
        let capability = loop {
            if let Some(viewer) = &*slot.read().unwrap() {
                break viewer.capability.clone();
            }
            assert!(!probe.is_finished(), "read-only host startup failed");
            assert!(
                tokio::time::Instant::now() < until,
                "lazy viewer startup timeout"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        let selected: Binding = journal.read(&key()).unwrap().unwrap();
        assert_eq!(
            selected.viewer_capability.as_deref(),
            Some(capability.as_str())
        );
        let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        socket.write_all(format!("GET /viewer/{capability}/vnc.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).await.unwrap();
        assert!(bytes.starts_with(b"HTTP/1.1 200"));
        probe.abort();
        let _ = probe.await;
        if let Some((mut owned, _)) = host.lock().await.take() {
            owned.shutdown().await;
        }
        if let Some(viewer) = &*slot.read().unwrap() {
            viewer.close();
        }
        server.abort();
        let _ = server.await;
    }
    #[tokio::test]
    async fn viewer_activity_errors_remain_bounded_static_and_do_not_export_the_url() {
        use std::os::unix::fs::PermissionsExt;
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root =
            Cleanup(std::env::temp_dir().join(format!("viewer-activity-{}", random_id().unwrap())));
        std::fs::create_dir(&root.0).unwrap();
        let program = root.0.join("am");
        let shell = std::env::var_os("PREFIX")
            .map(|v| PathBuf::from(v).join("bin/sh"))
            .unwrap_or_else(|| PathBuf::from("/bin/sh"));
        let url = "http://127.0.0.1:1234/private-fixture-capability";
        std::fs::write(
            &program,
            format!(
                "#!{}\nprintf '%s\\n' \"$1\"\nprintf 'Error: fixture activity rejected\\n'\n",
                shell.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            request_viewer_activity(&program, url).await,
            Err(Error("viewer_open_unconfirmed"))
        );
        std::fs::write(
            &program,
            format!(
                "#!{}\n[ \"$#\" -eq 1 ] || exit 1\nprintf 'Starting: owned activity\\n'\n",
                shell.display()
            ),
        )
        .unwrap();
        assert_eq!(request_viewer_activity(&program, url).await, Ok(()));
    }
}

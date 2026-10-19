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

struct Service(std::process::Child);
impl Service {
    fn start(root: &Root) -> (Self, Value) {
        Self::start_from(
            root,
            std::path::Path::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider")),
        )
    }
    fn start_from(root: &Root, binary: &std::path::Path) -> (Self, Value) {
        let mut child = Self(
            Command::new(binary)
                .args(["serve", "--descriptor"])
                .arg(root.0.join("descriptor.json"))
                .arg("--state")
                .arg(root.0.join("state"))
                .env("HOME", &root.0)
                .env("XDG_CONFIG_HOME", root.0.join("config"))
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut line = String::new();
        BufReader::new(child.0.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let value = serde_json::from_str(&line).expect("service must publish a listener");
        (child, value)
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0.id() as i32, libc::SIGTERM);
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while self.0.try_wait().unwrap().is_none() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if self.0.try_wait().unwrap().is_none() {
            self.0.kill().unwrap();
        }
        self.0.wait().unwrap();
    }
}

#[test]
fn reveal_never_bootstraps_a_missing_service_or_creates_state() {
    let root = Root::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
        .args(["reveal", "--thread", "native-thread", "--state"])
        .arg(root.0.join("missing-state"))
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", root.0.join("config"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        !root.0.exists(),
        "navigation is not a startup or configuration operation"
    );
}

#[test]
fn immutable_service_snapshot_survives_build_replacement_while_mutable_owner_refuses_reentry() {
    let root = Root::new();
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let process = Process::read(std::process::id()).unwrap();
    let descriptor = root.0.join("descriptor.json");
    std::fs::write(&descriptor, json!({"version":1,"kind":"termux-electron-pipe","endpoint":"http://127.0.0.1:9/","token":"a".repeat(43),"host":process,"browser":process}).to_string()).unwrap();
    std::fs::set_permissions(&descriptor, std::fs::Permissions::from_mode(0o600)).unwrap();
    let build = root.0.join("build-provider");
    let snapshot = root.0.join("immutable-provider");
    std::fs::copy(env!("CARGO_BIN_EXE_codex-chatgpt-provider"), &build).unwrap();
    std::fs::copy(&build, &snapshot).unwrap();
    let replace_build = || {
        let next = root.0.join("next-build");
        std::fs::copy(&snapshot, &next).unwrap();
        std::fs::rename(next, &build).unwrap();
    };
    let command = |operation: &str| {
        Command::new(&snapshot)
            .arg(operation)
            .arg("--state")
            .arg(root.0.join("state"))
            .env("HOME", &root.0)
            .env("XDG_CONFIG_HOME", root.0.join("config"))
            .output()
            .unwrap()
    };
    let (service, _) = Service::start_from(&root, &snapshot);
    replace_build();
    assert!(command("status").status.success());
    assert!(command("token").status.success());
    drop(service);

    let (mut service, _) = Service::start_from(&root, &build);
    let before = Process::read(service.0.id()).unwrap();
    replace_build();
    assert!(
        before.validate().is_err(),
        "an unlinked executable is not an unchanged owner"
    );
    let status = command("status");
    assert!(!status.status.success());
    assert!(String::from_utf8_lossy(&status.stderr).contains("stale_process_owner"));
    let token = command("token");
    assert!(!token.status.success());
    assert!(
        token.stdout.is_empty(),
        "failed admission must not export a credential"
    );
    assert!(
        service.0.try_wait().unwrap().is_none(),
        "failed auth must not restart or stop the existing writer"
    );
    assert!(!root.0.join("state/desktop").exists());
    drop(service);
}

#[test]
fn restart_retains_registered_endpoint_rotates_credentials_and_refuses_port_collision() {
    let root = Root::new();
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let descriptor = root.0.join("descriptor.json");
    let process = Process::read(std::process::id()).unwrap();
    std::fs::write(&descriptor,json!({"version":1,"kind":"termux-electron-pipe","endpoint":"http://127.0.0.1:9/","token":"a".repeat(43),"host":process,"browser":process}).to_string()).unwrap();
    std::fs::set_permissions(&descriptor, std::fs::Permissions::from_mode(0o600)).unwrap();
    let state = root.0.join("state");
    let binary = env!("CARGO_BIN_EXE_codex-chatgpt-provider");
    let credential = || {
        let output = Command::new(binary)
            .arg("token")
            .arg("--state")
            .arg(&state)
            .output()
            .unwrap();
        assert!(output.status.success());
        output.stdout
    };
    let (first, original) = Service::start(&root);
    let old_token = credential();
    let url = original["base_url"].as_str().unwrap();
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
    assert!(prepared.status.success());
    let profile = std::fs::read(home.join("chatgpt.config.toml")).unwrap();
    drop(first);
    let (second, restarted) = Service::start(&root);
    assert_eq!(restarted["base_url"], url);
    assert!(
        credential() != old_token,
        "a restart must rotate the credential"
    );
    assert_eq!(
        std::fs::read(home.join("chatgpt.config.toml")).unwrap(),
        profile
    );
    let fake_state = root.0.join("untrusted-service-record");
    let journal = Journal::open(&fake_state).unwrap();
    let mut wrong = Process::read(second.0.id()).unwrap();
    wrong.start.push('0');
    journal
        .write(
            &digest(&"termux-codex-responses-service-v1"),
            &json!({"schema":"termux-codex-responses-service-v1","process":wrong,
            "port":1,"token":"c".repeat(48)}),
        )
        .unwrap();
    let refused = Command::new(binary)
        .arg("stop")
        .arg("--state")
        .arg(&fake_state)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("stale_process_owner"));
    assert!(
        credential().len() == 49,
        "a mismatched record must not stop the real owner"
    );
    let stopped = Command::new(binary)
        .arg("stop")
        .arg("--state")
        .arg(&state)
        .output()
        .unwrap();
    assert!(
        stopped.status.success(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    let stopped: Value = serde_json::from_slice(&stopped.stdout).unwrap();
    assert_eq!(stopped["stopped"], true);
    assert_eq!(stopped["forced_service_termination"], false);
    assert_eq!(
        std::fs::read(home.join("chatgpt.config.toml")).unwrap(),
        profile
    );
    assert!(state.join("owner-v1").is_file());
    drop(second);

    let port = url
        .strip_prefix("http://127.0.0.1:")
        .unwrap()
        .strip_suffix("/v1")
        .unwrap()
        .parse::<u16>()
        .unwrap();
    let collision = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    let record = state.join(format!(
        "{}.json",
        digest(&"termux-codex-responses-service-v1")
    ));
    let before = std::fs::read(&record).unwrap();
    let failed = Command::new(binary)
        .arg("serve")
        .arg("--state")
        .arg(&state)
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", root.0.join("config"))
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("provider_port_unavailable"));
    assert_eq!(std::fs::read(&record).unwrap(), before);
    assert!(
        !state.join("desktop").exists(),
        "collision must precede app startup"
    );
    drop(collision);
    let (last, listener) = Service::start(&root);
    assert_eq!(listener["base_url"], url);
    drop(last);
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
async fn stop_requires_kernel_exit_confirmation_after_executable_identity_changes() {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let root = Root::new();
    std::fs::create_dir(&root.0).unwrap();
    let image = root.0.join("owned-shell");
    let shell = std::env::var_os("PREFIX")
        .map(PathBuf::from)
        .map(|p| p.join("bin/sh"))
        .unwrap_or_else(|| PathBuf::from("/bin/sh"));
    std::fs::copy(shell, &image).unwrap();
    // Builtins only: no descendant jobs. Keep stdin open so the owned target
    // survives SIGTERM, acknowledges it, then waits for another line.
    let mut target = tokio::process::Command::new(&image)
        .args([
            "-c",
            "trap 'printf \"TERM_RECEIVED\\n\"' TERM; printf 'READY\\n'; while :; do read -r line; done",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(target.stdout.take().unwrap()).lines();
    assert_eq!(stdout.next_line().await.unwrap().unwrap(), "READY");
    let process = Process::read(target.id().unwrap()).unwrap();
    let state = root.0.join("state");
    let journal = Journal::open(&state).unwrap();
    journal
        .write(
            &digest(&"termux-codex-responses-service-v1"),
            &json!({"schema":"termux-codex-responses-service-v1","process":process,
                "port":1,"token":"b".repeat(48)}),
        )
        .unwrap();
    let before = std::fs::read_dir(&state)
        .unwrap()
        .map(|p| {
            let path = p.unwrap().path();
            (path.clone(), std::fs::read(path).unwrap())
        })
        .collect::<Vec<_>>();
    let stop = tokio::process::Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
        .arg("stop")
        .arg("--state")
        .arg(&state)
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        let changed = async {
            assert_eq!(stdout.next_line().await.unwrap().unwrap(), "TERM_RECEIVED");
            std::fs::remove_file(&image).unwrap();
            assert!(process.validate().is_err());
            assert!(target.try_wait().unwrap().is_none());
        };
        let (output, ()) = tokio::join!(stop, changed);
        output.unwrap()
    })
    .await
    .expect("stop acknowledgment must remain bounded");
    assert!(target.try_wait().unwrap().is_none());
    target.kill().await.unwrap();
    target.wait().await.unwrap();
    assert!(
        !output.status.success(),
        "a live process with an unlinked image must not be reported stopped"
    );
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("provider_stop_unconfirmed"));
    for (path, bytes) in before {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
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
    assert!(runtime::ensure_token(&root.0).await.is_err());
    assert!(!root.0.join("startup.log").exists());
    assert!(!root.0.join("desktop").exists());
}

#[test]
fn unconfigured_authentication_never_installs_an_app_or_changes_native_configuration() {
    let root = Root::new();
    let output = Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
        .arg("token")
        .arg("--state")
        .arg(root.0.join("state"))
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", root.0.join("config"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("host_rootfs_required"));
    assert!(!root.0.join("state/desktop").exists());
    assert!(!root.0.join("config").exists());
    assert!(!root.0.join(".codex").exists());
    let viewer = Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
        .arg("view")
        .arg("--state")
        .arg(root.0.join("view-state"))
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", root.0.join("config"))
        .output()
        .unwrap();
    assert!(!viewer.status.success());
    assert!(String::from_utf8_lossy(&viewer.stderr).contains("viewer_assets_required"));
    assert!(
        !root.0.join("view-state").exists(),
        "unconfigured viewing must not start an app or service"
    );
    let stdio = Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
        .arg("mcp-stdio")
        .arg("--state")
        .arg(root.0.join("mcp-state"))
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", root.0.join("config"))
        .output()
        .unwrap();
    assert!(!stdio.status.success());
    assert!(stdio.stdout.is_empty());
    assert!(
        !root.0.join("mcp-state").exists(),
        "stdio never starts an app or service"
    );
}

#[tokio::test]
async fn operator_bootstrap_and_authenticated_stdio_need_no_desktop_or_core_launch() {
    use codex_chatgpt_provider::config::{Config, ExecutionConfig, ExecutionMode, ReturnPath};
    use tokio::io::{AsyncBufReadExt, BufReader as AsyncReader};
    let root = Root::new();
    std::fs::create_dir(&root.0).unwrap();
    std::fs::set_permissions(&root.0, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut config = Config::default();
    config.transport.return_path = ReturnPath::Mcp;
    config.execution = ExecutionConfig {
        mode: ExecutionMode::Operator,
        core: Some(root.0.join("not-launched-core")),
        home: Some(root.0.join("not-opened-home")),
        workspace: Some(root.0.join("not-opened-workspace")),
    };
    assert!(config.host.rootfs.is_none());
    let path = root.0.join("config/codex/providers/chatgpt.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, config.render().unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut service = Service(
        Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"))
            .args(["serve", "--state"])
            .arg(root.0.join("state"))
            .env("HOME", &root.0)
            .env("XDG_CONFIG_HOME", root.0.join("config"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut line = String::new();
    BufReader::new(service.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let ready: Value =
        serde_json::from_str(&line).expect("operator must publish without a desktop rootfs");
    assert_eq!(ready["app_connected"], false);
    let state = root.0.join("state");
    let token = runtime::token(&state).await.unwrap();
    let port = ready["base_url"]
        .as_str()
        .unwrap()
        .strip_prefix("http://127.0.0.1:")
        .unwrap()
        .strip_suffix("/v1")
        .unwrap()
        .parse()
        .unwrap();
    let (mut send, input) = tokio::io::duplex(4096);
    let (output, receive) = tokio::io::duplex(65536);
    let tunnel = tokio::spawn(codex_chatgpt_provider::mcp_stdio::run_authenticated(
        input,
        output,
        port,
        Some(token),
    ));
    let mut read = AsyncReader::new(receive).lines();
    send.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n")
        .await
        .unwrap();
    let reply: Value = serde_json::from_str(
        &tokio::time::timeout(std::time::Duration::from_secs(5), read.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert!(reply["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "codex_handoff"));
    send.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"codex_sessions\",\"arguments\":{\"action\":\"list\"}}}\n").await.unwrap();
    let reply: Value = serde_json::from_str(
        &tokio::time::timeout(std::time::Duration::from_secs(5), read.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(reply["result"]["isError"], false);
    drop(send);
    tunnel.await.unwrap().unwrap();
    assert!(!state.join("desktop").exists());
    assert!(!root.0.join("not-opened-home").exists());
    assert!(!root.0.join(".local/share/codex/core").exists());
    drop(service);
}

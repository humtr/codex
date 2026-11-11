//! Owned transport peers exercise discovery, including Native's real kernel
//! writer lock. They expose only observations, never turn or tool execution.
use super::*;
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Child, Command},
};

const THREAD: &str = "11111111-2222-7333-8444-555555555555";
const OTHER: &str = "aaaaaaaa-bbbb-7ccc-8ddd-eeeeeeeeeeee";

const COPY_PROGRAM: &str = "CODEX_NATIVE_FIXTURE_COPY_PROGRAM";

pub(super) fn copy_program(program: &Path) {
    // fs::copy in the parallel parent can leave its writable executable FD in
    // another test's fork until exec, even with CLOEXEC. Publish in an already
    // exec'd, single-purpose child and wait for its exit: the parent never owns
    // the writer, so a sibling fork cannot make the eventual exec fail ETXTBSY.
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "operator_native::discovery_tests::discovery_peer",
        ])
        .env(COPY_PROGRAM, program)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "owned fixture executable publication failed"
    );
}

fn private_file(path: &Path, bytes: &[u8]) {
    File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

#[test]
#[ignore = "bounded owned subprocess for Native discovery regressions"]
fn discovery_peer() {
    if let Some(program) = std::env::var_os(COPY_PROGRAM) {
        let mut input = File::open(std::env::current_exe().unwrap()).unwrap();
        let mut output = File::options()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .custom_flags(libc::O_NOFOLLOW)
            .open(program)
            .unwrap();
        std::io::copy(&mut input, &mut output).unwrap();
        output.sync_all().unwrap();
        return;
    }
    let root = PathBuf::from(std::env::var_os("CODEX_NATIVE_DISCOVERY_FIXTURE").unwrap());
    let settings: Value =
        serde_json::from_slice(&fs::read(root.join("owned.json")).unwrap()).unwrap();
    // The descriptor stays in the selected runtime process for its whole life.
    // This is the same exclusive flock that Native's WriterLockGuard retains.
    let writer = settings["writer_lock"].as_str().map(|path| {
        let file = File::options()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .unwrap();
        file.try_lock().unwrap();
        file
    });
    if settings["listening"] != true {
        private_file(&root.join("ready"), b"ready");
        std::thread::sleep(Duration::from_secs(30));
        drop(writer);
        return;
    }
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let path = Path::new(settings["socket"].as_str().unwrap());
        let listener = tokio::net::UnixListener::bind(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        private_file(&root.join("ready"), b"ready");
        let _ = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let settings = settings.clone();
                let root = root.clone();
                tokio::spawn(async move {
                    // Peer-identity rejection closes the connection before a
                    // WebSocket handshake; that is an expected negative case.
                    let Ok(mut socket) = tokio_tungstenite::accept_async(stream).await else {
                        return;
                    };
                    while let Some(Ok(message)) = socket.next().await {
                        if !message.is_text() {
                            continue;
                        }
                        let row: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                        let Some(id) = row.get("id") else { continue };
                        let method = row["method"].as_str().unwrap();
                        File::options()
                            .append(true)
                            .open(root.join("requests"))
                            .unwrap()
                            .write_all(format!("{method}\n").as_bytes())
                            .unwrap();
                        let result = match method {
                            "initialize" => json!({}),
                            "thread/loaded/list" => {
                                json!({"data":[settings["thread"]],"nextCursor":null})
                            }
                            "thread/read" => {
                                assert_eq!(row["params"]["threadId"], settings["thread"]);
                                json!({"thread":{"id":settings["thread"],
                                "status":{"type":"idle"},"turns":[{
                                    "id":"original-turn","status":"completed","items":[{
                                        "id":"original-input","type":"userMessage",
                                        "content":[{"type":"text","text":"retained input"}]
                                    }]
                                }]}})
                            }
                            method => panic!("discovery attempted a mutation: {method}"),
                        };
                        if socket
                            .send(json!({"id":id,"result":result}).to_string().into())
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                });
            }
        })
        .await;
    });
    drop(writer);
}

struct Fixture {
    root: PathBuf,
    installation: PathBuf,
    home: PathBuf,
    program: PathBuf,
    children: Vec<Child>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("nd.{}", &crate::journal::random_id().unwrap()[..8]));
        let installation = root.join("i");
        let home = installation.join(".codex");
        let program = installation.join(".local/lib/codex/core/generations/f/runtime");
        for path in [&home.join("thread-writer-locks"), program.parent().unwrap()] {
            crate::journal::private_directory(path).unwrap();
        }
        let executable = std::env::current_exe().unwrap();
        if fs::hard_link(&executable, &program).is_err() {
            copy_program(&program);
        }
        Self {
            root,
            installation,
            home,
            program,
            children: vec![],
        }
    }
    fn lock_path(&self, thread: &str) -> PathBuf {
        self.home
            .join("thread-writer-locks")
            .join(format!("{thread}.lock"))
    }
    async fn peer(&mut self, home: &Path, thread: &str, listening: bool, writer: bool) -> Owner {
        let number = self.children.len();
        let root = self.root.join(format!("p{number}"));
        crate::journal::private_directory(&root).unwrap();
        crate::journal::private_directory(home).unwrap();
        let invocation = listening.then(|| {
            codex_runtime_binding::encode_invocation(&[
                "--config".into(),
                "model_provider=\"chatgpt_desktop\"".into(),
            ])
            .unwrap()
            .unwrap()
        });
        let config = format!("# owned peer {number}\n").into_bytes();
        let name = codex_runtime_binding::configuration_namespace(
            home,
            self.program.as_os_str(),
            None,
            invocation.as_deref(),
            &config,
            None,
        );
        // FAMILIES visits the unavailable server before the healthy catalog,
        // deterministically reproducing the old fail-on-first-socket behavior.
        let directory = self
            .installation
            .join(".local/share/codex/core")
            .join(if listening { "catalogs" } else { "servers" })
            .join(name);
        crate::journal::private_directory(&directory.join("config")).unwrap();
        let lock = writer.then(|| self.lock_path(thread));
        if let Some(path) = &lock {
            private_file(path, b"");
        }
        private_file(&root.join("requests"), b"");
        private_file(
            &root.join("owned.json"),
            serde_json::to_string(&json!({
                "socket":directory.join("s"),"thread":thread,"writer_lock":lock,
                "listening":listening,
            }))
            .unwrap()
            .as_bytes(),
        );
        let child = Command::new(&self.program)
            .args([
                "--ignored",
                "--exact",
                "operator_native::discovery_tests::discovery_peer",
                "--nocapture",
            ])
            .env("CODEX_NATIVE_DISCOVERY_FIXTURE", &root)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        self.children.push(child);
        private_file(
            &directory.join("owner"),
            &codex_runtime_binding::configuration_owner_bytes(
                home,
                self.program.as_os_str(),
                false,
                listening,
            ),
        );
        private_file(&directory.join("config/config.toml"), &config);
        private_file(&directory.join("pid"), pid.to_string().as_bytes());
        if let Some(invocation) = invocation {
            private_file(&directory.join("invocation"), &invocation);
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while !root.join("ready").exists() {
                assert!(
                    self.children[number].try_wait().unwrap().is_none(),
                    "owned peer exited before readiness"
                );
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        Owner::load(&directory, home, &self.installation).unwrap()
    }
    fn pool(&self) -> Arc<Pool> {
        Pool::for_test(
            Config::default(),
            self.root.join("state"),
            self.installation.clone(),
            self.home.clone(),
            self.root.clone(),
            self.program.clone(),
        )
    }
    fn records(owner: &Owner) -> Vec<Vec<u8>> {
        ["owner", "pid", "config/config.toml", "invocation"]
            .into_iter()
            .filter_map(|name| match fs::read(owner.directory.join(name)) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => panic!("cannot read fixture record: {error}"),
            })
            .collect()
    }
}

#[tokio::test]
async fn writer_proof_is_bound_to_the_original_kernel_lock_and_home() {
    let mut fixture = Fixture::new();
    let home = fixture.home.clone();
    let selected = fixture.peer(&home, THREAD, false, true).await;
    let unavailable = fixture.peer(&home, OTHER, false, false).await;
    let pool = fixture.pool();
    let proves = |thread: &str, owner: &Owner, other: &Owner| {
        pool.owns_thread_despite_unavailable_candidates(thread, owner, std::slice::from_ref(other))
    };
    assert!(proves(THREAD, &selected, &unavailable));
    assert!(!proves(OTHER, &selected, &unavailable));
    assert!(!proves("../not-a-thread", &selected, &unavailable));
    let mut changed = selected.clone();
    changed.process.start = "0".into();
    assert!(!proves(THREAD, &changed, &unavailable));
    changed = unavailable.clone();
    changed.home = fixture.root.join("other-home");
    assert!(!proves(THREAD, &selected, &changed));

    let lock = fixture.lock_path(THREAD);
    let retained = lock.with_extension("retained");
    fs::rename(&lock, &retained).unwrap();
    private_file(&lock, b"");
    assert!(
        !proves(THREAD, &selected, &unavailable),
        "a replacement inode is not the held writer"
    );
    fs::remove_file(&lock).unwrap();
    std::os::unix::fs::symlink(&retained, &lock).unwrap();
    assert!(
        !proves(THREAD, &selected, &unavailable),
        "unsafe lock paths are not authority"
    );
    fs::remove_file(&lock).unwrap();
    fs::rename(&retained, &lock).unwrap();
    assert!(proves(THREAD, &selected, &unavailable));
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(!proves(THREAD, &selected, &unavailable));
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(proves(THREAD, &selected, &unavailable));
    fixture.children[0].kill().unwrap();
    fixture.children[0].wait().unwrap();
    assert!(!proves(THREAD, &selected, &unavailable));
}

#[tokio::test]
async fn unavailable_namespace_without_a_matching_writer_remains_uncertain() {
    let mut fixture = Fixture::new();
    let home = fixture.home.clone();
    let unavailable = fixture.peer(&home, OTHER, false, false).await;
    let before = Fixture::records(&unavailable);
    let pool = fixture.pool();
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_socket_unavailable"))
    );
    assert_eq!(
        pool.discover_thread(THREAD).await.err(),
        Some(Error("native_socket_unavailable"))
    );
    assert_eq!(Fixture::records(&unavailable), before);
    fs::set_permissions(
        unavailable.directory.join("owner"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_binding_invalid"))
    );
}

#[tokio::test]
async fn same_home_socket_failure_does_not_block_the_loaded_kernel_writer() {
    let mut fixture = Fixture::new();
    let home = fixture.home.clone();
    let selected = fixture.peer(&home, THREAD, true, true).await;
    let unavailable = fixture.peer(&home, OTHER, false, false).await;
    let before = [Fixture::records(&selected), Fixture::records(&unavailable)];
    let pool = fixture.pool();
    let found = pool.for_thread(THREAD).await.unwrap();
    assert_eq!(found.owner, selected);
    assert_eq!(pool.discover_thread(THREAD).await.unwrap().owner, selected);
    let history = found
        .call(
            "thread/read",
            json!({"threadId":THREAD,"includeTurns":true}),
        )
        .await
        .unwrap();
    assert_eq!(history["thread"]["id"], THREAD);
    assert_eq!(history["thread"]["turns"][0]["id"], "original-turn");
    assert_eq!(
        history["thread"]["turns"][0]["items"][0]["content"][0]["text"],
        "retained input"
    );
    assert_eq!(
        [Fixture::records(&selected), Fixture::records(&unavailable)],
        before
    );
    let requests = fs::read_to_string(fixture.root.join("p0/requests")).unwrap();
    assert_eq!(
        requests.lines().collect::<Vec<_>>(),
        [
            "initialize",
            "thread/loaded/list",
            "thread/loaded/list",
            "thread/read"
        ]
    );
    assert_eq!(fs::read(fixture.root.join("p1/requests")).unwrap(), b"");
    selected.process.validate().unwrap();
    unavailable.process.validate().unwrap();
}

#[tokio::test]
async fn unavailable_socket_requires_proof_even_for_a_unique_loaded_match() {
    let mut fixture = Fixture::new();
    let home = fixture.home.clone();
    fixture.peer(&home, THREAD, true, false).await;
    fixture.peer(&home, OTHER, false, false).await;
    let pool = fixture.pool();
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_socket_unavailable"))
    );
    assert_eq!(
        pool.discover_thread(THREAD).await.err(),
        Some(Error("native_socket_unavailable"))
    );
}

#[tokio::test]
async fn discovery_preserves_duplicate_and_cross_home_ambiguity() {
    let mut fixture = Fixture::new();
    let home = fixture.home.clone();
    let selected = fixture.peer(&home, THREAD, true, true).await;
    let other_home = fixture.root.join("other-home");
    fixture.peer(&other_home, OTHER, false, false).await;
    let pool = fixture.pool();
    assert_eq!(pool.for_thread(THREAD).await.unwrap().owner, selected);
    assert_eq!(
        pool.discover_thread(THREAD).await.err(),
        Some(Error("native_socket_unavailable"))
    );
    // Even exact kernel writer proof cannot hide a second live peer that
    // advertises this thread. Observe all candidates; do not return early.
    fixture.peer(&home, OTHER, false, false).await;
    fixture.peer(&home, THREAD, true, false).await;
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_writer_ambiguous"))
    );
    assert_eq!(
        pool.discover_thread(THREAD).await.err(),
        Some(Error("native_writer_ambiguous"))
    );
}

#[tokio::test]
async fn unsafe_candidates_still_fail_beside_a_verified_writer() {
    let mut fixture = Fixture::new();
    let home = fixture.home.clone();
    let selected = fixture.peer(&home, THREAD, true, true).await;
    let candidate = fixture.peer(&home, OTHER, false, false).await;
    let pool = fixture.pool();
    let socket = candidate.directory.join("s");
    private_file(&socket, b"not a socket");
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_private_socket_required"))
    );
    fs::remove_file(&socket).unwrap();
    std::os::unix::fs::symlink(selected.directory.join("s"), &socket).unwrap();
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_socket_owner_changed"))
    );
    fs::remove_file(&socket).unwrap();
    fs::write(
        candidate.directory.join("config/config.toml"),
        b"changed configuration",
    )
    .unwrap();
    assert_eq!(
        pool.for_thread(THREAD).await.err(),
        Some(Error("native_binding_invalid"))
    );
}

#[test]
fn terminal_display_uses_exact_native_identity_without_inference_or_reentry() {
    use std::ffi::OsStr;
    let fixture = Fixture::new();
    let core = fixture.root.join("core");
    fs::write(&core, b"owned executable fixture").unwrap();
    fs::set_permissions(&core, fs::Permissions::from_mode(0o700)).unwrap();
    let mut config = Config::default();
    config.execution.core = Some(core.clone());
    let pool = Pool::for_test(
        config,
        fixture.root.join("state"),
        fixture.installation.clone(),
        fixture.home.clone(),
        fixture.root.clone(),
        fixture.program.clone(),
    );
    let command = pool.terminal_display_command(THREAD).unwrap();
    let command = command.as_std();
    let metadata = fs::symlink_metadata(&fixture.home).unwrap();
    let expected = [
        "termux".into(),
        "__terminal-auto-show-v1".into(),
        THREAD.into(),
        fixture.home.as_os_str().to_owned(),
        metadata.dev().to_string().into(),
        metadata.ino().to_string().into(),
    ];
    assert_eq!(command.get_program(), core.as_os_str());
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        expected
            .iter()
            .map(|v: &std::ffi::OsString| v.as_os_str())
            .collect::<Vec<_>>()
    );
    let env = command
        .get_envs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        env.get(OsStr::new("HOME")),
        Some(&Some(fixture.installation.as_os_str()))
    );
    assert_eq!(
        env.get(OsStr::new("CODEX_HOME")),
        Some(&Some(fixture.home.as_os_str()))
    );
    assert_eq!(env.get(OsStr::new("CODEX_SQLITE_HOME")), Some(&None));
    assert!(pool.terminal_display_command("../other").is_err());
    fs::set_permissions(&core, fs::Permissions::from_mode(0o722)).unwrap();
    assert!(pool.terminal_display_command(THREAD).is_err());
    fs::set_permissions(&core, fs::Permissions::from_mode(0o700)).unwrap();
    let retained = fixture.installation.join("retained");
    fs::rename(&fixture.home, &retained).unwrap();
    std::os::unix::fs::symlink(&retained, &fixture.home).unwrap();
    assert!(pool.terminal_display_command(THREAD).is_err());
    assert!(retained.join("thread-writer-locks").is_dir());
}

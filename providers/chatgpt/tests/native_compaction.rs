//! Actual Android app-server performs native history replacement. App replies
//! are synthetic; the production Rust Provider owns compaction and warm state.
use codex_chatgpt_provider::{
    app::{AppHost, Bridge},
    cdp::Process,
    config::Config,
    context::digest,
    journal::{random_id, Journal},
    protocol::Error,
    server::Service,
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::DirBuilderExt,
    },
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct App(Mutex<Vec<Value>>);
impl Bridge for App {
    fn operation(&self, value: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let mut commands = self.0.lock().unwrap();
        commands.push(value.clone());
        let submissions = commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .collect::<Vec<_>>();
        let result = match value["op"].as_str().unwrap() {
            "identity" => {
                json!({"generation":"fixture","account":"fixture-account","user":"fixture-user","structure":"personal","present":commands.iter().any(|v|v["op"]=="open"&&v["lane"]==value["lane"])})
            }
            "open" => json!({"localId":"fixture"}),
            "release" => json!({"released":true}),
            "submit" => json!({"started":true}),
            "snapshot" => {
                let last = submissions.last().unwrap();
                let text = if submissions.len() == 2 {
                    "COMPACTED_PUBLIC_WORK_STATE"
                } else {
                    "NATIVE_TURN_COMPLETE"
                };
                let polls = commands
                    .iter()
                    .rev()
                    .take_while(|v| v["op"] != "submit")
                    .filter(|v| v["op"] == "snapshot")
                    .count();
                let settled = polls >= 3;
                json!({"round":last["round"],"messages":[{"id":format!("reply-{}",submissions.len()),"author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":[if polls==1{&text[..6]}else{text}]},"status":if settled{"finished_successfully"}else if polls==2{"finished_partial_completion"}else{"in_progress"},"end_turn":polls>=2}],"terminals":if settled{json!(["done"])}else{json!([])},"roundStarts":1,"userMessages":submissions.len(),"toolMessages":0,"dispatches":submissions.len()})
            }
            _ => return Box::pin(async { Err(Error("unexpected_fixture_operation")) }),
        };
        Box::pin(async move { Ok(result) })
    }
}
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "owned native compaction diagnostic retained: {}",
                self.0.display()
            );
            return;
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Rpc {
    input: tokio::process::ChildStdin,
    output: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    buffered: Vec<Value>,
}
impl Rpc {
    async fn send(&mut self, value: Value) {
        self.input
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
    }
    async fn matching(&mut self, predicate: impl Fn(&Value) -> bool) -> Value {
        if let Some(index) = self.buffered.iter().position(&predicate) {
            return self.buffered.remove(index);
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let line = self
                    .output
                    .next_line()
                    .await
                    .unwrap()
                    .expect("native app-server exited");
                let value: Value = serde_json::from_str(&line).unwrap();
                if value["method"] == "error" {
                    panic!("native fixture error: {}", value["params"]["error"]);
                }
                if predicate(&value) {
                    return value;
                }
                self.buffered.push(value);
            }
        })
        .await;
        result.unwrap_or_else(|_| {
            panic!(
                "native RPC observation timeout; buffered methods: {:?}",
                self.buffered
                    .iter()
                    .map(|v| v["method"].as_str().unwrap_or("response"))
                    .collect::<Vec<_>>()
            )
        })
    }
    async fn call(&mut self, id: u32, method: &str, params: Value) -> Value {
        eprintln!("native RPC {id}: {method}");
        self.send(json!({"id":id,"method":method,"params":params}))
            .await;
        let response = self.matching(|v| v["id"] == id).await;
        assert!(
            response.get("error").is_none(),
            "native RPC error: {response}"
        );
        response["result"].clone()
    }
    async fn turn(&mut self, id: u32, thread: &str, text: &str) {
        let response = self
            .call(
                id,
                "turn/start",
                json!({"threadId":thread,"input":[{"type":"text","text":text}]}),
            )
            .await;
        let turn = &response["turn"]["id"];
        let done = self
            .matching(|v| v["method"] == "turn/completed" && v["params"]["turn"]["id"] == *turn)
            .await;
        assert_eq!(done["params"]["turn"]["status"], "completed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_manual_compaction_commits_public_checkpoint_and_continues_same_thread() {
    native_context_fixture(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_public_handoff_preserves_original_opaque_history_and_user_turns() {
    native_context_fixture(true).await;
}

async fn native_context_fixture(handoff: bool) {
    let runtime = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_NATIVE").expect("qualified runtime required"),
    );
    let prefix = PathBuf::from(std::env::var_os("PREFIX").unwrap());
    let root = Root(std::env::temp_dir().join(format!("pc-{}", &random_id().unwrap()[..8])));
    fs::DirBuilder::new().mode(0o700).create(&root.0).unwrap();
    for name in ["home", "codex", "system", "tmp", "work"] {
        fs::create_dir(root.0.join(name)).unwrap();
    }
    let app = Arc::new(App::default());
    let journal = Arc::new(Journal::open(&root.0.join("journal")).unwrap());
    let host = Arc::new(AppHost::new(app.clone(), journal.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let token = random_id().unwrap();
    journal.write(&digest(&"termux-codex-responses-service-v1"),&json!({"schema":"termux-codex-responses-service-v1","process":Process::read(std::process::id()).unwrap(),"port":port,"token":token})).unwrap();
    codex_chatgpt_provider::registration::prepare(
        &Config::default(),
        &journal.root,
        &root.0.join("registry"),
        &root.0.join("codex"),
        std::path::Path::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider")),
        &format!("http://127.0.0.1:{port}/v1"),
    )
    .unwrap();
    let profile = fs::read_to_string(root.0.join("codex/chatgpt.config.toml")).unwrap();
    let mut config: toml::Table = toml::from_str(&profile).unwrap();
    config.insert("sandbox_mode".into(), "danger-full-access".into());
    config.insert("approval_policy".into(), "never".into());
    fs::write(
        root.0.join("system/config.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let service = Service::new(Config::default(), token, host.clone()).unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let trace = root.0.join("request-trace.jsonl");
    let router = service.router().layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let trace = trace.clone();
            async move {
                let (parts, body) = request.into_parts();
                let bytes = axum::body::to_bytes(body, 32 * 1024 * 1024).await.unwrap();
                if parts.uri.path() == "/v1/responses" {
                    use std::io::Write;
                    use std::os::unix::fs::OpenOptionsExt;
                    let data = if parts
                        .headers
                        .get("content-encoding")
                        .is_some_and(|v| v == "zstd")
                    {
                        zstd::stream::decode_all(&bytes[..]).unwrap()
                    } else {
                        bytes.to_vec()
                    };
                    let value: Value = serde_json::from_slice(&data).unwrap();
                    let mut file = File::options()
                        .create(true)
                        .append(true)
                        .mode(0o600)
                        .open(&trace)
                        .unwrap();
                    writeln!(file, "{value}").unwrap();
                }
                next.run(axum::extract::Request::from_parts(
                    parts,
                    axum::body::Body::from(bytes),
                ))
                .await
            }
        },
    ));
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move { shutdown.cancelled().await })
            .await
            .unwrap()
    });
    let descriptors = [
        File::open(prefix.join("etc/resolv.conf")).unwrap(),
        File::open(root.0.join("system")).unwrap(),
        File::open(root.0.join("tmp")).unwrap(),
    ]
    .map(|f| {
        let fd = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
        assert!(fd >= 64);
        unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) }
    });
    let fds = descriptors.each_ref().map(AsRawFd::as_raw_fd);
    let mut command = tokio::process::Command::new(runtime);
    command
        .env_clear()
        .env("PATH", prefix.join("bin"))
        .env("PREFIX", &prefix)
        .env("HOME", root.0.join("home"))
        .env("CODEX_HOME", root.0.join("codex"))
        .env("TMPDIR", root.0.join("tmp"))
        .env("TERM", "dumb")
        .arg("app-server")
        .current_dir(root.0.join("work"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    unsafe {
        command.pre_exec(move || {
            for (source, destination) in fds.into_iter().zip(33..=35) {
                if libc::dup2(source, destination) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    let mut rpc = Rpc {
        input: child.stdin.take().unwrap(),
        output: BufReader::new(child.stdout.take().unwrap()).lines(),
        buffered: Vec::new(),
    };
    rpc.call(1,"initialize",json!({"clientInfo":{"name":"termux-provider-qualification","version":"1"},"capabilities":{"experimentalApi":true}})).await;
    rpc.send(json!({"method":"initialized","params":{}})).await;
    let response=rpc.call(2,"thread/start",json!({"cwd":root.0.join("work"),"model":"chatgpt-desktop/gpt-6","modelProvider":"chatgpt_desktop","approvalPolicy":"never","sandbox":"danger-full-access"})).await;
    let thread = response["thread"]["id"].as_str().unwrap();
    rpc.turn(3, thread, "Remember the completed fixture work")
        .await;
    if handoff {
        let before = rpc
            .call(
                4,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        let checkpoint = codex_chatgpt_provider::handoff::Handoff::prepare(
            &before["thread"],
            thread,
            "gpt-6.1-sol",
            "chatgpt-desktop/gpt-6",
            "none",
            &Config::default(),
        )
        .unwrap();
        rpc.call(5,"thread/inject_items",json!({"threadId":thread,"items":[{"type":"compaction","encrypted_content":"UNREADABLE_FOREIGN_NATIVE_CHECKPOINT"},checkpoint]})).await;
        let after = rpc
            .call(
                6,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert_eq!(
            before["thread"]["turns"], after["thread"]["turns"],
            "external context must not manufacture a user turn/bubble"
        );
        rpc.turn(
            7,
            thread,
            "Continue from public history without repeating tools",
        )
        .await;
        let continued = rpc
            .call(
                8,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert_eq!(continued["thread"]["id"], thread);
        assert_eq!(continued["thread"]["turns"].as_array().unwrap().len(), 2);
        let commands = app.0.lock().unwrap().clone();
        assert_eq!(
            commands.iter().filter(|v| v["op"] == "release").count(),
            1,
            "an explicit new handoff retires the settled old lane once"
        );
        let submissions = commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .collect::<Vec<_>>();
        assert_eq!(submissions.len(), 2, "handoff itself starts no inference");
        assert!(submissions[1]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["hidden"] == true
                && v["role"] == "user"
                && v["text"]
                    .as_str()
                    .is_some_and(|s| s.starts_with(codex_chatgpt_provider::handoff::OPEN))));
        assert!(!submissions[1]
            .to_string()
            .contains("UNREADABLE_FOREIGN_NATIVE_CHECKPOINT"));
        assert_eq!(
            submissions[1]["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["hidden"] == false)
                .count(),
            1
        );
        drop(commands);
        drop(rpc);
        let _ = child.kill().await;
        let _ = child.wait().await;
        stop.cancel();
        server.await.unwrap();
        host.shutdown().await;
        fn preserves_opaque(path: &std::path::Path) -> bool {
            fs::read_dir(path)
                .unwrap()
                .map(Result::unwrap)
                .any(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        return preserves_opaque(&path);
                    }
                    path.extension().is_some_and(|s| s == "jsonl")
                        && fs::read_to_string(path).unwrap().lines().any(|line| {
                            serde_json::from_str::<Value>(line).is_ok_and(|v| {
                                v["type"] == "response_item"
                                    && v["payload"]["encrypted_content"]
                                        == "UNREADABLE_FOREIGN_NATIVE_CHECKPOINT"
                            })
                        })
                })
        }
        assert!(
            preserves_opaque(&root.0.join("codex/sessions")),
            "handoff projection must not rewrite native canonical history"
        );
        println!(
            "{}",
            json!({"accepted":true,"actualNativePublicHandoff":true,"sameThread":true,"noExtraUserTurnOnInject":true,"originalOpaqueInputNotDecoded":true,"handoffModelStarts":0,"fixtureHostSubmissions":2,"chatSubmissions":0})
        );
        return;
    }
    rpc.call(4, "thread/compact/start", json!({"threadId":thread}))
        .await;
    rpc.matching(|v| {
        v["method"] == "item/completed" && v["params"]["item"]["type"] == "contextCompaction"
    })
    .await;
    rpc.matching(|v| v["method"] == "turn/completed" && v["params"]["threadId"] == thread)
        .await;
    rpc.turn(5, thread, "Continue from the accepted public summary")
        .await;
    let status = rpc
        .call(
            6,
            "thread/read",
            json!({"threadId":thread,"includeTurns":true}),
        )
        .await;
    assert_eq!(status["thread"]["id"], thread);
    drop(rpc);
    let _ = child.kill().await;
    let _ = child.wait().await;
    stop.cancel();
    server.await.unwrap();
    host.shutdown().await;
    let commands = app.0.lock().unwrap();
    let submissions = commands
        .iter()
        .filter(|v| v["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(
        submissions.len(),
        3,
        "one summary request, no repeated compaction loop"
    );
    assert!(submissions[1]["instructions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == codex_chatgpt_provider::context::SUMMARY_INSTRUCTION));
    assert!(submissions[2]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["text"]
            .as_str()
            .is_some_and(|s| s.contains("COMPACTED_PUBLIC_WORK_STATE"))));
    assert_eq!(commands.iter().filter(|v| v["op"] == "release").count(), 1);
    println!(
        "{}",
        json!({"accepted":true,"actualNativeManualCompaction":true,"sameThreadAfterCompaction":true,"checkpointAppliedOnce":true,"fixtureHostSubmissions":3,"chatSubmissions":0})
    );
}

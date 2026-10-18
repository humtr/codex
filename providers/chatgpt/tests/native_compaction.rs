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

struct Live {
    remote: codex_chatgpt_provider::app::Remote,
    commands: Mutex<Vec<Value>>,
    observed: Arc<Mutex<Value>>,
}
impl Bridge for Live {
    fn operation(&self, value: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let mut commands = self.commands.lock().unwrap();
        if value["op"] == "submit" && commands.iter().filter(|v| v["op"] == "submit").count() >= 2 {
            return Box::pin(async { Err(Error("qualification_budget_exhausted")) });
        }
        commands.push(value.clone());
        drop(commands);
        let snapshot = value["op"] == "snapshot";
        let future = self.remote.operation(value);
        let observed = self.observed.clone();
        Box::pin(async move {
            let value = future.await?;
            if snapshot {
                *observed.lock().unwrap() = json!({"dispatches":value["dispatches"],"userMessages":value["userMessages"],"toolMessages":value["toolMessages"]});
            }
            Ok(value)
        })
    }
}
struct App(Mutex<Vec<Value>>, Scenario);
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
            "stop" => json!({}),
            "submit" => json!({"started":true}),
            "snapshot" => {
                let last = submissions.last().unwrap();
                let text = if matches!(self.1, Scenario::Automatic | Scenario::InterruptedAutomatic)
                    && submissions.len() == 1
                {
                    "OWNED_LARGE_PUBLIC_STATE ".repeat(8192)
                } else if matches!(self.1, Scenario::Question) {
                    "NATIVE_QUESTION_COMPLETE".into()
                } else if submissions.len() == 2 {
                    "COMPACTED_PUBLIC_WORK_STATE".into()
                } else {
                    "NATIVE_TURN_COMPLETE".into()
                };
                let polls = commands
                    .iter()
                    .rev()
                    .take_while(|v| v["op"] != "submit")
                    .filter(|v| v["op"] == "snapshot")
                    .count();
                if commands
                    .iter()
                    .rev()
                    .take_while(|v| v["op"] != "submit")
                    .any(|v| v["op"] == "stop" && v["lane"] == last["lane"])
                {
                    let stopped = json!({"round":last["round"],"stopped":true});
                    return Box::pin(async move { Ok(stopped) });
                }
                let settled = polls >= 3
                    && !(matches!(self.1, Scenario::InterruptedAutomatic)
                        && submissions.len() == 2);
                if matches!(self.1, Scenario::Question) && submissions.len() == 1 {
                    let catalog=last["instructions"].as_array().unwrap().iter().filter_map(Value::as_str).find_map(|v|v.strip_prefix("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from native Codex; the app must not execute tools itself.\n")).unwrap();
                    let tools: Value = serde_json::from_str(catalog).unwrap();
                    let key = tools
                        .as_object()
                        .unwrap()
                        .keys()
                        .find(|k| k.rsplit('.').next() == Some("request_user_input"))
                        .unwrap();
                    let gateway = commands.iter().find(|v| v["op"] == "open").unwrap()["signature"]
                        ["name"]
                        .as_str()
                        .unwrap();
                    let input=json!({"questions":[{"id":"owned_choice","header":"Fixture","question":"Which owned fixture option should continue?","options":[{"label":"Alpha (Recommended)","description":"Continue the owned fixture."},{"label":"Beta","description":"Use the alternate fixture choice."}]}]}).to_string();
                    let snapshot = json!({"round":last["round"],"messages":[{"id":"question-host-call","author":{"role":"assistant"},"recipient":format!("local.{gateway}"),"content":{"content_type":"code","text":json!({"kwargs":{"name":key,"input":input,"report":"Waiting for the owned native question"}}).to_string()},"status":"finished_successfully","end_turn":false}],"terminals":["done"],"roundStarts":1,"userMessages":1,"toolMessages":0,"dispatches":1});
                    return Box::pin(async move { Ok(snapshot) });
                }
                json!({"round":last["round"],"messages":[{"id":format!("reply-{}",submissions.len()),"author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":[if polls==1{&text[..6]}else{&text}]},"status":if settled{"finished_successfully"}else if polls==2{"finished_partial_completion"}else{"in_progress"},"end_turn":polls>=2}],"terminals":if settled{json!(["done"])}else{json!([])},"roundStarts":1,"userMessages":submissions.len(),"toolMessages":0,"dispatches":submissions.len()})
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
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(
                if std::env::var_os("CODEX_PROVIDER_LIVE_HANDOFF").is_some() {
                    120
                } else {
                    30
                },
            ),
            async {
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
            },
        )
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
    native_context_fixture(Scenario::Compact).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_public_handoff_preserves_original_opaque_history_and_user_turns() {
    native_context_fixture(Scenario::Handoff).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_bidirectional_provider_reentry_keeps_same_thread_and_readable_context() {
    native_context_fixture(Scenario::Bidirectional).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_fork_inherits_readable_handoff_and_keeps_parent_lane_independent() {
    native_context_fixture(Scenario::Fork).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_question_returns_the_actual_choice_on_original_host_call_without_user_resubmit() {
    native_context_fixture(Scenario::Question).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn native_token_trigger_compacts_once_and_continues_without_a_manual_compact_request() {
    native_context_fixture(Scenario::Automatic).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn interrupted_native_automatic_compaction_preserves_history_and_resumes_latest_user_work() {
    native_context_fixture(Scenario::InterruptedAutomatic).await;
}

#[derive(Clone, Copy)]
enum Scenario {
    Compact,
    Handoff,
    Bidirectional,
    Fork,
    Question,
    Automatic,
    InterruptedAutomatic,
}
async fn native_context_fixture(scenario: Scenario) {
    let handoff = matches!(
        scenario,
        Scenario::Handoff | Scenario::Bidirectional | Scenario::Fork
    );
    let cross_provider = matches!(scenario, Scenario::Bidirectional);
    let fork_child = matches!(scenario, Scenario::Fork);
    let runtime = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_NATIVE").expect("qualified runtime required"),
    );
    let prefix = PathBuf::from(std::env::var_os("PREFIX").unwrap());
    let root = Root(std::env::temp_dir().join(format!("pc-{}", &random_id().unwrap()[..8])));
    fs::DirBuilder::new().mode(0o700).create(&root.0).unwrap();
    for name in ["home", "codex", "system", "tmp", "work"] {
        fs::create_dir(root.0.join(name)).unwrap();
    }
    let app = Arc::new(App(Mutex::new(Vec::new()), scenario));
    let live = if cross_provider && std::env::var_os("CODEX_PROVIDER_LIVE_HANDOFF").is_some() {
        assert_eq!(
            std::env::var("CODEX_PROVIDER_LIVE_HANDOFF").as_deref(),
            Ok("qualified-gpt6-no-pro")
        );
        let proof = PathBuf::from(
            std::env::var_os("CODEX_PROVIDER_LIVE_HANDOFF_PROOF")
                .expect("new private proof destination required"),
        );
        assert!(
            proof.is_absolute()
                && fs::symlink_metadata(&proof)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "never overwrite a retained live proof"
        );
        let remote = codex_chatgpt_provider::app::Remote::new(
            PathBuf::from(
                std::env::var_os("CHATGPT_PROVIDER_DESCRIPTOR")
                    .expect("explicit authorized app descriptor required"),
            ),
            format!("handoff-{}", random_id().unwrap()),
        );
        let expected: Value = serde_json::from_slice(
            &codex_chatgpt_provider::cdp::private_read(
                &PathBuf::from(
                    std::env::var_os("CODEX_PROVIDER_EXPECTED_IDENTITY")
                        .expect("private expected account identity required"),
                ),
                4096,
            )
            .unwrap(),
        )
        .unwrap();
        let identity = remote
            .operation(json!({"op":"identity","lane":"no-inference-preflight"}))
            .await
            .unwrap();
        assert!(
            ["account", "user", "structure"]
                .iter()
                .all(|k| identity[k] == expected[k]),
            "expected app identity changed; no inference authorized"
        );
        Some(Arc::new(Live {
            remote,
            commands: Mutex::new(Vec::new()),
            observed: Arc::new(Mutex::new(Value::Null)),
        }))
    } else {
        None
    };
    let mut provider_config = Config::default();
    if live.is_some() {
        provider_config.history.mode = codex_chatgpt_provider::config::HistoryMode::Project;
        provider_config.history.project_name = Some("Termux Codex".into());
    }

    let journal = Arc::new(Journal::open(&root.0.join("journal")).unwrap());
    let bridge: Arc<dyn Bridge> = if let Some(live) = &live {
        live.clone()
    } else {
        app.clone()
    };
    let host = Arc::new(AppHost::new(bridge, journal.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let token = random_id().unwrap();
    journal.write(&digest(&"termux-codex-responses-service-v1"),&json!({"schema":"termux-codex-responses-service-v1","process":Process::read(std::process::id()).unwrap(),"port":port,"token":token})).unwrap();
    codex_chatgpt_provider::registration::prepare(
        &provider_config,
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
    if matches!(
        scenario,
        Scenario::Automatic | Scenario::InterruptedAutomatic
    ) {
        config.insert("model_auto_compact_token_limit".into(), 20000.into());
    }
    if cross_provider {
        let mut catalog = codex_chatgpt_provider::projection::catalog(&Config::default()).unwrap();
        let mut native = catalog["models"][0].clone();
        native["slug"] = json!("native-fixture");
        native["display_name"] = json!("Native fixture");
        native["comp_hash"] = json!("native-fixture-format");
        catalog["models"].as_array_mut().unwrap().push(native);
        let catalog_path = root.0.join("fixture-catalog.json");
        fs::write(&catalog_path, catalog.to_string()).unwrap();
        config.insert(
            "model_catalog_json".into(),
            catalog_path.display().to_string().into(),
        );
        let provider:toml::Value=toml::from_str(&format!("name=\"Native fixture\"\nbase_url=\"http://127.0.0.1:{port}/native\"\nwire_api=\"responses\"\nrequires_openai_auth=false\nsupports_websockets=false\nrequest_max_retries=0\nstream_max_retries=0\n")).unwrap();
        config
            .get_mut("model_providers")
            .unwrap()
            .as_table_mut()
            .unwrap()
            .insert("native_fixture".into(), provider);
    }
    fs::write(
        root.0.join("system/config.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let service = Service::new(provider_config, token, host.clone()).unwrap();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let trace = root.0.join("request-trace.jsonl");
    let native_requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = native_requests.clone();
    let router = service
        .router()
        .route(
            "/native/responses",
            axum::routing::post(
                move |headers: axum::http::HeaderMap, bytes: axum::body::Bytes| {
                    let captured = captured.clone();
                    async move {
                        let bytes = if headers.get("content-encoding").is_some_and(|v| v == "zstd")
                        {
                            zstd::stream::decode_all(&bytes[..]).unwrap()
                        } else {
                            bytes.to_vec()
                        };
                        let request: Value = serde_json::from_slice(&bytes).unwrap();
                        let mut requests = captured.lock().unwrap();
                        requests.push(request);
                        let mut projection = codex_chatgpt_provider::responses::Projection::new(
                            format!("native-response-{}", requests.len()),
                            "native-fixture".into(),
                        );
                        let mut frames = projection.start();
                        frames.extend(
                            projection
                                .push(codex_chatgpt_provider::responses::Event::Text {
                                    id: format!("native-answer-{}", requests.len()),
                                    phase: "final_answer".into(),
                                    delta: "PUBLIC_NATIVE_FIXTURE_WORK".into(),
                                })
                                .unwrap(),
                        );
                        frames.extend(
                            projection
                                .push(codex_chatgpt_provider::responses::Event::Completed {
                                    input_tokens: 100,
                                    output_tokens: 10,
                                })
                                .unwrap(),
                        );
                        let body = frames
                            .into_iter()
                            .map(|v| {
                                format!("event: {}\ndata: {v}\n\n", v["type"].as_str().unwrap())
                            })
                            .collect::<String>();
                        ([("content-type", "text/event-stream")], body)
                    }
                },
            ),
        )
        .layer(axum::middleware::from_fn(
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
    let launch = || {
        let mut command = tokio::process::Command::new(&runtime);
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
        command.spawn().unwrap()
    };
    let mut child = launch();
    let mut rpc = Rpc {
        input: child.stdin.take().unwrap(),
        output: BufReader::new(child.stdout.take().unwrap()).lines(),
        buffered: Vec::new(),
    };
    rpc.call(1,"initialize",json!({"clientInfo":{"name":"termux-provider-qualification","version":"1"},"capabilities":{"experimentalApi":true}})).await;
    rpc.send(json!({"method":"initialized","params":{}})).await;
    let response=rpc.call(2,"thread/start",json!({"cwd":root.0.join("work"),"model":if cross_provider{"native-fixture"}else{"chatgpt-desktop/gpt-6"},"modelProvider":if cross_provider{"native_fixture"}else{"chatgpt_desktop"},"approvalPolicy":"never","sandbox":"danger-full-access"})).await;
    let thread = response["thread"]["id"].as_str().unwrap();
    if matches!(scenario, Scenario::Question) {
        let started=rpc.call(3,"turn/start",json!({"threadId":thread,"input":[{"type":"text","text":"Ask the owned native question and use its actual answer"}],"collaborationMode":{"mode":"plan","settings":{"model":"chatgpt-desktop/gpt-6","reasoning_effort":"none","developer_instructions":null}}})).await;
        let question = rpc
            .matching(|v| v["method"] == "item/tool/requestUserInput")
            .await;
        assert_eq!(question["params"]["threadId"], thread);
        assert_eq!(question["params"]["questions"][0]["id"], "owned_choice");
        assert_eq!(
            app.0
                .lock()
                .unwrap()
                .iter()
                .filter(|v| v["op"] == "submit")
                .count(),
            1,
            "waiting for a native answer must not start another app request"
        );
        rpc.send(json!({"id":question["id"],"result":{"answers":{"owned_choice":{"answers":["Alpha (Recommended)"]}}}})).await;
        let done = rpc
            .matching(|v| {
                v["method"] == "turn/completed"
                    && v["params"]["turn"]["id"] == started["turn"]["id"]
            })
            .await;
        assert_eq!(done["params"]["turn"]["status"], "completed");
        let after = rpc
            .call(
                4,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert_eq!(after["thread"]["turns"].as_array().unwrap().len(), 1);
        assert!(after.to_string().contains("NATIVE_QUESTION_COMPLETE"));
        {
            let commands = app.0.lock().unwrap();
            let submissions = commands
                .iter()
                .filter(|v| v["op"] == "submit")
                .collect::<Vec<_>>();
            assert_eq!(submissions.len(), 2);
            assert!(submissions[1]["messages"].as_array().unwrap().is_empty());
            assert_eq!(submissions[1]["results"][0]["id"], "question-host-call");
            assert!(submissions[1]["results"][0]
                .to_string()
                .contains("Alpha (Recommended)"));
        }
        child.kill().await.unwrap();
        child.wait().await.unwrap();
        stop.cancel();
        server.await.unwrap();
        println!(
            "{}",
            json!({"accepted":true,"actualNativeBlockingQuestion":true,"originalHostCallReturn":true,"userMessageResubmitted":false,"appFixtureDispatches":2,"chatSubmissions":0,"nativeUserTurns":1,"physicalTuiObserved":false})
        );
        return;
    }
    rpc.turn(3, thread, if live.is_some(){"Remember that HANDOFF_STATE_7K2Q identifies the completed fixture work. Do not execute anything."}else{"Remember the completed fixture work"})
        .await;
    if matches!(scenario, Scenario::InterruptedAutomatic) {
        let started=rpc.call(4,"turn/start",json!({"threadId":thread,"input":[{"type":"text","text":"Owned turn to interrupt during automatic summary"}]})).await;
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let count = app
                    .0
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|v| v["op"] == "submit")
                    .count();
                if count >= 2 {
                    assert_eq!(count, 2);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        rpc.call(
            5,
            "turn/interrupt",
            json!({"threadId":thread,"turnId":started["turn"]["id"]}),
        )
        .await;
        let done = rpc
            .matching(|v| {
                v["method"] == "turn/completed"
                    && v["params"]["turn"]["id"] == started["turn"]["id"]
            })
            .await;
        assert_eq!(done["params"]["turn"]["status"], "interrupted");
        let key = digest(&thread);
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let lane: Value = journal.read(&key).unwrap().unwrap();
                if lane["phase"] == "stopped" {
                    assert!(
                        lane["checkpoint"].is_null(),
                        "partial summary must never become a committed checkpoint"
                    );
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        let before = rpc
            .call(
                6,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert!(before.to_string().contains("OWNED_LARGE_PUBLIC_STATE"));
        rpc.turn(
            7,
            thread,
            "Resume the latest owned work after the interrupted summary",
        )
        .await;
        let after = rpc
            .call(
                8,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert_eq!(after["thread"]["id"], thread);
        assert!(after.to_string().contains("NATIVE_TURN_COMPLETE"));
        {
            let commands = app.0.lock().unwrap();
            assert_eq!(
                commands.iter().filter(|v| v["op"] == "submit").count(),
                4,
                "one stopped summary, one replacement at a new native turn, no replay"
            );
            assert_eq!(commands.iter().filter(|v| v["op"] == "stop").count(), 1);
        }
        drop(rpc);
        child.kill().await.unwrap();
        child.wait().await.unwrap();
        stop.cancel();
        server.await.unwrap();
        host.shutdown().await;
        println!(
            "{}",
            json!({"accepted":true,"actualNativeInterruptedCompaction":true,"stopAcknowledged":true,"partialCheckpointCommitted":false,"sameThread":true,"resumedFromLatestNativeWork":true,"fixtureAppDispatches":4,"chatSubmissions":0})
        );
        return;
    }
    if matches!(scenario, Scenario::Automatic) {
        rpc.turn(
            4,
            thread,
            "Continue the latest owned work after the token trigger",
        )
        .await;
        let after = rpc
            .call(
                5,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert_eq!(after["thread"]["id"], thread);
        assert!(after.to_string().contains("NATIVE_TURN_COMPLETE"));
        let trace = fs::read_to_string(root.0.join("request-trace.jsonl"))
            .unwrap()
            .lines()
            .map(|v| serde_json::from_str::<Value>(v).unwrap())
            .collect::<Vec<_>>();
        let requests = trace
            .iter()
            .map(|v| {
                codex_chatgpt_provider::protocol::Request::parse(
                    v.clone(),
                    &Config::default(),
                    None,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            requests.iter().filter(|v| v.compaction.is_some()).count(),
            1,
            "native token trigger must not enter a compaction loop"
        );
        assert_eq!(requests.len(), 3);
        {
            let commands = app.0.lock().unwrap();
            assert_eq!(commands.iter().filter(|v| v["op"] == "submit").count(), 3);
            assert_eq!(
                commands.iter().filter(|v| v["op"] == "release").count(),
                1,
                "committed automatic checkpoint retires once"
            );
        }
        drop(rpc);
        child.kill().await.unwrap();
        child.wait().await.unwrap();
        stop.cancel();
        server.await.unwrap();
        host.shutdown().await;
        println!(
            "{}",
            json!({"accepted":true,"actualNativeAutomaticCompaction":true,"manualCompactRequested":false,"sameThread":true,"compactionRequests":1,"fixtureAppDispatches":3,"chatSubmissions":0})
        );
        return;
    }
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
            if cross_provider {
                "native-fixture"
            } else {
                "gpt-6.1-sol"
            },
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
        if cross_provider {
            drop(rpc);
            child.kill().await.unwrap();
            child.wait().await.unwrap();
            child = launch();
            rpc = Rpc {
                input: child.stdin.take().unwrap(),
                output: BufReader::new(child.stdout.take().unwrap()).lines(),
                buffered: Vec::new(),
            };
            rpc.call(20,"initialize",json!({"clientInfo":{"name":"termux-transition-fixture","version":"1"},"capabilities":{"experimentalApi":true}})).await;
            rpc.send(json!({"method":"initialized","params":{}})).await;
            rpc.call(21,"thread/resume",json!({"threadId":thread,"model":"chatgpt-desktop/gpt-6","modelProvider":"chatgpt_desktop"})).await;
        }
        rpc.turn(
            7,
            thread,
            if live.is_some(){"Using the public conversation context, respond with the exact HANDOFF_STATE_7K2Q marker and confirm that no tools should be repeated. Do not use tools or ask questions."}else{"Continue from public history without repeating tools"},
        )
        .await;
        if fork_child {
            let fork=rpc.call(30,"thread/fork",json!({"threadId":thread,"model":"chatgpt-desktop/gpt-6","modelProvider":"chatgpt_desktop"})).await;
            let child_thread = fork["thread"]["id"].as_str().unwrap();
            assert_ne!(child_thread, thread);
            rpc.turn(
                31,
                child_thread,
                "Continue the inherited public work in this child",
            )
            .await;
            rpc.turn(32, child_thread, "Continue once more in the same child")
                .await;
            let parent = rpc
                .call(
                    33,
                    "thread/read",
                    json!({"threadId":thread,"includeTurns":true}),
                )
                .await;
            assert_eq!(parent["thread"]["turns"].as_array().unwrap().len(), 2);
            drop(rpc);
            child.kill().await.unwrap();
            child.wait().await.unwrap();
            child = launch();
            rpc = Rpc {
                input: child.stdin.take().unwrap(),
                output: BufReader::new(child.stdout.take().unwrap()).lines(),
                buffered: Vec::new(),
            };
            rpc.call(34,"initialize",json!({"clientInfo":{"name":"termux-fork-fixture","version":"1"},"capabilities":{"experimentalApi":true}})).await;
            rpc.send(json!({"method":"initialized","params":{}})).await;
            rpc.call(35,"thread/resume",json!({"threadId":child_thread,"model":"chatgpt-desktop/gpt-6","modelProvider":"chatgpt_desktop"})).await;
            rpc.turn(36, child_thread, "Continue after cold native resume")
                .await;
            let nested=rpc.call(37,"thread/fork",json!({"threadId":child_thread,"model":"chatgpt-desktop/gpt-6","modelProvider":"chatgpt_desktop"})).await;
            let nested_thread = nested["thread"]["id"].as_str().unwrap();
            rpc.turn(
                38,
                nested_thread,
                "Continue in the nested fork with inherited public history",
            )
            .await;
            drop(rpc);
            child.kill().await.unwrap();
            child.wait().await.unwrap();
            stop.cancel();
            server.await.unwrap();
            host.shutdown().await;
            let records = fs::read_dir(&journal.root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|v| v.path().extension().is_some_and(|v| v == "json"))
                .filter_map(|v| {
                    journal
                        .read::<Value>(v.path().file_stem().unwrap().to_str().unwrap())
                        .unwrap()
                })
                .filter(|v| v.get("native_thread").is_some())
                .collect::<Vec<_>>();
            assert_eq!(records.len(), 3);
            for key in ["marker", "gateway", "native_thread"] {
                assert_eq!(
                    records
                        .iter()
                        .map(|v| v[key].as_str().unwrap())
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    3
                );
            }
            println!(
                "{}",
                json!({"accepted":true,"nativeForkHandoff":true,"sameParentHistory":true,"childWarmAndColdResume":true,"nestedFork":true,"independentLanes":true,"chatSubmissions":0})
            );
            return;
        }
        if cross_provider {
            rpc.call(22, "thread/compact/start", json!({"threadId":thread}))
                .await;
            rpc.matching(|v| {
                v["method"] == "item/completed"
                    && v["params"]["item"]["type"] == "contextCompaction"
            })
            .await;
            rpc.matching(|v| v["method"] == "turn/completed" && v["params"]["threadId"] == thread)
                .await;
            drop(rpc);
            child.kill().await.unwrap();
            child.wait().await.unwrap();
            child = launch();
            rpc = Rpc {
                input: child.stdin.take().unwrap(),
                output: BufReader::new(child.stdout.take().unwrap()).lines(),
                buffered: Vec::new(),
            };
            rpc.call(23,"initialize",json!({"clientInfo":{"name":"termux-transition-fixture","version":"1"},"capabilities":{"experimentalApi":true}})).await;
            rpc.send(json!({"method":"initialized","params":{}})).await;
            rpc.call(24,"thread/resume",json!({"threadId":thread,"model":"native-fixture","modelProvider":"native_fixture"})).await;
            rpc.turn(
                25,
                thread,
                "Continue on the native provider without repeating work",
            )
            .await;
            let requests = native_requests.lock().unwrap().clone();
            assert_eq!(
                requests.len(),
                2,
                "hash absence must not add cross-provider compaction requests"
            );
            let input = requests[1]["input"].to_string();
            if live.is_some() {
                assert!(input.contains(codex_chatgpt_provider::context::SUMMARY_PREFIX.trim()));
                assert!(
                    input.contains("HANDOFF_STATE_7K2Q"),
                    "real summary lost the user's continuation marker"
                );
            } else {
                assert!(input.contains("COMPACTED_PUBLIC_WORK_STATE"));
            }
            assert!(
                !input.contains("UNREADABLE_FOREIGN_NATIVE_CHECKPOINT") && !input.contains("ocx1:")
            );
        }
        let continued = rpc
            .call(
                8,
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await;
        assert_eq!(continued["thread"]["id"], thread);
        assert_eq!(
            continued["thread"]["turns"].as_array().unwrap().len(),
            if cross_provider { 4 } else { 2 }
        );
        assert_eq!(
            continued["thread"]["turns"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v["items"].as_array().unwrap())
                .filter(|v| v["type"] == "userMessage")
                .count(),
            if cross_provider { 3 } else { 2 }
        );
        let commands = if let Some(live) = &live {
            live.commands.lock().unwrap().clone()
        } else {
            app.0.lock().unwrap().clone()
        };
        assert_eq!(
            commands.iter().filter(|v| v["op"] == "release").count(),
            usize::from(!cross_provider),
            "an explicit new handoff retires the settled old lane once"
        );
        let submissions = commands
            .iter()
            .filter(|v| v["op"] == "submit")
            .collect::<Vec<_>>();
        assert_eq!(submissions.len(), 2, "handoff itself starts no inference");
        let imported = submissions[usize::from(!cross_provider)];
        assert!(imported["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["hidden"] == true
                && v["role"] == "user"
                && v["text"]
                    .as_str()
                    .is_some_and(|s| s.starts_with(codex_chatgpt_provider::handoff::OPEN))));
        assert!(!imported
            .to_string()
            .contains("UNREADABLE_FOREIGN_NATIVE_CHECKPOINT"));
        assert_eq!(
            imported["messages"]
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
        if live.is_some() {
            let proof = PathBuf::from(
                std::env::var_os("CODEX_PROVIDER_LIVE_HANDOFF_PROOF")
                    .expect("new private proof destination required"),
            );
            assert!(!proof.exists(), "never overwrite a retained live proof");
            fs::rename(&root.0, &proof).unwrap();
        }
        println!(
            "{}",
            json!({"accepted":true,"bidirectionalProviderReentry":cross_provider,"nativeFixtureRequests":native_requests.lock().unwrap().len(),"actualNativePublicHandoff":true,"sameThread":true,"noExtraUserTurnOnInject":true,"originalOpaqueInputNotDecoded":true,"handoffModelStarts":0,"providerSubmissions":2,"chatInferenceEnabled":live.is_some(),"hostObservation":live.as_ref().map(|v|v.observed.lock().unwrap().clone()),"chatAllowanceDebits":"unknown","liveChatSummary":live.is_some(),"proSampled":false,"automaticRetry":false})
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

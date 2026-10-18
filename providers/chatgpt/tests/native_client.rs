//! Explicit Android acceptance: a real qualified Codex executable drives the
//! production Rust Responses/lane stack. Only the app-side replies are synthetic.
//! Default proof uses no inference or user workspace. A separately armed live
//! mode qualifies the same stack against an authorized app, capped at four starts.
use codex_chatgpt_provider::{
    app::{AppHost, Bridge},
    cdp::Process,
    config::Config,
    journal::{random_id, Journal},
    protocol::Error,
    server::Service,
};
use futures_util::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::Write,
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct State {
    gateway: String,
    round: String,
    stage: usize,
    key: String,
    patch_key: String,
    commands: Vec<Value>,
    mcp: bool,
    reject_first: bool,
    mcp_key: String,
    search_key: String,
    fail_identity: bool,
    stopped: bool,
}
#[derive(Clone, Default)]
struct App(Arc<Mutex<State>>);
struct Live {
    remote: codex_chatgpt_provider::app::Remote,
    commands: Mutex<Vec<Value>>,
    budget: usize,
}
impl Bridge for Live {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        if command["op"] == "submit" {
            let mut commands = self.commands.lock().unwrap();
            if commands.iter().filter(|v| v["op"] == "submit").count() >= self.budget {
                return Box::pin(async { Err(Error("qualification_budget_exhausted")) });
            }
            commands.push(command.clone());
        }
        self.remote.operation(command)
    }
}
impl Bridge for App {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let this = self.clone();
        Box::pin(async move {
            let mut s = this.0.lock().unwrap();
            s.commands.push(command.clone());
            if s.fail_identity && command["op"] == "identity" {
                return Err(Error("qualification_expected_failure"));
            }
            match command["op"].as_str().unwrap() {
                "identity" => Ok(
                    json!({"generation":"fixture","account":"fixture-account","user":"fixture-user","structure":"personal","present":!s.gateway.is_empty()}),
                ),
                "open" => {
                    s.gateway = command["signature"]["name"].as_str().unwrap().into();
                    Ok(json!({"localId":"fixture"}))
                }
                "submit" => {
                    s.stage += 1;
                    s.round = command["round"].as_str().unwrap().into();
                    if s.mcp {
                        if let Some(catalog)=command["instructions"].as_array().unwrap().iter().filter_map(Value::as_str).find_map(|v|v.strip_prefix("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from Core; the app must not execute tools itself.\n")) {
                            let tools:Value=serde_json::from_str(catalog).unwrap();
                            s.search_key=tools.as_object().unwrap().keys().find(|k|k.rsplit('.').next()==Some("tool_search")).ok_or(Error("fixture_search_unavailable"))?.clone();
                            let found=tools.as_object().unwrap().keys().find(|k|k.contains("fixture") && (k.ends_with(".echo") || k.ends_with("__echo"))).cloned();
                            if s.stage==1 {assert!(found.is_none(),"fixture MCP tools must actually start deferred")}
                            if let Some(key)=found {s.mcp_key=key;}
                        }
                    }
                    if s.stage == 1 {
                        let catalog=command["instructions"].as_array().unwrap().iter().filter_map(Value::as_str).find_map(|v|v.strip_prefix("Native tool catalog. Invoke only these exact keys through the local gateway. Tool results come from Core; the app must not execute tools itself.\n")).ok_or(Error("fixture_catalog_missing"))?;
                        let tools: Value = serde_json::from_str(catalog).unwrap();
                        s.patch_key = tools
                            .as_object()
                            .unwrap()
                            .keys()
                            .find(|k| k.rsplit('.').next() == Some("apply_patch"))
                            .ok_or(Error("fixture_custom_patch_unavailable"))?
                            .clone();
                        s.key = tools
                            .as_object()
                            .unwrap()
                            .keys()
                            .find(|k| {
                                matches!(
                                    k.rsplit('.').next(),
                                    Some("exec_command" | "shell_command" | "shell")
                                )
                            })
                            .ok_or_else(|| {
                                eprintln!(
                                    "fixture tool names: {:?}",
                                    tools.as_object().unwrap().keys().collect::<Vec<_>>()
                                );
                                Error("fixture_shell_unavailable")
                            })?
                            .clone();
                    }
                    Ok(json!({"started":true}))
                }
                "snapshot" => {
                    if s.stopped {
                        return Ok(json!({"round":s.round,"stopped":true}));
                    }
                    let round = s.stage - usize::from(s.reject_first);
                    let stage = if s.mcp {
                        round.saturating_sub(2)
                    } else {
                        round
                    };
                    let message = if s.reject_first && s.stage == 1 {
                        json!({"id":"rejected-host-call","author":{"role":"assistant"},"recipient":format!("local.{}",s.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"name":"mcp__fixture.guessed_echo","input":"{}"}}).to_string()},"status":"finished_successfully","end_turn":false})
                    } else if s.mcp && round <= 2 {
                        let (name, input) = if round == 1 {
                            (
                                &s.search_key,
                                json!({"query":"fixture echo verification","limit":1}).to_string(),
                            )
                        } else {
                            if s.mcp_key.is_empty() {
                                return Err(Error("fixture_discovered_mcp_unavailable"));
                            }
                            (
                                &s.mcp_key,
                                json!({"text":"native_mcp_verified"}).to_string(),
                            )
                        };
                        json!({"id":format!("host-call-{}",s.stage),"author":{"role":"assistant"},"recipient":format!("local.{}",s.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"name":name,"input":input,"report":"Discover and verify the native MCP tool"}}).to_string()},"status":"finished_successfully","end_turn":false})
                    } else if stage < 4 {
                        let script = if stage == 1 {
                            "printf 'effect-1\\n' >> effect-count; printf 'first result'"
                        } else {
                            "test \"$(wc -l < effect-count)\" -eq 1 && grep -qx 'real custom tool' patched.txt && printf 'effect-2\\n' >> effect-count && printf 'verified'"
                        };
                        let args = if s.key.ends_with("exec_command") {
                            json!({"cmd":script,"max_output_tokens":1000})
                        } else if s.key.ends_with("shell_command") {
                            json!({"command":script,"timeout_ms":10000,"max_output_chars":1000})
                        } else {
                            json!({"command":["bash","-c",script],"timeout_ms":10000})
                        };
                        let (key, input) = if stage == 2 {
                            (&s.patch_key,"*** Begin Patch\n*** Add File: patched.txt\n+real custom tool\n*** End Patch".to_string())
                        } else {
                            (&s.key, args.to_string())
                        };
                        json!({"id":format!("host-call-{}",s.stage),"author":{"role":"assistant"},"recipient":format!("local.{}",s.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"name":key,"input":input,"report":format!("Native fixture step {}",s.stage)}}).to_string()},"status":"finished_successfully","end_turn":false})
                    } else {
                        json!({"id":"final","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Native executor verified"]},"status":"finished_successfully","end_turn":true})
                    };
                    let user_objects = s
                        .commands
                        .iter()
                        .filter(|v| v["op"] == "submit")
                        .flat_map(|v| v["messages"].as_array().unwrap().iter())
                        .filter(|v| v["role"] == "user")
                        .count();
                    Ok(
                        json!({"round":s.round,"messages":[message],"terminals":["done"],"roundStarts":1,"userMessages":user_objects,"toolMessages":s.stage-1,"dispatches":s.stage}),
                    )
                }
                "stop" => {
                    s.stopped = true;
                    Ok(json!({}))
                }
                _ => Err(Error("fixture_unexpected_operation")),
            }
        })
    }
}
struct Root(PathBuf, bool);
impl Drop for Root {
    fn drop(&mut self) {
        if self.1 || std::thread::panicking() {
            eprintln!("qualification state retained at {}", self.0.display());
            return;
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn actual_native_client_executes_commands_custom_patch_and_validation_with_generated_catalog()
{
    native_fixture(false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn actual_native_client_discovers_and_executes_deferred_mcp_tools_before_local_work() {
    native_fixture(true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn actual_native_client_recovers_unadvertised_tool_with_original_call_error_then_discovers() {
    native_fixture(true, true).await;
}

async fn native_fixture(mcp: bool, reject_first: bool) {
    let runtime = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_NATIVE").expect("qualified runtime path required"),
    );
    let live_descriptor = std::env::var_os("CODEX_PROVIDER_LIVE_DESCRIPTOR");
    let owned_state = std::env::var_os("CODEX_PROVIDER_OWNED_STATE").map(PathBuf::from);
    assert!(!(live_descriptor.is_some() && owned_state.is_some()));
    let is_live = live_descriptor.is_some() || owned_state.is_some();
    assert!(
        !mcp || !is_live,
        "MCP discovery fixture never enables live inference"
    );
    let failure_proof = std::env::var_os("CODEX_PROVIDER_EXPECT_FAILURE").is_some_and(|value| {
        assert_eq!(value, "qualification_expected_failure");
        assert!(!is_live);
        true
    });
    let development = std::env::var_os("CODEX_PROVIDER_DEVELOPMENT").map(|value| {
        assert_eq!(value, "direct-ai-development");
        assert!(is_live, "actual development requires live arming");
        let workspace = PathBuf::from(std::env::var_os("HOME").unwrap()).join("prj/ai");
        assert!(workspace.join("AGENTS.md").is_file() && workspace.join(".git").exists());
        let path = PathBuf::from(
            std::env::var_os("CODEX_PROVIDER_DEVELOPMENT_TASK")
                .expect("private task file required"),
        );
        let task =
            String::from_utf8(codex_chatgpt_provider::cdp::private_read(&path, 32768).unwrap())
                .unwrap();
        (workspace, task)
    });
    if is_live {
        assert_eq!(
            std::env::var("CODEX_PROVIDER_LIVE_CHAT").as_deref(),
            Ok("qualified-gpt6-no-pro"),
            "live qualification requires separate explicit arming"
        );
    }
    let mut live_transcript = is_live.then(|| {
        use std::os::unix::fs::OpenOptionsExt;
        let path = PathBuf::from(
            std::env::var_os("CODEX_PROVIDER_LIVE_TRANSCRIPT")
                .expect("explicit private transcript output required"),
        );
        assert!(path.is_absolute());
        File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .expect("new transcript output required")
    });
    let prefix = PathBuf::from(std::env::var_os("PREFIX").expect("Termux prefix required"));
    assert!(runtime.is_absolute());
    let resume = std::env::var_os("CODEX_PROVIDER_RESUME_PROOF").map(PathBuf::from);
    if let Some(path) = &resume {
        assert!(development.is_some());
        assert_eq!(path.parent(), Some(std::env::temp_dir().as_path()));
        assert!(path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("pn-"));
        assert!(path.join("codex/chatgpt.config.toml").is_symlink());
    }
    let mut root = Root(
        resume.clone().unwrap_or_else(|| {
            std::env::temp_dir().join(format!("pn-{}", &random_id().unwrap()[..8]))
        }),
        is_live,
    );
    use std::os::unix::fs::DirBuilderExt;
    if resume.is_none() {
        fs::DirBuilder::new().mode(0o700).create(&root.0).unwrap();
    }
    for name in ["home", "codex", "work", "tmp", "system"] {
        fs::create_dir_all(root.0.join(name)).unwrap();
    }
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(
        root.0.join("system/config.toml"),
        "sandbox_mode = \"danger-full-access\"\napproval_policy = \"never\"\n",
    )
    .unwrap();
    let app = Arc::new(App::default());
    app.0.lock().unwrap().fail_identity = failure_proof;
    app.0.lock().unwrap().mcp = mcp;
    app.0.lock().unwrap().reject_first = reject_first;
    let journal = Arc::new(Journal::open(&root.0.join("journal")).unwrap());
    let mut owned = if let Some(state) = owned_state {
        Some(
            codex_chatgpt_provider::host::OwnedHost::start(
                &codex_chatgpt_provider::config::HostConfig {
                    rootfs: Some(
                        std::env::var_os("CHATGPT_PROVIDER_ROOTFS")
                            .expect("qualified app rootfs required")
                            .into(),
                    ),
                },
                &state,
            )
            .await
            .unwrap(),
        )
    } else {
        None
    };
    let live = if is_live {
        Some({
            Arc::new(Live {
                remote: if let Some(owned) = &owned {
                    codex_chatgpt_provider::app::Remote::pipe(
                        owned.client.clone(),
                        journal.owner.clone(),
                    )
                } else {
                    codex_chatgpt_provider::app::Remote::new(
                        live_descriptor.unwrap().into(),
                        journal.owner.clone(),
                    )
                },
                commands: Mutex::new(Vec::new()),
                budget: if development.is_some() { 40 } else { 4 },
            })
        })
    } else {
        None
    };
    if let Some(host) = &mut owned {
        let expected: Value = serde_json::from_slice(
            &codex_chatgpt_provider::cdp::private_read(
                &PathBuf::from(
                    std::env::var_os("CODEX_PROVIDER_EXPECTED_IDENTITY")
                        .expect("private pre-migration identity required"),
                ),
                4096,
            )
            .unwrap(),
        )
        .unwrap();
        let remote = &live.as_ref().unwrap().remote;
        let until = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
        let identity = loop {
            match remote
                .operation(json!({"op":"identity","lane":"no-inference-auth-preflight"}))
                .await
            {
                Ok(value) => break value,
                Err(error)
                    if tokio::time::Instant::now() < until
                        && matches!(
                            error.0,
                            "app_auth_context_unavailable" | "app_target_unavailable"
                        ) =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await
                }
                Err(error) => {
                    host.shutdown().await;
                    panic!("owned identity preflight: {error}");
                }
            }
        };
        let same = ["account", "user", "structure"]
            .iter()
            .all(|k| identity[k] == expected[k]);
        if !same {
            host.shutdown().await;
            panic!("owned app account changed; no inference authorized");
        }
    }
    let bridge: Arc<dyn Bridge> = match &live {
        Some(live) => live.clone(),
        None => app.clone(),
    };
    let host = Arc::new(AppHost::new(bridge, journal.clone()));
    let token = random_id().unwrap();
    let mut config = Config::default();
    if development.is_some() {
        config.model.reasoning_effort = "high".into();
    }
    if let Some(name) = std::env::var_os("CODEX_PROVIDER_LIVE_PROJECT") {
        assert!(
            live.is_some(),
            "project qualification needs an explicitly armed app"
        );
        config.history.mode = codex_chatgpt_provider::config::HistoryMode::Project;
        config.history.project_name = Some(name.into_string().unwrap());
    }
    let port = if resume.is_some() {
        journal
            .read::<Value>(&codex_chatgpt_provider::context::digest(
                &"termux-codex-responses-service-v1",
            ))
            .unwrap()
            .unwrap()["port"]
            .as_u64()
            .unwrap() as u16
    } else {
        0
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    if mcp {
        use std::fs::OpenOptions;
        let mut file = OpenOptions::new()
            .append(true)
            .open(root.0.join("system/config.toml"))
            .unwrap();
        writeln!(file,"\n[mcp_servers.fixture]\nurl=\"http://127.0.0.1:{port}/mcp\"\nstartup_timeout_sec=15\ntool_timeout_sec=15\n").unwrap();
    }

    journal.write(&codex_chatgpt_provider::context::digest(&"termux-codex-responses-service-v1"),&json!({"schema":"termux-codex-responses-service-v1","process":Process::read(std::process::id()).unwrap(),"port":port,"token":token})).unwrap();
    codex_chatgpt_provider::registration::prepare(
        &config,
        &root.0.join("journal"),
        &root.0.join("registry"),
        &root.0.join("codex"),
        std::path::Path::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider")),
        &format!("http://127.0.0.1:{port}/v1"),
    )
    .unwrap();
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let service = Service::new(config.clone(), token.clone(), host.clone()).unwrap();
    let native_requests = Arc::new(AtomicUsize::new(0));
    let response_count = native_requests.clone();
    let mcp_calls = Arc::new(Mutex::new(Vec::<Value>::new()));
    let recorded = mcp_calls.clone();
    let router = service.router().route("/mcp",axum::routing::post(move |axum::Json(request):axum::Json<Value>| {
        let recorded=recorded.clone();
        async move {
            use axum::response::IntoResponse;
            if request.get("id").is_none() {return axum::http::StatusCode::ACCEPTED.into_response()}
            recorded.lock().unwrap().push(request.clone());
            let result=match request["method"].as_str().unwrap() {
                "initialize"=>json!({"protocolVersion":request["params"]["protocolVersion"],"capabilities":{"tools":{}},"serverInfo":{"name":"Termux Native MCP Fixture","version":"1.0.0"}}),
                "tools/list"=>json!({"tools":[{"name":"echo","description":"Fixture echo verification of native deferred discovery and exact tool result","inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"],"additionalProperties":false}}]}),
                "tools/call"=>{
                    assert_eq!(request["params"]["name"],"echo");assert_eq!(request["params"]["arguments"]["text"],"native_mcp_verified");
                    json!({"content":[{"type":"text","text":"native_mcp_verified"}],"isError":false})
                },
                _=>panic!("unexpected MCP fixture method {}",request["method"]),
            };
            axum::Json(json!({"jsonrpc":"2.0","id":request["id"],"result":result})).into_response()
        }
    })).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
          let response_count=response_count.clone();
          async move {
            let (parts, body) = request.into_parts();
            let bytes = axum::body::to_bytes(body, 32 * 1024 * 1024).await.unwrap();
            if parts.uri.path() == "/v1/responses" {
                response_count.fetch_add(1,Ordering::Relaxed);
                let decoded = if parts
                    .headers
                    .get("content-encoding")
                    .is_some_and(|v| v == "zstd")
                {
                    zstd::stream::decode_all(&bytes[..]).unwrap()
                } else {
                    bytes.to_vec()
                };
                let value: Value = serde_json::from_slice(&decoded).unwrap();
                eprintln!(
                    "fixture request keys: {:?}",
                    value.as_object().unwrap().keys().collect::<Vec<_>>()
                );
                eprintln!(
                    "fixture declaration kinds: {:?}",
                    value["tools"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .map(|v| (v["type"].clone(), v["name"].clone()))
                        .collect::<Vec<_>>()
                );
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
            .with_graceful_shutdown(async move { stop.cancelled().await })
            .await
            .unwrap()
    });
    let resolver = File::open(prefix.join("etc/resolv.conf")).unwrap();
    let settings = File::open(root.0.join("system")).unwrap();
    let temporary = File::open(root.0.join("tmp")).unwrap();
    let descriptors = [resolver, settings, temporary].map(|f| {
        let fd = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
        assert!(fd >= 64);
        unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) }
    });
    use std::os::fd::FromRawFd;
    let fds = descriptors.each_ref().map(AsRawFd::as_raw_fd);
    let mut command = tokio::process::Command::new(&runtime);
    command
        .env_clear()
        .env("PATH", prefix.join("bin"))
        .env("PREFIX", &prefix)
        .env("HOME", root.0.join("home"))
        .env("CODEX_HOME", root.0.join("codex"))
        .env("TMPDIR", root.0.join("tmp"))
        .env("TERM", "dumb")
        .current_dir(
            development
                .as_ref()
                .map(|v| v.0.clone())
                .unwrap_or_else(|| root.0.join("work")),
        )
        .args([
            "exec",
            "--json",
            "--skip-git-repo-check",
            "--dangerously-bypass-approvals-and-sandbox",
            "--profile",
            "chatgpt",
        ]);
    if resume.is_some() {
        command.arg("resume").arg(
            std::env::var("CODEX_PROVIDER_RESUME_THREAD").expect("exact native thread required"),
        );
    }
    command.arg(if let Some((_,task)) = &development { task } else if live.is_some(){"Run this exact bounded local transport qualification. Use exec_command once to execute: printf 'effect-1\\n' >> effect-count . Then use apply_patch to add patched.txt containing the one line: real custom tool . Then use exec_command to execute: test \"$(wc -l < effect-count)\" -eq 1 && grep -qx 'real custom tool' patched.txt && printf 'effect-2\\n' >> effect-count . Do not use any other tools, commands, network or directories. After successful validation, answer exactly: Native executor verified"}else{"Execute the fixture tools and return the verified result"}).kill_on_drop(true);
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
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(if development.is_some() {
            1200
        } else if live.is_some() {
            240
        } else {
            90
        }),
        async {
            if development.is_none() {
                return command.output().await;
            }
            use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
            command
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            let mut child = command.spawn()?;
            let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
            let mut stderr = child.stderr.take().unwrap();
            let stdout = async {
                let mut output = Vec::new();
                while let Some(line) = lines.next_line().await? {
                    if let Some(file) = &mut live_transcript {
                        writeln!(file, "{line}")?;
                        file.sync_data()?;
                    }
                    if let Ok(event) = serde_json::from_str::<Value>(&line) {
                        eprintln!(
                            "development native event: {} {}",
                            event["type"], event["item"]["type"]
                        );
                    }
                    output.extend_from_slice(line.as_bytes());
                    output.push(b'\n');
                }
                Ok::<_, std::io::Error>(output)
            };
            let errors = async {
                let mut bytes = Vec::new();
                stderr.read_to_end(&mut bytes).await?;
                Ok::<_, std::io::Error>(bytes)
            };
            let (stdout, stderr, status) = tokio::join!(stdout, errors, child.wait());
            Ok(std::process::Output {
                stdout: stdout?,
                stderr: stderr?,
                status: status?,
            })
        },
    )
    .await
    .expect("native execution timeout")
    .unwrap();
    shutdown.cancel();
    server.await.unwrap();
    host.shutdown().await;
    if let Some(file) = &mut live_transcript.as_mut().filter(|_| development.is_none()) {
        file.write_all(&result.stdout).unwrap();
        file.sync_all().unwrap();
    }
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    if failure_proof {
        assert!(!result.status.success());
        assert!(
            stdout.contains("qualification_expected_failure"),
            "native client lost the Provider failure reason"
        );
        assert!(!stdout.contains("stream closed before response.completed"));
        println!(
            "{}",
            json!({"accepted":true,"nativeFailureReasonPreserved":true,"chatSubmissions":0})
        );
        return;
    }
    assert!(
        result.status.success(),
        "native exit {:?}\nstdout: {stdout}\nstderr: {stderr}",
        result.status.code()
    );
    if development.is_none() {
        assert!(
            stdout.contains("Native executor verified"),
            "{stdout}\n{stderr}"
        );
    } else {
        assert!(
            stdout
                .lines()
                .filter_map(|v| serde_json::from_str::<Value>(v).ok())
                .any(|v| v["type"] == "turn.completed"),
            "development turn did not complete"
        );
    }
    assert!(
        stdout.contains("command_execution"),
        "native transcript omitted tool execution: {stdout}"
    );
    if development.is_none() {
        assert_eq!(
            fs::read_to_string(root.0.join("work/effect-count")).unwrap(),
            "effect-1\neffect-2\n"
        );
    }
    if let Some(live) = live {
        if development.is_none() {
            assert_eq!(
                fs::read_to_string(root.0.join("work/patched.txt")).unwrap(),
                "real custom tool\n"
            );
        }
        let submissions = live.commands.lock().unwrap().clone();
        assert!((2..=if development.is_some() { 40 } else { 4 }).contains(&submissions.len()));
        for command in &submissions[1..] {
            if development.is_none() {
                assert!(command["messages"].as_array().unwrap().is_empty());
                assert!(command["instructions"].as_array().unwrap().is_empty());
            }
        }
        let mut observation = None;
        for entry in fs::read_dir(&journal.root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|v| v == "json") {
                let key = path.file_stem().unwrap().to_str().unwrap();
                if let Some(lane) = journal
                    .read::<Value>(key)
                    .unwrap()
                    .filter(|v| v.get("native_thread").is_some())
                {
                    assert_eq!(lane["phase"], "idle");
                    observation = Some(
                        json!({"appDispatches":lane["dispatches"],"appUserObjects":lane["user_messages"],"appToolObjects":lane["tool_messages"]}),
                    );
                    if config.history.mode == codex_chatgpt_provider::config::HistoryMode::Project {
                        assert!(lane["project"].as_str().unwrap().starts_with("g-p-"));
                        assert_eq!(
                            live.remote
                                .operation(json!({"op":"project_membership","lane":lane["marker"]}))
                                .await
                                .unwrap(),
                            json!({"verified":true})
                        );
                    }
                    live.remote
                        .operation(json!({"op":"release","lane":lane["marker"]}))
                        .await
                        .unwrap();
                }
            }
        }
        live.remote
            .operation(json!({"op":"dispose"}))
            .await
            .unwrap();
        if let Some(owned) = &mut owned {
            owned.shutdown().await;
        }
        assert!(observation.is_some());
        println!(
            "{}",
            json!({"accepted":true,"liveNoMcp":true,"rustOwnedAuthenticatedApp":owned.is_some(),"directAiDevelopment":development.is_some(),"nativeEffects":if development.is_some(){Value::Null}else{json!(3)},"providerSubmissions":submissions.len(),"observation":observation,"proSampled":false,"automaticSubmissionRetry":false,"independentDiffReviewRequired":development.is_some()})
        );
        if development.is_some() {
            let destination =
                PathBuf::from(std::env::var_os("CODEX_PROVIDER_LIVE_TRANSCRIPT").unwrap())
                    .parent()
                    .unwrap()
                    .join("ai-development-state");
            assert!(
                !destination.exists(),
                "never overwrite previous development evidence"
            );
            fs::rename(&root.0, &destination).unwrap();
            root.0 = destination;
        } else {
            root.1 = false;
        }
        return;
    }
    let s = app.0.lock().unwrap();
    let submissions = s
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(
        submissions.len(),
        if mcp { 6 } else { 4 } + usize::from(reject_first)
    );
    assert_eq!(
        fs::read_to_string(root.0.join("work/patched.txt")).unwrap(),
        "real custom tool\n"
    );
    assert!(!stdout.contains("Model metadata for"));
    assert_eq!(
        native_requests.load(Ordering::Relaxed),
        if mcp { 6 } else { 4 },
        "gateway correction stays within the current native Responses request"
    );
    for (index, command) in submissions[1..].iter().enumerate() {
        assert!(command["messages"].as_array().unwrap().is_empty());
        if !mcp || index != usize::from(reject_first) {
            assert!(command["instructions"].as_array().unwrap().is_empty());
        } else {
            assert!(
                !command["instructions"].as_array().unwrap().is_empty(),
                "new deferred declarations must reach the model before it invokes them"
            );
        }
        assert_eq!(command["results"].as_array().unwrap().len(), 1);
    }
    if reject_first {
        assert_eq!(submissions[1]["results"][0]["id"], "rejected-host-call");
        assert_eq!(
            submissions[1]["results"][0]["result"]["error"]["code"],
            "unadvertised_tool"
        );
        assert!(stdout.contains("Provider rejected a tool request before execution"));
    }
    if mcp {
        let calls = mcp_calls.lock().unwrap();
        assert_eq!(
            calls.iter().filter(|v| v["method"] == "tools/call").count(),
            1,
            "native MCP effect executes exactly once"
        );
        assert!(calls.iter().any(|v| v["method"] == "tools/list"));
        assert!(
            stdout.contains("mcp_tool_call") && stdout.contains("native_mcp_verified"),
            "native transcript must expose the actual tool/result"
        );
    }
    println!(
        "{}",
        json!({"accepted":true,"runtime":runtime,"process":Process::read(std::process::id()).unwrap(),"fixtureHostSubmissions":submissions.len(),"nativeResponsesRequests":native_requests.load(Ordering::Relaxed),"gatewayErrorReturns":usize::from(reject_first),"fixtureUserObjects":submissions.iter().flat_map(|v|v["messages"].as_array().unwrap().iter()).filter(|v|v["role"]=="user").count(),"fixtureVisibleUserObjects":submissions.iter().flat_map(|v|v["messages"].as_array().unwrap().iter()).filter(|v|v["role"]=="user" && v["hidden"]==false).count(),"fixtureToolReturns":if mcp{5}else{3},"nativeEffects":if mcp{4}else{3},"nativeMcpDiscovery":mcp,"rejectedCallCorrected":reject_first,"chatSubmissions":0,"fixtureRemovedOnDrop":true})
    );
}

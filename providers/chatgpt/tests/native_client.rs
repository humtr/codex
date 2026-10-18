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
    sync::{Arc, Mutex},
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
}
#[derive(Clone, Default)]
struct App(Arc<Mutex<State>>);
struct Live {
    remote: codex_chatgpt_provider::app::Remote,
    commands: Mutex<Vec<Value>>,
}
impl Bridge for Live {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        if command["op"] == "submit" {
            let mut commands = self.commands.lock().unwrap();
            if commands.iter().filter(|v| v["op"] == "submit").count() >= 4 {
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
                    let message = if s.stage < 4 {
                        let script = if s.stage == 1 {
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
                        let (key, input) = if s.stage == 2 {
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
                "stop" => Err(Error("fixture_unexpected_stop")),
                _ => Err(Error("fixture_unexpected_operation")),
            }
        })
    }
}
struct Root(PathBuf, bool);
impl Drop for Root {
    fn drop(&mut self) {
        if self.1 {
            eprintln!(
                "unfinished live qualification preserved at {}",
                self.0.display()
            );
            return;
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn actual_native_client_executes_commands_custom_patch_and_validation_with_generated_catalog()
{
    let runtime = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_NATIVE").expect("qualified runtime path required"),
    );
    let live_descriptor = std::env::var_os("CODEX_PROVIDER_LIVE_DESCRIPTOR");
    if live_descriptor.is_some() {
        assert_eq!(
            std::env::var("CODEX_PROVIDER_LIVE_CHAT").as_deref(),
            Ok("qualified-gpt6-no-pro"),
            "live qualification requires separate explicit arming"
        );
    }
    let mut live_transcript = live_descriptor.as_ref().map(|_| {
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
    let mut root = Root(
        std::env::temp_dir().join(format!("pn-{}", &random_id().unwrap()[..8])),
        live_descriptor.is_some(),
    );
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(&root.0).unwrap();
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
    let journal = Arc::new(Journal::open(&root.0.join("journal")).unwrap());
    let live = live_descriptor.map(|path| {
        Arc::new(Live {
            remote: codex_chatgpt_provider::app::Remote::new(path.into(), journal.owner.clone()),
            commands: Mutex::new(Vec::new()),
        })
    });
    let bridge: Arc<dyn Bridge> = match &live {
        Some(live) => live.clone(),
        None => app.clone(),
    };
    let host = Arc::new(AppHost::new(bridge, journal.clone()));
    let token = random_id().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    journal.write(&codex_chatgpt_provider::context::digest(&"termux-codex-responses-service-v1"),&json!({"schema":"termux-codex-responses-service-v1","process":Process::read(std::process::id()).unwrap(),"port":port,"token":token})).unwrap();
    codex_chatgpt_provider::registration::prepare(
        &Config::default(),
        &root.0.join("journal"),
        &root.0.join("registry"),
        &root.0.join("codex"),
        std::path::Path::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider")),
        &format!("http://127.0.0.1:{port}/v1"),
    )
    .unwrap();
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let service = Service::new(Config::default(), token.clone(), host.clone()).unwrap();
    let router = service.router().layer(axum::middleware::from_fn(
        async |request: axum::extract::Request, next: axum::middleware::Next| {
            let (parts, body) = request.into_parts();
            let bytes = axum::body::to_bytes(body, 32 * 1024 * 1024).await.unwrap();
            if parts.uri.path() == "/v1/responses" {
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
        .current_dir(root.0.join("work"))
        .args([
            "exec",
            "--json",
            "--skip-git-repo-check",
            "--dangerously-bypass-approvals-and-sandbox",
            "--profile",
            "chatgpt",
            if live.is_some(){"Run this exact bounded local transport qualification. Use exec_command once to execute: printf 'effect-1\\n' >> effect-count . Then use apply_patch to add patched.txt containing the one line: real custom tool . Then use exec_command to execute: test \"$(wc -l < effect-count)\" -eq 1 && grep -qx 'real custom tool' patched.txt && printf 'effect-2\\n' >> effect-count . Do not use any other tools, commands, network or directories. After successful validation, answer exactly: Native executor verified"}else{"Execute the fixture tools and return the verified result"},
        ])
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
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(if live.is_some() { 240 } else { 90 }),
        command.output(),
    )
    .await
    .expect("native execution timeout")
    .unwrap();
    shutdown.cancel();
    server.await.unwrap();
    host.shutdown().await;
    if let Some(file) = &mut live_transcript {
        file.write_all(&result.stdout).unwrap();
        file.sync_all().unwrap();
    }
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.success(),
        "native exit {:?}\nstdout: {stdout}\nstderr: {stderr}",
        result.status.code()
    );
    assert!(
        stdout.contains("Native executor verified"),
        "{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("command_execution"),
        "native transcript omitted tool execution: {stdout}"
    );
    assert_eq!(
        fs::read_to_string(root.0.join("work/effect-count")).unwrap(),
        "effect-1\neffect-2\n"
    );
    if let Some(live) = live {
        assert_eq!(
            fs::read_to_string(root.0.join("work/patched.txt")).unwrap(),
            "real custom tool\n"
        );
        let submissions = live.commands.lock().unwrap().clone();
        assert!((2..=4).contains(&submissions.len()));
        for command in &submissions[1..] {
            assert!(command["messages"].as_array().unwrap().is_empty());
            assert!(command["instructions"].as_array().unwrap().is_empty());
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
        assert!(observation.is_some());
        println!(
            "{}",
            json!({"accepted":true,"liveNoMcp":true,"nativeEffects":3,"providerSubmissions":submissions.len(),"observation":observation,"proSampled":false,"automaticSubmissionRetry":false})
        );
        root.1 = false;
        return;
    }
    let s = app.0.lock().unwrap();
    let submissions = s
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submissions.len(), 4);
    assert_eq!(
        fs::read_to_string(root.0.join("work/patched.txt")).unwrap(),
        "real custom tool\n"
    );
    assert!(!stdout.contains("Model metadata for"));
    for command in &submissions[1..] {
        assert!(command["messages"].as_array().unwrap().is_empty());
        assert!(command["instructions"].as_array().unwrap().is_empty());
        assert_eq!(command["results"].as_array().unwrap().len(), 1);
    }
    println!(
        "{}",
        json!({"accepted":true,"runtime":runtime,"process":Process::read(std::process::id()).unwrap(),"fixtureHostSubmissions":submissions.len(),"fixtureUserObjects":submissions.iter().flat_map(|v|v["messages"].as_array().unwrap().iter()).filter(|v|v["role"]=="user").count(),"fixtureVisibleUserObjects":submissions.iter().flat_map(|v|v["messages"].as_array().unwrap().iter()).filter(|v|v["role"]=="user" && v["hidden"]==false).count(),"fixtureToolReturns":3,"nativeEffects":3,"chatSubmissions":0,"fixtureRemovedOnDrop":true})
    );
}

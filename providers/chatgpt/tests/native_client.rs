//! Explicit Android acceptance: a real qualified Codex executable drives the
//! production Rust Responses/lane stack. Only the app-side replies are synthetic.
//! No OpenAI/ChatGPT inference, credentials, or user workspace are used.
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
    commands: Vec<Value>,
}
#[derive(Clone, Default)]
struct App(Arc<Mutex<State>>);
impl Bridge for App {
    fn operation(&self, command: Value) -> BoxFuture<'static, Result<Value, Error>> {
        let this = self.clone();
        Box::pin(async move {
            let mut s = this.0.lock().unwrap();
            s.commands.push(command.clone());
            match command["op"].as_str().unwrap() {
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
                    let message = if s.stage < 3 {
                        let script = if s.stage == 1 {
                            "printf 'effect-1\\n' >> effect-count; printf 'first result'"
                        } else {
                            "test \"$(wc -l < effect-count)\" -eq 1 && printf 'effect-2\\n' >> effect-count && printf 'verified'"
                        };
                        let args = if s.key.ends_with("exec_command") {
                            json!({"cmd":script,"max_output_tokens":1000})
                        } else if s.key.ends_with("shell_command") {
                            json!({"command":script,"timeout_ms":10000,"max_output_chars":1000})
                        } else {
                            json!({"command":["bash","-c",script],"timeout_ms":10000})
                        };
                        json!({"id":format!("host-call-{}",s.stage),"author":{"role":"assistant"},"recipient":format!("local.{}",s.gateway),"content":{"content_type":"code","text":json!({"kwargs":{"name":s.key,"input":args.to_string(),"report":format!("Native fixture step {}",s.stage)}}).to_string()},"status":"finished_successfully","end_turn":false})
                    } else {
                        json!({"id":"final","author":{"role":"assistant"},"recipient":"all","channel":"final","content":{"content_type":"text","parts":["Native executor verified"]},"status":"finished_successfully","end_turn":true})
                    };
                    Ok(
                        json!({"round":s.round,"messages":[message],"terminals":["done"],"roundStarts":1,"userMessages":1,"toolMessages":s.stage-1,"dispatches":s.stage}),
                    )
                }
                "stop" => Err(Error("fixture_unexpected_stop")),
                _ => Err(Error("fixture_unexpected_operation")),
            }
        })
    }
}
struct Root(PathBuf);
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires explicit CODEX_PROVIDER_NATIVE pointing to a qualified Android runtime"]
async fn actual_native_client_executes_two_dependent_tools_and_retains_transcript() {
    let runtime = PathBuf::from(
        std::env::var_os("CODEX_PROVIDER_NATIVE").expect("qualified runtime path required"),
    );
    let prefix = PathBuf::from(std::env::var_os("PREFIX").expect("Termux prefix required"));
    assert!(runtime.is_absolute());
    let root = Root(std::env::temp_dir().join(format!("pn-{}", &random_id().unwrap()[..8])));
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
    let host = Arc::new(AppHost::new(
        app.clone(),
        Arc::new(Journal::open(&root.0.join("journal")).unwrap()),
    ));
    let token = random_id().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let service = Service::new(Config::default(), token.clone(), host).unwrap();
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
                    "fixture declaration kinds: {:?}",
                    value["tools"]
                        .as_array()
                        .unwrap()
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
    command.env_clear().env("PATH",prefix.join("bin")).env("PREFIX",&prefix).env("HOME",root.0.join("home")).env("CODEX_HOME",root.0.join("codex")).env("TMPDIR",root.0.join("tmp")).env("TERM","dumb").env("TERMUX_FIXTURE_API_KEY",token)
  .current_dir(root.0.join("work")).args(["exec","--json","--skip-git-repo-check","--dangerously-bypass-approvals-and-sandbox","-c","features.code_mode=false","-c","model_context_window=1000000","-c","model_provider=\"termuxfixture\"","-c","model=\"chatgpt-desktop/gpt-6\"","-c",&format!("model_providers.termuxfixture={{name=\"Owned fixture\",base_url=\"http://127.0.0.1:{port}/v1\",env_key=\"TERMUX_FIXTURE_API_KEY\",wire_api=\"responses\",request_max_retries=0,stream_max_retries=0}}"),"Execute the fixture tools and return the verified result"])
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
    let result = tokio::time::timeout(std::time::Duration::from_secs(90), command.output())
        .await
        .expect("native execution timeout")
        .unwrap();
    shutdown.cancel();
    server.await.unwrap();
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
    let s = app.0.lock().unwrap();
    let submissions = s
        .commands
        .iter()
        .filter(|c| c["op"] == "submit")
        .collect::<Vec<_>>();
    assert_eq!(submissions.len(), 3);
    for command in &submissions[1..] {
        assert!(command["messages"].as_array().unwrap().is_empty());
        assert!(command["instructions"].as_array().unwrap().is_empty());
        assert_eq!(command["results"].as_array().unwrap().len(), 1);
    }
    println!(
        "{}",
        json!({"accepted":true,"runtime":runtime,"process":Process::read(std::process::id()).unwrap(),"appSubmissions":3,"appUserObjects":1,"appToolReturns":2,"nativeEffects":2,"chatSubmissions":0,"fixtureRemovedOnDrop":true})
    );
}

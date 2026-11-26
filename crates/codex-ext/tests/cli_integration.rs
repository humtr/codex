use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn ext_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_codex-ext"))
}

fn temp_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_fake_codex(root: &Path, trace: &Path) -> PathBuf {
    let bin = root.join("usr/bin/codex");
    fs::create_dir_all(bin.parent().unwrap()).unwrap();
    fs::write(
        &bin,
        format!(
            "#!/bin/sh\n\
             printf 'HOME=%s SHARED=%s EXTRA=%s ARGS=%s\\n' \"${{CODEX_HOME:-DEFAULT}}\" \"${{CODEX_SHARED_HOME:-NONE}}\" \"${{CODEX_SHARED_EXTRA_FILES:-NONE}}\" \"$*\" >> '{}'\n\
             if [ \"$1\" = 'app-server' ]; then\n\
               printf '{{\"status\":\"stopped\"}}\\n'\n\
               exit 0\n\
             fi\n\
             if [ \"$1\" = 'resume' ] || [ \"$1\" = '-p' ]; then exit 0; fi\n\
             trap 'exit 0' TERM\n\
             sleep 15\n",
            trace.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn wait_for_session_count(sessions_dir: &Path, expected: usize) -> Vec<u32> {
    for _ in 0..150 {
        if let Ok(rd) = fs::read_dir(sessions_dir) {
            let mut pids: Vec<u32> = rd
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    e.path()
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.parse::<u32>().ok())
                })
                .collect();
            pids.sort();
            if pids.len() == expected {
                return pids;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "timed out waiting for {expected} sessions in {}",
        sessions_dir.display()
    );
}

#[test]
fn multi_window_disambiguation_and_provider_switch_end_to_end() {
    let root = temp_root("cdx-multiwin");
    let work = root.join("work");
    let tmp = root.join("tmp");
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&tmp).unwrap();
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(
        root.join(".codex/chatgpt.config.toml"),
        "model = \"gpt-5.4\"\n",
    )
    .unwrap();

    let trace = root.join("trace.log");
    let fake_codex = make_fake_codex(&root, &trace);

    // Create profile szob via CLI
    let r_create = Command::new(ext_bin())
        .args(["profile", "create", "szob"])
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .output()
        .unwrap();
    assert!(
        r_create.status.success(),
        "{}",
        String::from_utf8_lossy(&r_create.stderr)
    );

    // Launch TWO concurrent `cdx` sessions in the same HOME and cwd!
    let mut win1 = Command::new(ext_bin())
        .current_dir(&work)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut win2 = Command::new(ext_bin())
        .current_dir(&work)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let sessions_dir = root.join(".local/state/codex/ext/sessions");
    let pids = wait_for_session_count(&sessions_dir, 2);
    assert!(pids.contains(&win1.id()));
    assert!(pids.contains(&win2.id()));

    // Ambiguous `!cdx switch szob` without --session MUST fail closed (exit 2) and list both PIDs!
    let amb = Command::new(ext_bin())
        .args(["switch", "szob", "tid-1"])
        .current_dir(&work)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .output()
        .unwrap();
    assert_eq!(amb.status.code(), Some(2));
    let amb_err = String::from_utf8_lossy(&amb.stderr);
    assert!(amb_err.contains(&win1.id().to_string()), "{amb_err}");
    assert!(amb_err.contains(&win2.id().to_string()), "{amb_err}");

    // Targeted `--session <win1>` switches ONLY win1 to szob (`resume tid-1` exits 0)
    let sw1 = Command::new(ext_bin())
        .args([
            "switch",
            "szob",
            "tid-1",
            "--session",
            &win1.id().to_string(),
        ])
        .current_dir(&work)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .output()
        .unwrap();
    assert!(
        sw1.status.success(),
        "{}",
        String::from_utf8_lossy(&sw1.stderr)
    );
    let st1 = win1.wait().unwrap();
    assert!(st1.success());

    // Now only win2 remains live; `!cdx provider chatgpt` matches win2 unambiguously!
    let _ = wait_for_session_count(&sessions_dir, 1);
    let prov = Command::new(ext_bin())
        .args(["provider", "chatgpt"])
        .current_dir(&work)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .output()
        .unwrap();
    assert!(
        prov.status.success(),
        "{}",
        String::from_utf8_lossy(&prov.stderr)
    );
    let st2 = win2.wait().unwrap();
    assert!(st2.success());

    let trace_text = fs::read_to_string(&trace).unwrap();
    assert!(trace_text.contains("ARGS=resume tid-1"), "{trace_text}");
    assert!(trace_text.contains("EXTRA=hooks.json"), "{trace_text}");
    assert!(trace_text.contains("ARGS=-p chatgpt"), "{trace_text}");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn busy_daemon_blocks_unrelated_switch_and_defers_the_callers_own_switch() {
    exercise_deferred_handoff(["switch", "szob"], "resume active-tid-42");
}

#[test]
fn provider_handoff_waits_for_its_shell_turn_and_leaves_the_daemon_running() {
    exercise_deferred_handoff(["provider", "chatgpt"], "-p chatgpt");
}

#[test]
fn queued_handoff_survives_the_tui_exiting_before_the_launcher_reads_it() {
    let root = temp_root("cdx-exit-handoff");
    let fake = root.join("codex");
    let trace = root.join("trace");
    let child_pid = root.join("tui-pid");
    let exit_gate = root.join("exit-tui");
    fs::write(
        &fake,
        format!(
            concat!(
                "#!/bin/sh\n",
                "if [ \"$1\" = resume ]; then printf '%s\\n' \"$*\" > '{}'; exit 0; fi\n",
                "echo $$ > '{}'\n",
                "for i in $(seq 100); do [ -f '{}' ] && exit 0; sleep 0.05; done\n",
            ),
            trace.display(),
            child_pid.display(),
            exit_gate.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let command = || {
        let mut cmd = Command::new(ext_bin());
        cmd.env("HOME", &root)
            .env("PREFIX", root.join("usr"))
            .env("TMPDIR", &root)
            .env("CODEX_EXT_CODEX_BIN", &fake)
            .env_remove("CODEX_HOME")
            .env_remove("CODEX_THREAD_ID")
            .current_dir(&root);
        cmd
    };
    assert!(command()
        .args(["profile", "create", "fixture"])
        .output()
        .unwrap()
        .status
        .success());
    let mut launcher = command().stderr(Stdio::null()).spawn().unwrap();
    let pid = launcher.id() as libc::pid_t;
    let end = std::time::Instant::now() + Duration::from_secs(3);
    let child = loop {
        if let Some(id) = fs::read_to_string(&child_pid)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
        {
            break id;
        }
        assert!(std::time::Instant::now() < end, "fixture TUI did not start");
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
    let queued = command().args(["switch", "fixture", "kept-tid"]).output();
    let competing = command()
        .args(["switch", "fixture", "discarded-tid"])
        .output();
    let released = fs::write(&exit_gate, "");
    let end = std::time::Instant::now() + Duration::from_secs(3);
    let mut child_exited = false;
    while std::time::Instant::now() < end {
        child_exited = fs::read_to_string(format!("/proc/{child}/stat")).is_ok_and(|stat| {
            stat.rsplit_once(") ")
                .is_some_and(|(_, fields)| fields.starts_with("Z "))
        });
        if child_exited {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(unsafe { libc::kill(pid, libc::SIGCONT) }, 0);
    let status = launcher.wait().unwrap();
    assert!(released.is_ok() && child_exited && status.success());
    assert!(queued.unwrap().status.success());
    let competing = competing.unwrap();
    assert!(!competing.status.success());
    assert!(String::from_utf8_lossy(&competing.stderr).contains("already pending"));
    assert_eq!(fs::read_to_string(trace).unwrap(), "resume kept-tid\n");
    fs::remove_dir_all(root).unwrap();
}

fn exercise_deferred_handoff(args: [&str; 2], expected_codex_args: &str) {
    let root = temp_root("cdx-busy");
    let work = root.join("work");
    let tmp = root.join("tmp");
    let codex_home = root.join(".codex");
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&tmp).unwrap();
    fs::create_dir_all(codex_home.join("app-server-control")).unwrap();

    let sock_path = tmp.join("busy.sock");
    let listener = UnixListener::bind(&sock_path).unwrap();
    std::os::unix::fs::symlink(
        "/proc/self/fd/35/busy.sock",
        codex_home.join("app-server-control/app-server-control.sock"),
    )
    .unwrap();

    let active = Arc::new(AtomicBool::new(true));
    let concurrent_turn = Arc::new(AtomicBool::new(false));
    let stopped = Arc::new(AtomicBool::new(false));
    let server_active = Arc::clone(&active);
    let server_concurrent_turn = Arc::clone(&concurrent_turn);
    let server_stopped = Arc::clone(&stopped);
    let server = thread::spawn(move || {
        for incoming in listener.incoming() {
            let stream = incoming.unwrap();
            if server_stopped.load(Ordering::Acquire) {
                break;
            }
            let mut writer = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            let mut key = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        key = v.trim().to_owned();
                    }
                }
            }
            // Compute RFC 6455 accept via python3? No, keep test pure Rust using simple handshake helper.
            // Instead compute SHA-1 inline:
            let accept = ws_accept(&key);
            write!(
            writer,
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        )
        .unwrap();
            writer.flush().unwrap();

            loop {
                let mut hdr = [0u8; 2];
                if reader.read_exact(&mut hdr).is_err() {
                    break;
                }
                let op = hdr[0] & 0x0f;
                if op == 0x8 {
                    break;
                }
                let masked = (hdr[1] & 0x80) != 0;
                let mut len = (hdr[1] & 0x7f) as usize;
                if len == 126 {
                    let mut ext = [0u8; 2];
                    reader.read_exact(&mut ext).unwrap();
                    len = u16::from_be_bytes(ext) as usize;
                }
                let mut mask = [0u8; 4];
                if masked {
                    reader.read_exact(&mut mask).unwrap();
                }
                let mut data = vec![0u8; len];
                reader.read_exact(&mut data).unwrap();
                if masked {
                    for (i, b) in data.iter_mut().enumerate() {
                        *b ^= mask[i & 3];
                    }
                }
                let msg: Value = serde_json::from_slice(&data).unwrap();
                let method = msg["method"].as_str().unwrap_or("");
                let id = msg.get("id").and_then(Value::as_u64);
                let resp = match method {
                    "initialize" => Some(json!({"id": id, "result": {}})),
                    "initialized" => None,
                    "thread/loaded/list" => {
                        let mut threads = vec!["active-tid-42"];
                        if server_concurrent_turn.load(Ordering::Acquire) {
                            threads.push("other-active-tid");
                        }
                        Some(json!({"id": id, "result": {"data": threads, "nextCursor": null}}))
                    }
                    "thread/read" => Some(json!({
                        "id": id,
                        "result": {
                            "thread": {
                                "id": msg["params"]["threadId"],
                                "status": {"type": if server_active.load(Ordering::Acquire) { "active" } else { "idle" }, "activeFlags": []},
                                "cwd": "/tmp"
                            }
                        }
                    })),
                    _ => Some(json!({"id": id, "result": {}})),
                };
                if let Some(r) = resp {
                    let bytes = r.to_string().into_bytes();
                    send_ws_unmasked(&mut writer, &bytes);
                }
            }
        }
    });

    let trace = root.join("trace.log");
    let fake_codex = make_fake_codex(&root, &trace);
    fs::write(codex_home.join("chatgpt.config.toml"), "# fixture\n").unwrap();
    Command::new(ext_bin())
        .args(["profile", "create", "szob"])
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .status()
        .unwrap();

    let run_switch = |caller: &str| {
        let mut command = Command::new(ext_bin());
        command
            .args(args)
            .current_dir(&work)
            .env("HOME", &root)
            .env("PREFIX", root.join("usr"))
            .env("TMPDIR", &tmp)
            .env("CODEX_EXT_CODEX_BIN", &fake_codex)
            .env_remove("CODEX_HOME")
            .env("CODEX_THREAD_ID", caller);
        command.output().unwrap()
    };
    let unmanaged = run_switch("active-tid-42");
    assert_eq!(unmanaged.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unmanaged.stderr).contains("no cdx launcher"));

    let mut win = Command::new(ext_bin())
        .current_dir(&work)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let sessions = root.join(".local/state/codex/ext/sessions");
    let _ = wait_for_session_count(&sessions, 1);

    let sw = run_switch("another-thread");
    assert_eq!(sw.status.code(), Some(1));
    let err = String::from_utf8_lossy(&sw.stderr);
    assert!(err.contains("active-tid-42"), "{err}");

    concurrent_turn.store(true, Ordering::Release);
    let blocked = run_switch("active-tid-42");
    assert_eq!(blocked.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("other-active-tid"));
    concurrent_turn.store(false, Ordering::Release);

    if args[0] == "provider" {
        let record_path = sessions.join(format!("{}.json", win.id()));
        let original = fs::read(&record_path).unwrap();
        let mut legacy: Value = serde_json::from_slice(&original).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("deferred_provider_switch");
        fs::write(&record_path, legacy.to_string()).unwrap();
        let rejected = run_switch("active-tid-42");
        assert_eq!(rejected.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("older cdx launcher"));
        assert!(win.try_wait().unwrap().is_none());
        assert!(!root
            .join(format!(".local/state/codex/ext/handoff/{}.json", win.id()))
            .exists());
        fs::write(&record_path, original).unwrap();
    }

    let queued = run_switch("active-tid-42");
    assert!(queued.status.success(), "{:?}", queued);
    assert!(String::from_utf8_lossy(&queued.stdout).contains("queued"));
    thread::sleep(Duration::from_millis(350));
    assert!(win.try_wait().unwrap().is_none());
    assert!(!fs::read_to_string(&trace)
        .unwrap()
        .contains("ARGS=app-server"));
    active.store(false, Ordering::Release);
    let mut outcome = None;
    for _ in 0..250 {
        if let Some(status) = win.try_wait().unwrap() {
            outcome = Some(status);
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    if outcome.is_none() {
        let _ = win.kill();
        let _ = win.wait();
    }
    stopped.store(true, Ordering::Release);
    let _wake_server = UnixStream::connect(sock_path).unwrap();
    server.join().unwrap();
    assert!(outcome.is_some_and(|status| status.success()));
    let actual = fs::read_to_string(&trace).unwrap();
    let expected_home = if args[0] == "provider" {
        assert!(!actual.contains("ARGS=app-server"), "{actual}");
        "DEFAULT".to_owned()
    } else {
        root.join(".local/share/codex/manager/profiles/szob/home")
            .display()
            .to_string()
    };
    assert!(
        actual
            .lines()
            .any(|line| line.starts_with(&format!("HOME={expected_home} "))
                && line.ends_with(&format!("ARGS={expected_codex_args}"))),
        "{actual}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn notify_set_show_and_emit_dry_run_work_for_stop_and_hooks_json() {
    let root = temp_root("cdx-notify-cli");
    let tmp = root.join("tmp");
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::create_dir_all(&tmp).unwrap();

    let r_set = Command::new(ext_bin())
        .args([
            "notify",
            "set",
            "--enable",
            "--backend",
            "both",
            "--events",
            "Stop,PermissionRequest",
        ])
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .output()
        .unwrap();
    assert!(
        r_set.status.success(),
        "{}",
        String::from_utf8_lossy(&r_set.stderr)
    );
    let out = String::from_utf8_lossy(&r_set.stdout);
    assert!(
        out.contains("hooks.json require a one-time hook trust approval"),
        "{out}"
    );

    let r_show = Command::new(ext_bin())
        .args(["notify", "show", "--json"])
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .output()
        .unwrap();
    assert!(r_show.status.success());
    let v: Value = serde_json::from_slice(&r_show.stdout).unwrap();
    assert_eq!(v["enabled"], true);
    assert_eq!(v["config_toml_stop_notify"], true);
    assert_eq!(v["hooks_json_events"], json!(["PermissionRequest"]));

    // Emit Stop with trailing JSON arg (matching upstream config.toml notify)
    let r_emit = Command::new(ext_bin())
        .args([
            "notify",
            "emit",
            "Stop",
            r#"{"cwd":"/tmp/demo","last-assistant-message":"Done building package"}"#,
            "--dry-run",
        ])
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .output()
        .unwrap();
    assert!(r_emit.status.success());
    let ev: Value = serde_json::from_slice(&r_emit.stdout).unwrap();
    assert_eq!(ev["title"], "Codex (demo) — Stop");
    assert_eq!(ev["body"], "Done building package");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn notify_stop_conflict_preserves_command_and_preferences() {
    let root = temp_root("cdx-notify-conflict");
    let config = root.join(".codex/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    let original = "\"notify\" = [\n  'user-notifier', # keep\n  '--arg',\n]\n";
    fs::write(&config, original).unwrap();
    let prefs = root.join(".config/codex/ext/notify.json");
    let run = |args: &[&str]| {
        Command::new(ext_bin())
            .args(args)
            .env("HOME", &root)
            .env("PREFIX", root.join("usr"))
            .env("TMPDIR", root.join("tmp"))
            .output()
            .unwrap()
    };
    let initial = run(&["notify", "set", "--enable", "--events", "PermissionRequest"]);
    assert!(
        initial.status.success(),
        "{}",
        String::from_utf8_lossy(&initial.stderr)
    );
    let before = fs::read(&prefs).unwrap();
    let hooks = root.join(".codex/hooks.json");
    let before_hooks = fs::read(&hooks).unwrap();
    let failed = run(&[
        "notify",
        "set",
        "--enable",
        "--events",
        "Stop",
        "--backend",
        "both",
    ]);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("different notify command"));
    assert!(!String::from_utf8_lossy(&failed.stdout).contains("notify updated"));
    assert_eq!(fs::read_to_string(&config).unwrap(), original);
    assert_eq!(fs::read(&prefs).unwrap(), before);
    assert_eq!(fs::read(&hooks).unwrap(), before_hooks);
    let shown = run(&["notify", "show", "--json"]);
    assert!(shown.status.success());
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["events"], json!(["PermissionRequest"]));
    assert_eq!(shown["config_toml_stop_notify"], false);

    // Equivalent, non-canonical TOML is already the managed command.
    let managed = "'notify' = [\n 'codex-ext', 'notify', 'emit', 'Stop',\n]\n";
    fs::write(&config, managed).unwrap();
    assert!(run(&["notify", "set", "--enable", "--events", "Stop"])
        .status
        .success());
    assert_eq!(fs::read_to_string(&config).unwrap(), managed);
    assert!(run(&["notify", "set", "--disable"]).status.success());
    assert!(!fs::read_to_string(&config).unwrap().contains("notify"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parent_launcher_survives_sigint_and_fails_closed_on_corrupt_daemon_probe() {
    let root = temp_root("cdx-sigint-failclosed");
    let work_real = root.join("work-real");
    let work_link = root.join("work-link");
    let tmp = root.join("tmp");
    let codex_home = root.join(".codex");
    fs::create_dir_all(&work_real).unwrap();
    fs::create_dir_all(&tmp).unwrap();
    fs::create_dir_all(codex_home.join("app-server-control")).unwrap();
    std::os::unix::fs::symlink(&work_real, &work_link).unwrap();

    // Fake codex catches SIGINT (like Codex TUI!) and exits cleanly after 250ms.
    let fake_codex = root.join("usr/bin/codex");
    fs::create_dir_all(fake_codex.parent().unwrap()).unwrap();
    fs::write(&fake_codex, "#!/bin/sh\ntrap '' INT\nsleep 0.4\nexit 0\n").unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();

    let mut parent = Command::new(ext_bin())
        .current_dir(&work_link)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _ = wait_for_session_count(&root.join(".local/state/codex/ext/sessions"), 1);

    // Send SIGINT directly to parent codex-ext: parent MUST NOT die!
    unsafe {
        libc::kill(parent.id() as libc::pid_t, libc::SIGINT);
    }
    thread::sleep(Duration::from_millis(80));
    assert!(
        parent.try_wait().unwrap().is_none(),
        "parent codex-ext must survive SIGINT while child runs"
    );
    let st = parent.wait().unwrap();
    assert_eq!(st.code(), Some(0));

    // Now bind a broken UDS socket that immediately sends invalid HTTP (not 101)
    let broken_sock = tmp.join("broken.sock");
    let listener = UnixListener::bind(&broken_sock).unwrap();
    std::os::unix::fs::symlink(
        "/proc/self/fd/35/broken.sock",
        codex_home.join("app-server-control/app-server-control.sock"),
    )
    .unwrap();
    let srv = thread::spawn(move || {
        if let Ok((mut s, _)) = listener.accept() {
            let _ = s.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n");
        }
    });

    Command::new(ext_bin())
        .args(["profile", "create", "szob"])
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .status()
        .unwrap();

    // Standalone switch against broken daemon MUST fail closed (not treat daemon as NotRunning!)
    let r = Command::new(ext_bin())
        .args(["switch", "szob"])
        .current_dir(&work_real)
        .env("HOME", &root)
        .env("PREFIX", root.join("usr"))
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_HOME")
        .env_remove("CODEX_THREAD_ID")
        .output()
        .unwrap();
    assert!(!r.status.success());
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(err.contains("fail-closed"), "{err}");
    srv.join().unwrap();
    let _ = fs::remove_dir_all(&root);
}

fn send_ws_unmasked(writer: &mut UnixStream, payload: &[u8]) {
    let mut hdr = vec![0x81u8];
    if payload.len() < 126 {
        hdr.push(payload.len() as u8);
    } else {
        hdr.push(126);
        hdr.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    writer.write_all(&hdr).unwrap();
    writer.write_all(payload).unwrap();
    writer.flush().unwrap();
}

fn ws_accept(key: &str) -> String {
    let mut input = key.as_bytes().to_vec();
    input.extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let digest = sha1(&input);
    b64(&digest)
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let (mut h0, mut h1, mut h2, mut h3, mut h4) = (
        0x6745_2301u32,
        0xEFCD_AB89u32,
        0x98BA_DCFEu32,
        0x1032_5476u32,
        0xC3D2_E1F0u32,
    );
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut w = [0u32; 80];
    for chunk in msg.as_chunks::<64>().0 {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDCu32),
                _ => (b ^ c ^ d, 0xCA62_C1D6u32),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, h) in [h0, h1, h2, h3, h4].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&h.to_be_bytes());
    }
    out
}

fn b64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        s.push(T[((n >> 18) & 63) as usize] as char);
        s.push(T[((n >> 12) & 63) as usize] as char);
        s.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        s.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    s
}

#[test]
fn stop_wakes_live_loopback_relay_and_fails_open_when_absent_or_stopped() {
    use std::net::TcpListener;
    use std::time::Instant;

    let root = temp_root("cdx-relay-wake");
    let dir = root.join(".local/state/codex/services/termux-orch-relay");
    let emit = || {
        Command::new(ext_bin())
            .args(["notify", "emit", "Stop", "{}"])
            .env("HOME", &root)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let absent = emit();
    assert!(absent.status.success() && absent.stderr.is_empty());
    assert!(!dir.exists());

    fs::create_dir_all(&dir).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let token = "a".repeat(64);
    for (name, value) in [
        ("health.url", format!("http://{addr}\n")),
        ("wake.token", token.clone()),
    ] {
        let path = dir.join(name);
        fs::write(&path, value).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let delivered = emit();
    assert!(delivered.status.success() && delivered.stderr.is_empty());
    listener.set_nonblocking(true).unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut request = String::new();
    stream.read_to_string(&mut request).unwrap();
    assert!(request.starts_with("POST /wake HTTP/1.1\r\n"));
    assert!(request.contains(&format!("Authorization: Bearer {token}\r\n")));
    drop(listener);

    let start = Instant::now();
    let stopped = emit();
    assert!(stopped.status.success() && stopped.stderr.is_empty());
    assert!(start.elapsed() < Duration::from_millis(1500));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn profile_flag_records_provider_and_shares_provider_config_on_named_profiles() {
    let root = temp_root("cdx-prof-flag");
    let work = root.join("work");
    let tmp = root.join("tmp");
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&tmp).unwrap();
    fs::create_dir_all(root.join(".codex")).unwrap();
    fs::write(
        root.join(".codex/chatgpt.config.toml"),
        "model = \"gpt-6.2-sol\"\n",
    )
    .unwrap();

    let created = Command::new(ext_bin())
        .args(["profile", "create", "szob"])
        .env("HOME", &root)
        .env("TMPDIR", &tmp)
        .output()
        .unwrap();
    assert!(created.status.success());

    let trace = root.join("trace.log");
    let snap = root.join("session_snap.json");
    let fake_codex = root.join("usr/bin/codex");
    fs::create_dir_all(fake_codex.parent().unwrap()).unwrap();
    fs::write(
        &fake_codex,
        format!(
            "#!/bin/sh\n\
             printf 'EXTRA=%s HOME=%s ARGS=%s\\n' \"${{CODEX_SHARED_EXTRA_FILES:-NONE}}\" \"${{CODEX_HOME:-DEFAULT}}\" \"$*\" >> '{}'\n\
             cat \"{}\"/*.json > '{}' 2>/dev/null || true\n\
             exit 0\n",
            trace.display(),
            root.join(".local/state/codex/ext/sessions").display(),
            snap.display(),
        ),
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new(ext_bin())
        .args(["use", "szob", "-p", "chatgpt"])
        .current_dir(&work)
        .env("HOME", &root)
        .env("TMPDIR", &tmp)
        .env("CODEX_EXT_CODEX_BIN", &fake_codex)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cdx use szob -p chatgpt failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let logged = fs::read_to_string(&trace).unwrap();
    assert!(
        logged.contains("hooks.json:chatgpt.config.toml"),
        "{logged}"
    );
    assert!(logged.contains("ARGS=-p chatgpt"), "{logged}");

    let session_raw = fs::read_to_string(&snap).unwrap();
    let rec: Value = serde_json::from_str(&session_raw).unwrap();
    assert_eq!(rec["provider"].as_str(), Some("chatgpt"));
    assert_eq!(rec["account"].as_str(), Some("szob"));

    for bad in ["--profile=openai", "--profile=../evil"] {
        let _ = fs::remove_file(&trace);
        let _ = fs::remove_file(&snap);
        let r = Command::new(ext_bin())
            .args(["use", "szob", bad])
            .current_dir(&work)
            .env("HOME", &root)
            .env("TMPDIR", &tmp)
            .env("CODEX_EXT_CODEX_BIN", &fake_codex)
            .output()
            .unwrap();
        assert!(r.status.success());
        let logged = fs::read_to_string(&trace).unwrap();
        assert!(logged.contains("EXTRA=hooks.json "), "{logged}");
        assert!(!logged.contains("chatgpt.config.toml"), "{logged}");
        let rec: Value = serde_json::from_str(&fs::read_to_string(&snap).unwrap()).unwrap();
        assert!(
            rec["provider"].is_null(),
            "expected null provider for {bad}: {rec}"
        );
    }

    fs::remove_dir_all(root).unwrap();
}

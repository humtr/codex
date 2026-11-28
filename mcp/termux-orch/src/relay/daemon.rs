use super::{relay_log, RelayEngine};
use crate::ledger::Ledger;
use crate::owners::OwnerClient;
use serde_json::json;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_sig: libc::c_int) {
    STOP_FLAG.store(true, Ordering::Release);
}

pub fn run_requests(
    ledger: &Ledger,
    owners: &OwnerClient,
    receipt_dir: std::path::PathBuf,
) -> std::io::Result<()> {
    use mcp_kit::McpHandler;
    let mut executor = crate::mcp::OrchMcpHandler::new(ledger.clone(), owners.clone(), receipt_dir);
    crate::requests::Requests::open(ledger)?.run(|tool, args, interrupted| {
        if interrupted {
            executor.reconcile_request(tool, args)
        } else {
            Some(executor.call_tool(tool, args).0)
        }
    })
}

pub fn run_relay_daemon(
    ledger: Ledger,
    owners: OwnerClient,
    receipt_dir: std::path::PathBuf,
    bind_addr: &str,
    health_url_file: Option<&Path>,
    poll_ms: u64,
) -> Result<(), String> {
    let _executor = ledger
        .lock_relay()
        .map_err(|e| format!("relay execution unavailable: {e}"))?;
    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
        libc::signal(libc::SIGINT, handle_signal as *const () as usize);
    }
    STOP_FLAG.store(false, Ordering::Release);

    let listener = TcpListener::bind(bind_addr)
        .map_err(|e| format!("failed to bind relay health listener on {bind_addr}: {e}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| format!("failed to read relay local_addr: {e}"))?;
    if !local_addr.ip().is_loopback() {
        return Err("termux-orch relay health bind must be loopback".to_owned());
    }
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("failed to set nonblocking listener: {e}"))?;

    let mut random = [0u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut random))
        .map_err(|e| format!("cannot generate wake token: {e}"))?;
    let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let token_path = health_url_file.map(|url| url.with_file_name("wake.token"));
    if let (Some(url_file), Some(token_path)) = (health_url_file, &token_path) {
        if let Some(parent) = url_file.parent().filter(|p| !p.as_os_str().is_empty()) {
            crate::ledger::ensure_private_dir(parent).map_err(|e| e.to_string())?;
        }
        crate::ledger::write_atomic_private(token_path, token.as_bytes())
            .map_err(|e| e.to_string())?;
        crate::ledger::write_atomic_private(url_file, format!("http://{local_addr}\n").as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let health = Arc::new(RelayHealth::default());
    let server_health = Arc::clone(&health);
    let health_owners = owners.clone();
    let state_root_str = ledger.root().display().to_string();
    let (wake_tx, wake_rx) = mpsc::sync_channel(1);
    relay_log(
        "relay_started",
        json!({
            "bind": local_addr.to_string(),
            "poll_ms": poll_ms.clamp(250, 60_000),
            "state_root": state_root_str
        }),
    );
    let health_handle = std::thread::spawn(move || {
        while !STOP_FLAG.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let Some(request) = read_http_head(&mut stream) else {
                        continue;
                    };
                    let mut lines = request.split("\r\n");
                    let mut route = lines.next().unwrap_or_default().split_ascii_whitespace();
                    let (status, ctype, body) = match (route.next(), route.next()) {
                        (Some("GET"), Some("/readyz")) => {
                            ("200 OK", "text/plain", "ready\n".to_owned())
                        }
                        (Some("GET"), Some("/healthz" | "/health")) => {
                            let body = json!({
                                "ok": true,
                                "service": "termux-orch-relay",
                                "version": env!("CARGO_PKG_VERSION"),
                                "ticks": server_health.ticks.load(Ordering::Acquire),
                                "last_tick_errors": server_health.last_tick_errors.load(Ordering::Acquire),
                                "wake_pushes": server_health.wake_pushes.load(Ordering::Acquire),
                                "last_wake_tick_ms": server_health.last_wake_tick_ms.load(Ordering::Acquire),
                                "chatgpt_cooldown_remaining_secs": health_owners.chatgpt_cooldown_remaining_secs(),
                                "chatgpt_cooldown_until_unix": health_owners.chatgpt_cooldown_until_unix(),
                                "rate_limit_hits": health_owners.rate_limit_hits(),
                                "local_rate_limit_hits": health_owners.local_rate_limit_hits(),
                                "remote_chatgpt_calls": health_owners.remote_chatgpt_calls(),
                                "local_observe_calls": health_owners.local_observe_calls(),
                                "state_root": state_root_str
                            })
                            .to_string();
                            ("200 OK", "application/json", body)
                        }
                        (Some("POST"), Some("/wake")) if token_path.is_some() => {
                            if authorized_wake(lines, &token) {
                                server_health.wake_pushes.fetch_add(1, Ordering::Release);
                                let _ = wake_tx.try_send(Instant::now());
                                (
                                    "202 Accepted",
                                    "application/json",
                                    r#"{"state":"accepted"}"#.to_owned(),
                                )
                            } else {
                                (
                                    "401 Unauthorized",
                                    "application/json",
                                    r#"{"error":"unauthorized"}"#.to_owned(),
                                )
                            }
                        }
                        _ => (
                            "404 Not Found",
                            "application/json",
                            r#"{"error":"not_found"}"#.to_owned(),
                        ),
                    };
                    let resp = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                    let _ = stream.write_all(resp.as_bytes());
                }
                Err(_) => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    });

    let engine = RelayEngine::new(ledger.clone(), owners.clone());
    let interval = Duration::from_millis(poll_ms.clamp(250, 60_000));
    let mut wake_requested = None::<Instant>;
    while !STOP_FLAG.load(Ordering::Acquire) {
        if let Some(requested) = wake_requested.take() {
            health
                .last_wake_tick_ms
                .store(requested.elapsed().as_millis() as u64, Ordering::Release);
        }
        let request_error = run_requests(&ledger, &owners, receipt_dir.clone()).is_err();
        let tick = engine.tick_once();
        health.last_tick_errors.store(
            tick.errors as u64 + u64::from(request_error),
            Ordering::Release,
        );
        health.ticks.fetch_add(1, Ordering::Release);
        let deadline = Instant::now() + interval;
        while !STOP_FLAG.load(Ordering::Acquire) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            if let Ok(requested) = wake_rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
                wake_requested = Some(requested);
                break;
            }
        }
    }

    health_handle
        .join()
        .map_err(|_| "health server thread panicked")?;
    if let Some(url_file) = health_url_file {
        match fs::remove_file(url_file) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(format!("cannot remove owned health URL file: {error}")),
        }
    }
    Ok(())
}

#[derive(Default)]
struct RelayHealth {
    ticks: AtomicU64,
    last_tick_errors: AtomicU64,
    wake_pushes: AtomicU64,
    last_wake_tick_ms: AtomicU64,
}

fn read_http_head(stream: &mut TcpStream) -> Option<String> {
    let deadline = Instant::now() + Duration::from_millis(250);
    let mut buffer = [0u8; 4096];
    let mut used = 0;
    while used < buffer.len() {
        let remaining = deadline.checked_duration_since(Instant::now())?;
        stream.set_read_timeout(Some(remaining)).ok()?;
        let n = stream.read(&mut buffer[used..]).ok()?;
        if n == 0 {
            return None;
        }
        used += n;
        if let Some(end) = buffer[..used].windows(4).position(|w| w == b"\r\n\r\n") {
            return String::from_utf8(buffer[..end].to_vec()).ok();
        }
    }
    None
}

fn authorized_wake<'a>(headers: impl Iterator<Item = &'a str>, token: &str) -> bool {
    let mut auth = headers.filter_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("authorization")
            .then_some(value.trim())
    });
    match (auth.next(), auth.next()) {
        (Some(value), None) => value.strip_prefix("Bearer ").is_some_and(|given| {
            given.len() == token.len()
                && given
                    .bytes()
                    .zip(token.bytes())
                    .fold(0, |diff, (a, b)| diff | (a ^ b))
                    == 0
        }),
        _ => false,
    }
}

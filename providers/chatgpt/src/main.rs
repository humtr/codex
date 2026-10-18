use codex_chatgpt_provider::{config::FILE_LIMIT, store};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const USAGE: &str = "usage: codex-chatgpt-provider serve --descriptor PATH [--state PATH] [--port PORT] | status [--state PATH] | token [--state PATH] | models | config <show|effective|validate|edit|set FIELD VALUE [FIELD VALUE ...]|unset FIELD ...>";
fn text(value: &str) -> Result<(), String> {
    println!("{value}");
    Ok(())
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|v| {
            v.into_string()
                .map_err(|_| "arguments must be UTF-8".to_string())
        })
        .collect::<Result<_, _>>()?;
    if matches!(
        args.first().map(String::as_str),
        Some("serve" | "status" | "token")
    ) {
        return runtime_command(&args);
    }
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        return text(USAGE);
    }
    let path = store::config_path().map_err(|e| e.to_string())?;
    match args.first().map(String::as_str) {
        Some("models") if args.len() == 1 => {
            let snapshot = store::Snapshot::open(&path).map_err(|e| e.to_string())?;
            let effective = snapshot.effective().map_err(|e| e.to_string())?;
            let value = serde_json::json!({"schema":"codex-chatgpt-catalog-v1","source":effective["origins"]["catalog"],"candidate_server_implemented":true,"production_qualified":false,"catalog":snapshot.config().map_err(|e| e.to_string())?.catalog});
            text(
                &serde_json::to_string_pretty(&value)
                    .map_err(|_| "model catalog serialization failed")?,
            )
        }
        Some("config") => match args.get(1).map(String::as_str) {
            Some("show") if args.len() == 2 => {
                let snapshot = store::Snapshot::open(&path).map_err(|e| e.to_string())?;
                text(
                    &snapshot
                        .config()
                        .and_then(|v| v.render())
                        .map_err(|e| e.to_string())?,
                )
            }
            Some("effective") if args.len() == 2 => {
                let snapshot = store::Snapshot::open(&path).map_err(|e| e.to_string())?;
                text(
                    &serde_json::to_string_pretty(
                        &snapshot.effective().map_err(|e| e.to_string())?,
                    )
                    .map_err(|_| "configuration serialization failed")?,
                )
            }
            Some("validate") if args.len() == 2 => {
                store::Snapshot::open(&path)
                    .and_then(|v| v.config())
                    .map_err(|e| e.to_string())?;
                text("configuration valid; applies when the Rust provider is used")
            }
            Some("set") if args.len() >= 4 && args.len().is_multiple_of(2) => {
                let pairs = args[2..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|v| (v[0].clone(), v[1].clone()))
                    .collect::<Vec<_>>();
                let changed = store::update(&path, &pairs, &[]).map_err(|e| e.to_string())?;
                text(if changed {
                    "configuration saved; applies when the Rust provider is used"
                } else {
                    "configuration unchanged"
                })
            }
            Some("unset") if args.len() > 2 => {
                let changed = store::update(&path, &[], &args[2..]).map_err(|e| e.to_string())?;
                text(if changed {
                    "configuration saved; applies when the Rust provider is used"
                } else {
                    "configuration unchanged"
                })
            }
            Some("edit") if args.len() == 2 => edit(&path),
            _ => Err(USAGE.into()),
        },
        _ => Err(USAGE.into()),
    }
}
fn runtime_command(args: &[String]) -> Result<(), String> {
    use codex_chatgpt_provider::runtime;
    let mut fields = std::collections::BTreeMap::new();
    if !(args.len() - 1).is_multiple_of(2) {
        return Err("runtime options require flag/value pairs".into());
    }
    for pair in args[1..].as_chunks::<2>().0 {
        if !matches!(pair[0].as_str(), "--state" | "--descriptor" | "--port")
            || fields.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("invalid or duplicate runtime option".into());
        }
    }
    let root = fields
        .get("--state")
        .map(|value| std::path::PathBuf::from(*value))
        .map(Ok)
        .unwrap_or_else(runtime::state_path)
        .map_err(|e| e.to_string())?;
    if !root.is_absolute() {
        return Err("runtime state must be an absolute path".into());
    }
    let executor = tokio::runtime::Runtime::new().map_err(|_| "async runtime is unavailable")?;
    match args[0].as_str() {
        "serve" => {
            let descriptor = fields
                .get("--descriptor")
                .map(|value| std::path::PathBuf::from(*value))
                .ok_or("serve requires --descriptor PATH")?;
            let port = fields
                .get("--port")
                .map(|v| v.parse::<u16>())
                .transpose()
                .map_err(|_| "invalid listen port")?
                .unwrap_or(0);
            executor
                .block_on(runtime::serve(descriptor, root, port))
                .map_err(|e| e.to_string())
        }
        "status" if !fields.contains_key("--descriptor") && !fields.contains_key("--port") => {
            let value = executor
                .block_on(runtime::status(&root))
                .map_err(|e| e.to_string())?;
            text(&serde_json::to_string_pretty(&value).map_err(|_| "status serialization failed")?)
        }
        "token" if !fields.contains_key("--descriptor") && !fields.contains_key("--port") => {
            let value = executor
                .block_on(runtime::token(&root))
                .map_err(|e| e.to_string())?;
            text(&value)
        }
        _ => Err("invalid runtime command".into()),
    }
}

fn edit(path: &std::path::Path) -> Result<(), String> {
    let snapshot = store::Snapshot::open(path).map_err(|e| e.to_string())?;
    let name = format!(
        "codex-chatgpt-edit-{}-{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "system clock is invalid")?
            .as_nanos()
    );
    let temporary = std::env::temp_dir().join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| "editor buffer cannot be created")?;
    let result = (|| {
        file.write_all(
            snapshot
                .editor_text()
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .and_then(|()| file.sync_all())
        .map_err(|_| "editor buffer cannot be written")?;
        drop(file);
        let editor = std::env::var("VISUAL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                std::env::var("EDITOR")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            })
            .unwrap_or_else(|| "vi".into());
        // VISUAL/EDITOR are user-owned shell command specifications; the file path is a separate argument.
        let status = std::process::Command::new("sh")
            .args([
                "-c",
                &format!("exec {editor} \"$1\""),
                "codex-chatgpt-editor",
            ])
            .arg(&temporary)
            .status()
            .map_err(|_| "editor could not be started")?;
        if !status.success() {
            return Err("editor failed; configuration preserved".into());
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&temporary)
            .map_err(|_| "editor buffer cannot be read safely")?;
        let metadata = file
            .metadata()
            .map_err(|_| "editor buffer cannot be inspected")?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o7777 != 0o600
        {
            return Err("editor buffer must remain an owned private regular file".into());
        }
        let mut bytes = Vec::new();
        file.take(FILE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "editor buffer cannot be read")?;
        let changed = snapshot.commit_bytes(&bytes).map_err(|e| e.to_string())?;
        text(if changed {
            "configuration saved; applies when the Rust provider is used"
        } else {
            "configuration unchanged"
        })
    })();
    let _ = fs::remove_file(temporary);
    result
}
fn main() {
    if let Err(error) = run() {
        eprintln!("codex-chatgpt-provider: {error}");
        std::process::exit(2);
    }
}

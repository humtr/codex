use codex_chatgpt_provider::{config::FILE_LIMIT, store};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const USAGE: &str = "usage: codex-chatgpt-provider serve [--descriptor PATH] [--state PATH] [--port PORT] | presentation --thread ID [--state PATH] | status [--state PATH] [--thread ID] | stop [--state PATH] | approve --thread ID [--state PATH] | approve-ui --thread ID [--state PATH] | reveal --thread ID [--state PATH] | view [--state PATH] | token [--state PATH] | mcp-stdio [--state PATH] | work ACTION [--field VALUE ...] [--state PATH] | chats <list|read> [--field VALUE ...] [--state PATH] | projects <list|use> [--name NAME] [--project-id ID] [--state PATH] | host <status|ensure|open|stop|models|accounts|account-add|account-rename|account-use|account-remove|account-login|account-confirm> [--profile NAME] [--state PATH] | migrate-inference --state PATH --backup PATH --target attached-submitted|provider21|provider14 | models | catalog | checkpoint --thread ID --from-model ID --model ID --effort EFFORT [--state PATH] < public-thread.json | prepare [--home PATH] [--state PATH] | config <show|effective|validate|edit|set FIELD VALUE [FIELD VALUE ...]|unset FIELD ...>";
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
    if args.first().map(String::as_str) == Some("migrate-inference") {
        let mut options = std::collections::BTreeMap::new();
        if args.len() != 7 {
            return Err("migrate-inference requires --state PATH --backup PATH --target attached-submitted|provider21|provider14".into());
        }
        for pair in args[1..].as_chunks::<2>().0 {
            if !matches!(pair[0].as_str(), "--state" | "--backup" | "--target")
                || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
            {
                return Err("invalid migration option".into());
            }
        }
        let value = codex_chatgpt_provider::operator::migrate_inference(
            std::path::Path::new(options["--state"]),
            std::path::Path::new(options["--backup"]),
            options["--target"],
        )
        .map_err(|error| error.to_string())?;
        return text(&value.to_string());
    }
    if args.as_slice() == ["--artifact-probe"] {
        return text("codex-provider-artifact-v1\nprovider=chatgpt\ncontrol=provider-payload-v1");
    }
    if matches!(
        args.first().map(String::as_str),
        Some(
            "serve"
                | "status"
                | "stop"
                | "approve"
                | "approve-ui"
                | "reveal"
                | "view"
                | "token"
                | "prepare"
                | "mcp-stdio"
                | "presentation"
        )
    ) {
        return runtime_command(&args);
    }
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        return text(USAGE);
    }
    if matches!(
        args.first().map(String::as_str),
        Some("work" | "chats" | "projects" | "host")
    ) {
        return control_command(&args);
    }
    let path = store::config_path().map_err(|e| e.to_string())?;
    match args.first().map(String::as_str) {
        Some("checkpoint") => checkpoint(&args[1..], &path),
        Some("catalog") if args.len() == 1 => {
            let config = store::Snapshot::open(&path)
                .and_then(|v| v.config())
                .map_err(|e| e.to_string())?;
            let value =
                codex_chatgpt_provider::projection::catalog(&config).map_err(|e| e.to_string())?;
            text(
                &serde_json::to_string_pretty(&value)
                    .map_err(|_| "catalog serialization failed")?,
            )
        }
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
fn control_command(args: &[String]) -> Result<(), String> {
    use codex_chatgpt_provider::runtime;
    let action = args.get(1).ok_or("work/chats/host requires an action")?;
    let login = args[0] == "host" && action == "account-login";
    let mut return_manager = None;
    let mut login_wait = 600;
    let mut root = runtime::state_path().map_err(|e| e.to_string())?;
    let mut values = serde_json::Map::new();
    let mut diagnostic_caps = false;
    let mut flags = std::collections::BTreeSet::new();
    if !(args.len() - 2).is_multiple_of(2) {
        return Err("control options require flag/value pairs".into());
    }
    for pair in args[2..].as_chunks::<2>().0 {
        if !flags.insert(&pair[0]) {
            return Err("duplicate control option".into());
        }
        if pair[0] == "--return-manager" {
            if !login || !std::path::Path::new(&pair[1]).is_absolute() {
                return Err(
                    "return-manager requires an absolute Manager executable for account-login"
                        .into(),
                );
            }
            return_manager = Some(std::path::PathBuf::from(&pair[1]));
            continue;
        }
        if login && pair[0] == "--max-wait-seconds" {
            login_wait = pair[1]
                .parse::<u64>()
                .ok()
                .filter(|n| (1..=600).contains(n))
                .ok_or("login wait must be 1..600 seconds")?;
            continue;
        }
        if pair[0] == "--diagnostic-caps" {
            diagnostic_caps = pair[1]
                .parse::<bool>()
                .map_err(|_| "diagnostic-caps requires true or false")?;
            continue;
        }
        if pair[0] == "--state" {
            root = pair[1].clone().into();
            continue;
        }
        let field = pair[0]
            .strip_prefix("--")
            .ok_or("control options require --field")?
            .replace('-', "_");
        let value = if matches!(
            field.as_str(),
            "after"
                | "cursor"
                | "context_cursor"
                | "source_cursor"
                | "list_cursor"
                | "answers"
                | "dependencies"
                | "budget"
        ) {
            serde_json::from_str(&pair[1]).map_err(|_| "invalid JSON control value")?
        } else if matches!(
            field.as_str(),
            "offset" | "limit" | "updated_after" | "updated_before" | "max_wait_seconds"
        ) {
            serde_json::Value::from(
                pair[1]
                    .parse::<u64>()
                    .map_err(|_| "invalid integer control value")?,
            )
        } else if matches!(field.as_str(), "refresh" | "purge") {
            serde_json::Value::Bool(
                pair[1]
                    .parse::<bool>()
                    .map_err(|_| format!("{field} requires true or false"))?,
            )
        } else {
            serde_json::Value::String(pair[1].clone())
        };
        if values.insert(field, value).is_some() {
            return Err("duplicate control option".into());
        }
    }
    let tool = match args[0].as_str() {
        "chats" => {
            values.insert("action".into(), serde_json::json!(action));
            "codex_chats"
        }
        "projects" => {
            values.insert("action".into(), serde_json::json!(action));
            "codex_projects"
        }
        "host" => {
            values.insert("action".into(), serde_json::json!(action.replace('-', "_")));
            "codex_host"
        }
        "work" => match action.as_str() {
            "history" => "codex_history",
            "status" => "codex_status",
            "stop" => "codex_stop",
            "answer" => "codex_answer",
            "attach" => "codex_attach",
            "list" | "create" | "resume" | "fork" | "start" | "queue" | "synchronize" | "claim"
            | "compact" | "takeover" | "events" | "wait" | "chat-input" | "chat-result"
            | "transfer" | "transfer-accept" | "bind" | "unbind" | "join" | "evaluate"
            | "source-sync" | "source-join" | "stop-tree" => {
                values.insert("action".into(), serde_json::json!(action.replace('-', "_")));
                "codex_sessions"
            }
            _ => return Err("unsupported work action".into()),
        },
        _ => return Err("unsupported control surface".into()),
    };
    if args[0] == "work" {
        if !values.contains_key("controller") {
            if let Some(work) = values.get("work_id").and_then(serde_json::Value::as_str) {
                if !matches!(tool, "codex_history" | "codex_status" | "codex_stop")
                    && values.get("action").is_none_or(|value| {
                        !matches!(
                            value.as_str(),
                            Some(
                                "events"
                                    | "wait"
                                    | "synchronize"
                                    | "chat_input"
                                    | "chat_result"
                                    | "join"
                                    | "source_sync"
                                    | "source_join"
                            )
                        )
                    })
                {
                    values.insert(
                        "controller".into(),
                        serde_json::json!(
                            runtime::work_capability(&root, work).map_err(|e| e.to_string())?
                        ),
                    );
                }
            }
        }
        if !values.contains_key("parent_controller") {
            if let Some(parent) = values
                .get("parent_work_id")
                .and_then(serde_json::Value::as_str)
            {
                values.insert(
                    "parent_controller".into(),
                    serde_json::json!(
                        runtime::work_capability(&root, parent).map_err(|e| e.to_string())?
                    ),
                );
            }
        }
    }
    let executor = tokio::runtime::Runtime::new().map_err(|_| "async runtime is unavailable")?;
    if login {
        return account_login(
            &executor,
            &root,
            values,
            return_manager.as_deref(),
            login_wait,
        );
    }
    let value = executor
        .block_on(runtime::control(
            &root,
            tool,
            serde_json::Value::Object(values),
        ))
        .map_err(|e| e.to_string())?;
    let mut public = value["result"].clone();
    if !diagnostic_caps {
        omit_capabilities(&mut public);
    }
    text(&serde_json::to_string_pretty(&public).map_err(|_| "control serialization failed")?)?;
    if value["is_error"] == true {
        Err("control operation failed; state preserved".into())
    } else {
        Ok(())
    }
}
fn account_login(
    executor: &tokio::runtime::Runtime,
    root: &std::path::Path,
    values: serde_json::Map<String, serde_json::Value>,
    manager: Option<&std::path::Path>,
    wait: u64,
) -> Result<(), String> {
    use codex_chatgpt_provider::runtime;
    let profile = values
        .get("profile")
        .and_then(|v| v.as_str())
        .ok_or("account-login requires --profile")?
        .to_owned();
    let core = match manager {
        Some(path) => path.to_path_buf(),
        None => std::path::PathBuf::from(std::env::var_os("PREFIX").ok_or("PREFIX unavailable")?)
            .join("bin/codex"),
    };
    let invoke = |operation: &str, origin: Option<&str>| -> Result<serde_json::Value, String> {
        let mut command = std::process::Command::new(&core);
        if manager.is_none() {
            command.arg("termux");
        }
        command.arg(operation);
        if let Some(origin) = origin {
            command.arg(origin);
        }
        let result = command
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|_| "terminal return service unavailable")?;
        if !result.status.success() {
            return Err(match String::from_utf8_lossy(&result.stderr).trim() {
                "codex termux: terminal origin unavailable"=>"no originating Termux terminal is bound to this caller; run account-login in Termux",
                "codex termux: original terminal return unconfirmed"=>"the original Termux terminal could not be focused",
                "codex termux: invalid terminal origin"=>"the saved Termux terminal identity was rejected",
                _=>"terminal return service rejected the request; a compatible Manager is required",
            }.into());
        }
        if result.stdout.len() > 4096 || !result.stderr.is_empty() {
            return Err("terminal origin/return unavailable; use a compatible Manager".into());
        }
        serde_json::from_slice(&result.stdout)
            .map_err(|_| "invalid terminal return response".into())
    };
    // Capture the original terminal before opening another Android activity.
    let origin = invoke("__terminal-origin-v1", None)?.to_string();
    let opened = executor
        .block_on(runtime::control(
            root,
            "codex_host",
            serde_json::Value::Object(values),
        ))
        .map_err(|e| e.to_string())?;
    if opened["is_error"] == true {
        text(&opened["result"].to_string())?;
        return Err("account login could not open; inspect the reported cause".into());
    }
    eprintln!("Complete sign-in and OTP in the Linux app's noVNC screen. Waiting for account confirmation.");
    let result = executor.block_on(async {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(wait);
        loop {
            let response = tokio::time::timeout_at(
                deadline,
                runtime::control(
                    root,
                    "codex_host",
                    serde_json::json!({"action":"account_confirm","profile":profile}),
                ),
            )
            .await
            .map_err(|_| "login wait expired; profile preserved".to_owned())?
            .map_err(|e| e.to_string())?;
            if response["is_error"] != true {
                return Ok::<_, String>(response["result"].clone());
            }
            let cause = response["result"]["error"]
                .as_str()
                .unwrap_or("account_confirmation_failed");
            if !matches!(
                cause,
                "app_target_unavailable"
                    | "app_scope_unavailable"
                    | "app_auth_loading"
                    | "app_auth_context_unavailable"
            ) {
                return Err(format!("login confirmation failed: {cause}"));
            }
            tokio::time::sleep_until(std::cmp::min(
                deadline,
                tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            ))
            .await;
        }
    })?;
    let returned = invoke("__terminal-return-v1", Some(&origin));
    let confirmed = returned
        .as_ref()
        .is_ok_and(|v| v["return_confirmed"] == true);
    let mut result = result;
    result["return_confirmed"] = serde_json::json!(confirmed);
    result["physical_focus_observed"] = serde_json::json!(false);
    if let Err(error) = &returned {
        result["return_error"] = serde_json::json!(error);
    }
    text(&serde_json::to_string_pretty(&result).map_err(|_| "login serialization failed")?)?;
    if !confirmed {
        return Err("login succeeded; original terminal return unconfirmed".into());
    }
    Ok(())
}
fn omit_capabilities(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(object) = value {
        for field in [
            "controller",
            "parent_controller",
            "turn_token",
            "accept",
            "receipt",
        ] {
            object.remove(field);
        }
    }
}
fn checkpoint(args: &[String], path: &std::path::Path) -> Result<(), String> {
    use codex_chatgpt_provider::{config::MAX_DECODED_BYTES, handoff::Handoff};
    let mut options = std::collections::BTreeMap::new();
    if !matches!(args.len(), 8 | 10) {
        return Err(USAGE.into());
    }
    for pair in args.as_chunks::<2>().0 {
        if !matches!(
            pair[0].as_str(),
            "--thread" | "--from-model" | "--model" | "--effort" | "--state"
        ) || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("invalid checkpoint options".into());
        }
    }
    if ["--thread", "--from-model", "--model", "--effort"]
        .iter()
        .any(|key| !options.contains_key(key))
    {
        return Err("invalid checkpoint options".into());
    }
    let config = store::Snapshot::open(path)
        .and_then(|v| v.config())
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(MAX_DECODED_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "public history unavailable")?;
    if bytes.len() as u64 > MAX_DECODED_BYTES {
        return Err("public history exceeds transport limit".into());
    }
    let snapshot: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "invalid public history")?;
    let value = Handoff::prepare(
        &snapshot,
        options["--thread"],
        options["--from-model"],
        options["--model"],
        options["--effort"],
        &config,
    )
    .map_err(|e| e.to_string())?;
    let state = options
        .get("--state")
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(codex_chatgpt_provider::runtime::state_path)
        .map_err(|e| e.to_string())?;
    let value = codex_chatgpt_provider::handoff::Issuer::from_state(&state)
        .and_then(|issuer| issuer.issue(value))
        .map_err(|e| e.to_string())?;
    text(&value.to_string())
}
fn runtime_command(args: &[String]) -> Result<(), String> {
    use codex_chatgpt_provider::runtime;
    let mut fields = std::collections::BTreeMap::new();
    if !(args.len() - 1).is_multiple_of(2) {
        return Err("runtime options require flag/value pairs".into());
    }
    for pair in args[1..].as_chunks::<2>().0 {
        if !matches!(
            pair[0].as_str(),
            "--state" | "--descriptor" | "--port" | "--home" | "--thread"
        ) || fields.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("invalid or duplicate runtime option".into());
        }
    }
    if fields.contains_key("--thread")
        && !matches!(
            args[0].as_str(),
            "status" | "approve" | "approve-ui" | "reveal" | "presentation"
        )
    {
        return Err(
            "--thread is only supported by status, approve, reveal and presentation".into(),
        );
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
        "presentation"
            if fields
                .keys()
                .all(|key| matches!(*key, "--state" | "--thread")) =>
        {
            let thread = fields.get("--thread").ok_or("--thread is required")?;
            let value = codex_chatgpt_provider::app::presentation_groups(&root, thread)
                .map_err(|e| e.to_string())?;
            text(&value.to_string())
        }
        "mcp-stdio" if fields.keys().all(|key| *key == "--state") => executor
            .block_on(runtime::mcp_stdio(&root))
            .map_err(|e| e.to_string()),
        "prepare" if !fields.contains_key("--descriptor") && !fields.contains_key("--port") => {
            let home = fields
                .get("--home")
                .map(std::path::PathBuf::from)
                .or_else(|| std::env::var_os("CODEX_HOME").map(std::path::PathBuf::from))
                .or_else(|| {
                    std::env::var_os("HOME").map(|v| std::path::PathBuf::from(v).join(".codex"))
                })
                .ok_or("Codex home required")?;
            let registry =
                std::path::PathBuf::from(std::env::var_os("HOME").ok_or("HOME required")?)
                    .join(".local/share/codex/core/config/profiles");
            let path = store::config_path().map_err(|e| e.to_string())?;
            let snapshot = store::Snapshot::open(&path).map_err(|e| e.to_string())?;
            let mut config = snapshot.config().map_err(|e| e.to_string())?;
            let status = executor
                .block_on(async {
                    runtime::ensure_token(&root).await?;
                    runtime::status(&root).await
                })
                .map_err(|e| e.to_string())?;
            let offered = executor
                .block_on(runtime::control(
                    &root,
                    "codex_host",
                    serde_json::json!({"action":"models","refresh":false}),
                ))
                .map_err(|e| e.to_string())?;
            if offered["is_error"] == true {
                return Err(offered["result"]["error"]
                    .as_str()
                    .unwrap_or("app_model_catalog_unavailable")
                    .into());
            }
            config.catalog = serde_json::from_value(offered["result"]["catalog"].clone())
                .map_err(|_| "app_model_catalog_invalid")?;
            if config
                .catalog
                .route(&config.model.default, &config.model.reasoning_effort)
                .is_none()
            {
                return Err("configured_model_effort_not_offered".into());
            }
            if !snapshot.unchanged().map_err(|e| e.to_string())? {
                return Err("provider_configuration_changed".into());
            }
            let value = codex_chatgpt_provider::registration::prepare(
                &config,
                &root,
                &registry,
                &home,
                &std::env::current_exe().map_err(|_| "binary path unavailable")?,
                status["base_url"]
                    .as_str()
                    .ok_or("invalid provider endpoint")?,
            )
            .map_err(|e| e.to_string())?;
            text(
                &serde_json::to_string_pretty(&value)
                    .map_err(|_| "projection serialization failed")?,
            )
        }
        "serve" if !fields.contains_key("--home") => {
            let descriptor = fields
                .get("--descriptor")
                .map(|value| std::path::PathBuf::from(*value));
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
        "status"
            if !fields.contains_key("--descriptor")
                && !fields.contains_key("--port")
                && !fields.contains_key("--home") =>
        {
            let value = executor
                .block_on(async {
                    match fields.get("--thread") {
                        Some(thread) => runtime::thread_status(&root, thread).await,
                        None => runtime::status(&root).await,
                    }
                })
                .map_err(|e| e.to_string())?;
            text(&serde_json::to_string_pretty(&value).map_err(|_| "status serialization failed")?)
        }
        "stop"
            if !fields.contains_key("--descriptor")
                && !fields.contains_key("--port")
                && !fields.contains_key("--home") =>
        {
            let value = executor
                .block_on(runtime::stop(&root))
                .map_err(|e| e.to_string())?;
            text(&serde_json::to_string_pretty(&value).map_err(|_| "status serialization failed")?)
        }
        "reveal" | "approve" | "approve-ui"
            if fields
                .keys()
                .all(|key| matches!(*key, "--state" | "--thread")) =>
        {
            let thread = fields
                .get("--thread")
                .ok_or("app control requires --thread")?;
            let value = executor
                .block_on(async {
                    if matches!(args[0].as_str(), "approve" | "approve-ui") {
                        runtime::approve(&root, thread, args[0] == "approve-ui").await
                    } else {
                        runtime::reveal(&root, thread).await
                    }
                })
                .map_err(|e| e.to_string())?;
            text(&serde_json::to_string_pretty(&value).map_err(|_| "status serialization failed")?)
        }
        "view"
            if !fields.contains_key("--descriptor")
                && !fields.contains_key("--port")
                && !fields.contains_key("--home") =>
        {
            let value = executor
                .block_on(runtime::view(&root))
                .map_err(|e| e.to_string())?;
            text(&serde_json::to_string_pretty(&value).map_err(|_| "status serialization failed")?)
        }
        "token"
            if !fields.contains_key("--descriptor")
                && !fields.contains_key("--port")
                && !fields.contains_key("--home") =>
        {
            let value = executor
                .block_on(runtime::ensure_token(&root))
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
// Keep Core's payload lease in this process. Child launches export only their
// explicitly mapped descriptors, not incidental handles inherited by this CLI.
fn mark_inherited_descriptors_close_on_exec() -> std::io::Result<()> {
    for entry in fs::read_dir("/proc/self/fd")? {
        let fd = entry?
            .file_name()
            .to_string_lossy()
            .parse::<i32>()
            .map_err(|_| std::io::Error::other("invalid inherited descriptor entry"))?;
        if fd > 2 && unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = mark_inherited_descriptors_close_on_exec()
        .map_err(|error| format!("inherited_descriptor_setup_failed: {error}"))
        .and_then(|()| run())
    {
        eprintln!("codex-chatgpt-provider: {error}");
        std::process::exit(2);
    }
}

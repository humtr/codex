mod launcher;
mod notify;
mod paths;
mod profiles;
mod session;

use crate::launcher::{run_launcher, run_provider_command, run_switch_command};
use crate::notify::{
    config_toml_has_stop_notify, deliver_notification, format_notification,
    list_configured_hooks_json_events, load_notify_config, read_event_payload, save_notify_config,
    sync_config_toml_stop_notify, sync_hooks_json, validate_backend, validate_event_name,
};
use crate::paths::ExtPaths;
use crate::profiles::{
    create_profile, delete_profile, effective_current_profile, list_profiles, read_default_profile,
    rename_profile, resolve_default_target, resolve_profile, write_default_profile,
};
use crate::session::list_live_sessions;
use serde_json::json;
use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};

fn print_usage() {
    println!(
        "codex-ext (cdx) {} — standalone Termux companion for Codex\n\n\
         Usage:\n\
         \x20 cdx [CODEX_ARGS...]                       Launch Codex in default profile (non-PTY)\n\
         \x20 cdx use <ACCOUNT> [CODEX_ARGS...]         Launch Codex in named OpenAI OAuth profile\n\
         \x20 cdx switch <ACCOUNT> [TID] [--wait] [--session PID]\n\
         \x20                                           Switch account (works inside TUI via !cdx switch)\n\
         \x20 cdx provider <openai|NAME> [--session PID]\n\
         \x20                                           Switch provider on ~/.codex (new thread; works via !cdx provider)\n\
         \x20 cdx profile <list|current|create|rename|delete|default> [...]\n\
         \x20                                           Manage OpenAI OAuth account profiles\n\
         \x20 cdx notify <show|set|test|emit> [...]     Configure/emit Termux:API notifications\n\
         \x20 cdx status [--json]                       Show profiles, daemons, sessions, and notify state\n\
         \x20 cdx --version | --help                    Show version or this help",
        env!("CARGO_PKG_VERSION")
    );
}

fn run_profile_cli(paths: &ExtPaths, args: &[String]) -> Result<i32, String> {
    let Some(sub) = args.first().map(String::as_str) else {
        return Err("usage: cdx profile <list|current|create|rename|delete|default>".to_owned());
    };
    let rest = &args[1..];
    match sub {
        "list" | "ls" => {
            let mut as_json = false;
            let mut fast = false;
            for a in rest {
                match a.as_str() {
                    "--json" => as_json = true,
                    "--no-daemon-check" => fast = true,
                    other => return Err(format!("unknown option for profile list: {other}")),
                }
            }
            let items = list_profiles(paths, !fast)?;
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&items)
                        .map_err(|e| format!("json error: {e}"))?
                );
            } else {
                for p in &items {
                    let star = if p.is_default { "*" } else { " " };
                    let auth = if p.has_auth { "auth" } else { "no-auth" };
                    let daemon = match p.daemon_pid {
                        Some(pid) => format!("{} (pid {})", p.daemon_status, pid),
                        None => p.daemon_status.clone(),
                    };
                    println!("{star} {:<16} {:<8} {}", p.name, auth, daemon);
                }
            }
            Ok(0)
        }
        "current" => {
            println!("{}", effective_current_profile(paths)?);
            Ok(0)
        }
        "create" => {
            let [name] = rest else {
                return Err("usage: cdx profile create <NAME>".to_owned());
            };
            let target = create_profile(paths, name)?;
            println!(
                "created profile '{}' at {}",
                target.name(),
                target.codex_home(paths).display()
            );
            Ok(0)
        }
        "rename" => {
            let [old_name, new_name] = rest else {
                return Err("usage: cdx profile rename <OLD> <NEW>".to_owned());
            };
            rename_profile(paths, old_name, new_name)?;
            println!("renamed profile '{old_name}' -> '{new_name}'");
            Ok(0)
        }
        "delete" | "rm" => {
            let mut name: Option<&str> = None;
            let mut yes = false;
            for a in rest {
                if a == "--yes" || a == "-y" {
                    yes = true;
                } else if name.is_none() && !a.starts_with('-') {
                    name = Some(a.as_str());
                } else {
                    return Err("usage: cdx profile delete <NAME> [--yes]".to_owned());
                }
            }
            let Some(name) = name else {
                return Err("usage: cdx profile delete <NAME> [--yes]".to_owned());
            };
            if !yes {
                if !io::stdin().is_terminal() {
                    return Err(format!(
                        "refusing non-interactive delete of '{name}' without --yes"
                    ));
                }
                eprint!("Delete profile '{name}' and its auth.json? [y/N]: ");
                let _ = io::stderr().flush();
                let mut answer = String::new();
                let _ = io::stdin().read_line(&mut answer);
                if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
                    println!("cancelled");
                    return Ok(1);
                }
            }
            delete_profile(paths, name)?;
            println!("deleted profile '{name}'");
            Ok(0)
        }
        "default" => match rest {
            [] => {
                println!(
                    "{}",
                    read_default_profile(paths)?.unwrap_or_else(|| "default".to_owned())
                );
                Ok(0)
            }
            [arg] if arg == "clear" || arg == "default" || arg == "home" => {
                write_default_profile(paths, None)?;
                println!("default profile set to 'default'");
                Ok(0)
            }
            [name] => {
                write_default_profile(paths, Some(name))?;
                println!("default profile set to '{name}'");
                Ok(0)
            }
            _ => Err("usage: cdx profile default [<NAME>|clear]".to_owned()),
        },
        other => Err(format!(
            "unknown profile subcommand '{other}' (expected list|current|create|rename|delete|default)"
        )),
    }
}

fn run_notify_cli(paths: &ExtPaths, args: &[String]) -> Result<i32, String> {
    let Some(sub) = args.first().map(String::as_str) else {
        return Err("usage: cdx notify <show|set|test|emit>".to_owned());
    };
    let rest = &args[1..];
    let config_toml = paths.shared_home.join("config.toml");
    let hooks_json = paths.shared_home.join("hooks.json");

    match sub {
        "show" => {
            let as_json = rest.iter().any(|a| a == "--json");
            let cfg = load_notify_config(paths)?;
            let stop_configured = config_toml_has_stop_notify(&config_toml)?;
            let hook_events = list_configured_hooks_json_events(&hooks_json)?;
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "enabled": cfg.enabled,
                        "backend": cfg.backend,
                        "group": cfg.group,
                        "events": cfg.events,
                        "config_toml_stop_notify": stop_configured,
                        "hooks_json_events": hook_events,
                    }))
                    .map_err(|e| format!("json error: {e}"))?
                );
            } else {
                println!("enabled: {}", cfg.enabled);
                println!("backend: {}", cfg.backend);
                println!("group: {}", cfg.group);
                println!("events: {}", cfg.events.join(", "));
                println!("config.toml Stop notify: {stop_configured}");
                println!(
                    "hooks.json extra events: {}",
                    if hook_events.is_empty() {
                        "-".to_owned()
                    } else {
                        hook_events.join(", ")
                    }
                );
            }
            Ok(0)
        }
        "set" => {
            let mut cfg = load_notify_config(paths)?;
            let mut idx = 0usize;
            while idx < rest.len() {
                match rest[idx].as_str() {
                    "--enable" => {
                        cfg.enabled = true;
                        idx += 1;
                    }
                    "--disable" => {
                        cfg.enabled = false;
                        idx += 1;
                    }
                    "--backend" => {
                        let val = rest
                            .get(idx + 1)
                            .ok_or_else(|| "--backend requires a value".to_owned())?;
                        validate_backend(val)?;
                        cfg.backend = val.clone();
                        idx += 2;
                    }
                    "--group" => {
                        let val = rest
                            .get(idx + 1)
                            .ok_or_else(|| "--group requires a value".to_owned())?;
                        if val.is_empty() || val.len() > 64 {
                            return Err("invalid --group value".to_owned());
                        }
                        cfg.group = val.clone();
                        idx += 2;
                    }
                    "--events" => {
                        let val = rest
                            .get(idx + 1)
                            .ok_or_else(|| "--events requires comma-separated names".to_owned())?;
                        let mut parsed = Vec::new();
                        for ev in val.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                            validate_event_name(ev)?;
                            if !parsed.iter().any(|x| x == ev) {
                                parsed.push(ev.to_owned());
                            }
                        }
                        cfg.events = parsed;
                        idx += 2;
                    }
                    other => return Err(format!("unknown option for notify set: {other}")),
                }
            }
            save_notify_config(paths, &cfg)?;
            let stop_enabled = cfg.enabled && cfg.events.iter().any(|e| e == "Stop");
            sync_config_toml_stop_notify(&config_toml, stop_enabled)?;
            let extra_events: Vec<String> = if cfg.enabled {
                cfg.events
                    .iter()
                    .filter(|e| e.as_str() != "Stop")
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };
            let wrote_hooks = sync_hooks_json(&hooks_json, &extra_events)?;
            println!(
                "notify updated (enabled={}, backend={}, events={})",
                cfg.enabled,
                cfg.backend,
                if cfg.events.is_empty() {
                    "-".to_owned()
                } else {
                    cfg.events.join(",")
                }
            );
            if wrote_hooks && !extra_events.is_empty() {
                println!(
                    "note: extra events in ~/.codex/hooks.json require a one-time hook trust approval in Codex 0.171.0+."
                );
            }
            Ok(0)
        }
        "test" => {
            let dry_run = rest.iter().any(|a| a == "--dry-run");
            let cfg = load_notify_config(paths)?;
            deliver_notification(
                paths,
                &cfg.backend,
                &cfg.group,
                "Test",
                "Codex — Test",
                "codex-ext Termux:API notification test succeeded.",
                dry_run,
            )?;
            Ok(0)
        }
        "emit" => {
            let mut event: Option<&str> = None;
            let mut payload_arg: Option<&str> = None;
            let mut dry_run = false;
            for a in rest {
                if a == "--dry-run" {
                    dry_run = true;
                } else if event.is_none() {
                    event = Some(a.as_str());
                } else if payload_arg.is_none() {
                    payload_arg = Some(a.as_str());
                }
            }
            let event = event.unwrap_or("Stop");
            if event == "Stop" && !dry_run {
                notify::nudge_orchestration_relay(paths);
            }
            let cfg = load_notify_config(paths)?;
            if !cfg.enabled && !dry_run {
                return Ok(0);
            }
            let payload = read_event_payload(payload_arg);
            let (title, body) = format_notification(event, payload.as_ref());
            if let Err(err) = deliver_notification(
                paths,
                &cfg.backend,
                &cfg.group,
                event,
                &title,
                &body,
                dry_run,
            ) {
                // Never fail upstream Codex on notification delivery error; log bounded stderr.
                eprintln!("codex-ext notify emit: {err}");
            }
            Ok(0)
        }
        other => Err(format!(
            "unknown notify subcommand '{other}' (expected show|set|test|emit)"
        )),
    }
}

fn run_status_cli(paths: &ExtPaths, args: &[String]) -> Result<i32, String> {
    let as_json = args.iter().any(|a| a == "--json");
    let profiles = list_profiles(paths, true)?;
    let sessions = list_live_sessions(paths)?;
    let notify = load_notify_config(paths)?;
    let stop_configured = config_toml_has_stop_notify(&paths.shared_home.join("config.toml"))?;
    let hooks_events = list_configured_hooks_json_events(&paths.shared_home.join("hooks.json"))?;

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "version": env!("CARGO_PKG_VERSION"),
                "codex_bin": paths.codex_bin.display().to_string(),
                "profiles": profiles,
                "live_sessions": sessions,
                "notify": {
                    "enabled": notify.enabled,
                    "backend": notify.backend,
                    "config_toml_stop": stop_configured,
                    "hooks_json_events": hooks_events,
                }
            }))
            .map_err(|e| format!("json error: {e}"))?
        );
    } else {
        println!("codex-ext {}", env!("CARGO_PKG_VERSION"));
        println!("codex_bin: {}", paths.codex_bin.display());
        println!("profiles ({}):", profiles.len());
        for p in &profiles {
            let star = if p.is_default { "*" } else { " " };
            println!(
                "  {star} {:<16} auth={:<5} daemon={}",
                p.name, p.has_auth, p.daemon_status
            );
        }
        println!("live cdx sessions ({}):", sessions.len());
        for s in &sessions {
            println!("  pid={} account={} cwd={}", s.pid, s.account, s.cwd);
        }
        println!(
            "notify: enabled={} backend={} stop={} hooks=[{}]",
            notify.enabled,
            notify.backend,
            stop_configured,
            hooks_events.join(",")
        );
    }
    Ok(0)
}

fn parse_switch_and_run(paths: &ExtPaths, args: &[String]) -> Result<i32, String> {
    let mut target: Option<&str> = None;
    let mut tid: Option<String> = None;
    let mut wait = false;
    let mut session_pid: Option<u32> = None;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--wait" => {
                wait = true;
                idx += 1;
            }
            "--session" => {
                let raw = args
                    .get(idx + 1)
                    .ok_or_else(|| "--session requires a numeric PID".to_owned())?;
                let pid = raw
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --session PID: '{raw}'"))?;
                session_pid = Some(pid);
                idx += 2;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option for switch: {other}"));
            }
            pos => {
                if target.is_none() {
                    target = Some(pos);
                } else if tid.is_none() {
                    tid = Some(pos.to_owned());
                } else {
                    return Err(
                        "usage: cdx switch <ACCOUNT> [TID] [--wait] [--session PID]".to_owned()
                    );
                }
                idx += 1;
            }
        }
    }
    let Some(target) = target else {
        return Err("usage: cdx switch <ACCOUNT> [TID] [--wait] [--session PID]".to_owned());
    };
    run_switch_command(paths, target, tid, wait, session_pid)
}

fn parse_provider_and_run(paths: &ExtPaths, args: &[String]) -> Result<i32, String> {
    let mut name: Option<&str> = None;
    let mut session_pid: Option<u32> = None;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--session" => {
                let raw = args
                    .get(idx + 1)
                    .ok_or_else(|| "--session requires a numeric PID".to_owned())?;
                let pid = raw
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --session PID: '{raw}'"))?;
                session_pid = Some(pid);
                idx += 2;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option for provider: {other}"));
            }
            pos => {
                if name.is_none() {
                    name = Some(pos);
                } else {
                    return Err("usage: cdx provider <openai|NAME> [--session PID]".to_owned());
                }
                idx += 1;
            }
        }
    }
    let Some(name) = name else {
        return Err("usage: cdx provider <openai|NAME> [--session PID]".to_owned());
    };
    run_provider_command(paths, name, session_pid)
}

fn run_main(raw_args: Vec<OsString>) -> Result<i32, String> {
    let first = raw_args.first().and_then(|s| s.to_str());
    match first {
        Some("-h" | "--help" | "help") if raw_args.len() == 1 => {
            print_usage();
            return Ok(0);
        }
        Some("-V" | "--version") if raw_args.len() == 1 => {
            println!("codex-ext {}", env!("CARGO_PKG_VERSION"));
            return Ok(0);
        }
        _ => {}
    }

    let paths = ExtPaths::from_env()?;
    let str_args: Vec<String> = raw_args
        .iter()
        .skip(1)
        .map(|s| s.to_string_lossy().into_owned())
        .collect();

    match first {
        None => {
            let target = resolve_default_target(&paths)?;
            run_launcher(&paths, target, None, Vec::new())
        }
        Some("--") => {
            let target = resolve_default_target(&paths)?;
            run_launcher(&paths, target, None, raw_args.into_iter().skip(1).collect())
        }
        Some("use") => {
            let Some(acct) = raw_args.get(1).and_then(|s| s.to_str()) else {
                return Err("usage: cdx use <ACCOUNT> [CODEX_ARGS...]".to_owned());
            };
            let target = resolve_profile(&paths, acct)?;
            let mut rest: Vec<OsString> = raw_args.into_iter().skip(2).collect();
            if rest.first().and_then(|s| s.to_str()) == Some("--") {
                rest.remove(0);
            }
            run_launcher(&paths, target, None, rest)
        }
        Some("switch") => parse_switch_and_run(&paths, &str_args),
        Some("provider") => parse_provider_and_run(&paths, &str_args),
        Some("profile") => run_profile_cli(&paths, &str_args),
        Some("notify") => run_notify_cli(&paths, &str_args),
        Some("status") => run_status_cli(&paths, &str_args),
        _ => {
            // Passthrough unknown top-level flags/args directly to default Codex profile!
            let target = resolve_default_target(&paths)?;
            run_launcher(&paths, target, None, raw_args)
        }
    }
}

fn main() {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match run_main(args) {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("codex-ext: {err}");
            std::process::exit(2);
        }
    }
}

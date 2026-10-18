use codex_chatgpt_provider::config::{self, HistoryMode};

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("config") => match args.next().as_deref().and_then(|arg| arg.to_str()) {
            Some("show") | Some("effective") => {
                if args.next().is_some() {
                    return Err("unexpected configuration argument".into());
                }
                let path = config::config_path().map_err(|error| error.to_string())?;
                let value = config::load(&path).map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value)
                        .map_err(|_| "configuration serialization failed")?
                );
                Ok(())
            }
            Some("validate") => {
                if args.next().is_some() {
                    return Err("unexpected configuration argument".into());
                }
                let path = config::config_path().map_err(|error| error.to_string())?;
                config::load(&path).map_err(|error| error.to_string())?;
                println!("configuration valid");
                Ok(())
            }
            Some("set") => {
                let field = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or("configuration field required")?;
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or("configuration value required")?;
                if args.next().is_some() {
                    return Err("unexpected configuration argument".into());
                }
                let path = config::config_path().map_err(|error| error.to_string())?;
                let mut config = config::load(&path).map_err(|error| error.to_string())?;
                match field.as_str() {
                    "model.default" => config.model.default = value,
                    "model.reasoning_effort" => config.model.reasoning_effort = value,
                    "context.window_tokens" => {
                        config.context.window_tokens = value
                            .parse()
                            .map_err(|_| "context.window_tokens must be an integer")?
                    }
                    "context.auto_compact_token_limit" => {
                        config.context.auto_compact_token_limit = if value == "none" {
                            None
                        } else {
                            Some(value.parse().map_err(|_| {
                                "context.auto_compact_token_limit must be an integer"
                            })?)
                        }
                    }
                    "history.mode" => {
                        config.history.mode = match value.as_str() {
                            "retain" => HistoryMode::Retain,
                            "project" => HistoryMode::Project,
                            _ => return Err("history.mode must be retain or project".into()),
                        }
                    }
                    "history.project_name" => {
                        config.history.project_name = (value != "none").then_some(value)
                    }
                    "history.create_if_missing" => {
                        config.history.create_if_missing = match value.as_str() {
                            "true" => true,
                            "false" => false,
                            _ => {
                                return Err("history.create_if_missing must be true or false".into())
                            }
                        }
                    }
                    _ => return Err("unknown configuration field".into()),
                }
                config::save(&path, &config).map_err(|error| error.to_string())?;
                println!("configuration updated");
                Ok(())
            }
            _ => Err(
                "usage: codex-chatgpt-provider config <show|effective|validate|set FIELD VALUE>"
                    .into(),
            ),
        },
        _ => Err(
            "usage: codex-chatgpt-provider config <show|effective|validate|set FIELD VALUE>".into(),
        ),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("codex-chatgpt-provider: {error}");
        std::process::exit(2);
    }
}

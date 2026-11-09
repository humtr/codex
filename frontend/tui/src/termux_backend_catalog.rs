//! Upstream loads configuration; Core binds its resolved catalog to the backend.
use codex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use std::ffi::OsString;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const LIMIT: usize = 4 * 1024 * 1024;

/// Called before configuration or terminal setup; safe without a terminal/auth.
#[expect(
    clippy::print_stdout,
    reason = "This static pre-start capability query is an explicit CLI stdout contract."
)]
pub(crate) fn capabilities(requested: bool) -> Option<crate::AppExitInfo> {
    if !requested {
        return None;
    }
    println!("codex-termux-resolved-startup-v1");
    Some(crate::AppExitInfo {
        token_usage: Default::default(),
        thread_id: None,
        resume_hint: None,
        disconnect_info: None,
        update_action: None,
        exit_reason: crate::ExitReason::UserRequested,
    })
}

/// Keep the complete configuration snapshot across the UI/backend boundary.
fn catalog_bytes(config: &crate::legacy_core::config::Config) -> color_eyre::Result<Option<&[u8]>> {
    if config.model_catalog.is_none() {
        return Ok(None);
    }
    let source = config
        .model_catalog_source
        .as_deref()
        .ok_or_else(|| color_eyre::eyre::eyre!("complete model catalog source is unavailable"))?;
    Ok(Some(source))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionReply {
    schema: String,
    #[serde(deserialize_with = "Option::deserialize")]
    connection: Option<OwnerConnection>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerConnection {
    execution_home: std::path::PathBuf,
    socket: AbsolutePathBuf,
}

/// Resolve a known thread's execution identity before config/auth/draft startup.
pub(crate) async fn reenter_owner(
    cli: &crate::Cli,
    endpoint: Option<&codex_app_server_client::RemoteAppServerEndpoint>,
) -> std::io::Result<()> {
    if !crate::termux_permissions::enabled()
        || std::env::var("CODEX_TERMUX_BACKEND_STARTUP").as_deref() != Ok("resolved-v1")
    {
        return Ok(());
    }
    let Some(codex_app_server_client::RemoteAppServerEndpoint::UnixSocket { socket_path }) =
        endpoint
    else {
        return Ok(());
    };
    let Some(thread) = cli
        .resume_session_id
        .as_deref()
        .or(cli.fork_session_id.as_deref())
        .and_then(|id| codex_protocol::ThreadId::from_string(id).ok())
    else {
        return Ok(());
    };
    let core = std::env::var_os("CODEX_PROFILE_CORE")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| std::io::Error::other("qualified Core is unavailable"))?;
    let owner = read_owner(&core, thread)
        .await
        .map_err(|error| std::io::Error::other(format!("Native owner lookup failed: {error:#}")))?;
    if let Some(owner) = owner {
        let current = codex_utils_home_dir::find_codex_home()?;
        if owner.execution_home.as_path() != current.as_path() {
            use std::os::unix::process::CommandExt;
            let invocation: Vec<_> = std::env::args_os().skip(1).collect();
            return Err(owner_reentry_command(&core, thread, socket_path, &invocation)?.exec());
        }
    }
    Ok(())
}

fn owner_reentry_command(
    core: &Path,
    thread: codex_protocol::ThreadId,
    socket: &Path,
    invocation: &[OsString],
) -> std::io::Result<std::process::Command> {
    let mut injected = OsString::from("unix://");
    injected.push(socket);
    if invocation.len() < 3 || invocation[0] != "--remote" || invocation[1] != injected {
        return Err(std::io::Error::other(
            "qualified startup invocation does not match its Native endpoint",
        ));
    }
    let mut command = std::process::Command::new(core);
    command.args(["termux", "__task-reentry-v1", &thread.to_string(), "--"]);
    command.args(&invocation[2..]);
    Ok(command)
}

/// Resolve startup intent before connecting; a live writer precedes configuration.
pub(crate) async fn bind(
    target: &mut crate::AppServerTarget,
    config: &crate::legacy_core::config::Config,
    cli_kv_overrides: &[(String, toml::Value)],
    loader_overrides: &codex_config::LoaderOverrides,
    thread: Option<codex_protocol::ThreadId>,
) -> color_eyre::Result<bool> {
    if crate::termux_permissions::enabled()
        && matches!(target, crate::AppServerTarget::LocalDaemon { .. })
    {
        let core = std::env::var_os("CODEX_PROFILE_CORE")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| color_eyre::eyre::eyre!("qualified Core is unavailable"))?;
        let owner = match thread {
            Some(thread) => owner_connection(&core, &config.codex_home, thread).await?,
            None => None,
        };
        let live_owner = owner.is_some();
        if let Some(socket_path) = owner {
            *target = crate::AppServerTarget::LocalDaemon {
                endpoint: codex_app_server_client::RemoteAppServerEndpoint::UnixSocket {
                    socket_path,
                },
                allow_embedded_fallback: false,
            };
        } else {
            let catalog = catalog_bytes(config)?;
            if catalog.is_some()
                || std::env::var("CODEX_TERMUX_BACKEND_STARTUP").as_deref() == Ok("resolved-v1")
            {
                let arguments = invocation_args(cli_kv_overrides, loader_overrides);
                let socket_path = prepare(
                    &core,
                    &config.codex_home,
                    catalog,
                    &arguments,
                    config.cwd.as_path(),
                )
                .await?;
                *target = crate::AppServerTarget::LocalDaemon {
                    endpoint: codex_app_server_client::RemoteAppServerEndpoint::UnixSocket {
                        socket_path,
                    },
                    allow_embedded_fallback: false,
                };
            }
        }
        // Preserve upstream's read-only feature admission against the actual
        // resolved backend, without offering an installer/restart/embedded path.
        if crate::profile_transport::core_owned(target, true)
            && let Some(warning) =
                crate::daemon_startup::compatibility_warning(target, config, cli_kv_overrides)
                    .await?
        {
            color_eyre::eyre::bail!("qualified backend is incompatible: {warning}");
        }
        return Ok(live_owner);
    }
    Ok(false)
}

async fn owner_connection(
    core: &Path,
    home: &Path,
    thread: codex_protocol::ThreadId,
) -> color_eyre::Result<Option<AbsolutePathBuf>> {
    match read_owner(core, thread).await? {
        Some(owner) => {
            if owner.execution_home != home {
                color_eyre::eyre::bail!(
                    "Native task belongs to another execution profile; startup context rebinding is required before attachment"
                );
            }
            Ok(Some(owner.socket))
        }
        None => Ok(None),
    }
}

async fn read_owner(
    core: &Path,
    thread: codex_protocol::ThreadId,
) -> color_eyre::Result<Option<OwnerConnection>> {
    let bytes = crate::profile_transport::call_manager(
        core,
        &["__task-connection-v1".to_owned(), thread.to_string()],
        12,
    )
    .await
    .ok_or_else(|| color_eyre::eyre::eyre!("qualified Native owner discovery is unavailable"))?;
    let reply: ConnectionReply = serde_json::from_slice(&bytes)?;
    if reply.schema != "codex-manager-task-connection-v1" {
        color_eyre::eyre::bail!("unsupported Native connection metadata");
    }
    Ok(reply.connection)
}

fn invocation_args(
    cli: &[(String, toml::Value)],
    loader: &codex_config::LoaderOverrides,
) -> Vec<OsString> {
    let mut args = Vec::new();
    if let Some(profile) = &loader.user_config_profile {
        args.extend([OsString::from("--profile"), profile.to_string().into()]);
    }
    for (key, value) in cli {
        args.extend([OsString::from("--config"), format!("{key}={value}").into()]);
    }
    args
}

pub(crate) async fn prepare(
    core: &Path,
    home: &Path,
    catalog: Option<&[u8]>,
    arguments: &[OsString],
    cwd: &Path,
) -> color_eyre::Result<AbsolutePathBuf> {
    let metadata = std::fs::symlink_metadata(core)?;
    if !core.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
        || catalog.is_some_and(|bytes| bytes.is_empty() || bytes.len() > LIMIT)
    {
        color_eyre::eyre::bail!("qualified catalog startup is unavailable");
    }
    tokio::time::timeout(Duration::from_secs(20), async {
        let mut child = Command::new(core)
            .arg(if catalog.is_some() {
                "__app-server-catalog-v1"
            } else {
                "__app-server-default-v1"
            })
            .args(arguments)
            .env("CODEX_HOME", home)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let mut input = child.stdin.take().ok_or_else(|| {
            color_eyre::eyre::eyre!("qualified backend catalog stdin unavailable")
        })?;
        if let Some(catalog) = catalog {
            input.write_all(catalog).await?;
        }
        input.shutdown().await?;
        drop(input);
        let mut output = Vec::new();
        child
            .stdout
            .take()
            .ok_or_else(|| color_eyre::eyre::eyre!("qualified backend catalog stdout unavailable"))?
            .take(109)
            .read_to_end(&mut output)
            .await?;
        if output.len() > 108 || !child.wait().await?.success() {
            color_eyre::eyre::bail!("qualified backend catalog startup failed");
        }
        decode_socket(&output)
    })
    .await
    .map_err(|_| color_eyre::eyre::eyre!("qualified backend catalog startup timed out"))?
}

fn decode_socket(bytes: &[u8]) -> color_eyre::Result<AbsolutePathBuf> {
    let path = std::str::from_utf8(bytes)?
        .strip_suffix('\n')
        .filter(|s| Path::new(s).is_absolute() && s.len() <= 107 && !s.contains(['\n', '\r', '\0']))
        .ok_or_else(|| color_eyre::eyre::eyre!("invalid qualified backend socket"))?;
    Ok(AbsolutePathBuf::from_absolute_path_checked(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn termux_backend_catalog_owner_reentry_preserves_opaque_user_arguments()
    -> color_eyre::Result<()> {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let root = tempfile::tempdir()?;
        let core = root.path().join("core");
        std::fs::write(&core, "#!/bin/sh\nprintf '%s\\0' \"$@\"\n")?;
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700))?;
        let thread: codex_protocol::ThreadId =
            codex_protocol::ThreadId::from_string("12345678-1234-1234-1234-123456789abc")?;
        let mut invocation: Vec<OsString> = [
            "--remote",
            "unix:///owned/socket",
            "--profile",
            "chosen",
            "-c",
            "value='--remote 한 글'",
            "resume",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        invocation.push(thread.to_string().into());
        invocation.push(OsString::from_vec(b"opaque-\xff".to_vec()));
        let output = owner_reentry_command(&core, thread, Path::new("/owned/socket"), &invocation)?
            .output()?;
        assert!(output.status.success());
        let mut expected = format!("termux\0__task-reentry-v1\0{thread}\0--\0").into_bytes();
        for argument in &invocation[2..] {
            expected.extend_from_slice(argument.as_bytes());
            expected.push(0);
        }
        assert_eq!(output.stdout, expected);
        assert!(
            owner_reentry_command(&core, thread, Path::new("/another/socket"), &invocation)
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn termux_backend_catalog_owner_query_preserves_identity_and_rejects_unknown_ownership()
    -> color_eyre::Result<()> {
        let root = tempfile::tempdir()?;
        let core = root.path().join("core");
        let thread: codex_protocol::ThreadId =
            codex_protocol::ThreadId::from_string("12345678-1234-1234-1234-123456789abc")?;
        let socket = root.path().join("native-socket");
        let install = |value: serde_json::Value, status: i32| -> color_eyre::Result<()> {
            std::fs::write(
                &core,
                format!(
                    "#!/bin/sh\n[ \"$#\" -eq 3 ] && [ \"$1\" = termux ] && [ \"$2\" = __task-connection-v1 ] && [ \"$3\" = {thread} ] || exit 9\nprintf '%s\\n' '{value}'\nexit {status}\n"
                ),
            )?;
            std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700))?;
            Ok(())
        };
        let mut reply = serde_json::json!({"schema":"codex-manager-task-connection-v1",
            "connection":{"execution_home":root.path(), "socket":socket}});
        install(reply.clone(), 0)?;
        assert_eq!(
            owner_connection(&core, root.path(), thread)
                .await?
                .as_deref(),
            Some(socket.as_path())
        );
        assert!(
            owner_connection(&core, &root.path().join("other-home"), thread)
                .await
                .is_err()
        );
        reply["connection"] = serde_json::Value::Null;
        install(reply.clone(), 0)?;
        assert!(
            owner_connection(&core, root.path(), thread)
                .await?
                .is_none()
        );
        install(reply.clone(), 1)?;
        assert!(
            owner_connection(&core, root.path(), thread).await.is_err(),
            "query failure cannot be interpreted as a free writer"
        );
        reply["schema"] = "unknown".into();
        install(reply, 0)?;
        assert!(owner_connection(&core, root.path(), thread).await.is_err());
        install(
            serde_json::json!({"schema":"codex-manager-task-connection-v1"}),
            0,
        )?;
        assert!(
            owner_connection(&core, root.path(), thread).await.is_err(),
            "missing ownership is not a confirmed free writer"
        );
        Ok(())
    }

    #[tokio::test]
    async fn termux_backend_catalog_preserves_full_config_source_instead_of_ui_projection()
    -> color_eyre::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("models.json");
        let mut original: serde_json::Value =
            serde_json::from_slice(include_bytes!("fixtures/chat-model-catalog.json"))?;
        original["native_only_root"] = serde_json::json!({"retained":true});
        original["models"][0]["native_only_setting"] = serde_json::json!({"retained":"original"});
        let original_bytes = serde_json::to_vec_pretty(&original)?;
        std::fs::write(&path, &original_bytes)?;
        let mut config = crate::legacy_core::config::ConfigBuilder::default()
            .codex_home(root.path().to_path_buf())
            .cli_overrides(vec![(
                "model_catalog_json".into(),
                path.to_str().unwrap().into(),
            )])
            .build()
            .await?;
        assert_eq!(catalog_bytes(&config)?.unwrap(), original_bytes);
        config.model_catalog_source = None;
        assert!(
            catalog_bytes(&config).is_err(),
            "missing source must not fall back to a lossy UI projection"
        );
        config.model_catalog = None;
        assert!(catalog_bytes(&config)?.is_none());
        Ok(())
    }

    #[test]
    fn termux_backend_catalog_rejects_non_socket_output() {
        assert!(decode_socket(b"/owned/s\n").is_ok());
        for value in [
            &b"relative\n"[..],
            b"~/owned/s\n",
            b"/owned/s",
            b"/owned/s\nextra\n",
            b"/owned/\0s\n",
            b"/owned/s\r\n",
            b"\xff\n",
        ] {
            assert!(
                decode_socket(value).is_err(),
                "accepted invalid socket output: {value:?}"
            );
        }
        assert!(decode_socket(format!("/{}\n", "s".repeat(107)).as_bytes()).is_err());
    }

    #[tokio::test]
    async fn termux_backend_catalog_propagates_failed_startup_and_invalid_reply()
    -> color_eyre::Result<()> {
        let root = tempfile::tempdir()?;
        let core = root.path().join("core");
        let home = root.path().join("home");
        std::fs::create_dir(&home)?;
        for (output, status, expected) in [
            ("/owned/s\\n", 7, "qualified backend catalog startup failed"),
            ("/owned/s", 0, "invalid qualified backend socket"),
        ] {
            std::fs::write(
                &core,
                format!(
                    "#!/bin/sh\ncat > \"$CODEX_HOME/catalog\"\nprintf '{output}'\nexit {status}\n"
                ),
            )?;
            std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700))?;
            let bytes = br#"{"models":[]}"#;
            let error = prepare(&core, &home, Some(bytes), &[], root.path())
                .await
                .unwrap_err();
            assert_eq!(error.to_string(), expected);
            assert_eq!(std::fs::read(home.join("catalog"))?, bytes);
        }
        Ok(())
    }

    #[tokio::test]
    async fn termux_backend_catalog_transfers_exact_resolved_bytes_and_home()
    -> color_eyre::Result<()> {
        let root = tempfile::tempdir()?;
        let core = root.path().join("core");
        let home = root.path().join("home");
        std::fs::create_dir(&home)?;
        std::fs::write(
            &core,
            "#!/bin/sh\n[ \"$#\" = 1 ] && [ \"$1\" = __app-server-catalog-v1 ] || exit 2\ncat > \"$CODEX_HOME/catalog\"\nprintf '/owned/s\\n'\n",
        )?;
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700))?;
        let bytes = br#"{"models":[{"slug":"owned"}]}"#;
        let socket = prepare(&core, &home, Some(bytes), &[], root.path()).await?;
        assert_eq!(socket.as_path(), Path::new("/owned/s"));
        assert_eq!(std::fs::read(home.join("catalog"))?, bytes);
        std::fs::write(
            &core,
            "#!/bin/sh\n[ \"$#\" = 1 ] && [ \"$1\" = __app-server-default-v1 ] || exit 2\ncat > \"$CODEX_HOME/default-input\"\nprintf '/owned/native\\n'\n",
        )?;
        assert_eq!(
            prepare(&core, &home, None, &[], root.path())
                .await?
                .as_path(),
            Path::new("/owned/native")
        );
        assert!(std::fs::read(home.join("default-input"))?.is_empty());
        let cli = vec![
            ("model_provider".into(), toml::Value::String("owned".into())),
            (
                "features".into(),
                toml::Value::Table(
                    [("plugins".into(), toml::Value::Boolean(false))]
                        .into_iter()
                        .collect(),
                ),
            ),
        ];
        let loader = codex_config::LoaderOverrides {
            user_config_profile: Some("work".parse()?),
            ..Default::default()
        };
        let arguments = invocation_args(&cli, &loader);
        assert_eq!(arguments[0], "--profile");
        assert_eq!(arguments[1], "work");
        let parsed = codex_utils_cli::CliConfigOverrides {
            raw_overrides: arguments
                .chunks_exact(2)
                .skip(1)
                .map(|pair| pair[1].to_str().unwrap().to_owned())
                .collect(),
        }
        .parse_overrides()
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
        assert_eq!(parsed, cli);
        std::fs::write(
            &core,
            "#!/bin/sh\nprintf '%s\\0' \"$@\" > \"$CODEX_HOME/arguments\"\npwd > \"$CODEX_HOME/cwd\"\ncat > \"$CODEX_HOME/catalog\"\nprintf '/owned/s\\n'\n",
        )?;
        prepare(&core, &home, Some(bytes), &arguments, &home).await?;
        let mut expected = b"__app-server-catalog-v1\0".to_vec();
        for arg in &arguments {
            expected.extend_from_slice(arg.to_str().unwrap().as_bytes());
            expected.push(0);
        }
        assert_eq!(std::fs::read(home.join("arguments"))?, expected);
        assert_eq!(std::fs::read(home.join("catalog"))?, bytes);
        assert_eq!(
            std::fs::read_to_string(home.join("cwd"))?.trim_end(),
            home.to_str().unwrap()
        );
        assert!(
            prepare(&core, &home, Some(b""), &[], root.path())
                .await
                .is_err()
        );
        assert!(
            prepare(&core, &home, Some(&vec![b'x'; LIMIT + 1]), &[], root.path())
                .await
                .is_err()
        );
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o777))?;
        assert!(
            prepare(&core, &home, Some(bytes), &[], root.path())
                .await
                .is_err()
        );
        Ok(())
    }
}

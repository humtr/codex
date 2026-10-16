//! Optional same-generation Manager frontend; the official runtime owns execution.
use super::{GenerationManifest, LocalProductError, RuntimeFdSources, TermuxBaseEnvPlan};
use std::ffi::{OsStr, OsString};
use std::io;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) const PREFIX: &str = "termux-manager-tui-v1:";
const STARTUP_CAPABILITY: &[u8] = b"codex-termux-resolved-startup-v1\n";
const STARTUP_ENV: &str = "CODEX_TERMUX_BACKEND_STARTUP";

pub(super) fn resolved_startup(
    frontend: &OsStr,
    resolver: &Path,
    config: &Path,
    plan: &TermuxBaseEnvPlan,
) -> bool {
    let prepare = || -> io::Result<_> {
        let fds = RuntimeFdSources::open(resolver, config)?;
        let mut command = Command::new(frontend);
        command.arg("--termux-backend-capabilities");
        super::apply_child_env_plan_and_fence(&mut command, Some(plan));
        fds.configure(&mut command);
        query_startup(command, Duration::from_secs(2))
    };
    prepare().unwrap_or(false)
}

fn query_startup(mut command: Command, deadline: Duration) -> io::Result<bool> {
    let (mut reader, writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let mut child = command
        .env_remove(STARTUP_ENV)
        .env("TERM", "xterm-256color")
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::null())
        .spawn()?;
    // Command retains its configured stdout handle after spawn; close our copy
    // so the reader can observe the child's EOF.
    drop(command);
    let started = Instant::now();
    let result = (|| {
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 64];
            let eof = match reader.read(&mut buffer) {
                Ok(0) => true,
                Ok(n) => {
                    bytes.extend_from_slice(&buffer[..n]);
                    if bytes.len() > STARTUP_CAPABILITY.len() {
                        return Ok(false);
                    }
                    false
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => false,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Ok(false);
                }
                if eof {
                    return Ok(bytes == STARTUP_CAPABILITY);
                }
            }
            if started.elapsed() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

pub(super) fn index(manifest: &GenerationManifest) -> Result<Option<usize>, LocalProductError> {
    let matches: Vec<_> = manifest
        .helper_digests
        .iter()
        .enumerate()
        .filter(|(_, helper)| helper.identity.starts_with(PREFIX))
        .collect();
    if matches.is_empty() {
        return Ok(None);
    }
    if matches.len() != 1
        || matches[0].0 != 2
        || manifest.helper_digests.len() != 3
        || matches[0].1.identity.strip_prefix(PREFIX)
            != Some(manifest.upstream_package_version.as_str())
        || manifest.manager_artifact_digest.is_none()
        || manifest.helper_digests[0].identity != super::TERMUX_BROWSER_OPEN_HELPER_IDENTITY
        || manifest.helper_digests[1].identity != super::TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
    {
        return Err(LocalProductError::Descriptor(
            "Manager frontend must match its generation version, Manager and helper layout",
        ));
    }
    Ok(Some(2))
}

pub(super) fn supported(args: &[OsString]) -> bool {
    !args.iter().any(|arg| {
        arg.to_str().is_some_and(|arg| {
            arg == "--add-dir"
                || arg.starts_with("--add-dir=")
                || arg == "--worktree"
                || arg.starts_with("--worktree=")
        })
    })
}

pub(super) fn local_interactive(args: &[OsString]) -> bool {
    super::shared_server::interactive(args)
        && supported(args)
        && std::env::var_os("CODEX_EXEC_SERVER_URL").is_none()
        && !args.iter().any(|arg| {
            arg.to_str().is_some_and(|arg| {
                matches!(
                    arg,
                    "--remote"
                        | "--no-daemon"
                        | "--oss"
                        | "--strict-config"
                        | "--dangerously-bypass-hook-trust"
                        | "--search"
                        | "--help"
                        | "-h"
                        | "--version"
                        | "-V"
                ) || arg.starts_with("--remote=")
            })
        })
}

pub(super) fn capability_args(mut args: Vec<OsString>, available: bool) -> Vec<OsString> {
    if args
        .first()
        .is_some_and(|a| a == "__terminal-capabilities-v1")
    {
        let native = available && supported(&args[1..]);
        args.insert(1, if native { "native" } else { "title" }.into());
    }
    args
}

fn native_args(socket: &Path, original: &[OsString]) -> io::Result<Vec<OsString>> {
    let socket = socket
        .to_str()
        .filter(|_| socket.is_absolute())
        .ok_or_else(|| io::Error::other("qualified local frontend socket is invalid"))?;
    let mut args = vec!["--remote".into(), format!("unix://{socket}").into()];
    args.extend_from_slice(original);
    Ok(args)
}

pub(super) struct BackendStartup {
    pub socket: PathBuf,
    pub resolved: bool,
}

pub(super) fn launch(
    frontend: &OsStr,
    startup: &BackendStartup,
    profile: &Path,
    original: &[OsString],
    resolver: &Path,
    config: &Path,
    plan: &TermuxBaseEnvPlan,
) -> io::Error {
    let prepare = || -> io::Result<_> {
        let core = super::validated_core_entrypoint()?;
        let fds = RuntimeFdSources::open(resolver, config)?;
        let mut command = Command::new(frontend);
        command
            .args(native_args(&startup.socket, original)?)
            .env("CODEX_HOME", profile)
            .env("CODEX_PROFILE_CORE", core)
            .env_remove(STARTUP_ENV);
        if startup.resolved {
            command.env(STARTUP_ENV, "resolved-v1");
        }
        super::apply_child_env_plan_and_fence(&mut command, Some(plan));
        fds.configure(&mut command);
        Ok((command, fds))
    };
    match prepare() {
        Ok((mut command, _fds)) => command.exec(),
        Err(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GenerationHelperDigest;
    use std::os::unix::ffi::OsStringExt;

    fn manifest() -> GenerationManifest {
        GenerationManifest {
            upstream_package_identity: "owned".into(),
            upstream_package_version: "0.160.0".into(),
            source_artifact_digest: "owned".into(),
            expected_platform: "android".into(),
            expected_architecture: "aarch64".into(),
            patch_policy_id: "owned".into(),
            patch_report: "owned".into(),
            runtime_digest: "owned".into(),
            helper_digests: [
                super::super::TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                super::super::TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
                "termux-manager-tui-v1:0.160.0",
            ]
            .map(|identity| GenerationHelperDigest {
                identity: identity.into(),
                digest: "owned".into(),
            })
            .into(),
            core_artifact_digest: "owned".into(),
            manager_artifact_digest: Some("owned".into()),
            core_api_identity: "owned".into(),
            persistent_schema_identity: "owned".into(),
            creation_metadata: "owned".into(),
        }
    }

    #[test]
    fn resolved_startup_requires_exact_success_and_reaps_bounded_probe() {
        let root = crate::tests::temp_root("frontend-startup");
        let program = root.join("frontend");
        let shell = crate::tests::resolve_test_tool("sh").unwrap();
        use std::os::unix::fs::PermissionsExt;
        for (body, expected) in [
            ("printf 'codex-termux-resolved-startup-v1\\n'", true),
            (
                "printf 'codex-termux-resolved-startup-v1\\n'; exit 2",
                false,
            ),
            ("exit 2", false),
            ("printf 'codex-termux-resolved-startup-v2\\n'", false),
            ("printf 'codex-termux-resolved-startup-v1\\nextra'", false),
            ("while :; do printf xxxxxxxxxxxxxxxxxxxxxxxxx; done", false),
            ("exec sleep 5", false),
        ] {
            std::fs::write(&program, format!("#!{}\n{body}\n", shell.display())).unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
            let start = Instant::now();
            assert_eq!(
                query_startup(Command::new(&program), Duration::from_millis(150)).unwrap(),
                expected,
                "{body}"
            );
            assert!(start.elapsed() < Duration::from_secs(2));
        }
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn manager_tui_declaration_requires_one_matching_version_manager_and_layout() {
        let valid = manifest();
        assert_eq!(index(&valid).unwrap(), Some(2));
        let mut absent = valid.clone();
        absent.helper_digests.pop();
        assert_eq!(index(&absent).unwrap(), None);
        let mut invalid = manifest();
        invalid.upstream_package_version = "0.161.0".into();
        assert!(index(&invalid).is_err());
        invalid = manifest();
        invalid.manager_artifact_digest = None;
        assert!(index(&invalid).is_err());
        invalid = manifest();
        invalid.helper_digests.swap(0, 1);
        assert!(index(&invalid).is_err());
        invalid = manifest();
        invalid.creation_metadata = super::super::R10_BROWSER_HELPER_BRIDGE_METADATA.into();
        assert_eq!(index(&invalid).unwrap(), Some(2));
        assert!(super::super::r10_browser_helper_bridge(&invalid).unwrap());
        invalid.helper_digests[2].identity = "unqualified-third-helper".into();
        assert!(super::super::r10_browser_helper_bridge(&invalid).is_err());
        invalid = manifest();
        invalid
            .helper_digests
            .push(invalid.helper_digests[2].clone());
        assert!(index(&invalid).is_err());
    }

    #[test]
    fn terminal_capability_uses_qualified_frontend_and_actual_supported_argv() {
        let query: Vec<OsString> = vec![
            "__terminal-capabilities-v1".into(),
            "--".into(),
            "resume".into(),
            "owned".into(),
        ];
        for available in [true, false] {
            let result = capability_args(query.clone(), available);
            assert_eq!(result[1], if available { "native" } else { "title" });
            assert_eq!(result[2..], query[1..]);
        }
        for flag in [
            "--add-dir",
            "--add-dir=/owned",
            "--worktree",
            "--worktree=owned",
        ] {
            let mut args = query.clone();
            args.push(flag.into());
            assert_eq!(capability_args(args, true)[1], "title");
        }
        let ordinary = vec!["notify".into(), "show".into()];
        assert_eq!(capability_args(ordinary.clone(), true), ordinary);
    }

    #[test]
    fn manager_tui_socket_route_preserves_non_utf8_and_upstream_workspace_options() {
        let args = vec!["resume".into(), OsString::from_vec(vec![0xff])];
        let planned = native_args(Path::new("/owned/s"), &args).unwrap();
        assert_eq!(&planned[2..], args);
        assert_eq!(planned[0], "--remote");
        assert_eq!(planned[1], "unix:///owned/s");
        assert!(native_args(Path::new("relative"), &args).is_err());
        assert!(supported(&args));
        for arg in [
            "--add-dir",
            "--add-dir=/owned",
            "--worktree",
            "--worktree=owned",
        ] {
            assert!(!supported(&[arg.into()]));
        }
    }

    #[test]
    fn local_frontend_preserves_invocation_settings_without_shared_daemon_reuse() {
        for args in [
            vec!["--profile", "chatgpt"],
            vec!["resume", "owned", "-c", "model_reasoning_effort=medium"],
            vec!["--enable=owned"],
        ] {
            let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
            assert!(local_interactive(&argv));
            assert!(!super::super::shared_server::eligible(&argv));
            assert_eq!(
                &native_args(Path::new("/owned/s"), &argv).unwrap()[2..],
                argv
            );
        }
        for args in [
            vec!["--no-daemon"],
            vec!["exec"],
            vec!["app-server"],
            vec!["login"],
            vec!["--version"],
            vec!["--remote=unix:///owned"],
            vec!["--oss"],
            vec!["--worktree"],
            vec!["--add-dir=/owned"],
        ] {
            let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
            assert!(!local_interactive(&argv), "{argv:?}");
        }
    }
}

use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PublicDispatchRoute {
    Update(Vec<OsString>),
    Doctor(Vec<OsString>),
    Wrapper(Vec<OsString>),
    Upstream(Vec<OsString>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DispatchPlanError {
    UnsupportedSandboxMode(String),
    UnsupportedSandboxCommand,
    UpstreamUpdaterDisabled,
    LeadingGlobalFlags(&'static str),
}

impl std::fmt::Display for DispatchPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchPlanError::UnsupportedSandboxMode(mode) => write!(
                f,
                "sandbox mode '{mode}' is unsupported on Termux because Android kernels lack Linux Landlock/user-namespace isolation; use 'danger-full-access' (the wrapper default)"
            ),
            DispatchPlanError::UnsupportedSandboxCommand => f.write_str(
                "'codex sandbox linux' is unsupported on Termux because Android kernels lack Linux Landlock/seccomp-user-namespace sandboxing",
            ),
            DispatchPlanError::LeadingGlobalFlags(command) => write!(
                f,
                "run 'codex {command}' directly without leading upstream flags"
            ),
            DispatchPlanError::UpstreamUpdaterDisabled => f.write_str(
                "upstream self-updater is disabled on Termux; run 'codex update' directly without global flags",
            ),
        }
    }
}

impl std::error::Error for DispatchPlanError {}

fn upstream_global_option_takes_separate_value(value: &str) -> bool {
    matches!(
        value,
        "-c" | "--config"
            | "--enable"
            | "--disable"
            | "--remote"
            | "--remote-auth-token-env"
            | "-s"
            | "--sandbox"
            | "-a"
            | "--ask-for-approval"
            | "-m"
            | "--model"
            | "-p"
            | "--profile"
            | "-C"
            | "--cd"
            | "-i"
            | "--image"
            | "--local-provider"
            | "--add-dir"
    )
}

fn app_server_option_takes_separate_value(value: &str) -> bool {
    matches!(
        value,
        "--startup-client-info"
            | "--cloud-skills-snapshot"
            | "--code-mode-host"
            | "--listen"
            | "--ws-auth"
            | "--ws-token-file"
            | "--ws-token-sha256"
            | "--ws-shared-secret-file"
            | "--ws-issuer"
            | "--ws-audience"
            | "--ws-max-clock-skew-seconds"
    )
}

fn is_non_daemon_top_level_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "exec"
            | "e"
            | "review"
            | "login"
            | "logout"
            | "mcp"
            | "plugin"
            | "completion"
            | "sandbox"
            | "debug"
            | "execpolicy"
            | "apply"
            | "a"
            | "queue"
            | "archive"
            | "delete"
            | "migrate-rollouts"
            | "migrate-nfs-db"
            | "unarchive"
            | "cloud"
            | "cloud-tasks"
            | "responses-api-proxy"
            | "stdio-to-uds"
            | "exec-server"
            | "features"
            | "help"
            | "doctor"
    )
}

fn is_upstream_help_or_version_flag(arg: &str) -> bool {
    matches!(arg, "--help" | "--version")
        || arg.strip_prefix('-').is_some_and(|flags| {
            !flags.is_empty() && flags.bytes().all(|flag| matches!(flag, b'h' | b'V'))
        })
}

fn unsupported_sandbox_mode(value: &str) -> Option<&str> {
    match value {
        "read-only" | "workspace-write" => Some(value),
        _ => None,
    }
}

#[derive(Default)]
pub(super) struct UpstreamArgScan {
    wrapper_command: Option<(usize, &'static str)>,
    sandbox_error: Option<DispatchPlanError>,
    pub(super) help_or_version: bool,
    pub(super) upstream_invocation_requires_daemon_settings: bool,
    pub(super) upstream_invocation_initializes_managed_daemon: bool,
}

impl UpstreamArgScan {
    pub(super) fn new(args: &[OsString]) -> Self {
        let mut scan = Self::default();
        let mut top_level_pending = true;
        let mut help_pending = true;
        let mut global_done = false;
        let mut daemon_done = false;
        let mut global_value = false;
        let mut daemon_value = false;
        let mut sandbox_value = false;
        let mut sandbox_positionals = [None; 2];
        let mut sandbox_count = 0;
        let mut daemon_positionals = [None; 3];
        let mut daemon_count = 0;
        let mut daemon_disabled = false;

        // Top-level routing and the sandbox guard consume global option values.
        // The daemon view also consumes app-server-specific values, including a
        // literal "--" when it occupies a value slot in that view.
        for (index, raw) in args.iter().enumerate() {
            if global_done && daemon_done {
                break;
            }
            let arg = raw.to_str();
            let takes_value = arg.is_some_and(upstream_global_option_takes_separate_value);
            if !global_done {
                if global_value {
                    if sandbox_value {
                        scan.check_sandbox_mode(arg);
                        sandbox_value = false;
                    }
                    global_value = false;
                } else if arg == Some("--") {
                    global_done = true;
                } else {
                    global_value = takes_value;
                    if help_pending {
                        if arg.is_some_and(|arg| {
                            arg == "help" || is_upstream_help_or_version_flag(arg)
                        }) {
                            scan.help_or_version = true;
                            help_pending = false;
                        } else if arg.is_none_or(|arg| !arg.starts_with('-')) {
                            help_pending = false;
                        }
                    }
                    if !raw.as_bytes().starts_with(b"-") {
                        if top_level_pending {
                            scan.wrapper_command = match arg {
                                Some("update") => Some((index, "update")),
                                Some("doctor") => Some((index, "doctor")),
                                Some("wrapper") => Some((index, "wrapper")),
                                _ => None,
                            };
                            top_level_pending = false;
                        }
                        if let Some(arg) = arg {
                            if sandbox_count < sandbox_positionals.len() {
                                sandbox_positionals[sandbox_count] = Some(arg);
                                sandbox_count += 1;
                            }
                        }
                    } else if let Some(arg) = arg {
                        sandbox_value = matches!(arg, "--sandbox" | "-s");
                        if !sandbox_value {
                            scan.check_sandbox_mode(arg.strip_prefix("--sandbox=").or_else(|| {
                                arg.strip_prefix("-s=").or_else(|| arg.strip_prefix("-s"))
                            }));
                        }
                    }
                }
            }

            if !daemon_done {
                match arg {
                    _ if daemon_value => daemon_value = false,
                    Some("--") => daemon_done = true,
                    Some(arg) if arg.starts_with('-') => {
                        daemon_disabled |= matches!(arg, "--no-daemon" | "--remote")
                            || arg.starts_with("--remote=")
                            || is_upstream_help_or_version_flag(arg);
                        daemon_value = takes_value
                            || (daemon_positionals[0] == Some("app-server")
                                && app_server_option_takes_separate_value(arg));
                    }
                    _ if daemon_count < daemon_positionals.len() => {
                        daemon_positionals[daemon_count] = Some(arg.unwrap_or(""));
                        daemon_count += 1;
                    }
                    _ => {}
                }
            }
        }
        if scan.sandbox_error.is_none() && sandbox_positionals == [Some("sandbox"), Some("linux")] {
            scan.sandbox_error = Some(DispatchPlanError::UnsupportedSandboxCommand);
        }
        scan.upstream_invocation_requires_daemon_settings = !scan.help_or_version
            && !daemon_disabled
            && !daemon_positionals[0].is_some_and(is_non_daemon_top_level_command);
        scan.upstream_invocation_initializes_managed_daemon =
            !daemon_disabled && initializes_managed_daemon(daemon_positionals);
        scan
    }

    fn check_sandbox_mode(&mut self, value: Option<&str>) {
        if self.sandbox_error.is_none() {
            if let Some(mode) = value.and_then(unsupported_sandbox_mode) {
                self.sandbox_error =
                    Some(DispatchPlanError::UnsupportedSandboxMode(mode.to_owned()));
            }
        }
    }

    fn sandbox_result(&self) -> Result<(), DispatchPlanError> {
        self.sandbox_error.clone().map_or(Ok(()), Err)
    }
}

fn initializes_managed_daemon(positionals: [Option<&str>; 3]) -> bool {
    match positionals {
        [None, ..] => true,
        [Some("resume" | "fork" | "agents"), ..] => true,
        [Some("app-server"), Some("daemon"), Some("start" | "restart" | "bootstrap")] => true,
        [Some("app-server"), ..] => false,
        [Some("remote-control"), None | Some("start"), ..] => true,
        [Some("remote-control"), ..] => false,
        [Some(cmd), ..] if is_non_daemon_top_level_command(cmd) || cmd == "update" => false,
        [Some(_), ..] => true,
    }
}

pub(super) fn plan_public_dispatch(
    args: Vec<OsString>,
    scan: &UpstreamArgScan,
) -> Result<PublicDispatchRoute, DispatchPlanError> {
    match scan.wrapper_command {
        Some((0, "update")) => return Ok(PublicDispatchRoute::Update(args[1..].to_vec())),
        Some((0, "doctor")) => return Ok(PublicDispatchRoute::Doctor(args[1..].to_vec())),
        Some((0, "wrapper")) => return Ok(PublicDispatchRoute::Wrapper(args[1..].to_vec())),
        Some((_, "update")) => return Err(DispatchPlanError::UpstreamUpdaterDisabled),
        Some((_, command)) => return Err(DispatchPlanError::LeadingGlobalFlags(command)),
        None => {}
    }
    scan.sandbox_result()?;
    Ok(PublicDispatchRoute::Upstream(args))
}

#[cfg(test)]
mod tests {
    use super::{DispatchPlanError, PublicDispatchRoute, UpstreamArgScan};
    use crate::run_public_main;
    use crate::tests::os_vec;
    use std::ffi::OsString;

    #[test]
    fn public_dispatch_routes_wrapper_commands_and_passes_daemon_and_remote_control() {
        assert_eq!(
            plan_public_dispatch(os_vec(&["update", "--upstream"])).unwrap(),
            PublicDispatchRoute::Update(os_vec(&["--upstream"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["doctor", "--json"])).unwrap(),
            PublicDispatchRoute::Doctor(os_vec(&["--json"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["wrapper", "status", "--json"])).unwrap(),
            PublicDispatchRoute::Wrapper(os_vec(&["status", "--json"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["app-server", "daemon", "--listen", "stdio://"]))
                .unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&[
                "app-server",
                "daemon",
                "--listen",
                "stdio://"
            ]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["app-server", "daemon", "update", "--from-cli"]))
                .unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&[
                "app-server",
                "daemon",
                "update",
                "--from-cli"
            ]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["--", "update"])).unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&["--", "update"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["-c", "model=update", "exec", "hi"])).unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&["-c", "model=update", "exec", "hi"]))
        );
        assert!(matches!(
            plan_public_dispatch(os_vec(&["-c", "foo=bar", "update"])),
            Err(DispatchPlanError::UpstreamUpdaterDisabled)
        ));
        assert!(matches!(
            plan_public_dispatch(os_vec(&["--search", "update"])),
            Err(DispatchPlanError::UpstreamUpdaterDisabled)
        ));
        assert_eq!(run_public_main(os_vec(&["-c", "foo=bar", "update"])), 2);
        assert_eq!(
            plan_public_dispatch(os_vec(&["remote-control"])).unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&["remote-control"]))
        );
    }

    #[test]
    fn dispatch_keeps_non_utf8_option_values_and_positional_prompts_distinct() {
        use std::os::unix::ffi::OsStringExt as _;

        let raw = OsString::from_vec(vec![0xff]);
        for owned in ["update", "doctor", "wrapper"] {
            assert_eq!(
                run_public_main([OsString::from("-c"), raw.clone(), OsString::from(owned)]),
                2
            );
        }
        let prompt = vec![raw, OsString::from("update")];
        assert_eq!(
            plan_public_dispatch(prompt.clone()).unwrap(),
            PublicDispatchRoute::Upstream(prompt)
        );
        let flag = OsString::from_vec(b"--unknown=\xff".to_vec());
        assert!(plan_public_dispatch(vec![flag, OsString::from("update")]).is_err());
    }

    #[test]
    fn passthrough_sandbox_guard_blocks_unsupported_linux_sandboxes() {
        assert!(plan_public_dispatch(os_vec(&["exec", "--sandbox", "read-only", "hi"])).is_err());
        for args in [
            ["-sread-only", "exec", "hi"],
            ["-sworkspace-write", "exec", "hi"],
            ["-s=read-only", "exec", "hi"],
            ["exec", "-sread-only", "hi"],
        ] {
            assert_eq!(run_public_main(os_vec(&args)), 2);
        }
        assert!(plan_public_dispatch(os_vec(&["exec", "--", "-sread-only"])).is_ok());
        assert!(plan_public_dispatch(os_vec(&["-sdanger-full-access", "exec", "hi"])).is_ok());
        assert!(plan_public_dispatch(os_vec(&["-s=workspace-write", "exec", "hi"])).is_err());
        assert!(plan_public_dispatch(os_vec(&["sandbox", "linux", "true"])).is_err());
        assert!(
            plan_public_dispatch(os_vec(&["--sandbox", "danger-full-access", "exec", "hi"]))
                .is_ok()
        );
        assert!(plan_public_dispatch(os_vec(&["exec", "--", "--sandbox", "read-only"])).is_ok());
    }

    #[test]
    fn help_and_version_fast_path_skips_daemon_settings() {
        assert!(is_help_or_version_invocation(&os_vec(&["--version"])));
        assert!(is_help_or_version_invocation(&os_vec(&["-V"])));
        assert!(is_help_or_version_invocation(&os_vec(&[
            "-c", "foo=bar", "--help"
        ])));
        assert!(is_help_or_version_invocation(&os_vec(&["help", "exec"])));
        assert!(!is_help_or_version_invocation(&os_vec(&[
            "exec",
            "--version"
        ])));
        assert!(!is_help_or_version_invocation(&os_vec(&[
            "app-server",
            "daemon"
        ])));
        assert!(!is_help_or_version_invocation(&[]));
    }

    #[test]
    fn managed_daemon_init_classification_matches_upstream_startup_routes() {
        // Routes that initialize managed daemon on a fresh CODEX_HOME
        for args in [
            &[][..],
            &["hello"][..],
            &["-p", "review", "검토해 줘"][..],
            &["-c", "k=--no-daemon", "prompt"][..],
            &["--", "--no-daemon"][..],
            &["--", "--help"][..],
            &["--", "-hV"][..],
            &["--model", "-hV", "prompt"][..],
            &["-m-hV", "prompt"][..],
            &["resume"][..],
            &["resume", "--last"][..],
            &["fork", "019c0000-0000-7000-8000-000000000001"][..],
            &["agents"][..],
            &["app-server", "daemon", "start"][..],
            &["app-server", "daemon", "restart"][..],
            &["app-server", "daemon", "bootstrap"][..],
            &["app-server", "-c", "k=v", "daemon", "start"][..],
            &["remote-control"][..],
            &["remote-control", "start"][..],
        ] {
            let v = os_vec(args);
            assert!(
                upstream_invocation_initializes_managed_daemon(&v),
                "expected daemon init for {args:?}"
            );
            assert!(
                upstream_invocation_requires_daemon_settings(&v),
                "expected daemon settings requirement for {args:?}"
            );
        }

        // Routes that must NOT pre-create `packages/app-server-daemon/current`
        for args in [
            &["--help"][..],
            &["--version"][..],
            &["-hV"][..],
            &["-Vh"][..],
            &["-p", "review", "-hV"][..],
            &["help"][..],
            &["app-server", "daemon", "start", "--help"][..],
            &["resume", "--help"][..],
            &["agents", "-h"][..],
            &["--no-daemon"][..],
            &["resume", "--no-daemon"][..],
            &["fork", "--remote", "unix:///tmp/s"][..],
            &["agents", "--remote=ws://127.0.0.1:8080"][..],
            &["remote-control", "--no-daemon"][..],
            &["app-server", "daemon", "stop"][..],
            &["app-server", "daemon", "version"][..],
            &["app-server", "daemon", "update", "--from-cli", "--yes"][..],
            &["app-server", "daemon", "enable-remote-control"][..],
            &["app-server", "daemon", "disable-remote-control"][..],
            &["app-server"][..],
            &["app-server", "--listen", "stdio://"][..],
            &["app-server", "proxy"][..],
            &["remote-control", "stop"][..],
            &["remote-control", "pair"][..],
            &["exec", "hi"][..],
            &["e", "hi"][..],
            &["review"][..],
            &["login"][..],
            &["logout"][..],
            &["queue", "--thread", "t", "--message", "m"][..],
            &["features", "list"][..],
        ] {
            let v = os_vec(args);
            assert!(
                !upstream_invocation_initializes_managed_daemon(&v),
                "unexpected daemon init for {args:?}"
            );
        }

        // `daemon version` / `daemon update` still require daemon settings integrity
        assert!(upstream_invocation_requires_daemon_settings(&os_vec(&[
            "app-server",
            "daemon",
            "version",
        ])));
        assert!(upstream_invocation_requires_daemon_settings(&os_vec(&[
            "app-server",
            "daemon",
            "update",
            "--from-cli",
            "--yes",
        ])));
    }

    fn plan_public_dispatch(args: Vec<OsString>) -> Result<PublicDispatchRoute, DispatchPlanError> {
        let scan = UpstreamArgScan::new(&args);
        super::plan_public_dispatch(args, &scan)
    }
    fn is_help_or_version_invocation(args: &[OsString]) -> bool {
        UpstreamArgScan::new(args).help_or_version
    }
    fn upstream_invocation_initializes_managed_daemon(args: &[OsString]) -> bool {
        UpstreamArgScan::new(args).upstream_invocation_initializes_managed_daemon
    }
    fn upstream_invocation_requires_daemon_settings(args: &[OsString]) -> bool {
        UpstreamArgScan::new(args).upstream_invocation_requires_daemon_settings
    }
}

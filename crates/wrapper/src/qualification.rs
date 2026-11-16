//! Bounded, account-free version and configuration admission of a candidate.
//! Never entered by ordinary launch, status, doctor, or release inspection.
use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::fs_util;
use crate::unix_app::{
    apply_child_env_plan_and_fence, plan_termux_env, termux_browser_helper_paths,
    LocalProductError, LocalWrapperRoots, QualifiedRuntimeAssets, RuntimeFdSources,
    TermuxBaseEnvPlan, TermuxProcessEnvSnapshot, LOCAL_STAGING_COUNTER,
};

const OUTPUT_LIMIT: u64 = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

fn rejected(message: &'static str) -> LocalProductError {
    LocalProductError::CandidateProbe(message)
}
fn io_error(source: io::Error) -> LocalProductError {
    LocalProductError::Io {
        operation: "qualify candidate in isolated temporary home",
        source,
    }
}

fn temporary_home() -> io::Result<PathBuf> {
    for _ in 0..32 {
        let seq = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cq-{}-{seq}", std::process::id()));
        match fs::DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::other(
        "candidate probe temporary directory collision",
    ))
}

struct Probe<'a> {
    program: &'a std::ffi::OsStr,
    home: &'a Path,
    fds: RuntimeFdSources,
    env: TermuxBaseEnvPlan,
    sequence: usize,
}

impl Probe<'_> {
    fn run(&mut self, args: &[&str]) -> Result<Vec<u8>, LocalProductError> {
        self.run_with_timeout(args, COMMAND_TIMEOUT)
    }

    fn run_with_timeout(
        &mut self,
        args: &[&str],
        timeout: Duration,
    ) -> Result<Vec<u8>, LocalProductError> {
        self.sequence += 1;
        let path = self.home.join(format!("probe-{}.out", self.sequence));
        let errors = self.home.join(format!("probe-{}.err", self.sequence));
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(io_error)?;
        let stderr = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&errors)
            .map_err(io_error)?;
        let output_watch = output.try_clone().map_err(io_error)?;
        let error_watch = stderr.try_clone().map_err(io_error)?;
        let mut command = Command::new(self.program);
        command.args(args);
        apply_child_env_plan_and_fence(&mut command, Some(&self.env));
        command
            .env("HOME", self.home)
            .env("CODEX_HOME", self.home)
            .current_dir(self.home)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .stdin(Stdio::null())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(stderr));
        self.fds.configure(&mut command);
        let mut child = command.spawn().map_err(io_error)?;
        let deadline = Instant::now() + timeout;
        let result = (|| loop {
            let status = child.try_wait().map_err(io_error)?;
            if output_watch.metadata().map_err(io_error)?.len() > OUTPUT_LIMIT
                || error_watch.metadata().map_err(io_error)?.len() > OUTPUT_LIMIT
            {
                return Err(rejected("candidate probe output exceeded its bound"));
            }
            if let Some(status) = status {
                return if status.success() {
                    fs_util::read_bounded_regular(
                        &path,
                        OUTPUT_LIMIT,
                        true,
                        "candidate output is unsafe or oversized",
                    )
                    .map_err(io_error)
                } else {
                    Err(rejected("candidate runtime command failed"))
                };
            }
            if Instant::now() >= deadline {
                return Err(rejected("candidate runtime command timed out"));
            }
            std::thread::sleep(Duration::from_millis(10));
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }
}

pub(super) fn probe(
    assets: QualifiedRuntimeAssets<'_, '_>,
    process_env: &TermuxProcessEnvSnapshot,
    roots: &LocalWrapperRoots,
    expected_version: &str,
) -> Result<(), LocalProductError> {
    let selection = assets.selection();
    let env = plan_termux_env(
        process_env,
        selection.generation_dir,
        termux_browser_helper_paths(selection)
            .map_err(|_| rejected("candidate browser environment unavailable"))?,
        roots.cert_file.as_os_str(),
        Some(roots.cert_dir.as_os_str()),
    )
    .map_err(|_| rejected("candidate Termux environment unavailable"))?;
    let home = temporary_home().map_err(io_error)?;
    let result = (|| {
        let fds =
            RuntimeFdSources::open(&roots.resolver_path, &roots.config_dir).map_err(io_error)?;
        let mut probe = Probe {
            program: selection.runtime.program_path,
            home: &home,
            fds,
            env,
            sequence: 0,
        };
        let version = probe.run(&["--version"])?;
        if String::from_utf8_lossy(&version).trim() != format!("codex-cli {expected_version}") {
            return Err(rejected(
                "candidate version output does not match archive version",
            ));
        }
        probe.run(&["features", "list"])?;
        Ok(())
    })();
    fs::remove_dir_all(&home).map_err(io_error)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::os::unix::fs::PermissionsExt as _;

    #[test]
    fn candidate_commands_reject_failure_timeout_and_both_oversized_streams() {
        let root = crate::tests::temp_root("candidate-bounds");
        let config = root.join("config");
        fs::create_dir(&config).unwrap();
        let resolver = root.join("resolv.conf");
        fs::write(&resolver, b"nameserver 1.1.1.1\n").unwrap();
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let program = root.join("probe.sh");
        let process_env = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(root.clone().into_os_string()),
            tmpdir: Some(root.clone().into_os_string()),
            inherited_path: Some("/system/bin:/bin:/usr/bin".into()),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let env = plan_termux_env(
            &process_env,
            root.as_os_str(),
            None,
            OsStr::new("/cert.pem"),
            None,
        )
        .unwrap();
        let fds = RuntimeFdSources::open(&resolver, &config).unwrap();
        let mut probe = Probe {
            program: program.as_os_str(),
            home: &root,
            fds,
            env,
            sequence: 0,
        };
        for source in [
            "exit 7",
            "while :; do :; done",
            "i=0; while [ $i -lt 70000 ]; do printf x; i=$((i+1)); done",
            "i=0; while [ $i -lt 70000 ]; do printf x >&2; i=$((i+1)); done",
        ] {
            fs::write(&program, format!("#!{shell}\n{source}\n")).unwrap();
            fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
            assert!(probe
                .run_with_timeout(&[], Duration::from_millis(150))
                .is_err());
        }
        fs::write(
            &program,
            format!("#!{shell}\nprintf '%s' \"$CODEX_HOME\"\n"),
        )
        .unwrap();
        assert_eq!(probe.run(&[]).unwrap(), root.as_os_str().as_encoded_bytes());
        crate::tests::remove_temp_root(root);
    }
}

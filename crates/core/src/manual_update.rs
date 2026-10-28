//! Manual freshness and owner CI requests; signed admission remains in Core.
use super::*;
use std::io::{IsTerminal, Read};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const WORKFLOW: &str = "repos/humtr/codex/actions/workflows/accepted-release-inputs.yml";
const WAIT_LIMIT: Duration = Duration::from_secs(2 * 60 * 60);
const RECEIPT: &[u8] = b"codex-manual-build-v1\nsubmitted-or-uncertain\n";

fn error(message: &'static str) -> LocalProductError {
    LocalProductError::LocalUpdate(message)
}

fn newer(candidate: &str, installed: &str) -> bool {
    fn parts(value: &str) -> Option<Vec<u64>> {
        valid_update_version(value)
            .then(|| {
                value
                    .split('.')
                    .map(str::parse)
                    .collect::<Result<_, _>>()
                    .ok()
            })
            .flatten()
    }
    matches!((parts(candidate), parts(installed)), (Some(a), Some(b)) if a > b)
}

// GitHub CLI owns credentials. Never inherit workload input or print its output
// on failure, and reap the exact child even when a dispatch result is uncertain.
fn gh(program: &Path, args: &[&str]) -> Result<Vec<u8>, LocalProductError> {
    let (mut reader, writer) = UnixStream::pair().map_err(|_| error("cannot start GitHub CLI"))?;
    reader
        .set_nonblocking(true)
        .map_err(|_| error("cannot read GitHub CLI"))?;
    let mut child = Command::new(program)
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_PAGER", "cat")
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            error("a missing build requires GitHub CLI; install gh and run gh auth login")
        })?;
    let started = Instant::now();
    let result = (|| {
        let mut bytes = Vec::new();
        let mut eof = false;
        loop {
            #[cfg(any(target_os = "android", target_os = "linux"))]
            if WAIT_SIGNAL.load(std::sync::atomic::Ordering::Relaxed) != 0 {
                return Err(error("GitHub CLI interrupted"));
            }
            let mut buffer = [0; 1024];
            match reader.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(n) => bytes.extend_from_slice(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(error("GitHub CLI response could not be read")),
            }
            if bytes.len() > 4096 || started.elapsed() > Duration::from_secs(30) {
                return Err(error("GitHub CLI exceeded its response/time bound"));
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|_| error("GitHub CLI status is unknown"))?
            {
                if !status.success() {
                    return Err(error("GitHub build request/status failed; check gh auth status and repository access"));
                }
                if eof {
                    return Ok(bytes);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn request(roots: &LocalCoreRoots, version: &str) -> Result<(), LocalProductError> {
    if !valid_update_version(version) {
        return Err(error("invalid official build request version"));
    }
    let program = roots.openssl.with_file_name("gh");
    let query = format!(
        "[.workflow_runs[] | select(.display_title==\"Codex build request {version}\" and .head_branch==\"main\" and .event==\"workflow_dispatch\" and .status!=\"completed\") | .id] | first // 0"
    );
    let bytes = gh(
        &program,
        &[
            "api",
            &format!("{WORKFLOW}/runs?per_page=100"),
            "--jq",
            &query,
        ],
    )?;
    let id = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .ok_or_else(|| error("GitHub build status is malformed; request not sent"))?;
    if id != 0 {
        println!("Reusing pending build: https://github.com/humtr/codex/actions/runs/{id}");
        return Ok(());
    }
    // The receipt precedes the mutation. Only a returned, subsequently observed
    // failed run permits a later explicit update invocation to request again.
    let receipt = roots.state_root.join(format!("manual-build-{version}"));
    match std::fs::symlink_metadata(&receipt) {
        Ok(_) => {
            let bytes = read_bounded_regular_file(
                &receipt,
                256,
                "read manual build receipt",
                error("manual build receipt exceeds its bound"),
                error("manual build receipt must be a regular file"),
            )?;
            let Some(tail) = bytes.strip_prefix(RECEIPT) else {
                return Err(error("manual build receipt is unknown; request not sent"));
            };
            if tail.is_empty() {
                println!("A build request for {version} is recorded but its result is uncertain; checking publication without resending it.");
                return Ok(());
            }
            let run = std::str::from_utf8(tail)
                .ok()
                .and_then(|s| s.strip_prefix("run\t"))
                .and_then(|s| s.strip_suffix('\n'))
                .and_then(|s| s.parse::<u64>().ok())
                .filter(|id| *id != 0)
                .ok_or_else(|| error("manual build receipt is unknown; request not sent"))?;
            let status = gh(
                &program,
                &[
                    "api",
                    &format!("repos/humtr/codex/actions/runs/{run}"),
                    "--jq",
                    "[.status, (.conclusion // \"pending\")] | @tsv",
                ],
            )?;
            let failed = matches!(
                status.as_slice(),
                b"completed\tfailure\n"
                    | b"completed\tcancelled\n"
                    | b"completed\ttimed_out\n"
                    | b"completed\taction_required\n"
                    | b"completed\tstartup_failure\n"
            );
            if !failed {
                println!("Build request: https://github.com/humtr/codex/actions/runs/{run}; checking qualification/publication without resending it.");
                return Ok(());
            }
            // Preserve the failed receipt and refuse a concurrent rotation rather
            // than overwriting another updater's newer intent.
            let archived = roots
                .state_root
                .join(format!("manual-build-{version}-failed-{run}"));
            rename_noreplace(&receipt, &archived)
                .map_err(|_| error("cannot preserve failed build receipt; request not sent"))?;
            sync_directory(&roots.state_root)?;
            println!(
                "Previous request {run} failed; this explicit update starts a new build request."
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(error("cannot inspect the existing build request receipt")),
    }
    let mut file = create_remote_output(&receipt)?;
    use std::io::Write;
    file.write_all(RECEIPT)
        .and_then(|()| file.sync_all())
        .map_err(|_| error("cannot preserve build request intent"))?;
    sync_directory(&roots.state_root)?;
    let result = gh(
        &program,
        &[
            "api",
            "--method",
            "POST",
            &format!("{WORKFLOW}/dispatches"),
            "--jq",
            ".workflow_run_id",
            "-F",
            "return_run_details=true",
            "-f",
            "ref=main",
            "-F",
            "inputs[discover_upstream]=true",
            "-f",
            &format!("inputs[requested_upstream]={version}"),
        ],
    );
    let run = result.ok().and_then(|bytes| std::str::from_utf8(&bytes).ok()
        .and_then(|text| text.trim().parse::<u64>().ok())).filter(|run| *run != 0)
        .ok_or_else(|| error("build dispatch failed or is uncertain; the intent is preserved and will not be resent; inspect GitHub Actions"))?;
    writeln!(file, "run\t{run}").and_then(|()| file.sync_all())
        .map_err(|_| error("build was requested but its run receipt could not be saved; request will not be resent"))?;
    println!("Requested Codex {version}: https://github.com/humtr/codex/actions/runs/{run}");
    Ok(())
}

fn official(roots: &LocalCoreRoots) -> Result<OfficialReleaseMetadata, LocalProductError> {
    let seq = REMOTE_ACQUISITION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = roots
        .state_root
        .join(format!(".manual-update-{}-{seq}", std::process::id()));
    create_remote_acquisition_root(&root)?;
    let result = resolve_official_release_metadata(roots, &root);
    let cleanup = std::fs::remove_dir_all(&root);
    if cleanup.is_err() {
        return Err(error("cannot remove manual update metadata staging"));
    }
    result
}

struct QualificationFailure {
    source: String,
    workflow: &'static str,
    run: u64,
    conclusion: String,
}

// This is diagnostic evidence. Only the signed index can admit an installation.
fn qualification_failure(
    roots: &LocalCoreRoots,
    version: &str,
) -> Result<Option<QualificationFailure>, LocalProductError> {
    let program = roots.openssl.with_file_name("gh");
    let source = gh(
        &program,
        &[
            "api",
            "repos/humtr/codex/git/ref/heads/rewrite/rust-core",
            "--jq",
            ".object.sha",
        ],
    )?;
    let source = std::str::from_utf8(&source)
        .map_err(|_| error("build source status is not UTF-8"))?
        .trim();
    if source.len() != 40
        || !source
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(error("build source status is malformed"));
    }
    let selected = gh(
        &program,
        &[
            "api",
            &format!("repos/humtr/codex/contents/release/upstream.json?ref={source}"),
            "--jq",
            ".content | @base64d | fromjson | .upstream.version",
        ],
    )?;
    let selected = std::str::from_utf8(&selected)
        .map_err(|_| error("build version status is not UTF-8"))?
        .trim();
    if !valid_update_version(selected) {
        return Err(error("build version status is malformed"));
    }
    if selected != version {
        return Ok(None);
    }
    for workflow in [
        "rald7-full-acceptance.yml",
        "core-runtime-qualification.yml",
    ] {
        let query = format!(
            "[.workflow_runs[] | select(.head_sha==\"{source}\" and .head_branch==\"rewrite/rust-core\" and .path==\".github/workflows/{workflow}\" and .repository.full_name==\"humtr/codex\" and (.event==\"push\" or .event==\"workflow_dispatch\"))] | if any(.status!=\"completed\") or any(.status==\"completed\" and .conclusion==\"success\") then \"0\\tpending\" else (.[0] // {{}}) | [(.id // 0), (.conclusion // \"pending\")] | @tsv end"
        );
        let status = gh(&program, &["api", &format!("repos/humtr/codex/actions/workflows/{workflow}/runs?head_sha={source}&per_page=100"), "--jq", &query])?;
        let status =
            std::str::from_utf8(&status).map_err(|_| error("qualification status is not UTF-8"))?;
        let (run, conclusion) = status
            .trim_end_matches('\n')
            .split_once('\t')
            .ok_or_else(|| error("qualification status is malformed"))?;
        let run = run
            .parse::<u64>()
            .map_err(|_| error("qualification run ID is malformed"))?;
        if run == 0 && conclusion == "pending" {
            continue;
        }
        if run == 0
            || !matches!(
                conclusion,
                "failure"
                    | "cancelled"
                    | "timed_out"
                    | "action_required"
                    | "startup_failure"
                    | "skipped"
                    | "stale"
            )
        {
            return Err(error("qualification conclusion is unknown"));
        }
        return Ok(Some(QualificationFailure {
            source: source.to_owned(),
            workflow,
            run,
            conclusion: conclusion.to_owned(),
        }));
    }
    Ok(None)
}

// This input scope is dropped before continuing into the installed TUI.
#[cfg(any(target_os = "android", target_os = "linux"))]
struct WaitInput(Option<Termios>, Option<[usize; 5]>);
#[cfg(any(target_os = "android", target_os = "linux"))]
static WAIT_SIGNAL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
#[cfg(any(target_os = "android", target_os = "linux"))]
const WAIT_SIGNALS: [i32; 5] = [1, 2, 3, 15, 20];
#[cfg(any(target_os = "android", target_os = "linux"))]
extern "C" fn record_wait_signal(number: i32) {
    WAIT_SIGNAL.store(number, std::sync::atomic::Ordering::Relaxed);
}
#[cfg(any(target_os = "android", target_os = "linux"))]
#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    input: u32,
    output: u32,
    control: u32,
    local: u32,
    line: u8,
    chars: [u8; 19],
}
#[cfg(any(target_os = "android", target_os = "linux"))]
unsafe extern "C" {
    fn ioctl(fd: i32, request: std::os::raw::c_ulong, ...) -> i32;
    fn read(fd: i32, buffer: *mut std::ffi::c_void, size: usize) -> isize;
    fn signal(number: i32, handler: usize) -> usize;
}
unsafe extern "C" {
    fn raise(signal: i32) -> i32;
}
#[cfg(not(any(target_os = "android", target_os = "linux")))]
struct WaitInput;
#[cfg(not(any(target_os = "android", target_os = "linux")))]
impl WaitInput {
    fn new(_: bool) -> Self {
        Self
    }
    fn key(&self) -> Option<u8> {
        std::thread::sleep(Duration::from_millis(100));
        None
    }
    fn signal(&self, _: u8) -> Option<i32> {
        None
    }
    fn pending_signal(&self) -> Option<i32> {
        None
    }
}
#[cfg(any(target_os = "android", target_os = "linux"))]
impl WaitInput {
    fn new(enabled: bool) -> Self {
        let mut saved: Termios = unsafe { std::mem::zeroed() };
        if !enabled || unsafe { ioctl(0, 0x5401, &mut saved) } != 0 {
            return Self(None, None);
        }
        WAIT_SIGNAL.store(0, std::sync::atomic::Ordering::Relaxed);
        let mut handlers = [0; 5];
        for (index, number) in WAIT_SIGNALS.into_iter().enumerate() {
            let previous = unsafe { signal(number, record_wait_signal as *const () as usize) };
            if previous == usize::MAX {
                for prior in 0..index {
                    let _ = unsafe { signal(WAIT_SIGNALS[prior], handlers[prior]) };
                }
                return Self(None, None);
            }
            handlers[index] = previous;
        }
        let mut input = Self(None, Some(handlers));
        let mut mode = saved;
        mode.local &= !(1 | 2 | 8 | 64); // Restore terminal state before raising control-key signals.
        mode.chars[6] = 0;
        mode.chars[5] = 1;
        if unsafe { ioctl(0, 0x5402, &mode) } != 0 {
            return input;
        }
        input.0 = Some(saved);
        input
    }
    fn key(&self) -> Option<u8> {
        if self.0.is_none() {
            std::thread::sleep(Duration::from_millis(100));
            return None;
        }
        let mut byte = 0;
        (unsafe { read(0, (&mut byte as *mut u8).cast(), 1) } == 1).then_some(byte)
    }
    fn signal(&self, key: u8) -> Option<i32> {
        let saved = self.0.as_ref()?;
        [(0, 2), (1, 3), (10, 20)]
            .into_iter()
            .find_map(|(index, signal)| (key != 0 && key == saved.chars[index]).then_some(signal))
    }
    fn pending_signal(&self) -> Option<i32> {
        let number = WAIT_SIGNAL.swap(0, std::sync::atomic::Ordering::Relaxed);
        (number != 0).then_some(number)
    }
}
#[cfg(any(target_os = "android", target_os = "linux"))]
impl Drop for WaitInput {
    fn drop(&mut self) {
        if let Some(saved) = &self.0 {
            let _ = unsafe { ioctl(0, 0x5402, saved) };
        }
        if let Some(handlers) = self.1 {
            for (number, handler) in WAIT_SIGNALS.into_iter().zip(handlers) {
                let _ = unsafe { signal(number, handler) };
            }
        }
        if let Some(number) = self.pending_signal() {
            let _ = unsafe { raise(number) };
        }
    }
}

fn launch_current(roots: &LocalCoreRoots, env: &TermuxProcessEnvSnapshot) -> i32 {
    match execute_activated_route(PublicDispatchRoute::Upstream(Vec::new()), roots, env) {
        Ok(PublicDispatchCompletion::TermuxUnavailable(message)) => {
            eprintln!("{message}");
            1
        }
        Ok(_) => 0,
        Err(error) => {
            eprintln!("codex: {error}");
            1
        }
    }
}

pub(super) fn finish(
    roots: &LocalCoreRoots,
    env: &TermuxProcessEnvSnapshot,
    outcome: SignedUpdateOutcome,
    no_wait: bool,
    presentation: &mut UpdatePresentation,
) -> Result<i32, LocalProductError> {
    if rollback_guard::effective_update_hold(roots, None)?.is_some() {
        presentation.clear_transient();
        println!(
            "Rollback hold is active; keeping the installed release without requesting a build."
        );
        return Ok(0);
    }
    let installed = match &outcome {
        SignedUpdateOutcome::Activated(v) => {
            presentation.finish_signed_activation(&v.current_version);
            &v.current_version
        }
        SignedUpdateOutcome::AlreadyCurrent(v) => &v.version,
    };
    let latest = official(roots)?;
    let upstream_is_newer = newer(&latest.version, installed);
    if !upstream_is_newer && matches!(outcome, SignedUpdateOutcome::AlreadyCurrent(_)) {
        presentation.finish_current(installed);
    }
    presentation.clear_transient();
    presentation.show_release_details();
    println!(
        "{}",
        release_display::upstream(&latest.version, latest.publication.as_ref())
    );
    if !upstream_is_newer {
        return Ok(0);
    }
    println!(
        "Installed Codex {installed}; official {} needs a qualified Termux release.",
        latest.version
    );
    request(roots, &latest.version)?;
    let tty = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
    if no_wait {
        println!("Build request recorded. Continue with Codex {installed}.");
        return Ok(if tty { launch_current(roots, env) } else { 0 });
    }
    println!(
        "Waiting for signed publication. [c] Continue with current version  [q] Leave updater"
    );
    let input = WaitInput::new(tty);
    let begin = Instant::now();
    let mut next_poll = Instant::now();
    let mut next_qualification = Instant::now();
    let mut seen = outcome.generation_id().to_owned();
    while begin.elapsed() < WAIT_LIMIT {
        let key = input.key();
        if let Some(signal) = input
            .pending_signal()
            .or_else(|| key.and_then(|key| input.signal(key)))
        {
            drop(input);
            let _ = unsafe { raise(signal) };
            return Ok(128 + signal);
        }
        match key.map(|b| b.to_ascii_lowercase()) {
            Some(b'c') => {
                drop(input);
                return Ok(launch_current(roots, env));
            }
            Some(b'q') => return Ok(0),
            _ => (),
        }
        if Instant::now() < next_poll {
            continue;
        }
        next_poll = Instant::now() + Duration::from_secs(20);
        let paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
            .map_err(LocalProductError::StateFormat)?;
        let state = m2_generation_state::read_pointer_state(&paths)
            .map_err(LocalProductError::State)?
            .ok_or(LocalProductError::NoCurrentGeneration)?;
        let index = match fetch_signed_update_index_with_timeout(roots, state.update_key, "5") {
            Ok(index) => Some(index),
            Err(LocalProductError::RemoteTransportFailed) => None,
            Err(error) => return Err(error),
        };
        if index
            .as_ref()
            .is_none_or(|index| index.generation_id == seen)
        {
            if Instant::now() >= next_qualification {
                next_qualification = Instant::now() + Duration::from_secs(120);
                match qualification_failure(roots, &latest.version) {
                    Ok(Some(failure)) => {
                        presentation.clear_transient();
                        println!(
                            "Termux qualification failed: {} ({}) at source {}.",
                            failure.workflow, failure.conclusion, failure.source
                        );
                        println!(
                            "https://github.com/humtr/codex/actions/runs/{}",
                            failure.run
                        );
                        println!("Codex {installed} remains installed. Build-request acceptance did not produce a qualified release.");
                        return Ok(1);
                    }
                    Ok(None) => (),
                    Err(reason) => {
                        if let Some(signal) = input.pending_signal() {
                            drop(input);
                            let _ = unsafe { raise(signal) };
                            return Ok(128 + signal);
                        }
                        eprintln!(
                            "Build status unavailable: {reason}; signed publication is still checked."
                        );
                    }
                }
            }
            continue;
        }
        // Only normal signed activation may admit the candidate. This also
        // rechecks any hold or concurrent selection made during the wait.
        let result = {
            let _lease = maintenance::UpdateLease::acquire(roots)?;
            activate_signed_update_channel_with_hold_policy(
                roots,
                env,
                UpdateHoldPolicy::Enforce,
                Some(presentation),
            )?
        };
        seen = result.generation_id().to_owned();
        let version = match &result {
            SignedUpdateOutcome::Activated(v) => &v.current_version,
            SignedUpdateOutcome::AlreadyCurrent(v) => &v.version,
        };
        if !newer(&latest.version, version) {
            match result {
                SignedUpdateOutcome::Activated(v) => {
                    presentation.finish_signed_activation(&v.current_version)
                }
                SignedUpdateOutcome::AlreadyCurrent(v) => presentation.finish_current(&v.version),
            }
            return Ok(0);
        }
    }
    Err(error("build is still pending after two hours; the request is preserved; codex starts the installed version"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn fixture(body: &str) -> (std::path::PathBuf, LocalCoreRoots) {
        let root = crate::tests::temp_root("manual-build");
        let roots = LocalCoreRoots {
            generation_root: root.join("generations"),
            state_root: root.clone(),
            config_dir: root.join("config"),
            resolver_path: root.join("resolv.conf"),
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
            openssl: root.join("openssl"),
            curl: root.join("curl"),
        };
        let sh = crate::tests::resolve_test_tool("sh").unwrap();
        let script = root.join("gh");
        std::fs::write(&script, format!("#!{}\n{body}\n", sh.to_string_lossy())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        (root, roots)
    }

    #[test]
    fn request_ack_and_exact_qualification_failure_are_separate() {
        for (selected, status, failure) in [
            ("0.165.0", "42\\tfailure", true),
            ("0.165.0", "0\\tpending", false),
            ("0.164.0", "42\\tfailure", false),
        ] {
            let (root, roots) = fixture(&format!(
                "case \"$2\" in */git/ref/*) printf '{}\\n';; */contents/release/*) printf '{selected}\\n';; *core-runtime-qualification*) printf '{status}\\n';; *) printf '0\\tpending\\n';; esac", "a".repeat(40)));
            let observed = qualification_failure(&roots, "0.165.0").unwrap();
            assert_eq!(observed.is_some(), failure);
            if let Some(observed) = observed {
                assert_eq!(observed.run, 42);
                assert_eq!(observed.workflow, "core-runtime-qualification.yml");
                assert_eq!(observed.conclusion, "failure");
            }
            assert!(!root.join("manual-build-0.165.0").exists());
            crate::tests::remove_temp_root(root);
        }
    }

    #[test]
    fn build_requests_are_once_and_uncertain_mutations_are_not_resent() {
        for exit in [0, 1] {
            let (root, roots) = fixture(&format!(
                "case \"$2\" in --method) printf 'effect\\n' >> \"$0.effects\"; printf '12345\\n'; exit {exit};; */actions/runs/*) printf 'completed\\tsuccess\\n';; *) printf '0\\n';; esac"));
            assert_eq!(request(&roots, "0.164.0").is_ok(), exit == 0);
            assert!(request(&roots, "0.164.0").is_ok());
            assert_eq!(std::fs::read(root.join("gh.effects")).unwrap(), b"effect\n");
            let mut expected = RECEIPT.to_vec();
            if exit == 0 {
                expected.extend_from_slice(b"run\t12345\n");
            }
            assert_eq!(
                std::fs::read(root.join("manual-build-0.164.0")).unwrap(),
                expected
            );
            crate::tests::remove_temp_root(root);
        }
    }

    #[test]
    fn only_a_confirmed_failed_request_permits_a_new_explicit_attempt() {
        let (root, roots) = fixture(
            "case \"$2\" in --method) printf 'effect\\n' >> \"$0.effects\"; printf '88\\n';; */actions/runs/77) printf 'completed\\tfailure\\n';; */actions/runs/88) printf 'in_progress\\tpending\\n';; *) printf '0\\n';; esac");
        let mut previous = RECEIPT.to_vec();
        previous.extend_from_slice(b"run\t77\n");
        std::fs::write(root.join("manual-build-0.164.0"), &previous).unwrap();
        assert!(request(&roots, "0.164.0").is_ok());
        assert!(request(&roots, "0.164.0").is_ok());
        assert_eq!(std::fs::read(root.join("gh.effects")).unwrap(), b"effect\n");
        assert_eq!(
            std::fs::read(root.join("manual-build-0.164.0-failed-77")).unwrap(),
            previous
        );
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn pending_build_is_reused_and_status_failure_never_dispatches() {
        for body in ["printf '12345\\n'", "exit 1"] {
            let (root, roots) = fixture(body);
            assert_eq!(
                request(&roots, "0.164.0").is_ok(),
                body.starts_with("printf")
            );
            assert!(!root.join("manual-build-0.164.0").exists());
            assert!(request(&roots, "0.164.0-alpha").is_err());
            crate::tests::remove_temp_root(root);
        }
    }

    #[test]
    fn unknown_receipt_is_preserved_without_dispatch() {
        let (root, roots) = fixture("printf '0\\n'");
        let receipt = root.join("manual-build-0.164.0");
        std::fs::write(&receipt, b"user-owned unknown state").unwrap();
        assert!(request(&roots, "0.164.0").is_err());
        assert_eq!(std::fs::read(receipt).unwrap(), b"user-owned unknown state");
        crate::tests::remove_temp_root(root);
    }
    #[test]
    fn stable_comparison_is_numeric_and_rejects_nonstable_values() {
        assert!(newer("0.164.0", "0.163.0"));
        assert!(newer("0.1000.0", "0.999.9"));
        for a in ["0.163.0", "0.162.9", "0.164.0-alpha", "00.164.0", "unknown"] {
            assert!(!newer(a, "0.163.0"));
        }
    }
}

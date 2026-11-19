use super::*;
use crate::fs_util::{copy_local_directory_tree, LOCK_EX as FLOCK_EX, LOCK_NB as FLOCK_NB};
use crate::generation::{load_local_generation, probe_release_candidate, sha256_regular_file};
use crate::runtime_env::prepare_wrapper_config;
use crate::state::generation_state;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);
pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct EnvRestoreGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvRestoreGuard {
    pub(crate) fn new(vars: &[(&'static str, Option<&Path>)]) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut saved = Vec::with_capacity(vars.len());
        for &(key, val) in vars {
            saved.push((key, std::env::var_os(key)));
            match val {
                Some(p) => unsafe { std::env::set_var(key, p) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
        Self { _lock: lock, saved }
    }
}

impl Drop for EnvRestoreGuard {
    fn drop(&mut self) {
        for (key, prev) in self.saved.iter().rev() {
            match prev {
                Some(v) => unsafe { std::env::set_var(key, v) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

pub(crate) fn temp_root(label: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "codex-wrapper-test-{label}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(crate) fn remove_temp_root(path: impl AsRef<Path>) {
    let _ = fs::remove_dir_all(path);
}

pub(crate) fn os_vec(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

pub(crate) fn test_process_env(roots: &LocalWrapperRoots) -> TermuxProcessEnvSnapshot {
    TermuxProcessEnvSnapshot {
        wsl_kernel: Some(false),
        prefix: roots
            .curl
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.as_os_str().to_owned()),
        tmpdir: Some(roots.state_root.clone().into_os_string()),
        inherited_path: Some(OsString::from("/system/bin")),
        inherited_ssl_cert_file: Some(roots.cert_file.clone().into_os_string()),
        inherited_ssl_cert_dir: Some(roots.cert_dir.clone().into_os_string()),
    }
}

pub(crate) fn create_private_test_dirs(path: &Path) {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
}

pub(crate) fn make_test_roots(root: &Path) -> LocalWrapperRoots {
    let prefix = std::env::var_os("PREFIX")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr"));
    let state_root = root.join("state");
    let config_dir = state_root.join("config");
    let generation_root = root.join("generations");
    create_private_test_dirs(&config_dir);
    fs::create_dir_all(&generation_root).unwrap();
    let resolver_path = root.join("resolv.conf");
    fs::write(&resolver_path, b"nameserver 1.1.1.1\n").unwrap();
    LocalWrapperRoots {
        generation_root,
        state_root,
        config_dir,
        resolver_path,
        cert_file: prefix.join("etc/tls/cert.pem"),
        cert_dir: prefix.join("etc/tls/certs"),
        curl: prefix.join("bin/curl"),
    }
}

pub(crate) fn write_sample_generation(dir: &Path, generation_id: &str, with_code_mode_host: bool) {
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::create_dir_all(dir.join("browser/open")).unwrap();
    fs::create_dir_all(dir.join("browser/manual")).unwrap();
    fs::create_dir_all(dir.join("codex-resources/sub")).unwrap();
    fs::write(dir.join("bin/codex"), b"#!/bin/sh\nif [ \"$1\" = --version ]; then echo codex-cli 1.2.3; else echo codex 1.2.3; fi\n").unwrap();
    fs::set_permissions(dir.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();
    if with_code_mode_host {
        fs::write(dir.join("bin/codex-code-mode-host"), b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(
            dir.join("bin/codex-code-mode-host"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    fs::write(dir.join("codex-package.json"), b"{\"version\":\"1.2.3\"}\n").unwrap();
    fs::write(dir.join("browser/open/curl"), b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(
        dir.join("browser/open/curl"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::write(dir.join("browser/manual/curl"), b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(
        dir.join("browser/manual/curl"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let _ = std::os::unix::fs::symlink("bin/codex", dir.join("codex"));

    let runtime_digest = sha256_regular_file(&dir.join("bin/codex")).unwrap();
    let open_digest = sha256_regular_file(&dir.join("browser/open/curl")).unwrap();
    let manual_digest = sha256_regular_file(&dir.join("browser/manual/curl")).unwrap();
    let digest = "a".repeat(64);
    let meta = format!(
            "codex-local-generation\n\
             generation_id\t{generation_id}\n\
             upstream_package_identity\topenai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz\n\
             upstream_package_version\t1.2.3\n\
             source_artifact_digest\t{digest}\n\
             expected_platform\tandroid\n\
             expected_architecture\taarch64\n\
             patch_policy_id\ttermux-fd-remap\n\
             patch_report\ttermux-fd-remap;changed_bytes=0\n\
             runtime_digest\t{runtime_digest}\n\
             wrapper_artifact_digest\t{digest}\n\
             wrapper_api_identity\twrapper-api\n\
             persistent_schema_identity\tschema\n\
             qualification\tqualified\n\
             creation_metadata\tcodex-wrapper\n\
             helper_count\t2\n\
             helper\ttermux-browser-open\t{open_digest}\n\
             helper\ttermux-browser-manual\t{manual_digest}\n"
        );
    fs::write(dir.join("generation.meta"), meta).unwrap();
}

#[test]
fn wrapper_status_holds_generation_lease_during_verification() {
    const CHILD: &str = "CODEX_WP3_STATUS_LEASE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::process::exit(run_public_main(os_vec(&["wrapper", "status", "--verify"])));
    }
    let root = temp_root("status-launch-lease");
    let home = root.join("home");
    let prefix = root.join("prefix");
    let _env = EnvRestoreGuard::new(&[
        ("HOME", Some(&home)),
        ("PREFIX", Some(&prefix)),
        ("TMPDIR", Some(&root)),
    ]);
    let roots = LocalWrapperRoots::from_environment().unwrap();
    create_private_test_dirs(&roots.state_root);
    create_private_test_dirs(roots.resolver_path.parent().unwrap());
    fs::write(&roots.resolver_path, b"nameserver 1.1.1.1\n").unwrap();

    let stage = roots.generation_root.join("gen-1");
    write_sample_generation(&stage, "gen-1", false);
    let runtime = stage.join("bin/codex");
    let old_digest = sha256_regular_file(&runtime).unwrap();
    let shell = if Path::new("/system/bin/sh").is_file() {
        "/system/bin/sh"
    } else {
        "/bin/sh"
    };
    let entered = root.join("entered-probe");
    let proceed = root.join("finish-probe");
    fs::write(
        &runtime,
        format!(
            r#"#!{shell}
if [ "$1" = --version ]; then
    : > '{}'
    while [ ! -f '{}' ]; do sleep 0.02; done
    echo codex-cli 1.2.3
fi
"#,
            entered.display(),
            proceed.display()
        ),
    )
    .unwrap();
    let new_digest = sha256_regular_file(&runtime).unwrap();
    let meta = stage.join("generation.meta");
    let manifest = fs::read_to_string(&meta).unwrap().replace(
        &format!("runtime_digest\t{old_digest}"),
        &format!("runtime_digest\t{new_digest}"),
    );
    fs::write(meta, manifest).unwrap();
    let paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
    let pointer = generation_state::plan_initial_pointer_state("gen-1").unwrap();
    fs::write(
        &paths.activation_state,
        generation_state::encode_pointer_state(&pointer).unwrap(),
    )
    .unwrap();

    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::wrapper_status_holds_generation_lease_during_verification",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !entered.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let entered_probe = entered.exists();
    let other = fs::File::open(roots.generation_root.parent().unwrap()).unwrap();
    let contention = crate::fs_util::flock_fd(&other, FLOCK_EX | FLOCK_NB);
    fs::write(&proceed, b"").unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(entered_probe && result.status.success(), "{result:?}");
    assert_eq!(
        contention.unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    crate::fs_util::flock_fd(&other, FLOCK_EX | FLOCK_NB).unwrap();
    drop(_env);
    remove_temp_root(root);
}

#[test]
fn e2e_activation_doctor_status_and_rollback_in_isolated_roots() {
    const LAUNCH_CHILD: &str = "CODEX_TEST_MISSING_ADDON_LAUNCH";
    if std::env::var_os(LAUNCH_CHILD).is_some() {
        std::process::exit(run_public_main(os_vec(&["exec", "hi"])));
    }
    let _guard = EnvRestoreGuard::new(&[]);
    let root = temp_root("e2e-update");
    let gen1 = root.join("src-g1");
    let gen2 = root.join("src-g2");
    write_sample_generation(&gen1, "gen-1", false);
    write_sample_generation(&gen2, "gen-2", true);

    let syn_prefix = root.join("prefix");
    fs::create_dir_all(syn_prefix.join("bin")).unwrap();
    fs::create_dir_all(syn_prefix.join("etc/tls/certs")).unwrap();
    fs::write(syn_prefix.join("etc/resolv.conf"), b"nameserver 1.1.1.1\n").unwrap();
    fs::write(syn_prefix.join("etc/tls/cert.pem"), b"").unwrap();
    let syn_home = root.join("home");
    fs::create_dir_all(syn_home.join(".codex/packages/standalone/releases")).unwrap();
    fs::create_dir_all(syn_home.join(".local/share/codex")).unwrap();

    drop(_guard);
    let _env_guard = EnvRestoreGuard::new(&[
        ("HOME", Some(&syn_home)),
        ("PREFIX", Some(&syn_prefix)),
        ("TMPDIR", Some(&root)),
        ("CODEX_HOME", None),
        ("CODEX_SHARED_HOME", None),
        ("CODEX_SQLITE_HOME", None),
    ]);

    let roots = LocalWrapperRoots::from_environment().unwrap();
    let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
    generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();
    prepare_wrapper_config(&roots.config_dir).unwrap();
    copy_local_directory_tree(&gen1, &roots.generation_root.join("gen-1")).unwrap();
    copy_local_directory_tree(&gen2, &roots.generation_root.join("gen-2")).unwrap();
    let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
    let mut io = generation_state::FsActivationIo;
    let s1 = generation_state::plan_initial_pointer_state("gen-1").unwrap();
    generation_state::activate_pointer_state_locked(
        &state_paths,
        generation_state::ActivationOperation::Update,
        None,
        &s1,
        &mut io,
        &lock,
    )
    .unwrap();
    let s2 = generation_state::plan_activation_pointer_state(&s1, "gen-2").unwrap();
    generation_state::activate_pointer_state_locked(
        &state_paths,
        generation_state::ActivationOperation::Update,
        Some(&s1),
        &s2,
        &mut io,
        &lock,
    )
    .unwrap();
    drop(lock);

    let loaded = load_local_generation(&roots.generation_root.join("gen-2")).unwrap();
    let snap = capture_termux_process_env();
    probe_release_candidate(&loaded, &roots, &snap).unwrap();

    assert_eq!(run_public_main(os_vec(&["wrapper", "status", "--json"])), 0);
    assert_eq!(
        run_public_main(os_vec(&[
            "wrapper",
            "status",
            "--verify",
            "--wrapper-sha256",
            &"a".repeat(64),
            "--upstream-version",
            "1.2.3",
            "--json"
        ])),
        0
    );
    assert_eq!(
        run_public_main(os_vec(&[
            "wrapper",
            "status",
            "--verify",
            "--wrapper-sha256",
            &"b".repeat(64)
        ])),
        1
    );
    let descriptor = loaded.generation_dir.join("generation.meta");
    let compatible = fs::read_to_string(&descriptor).unwrap();
    let incompatible = compatible.replace(
        &format!(
            "patch_policy_id\t{}\n",
            codex_release_builder::PATCH_POLICY_ID
        ),
        "patch_policy_id\toutdated-policy\n",
    );
    fs::write(&descriptor, incompatible).unwrap();
    assert_eq!(
        run_public_main(os_vec(&["wrapper", "status", "--verify", "--json"])),
        1
    );
    fs::write(&descriptor, compatible).unwrap();
    let outcome = execute_activated_route(
        PublicDispatchRoute::Doctor(os_vec(&["--json"])),
        &roots,
        &snap,
    )
    .unwrap();
    let PublicDispatchCompletion::Doctor(doc) = outcome;
    assert!(doc.output.contains("\"schema_version\":1"));
    assert!(doc.output.contains("\"layout\":\"package-root\""));
    assert!(doc.output.contains("\"status\":\"healthy\""));

    assert_eq!(run_public_main(os_vec(&["update", "--rollback"])), 0);
    let recovered = generation_state::recover_activation_state(&state_paths)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.current, "gen-1");
    assert_eq!(run_public_main(os_vec(&["doctor", "--json"])), 0);

    // Absent wrapper.toml performs zero snapshot writes or config creation on version/status.
    assert!(!roots.config_dir.join("snapshots").exists());

    // Malformed wrapper.toml keeps status/setup-show inspectable & non-mutating and rejects mutating setup/--with,
    // while ordinary execution degrades gracefully with warning without overwriting wrapper.toml.
    fs::write(roots.state_root.join("wrapper.toml"), "broken = invalid\n").unwrap();
    assert_eq!(run_public_main(os_vec(&["wrapper", "status"])), 0);
    assert_eq!(run_public_main(os_vec(&["wrapper", "setup", "show"])), 0);
    assert_eq!(run_public_main(os_vec(&["doctor", "--json"])), 0);
    assert_eq!(
        run_public_main(os_vec(&["wrapper", "setup", "--enable", "manager"])),
        2
    );
    assert_eq!(
        fs::read_to_string(roots.state_root.join("wrapper.toml")).unwrap(),
        "broken = invalid\n"
    );

    // Reset via removal -> test multi-flag setup and --bare preserving signed_update.
    fs::remove_file(roots.state_root.join("wrapper.toml")).unwrap();
    assert_eq!(
        run_public_main(os_vec(&[
            "wrapper",
            "setup",
            "--enable",
            "manager,tui",
            "--signed-update",
            "true"
        ])),
        0
    );
    // Persisted optional selection cannot brick standalone execution.
    // Run the actual exec boundary in a child, preserving this test runner.
    let launch = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::e2e_activation_doctor_status_and_rollback_in_isolated_roots",
            "--nocapture",
        ])
        .env(LAUNCH_CHILD, "1")
        .output()
        .unwrap();
    assert!(launch.status.success(), "{launch:?}");
    assert!(String::from_utf8_lossy(&launch.stdout).contains("codex 1.2.3"));
    assert!(String::from_utf8_lossy(&launch.stderr).contains("continuing with standalone upstream"));
    assert_eq!(
        run_public_main(os_vec(&["wrapper", "--with", "manager"])),
        1
    );
    // Mixing show with mutating flags fails with exit code 2 cleanly.
    assert_eq!(
        run_public_main(os_vec(&["wrapper", "setup", "show", "--bare"])),
        2
    );
    // --bare clears enabled modules while preserving signed_update = true.
    assert_eq!(run_public_main(os_vec(&["wrapper", "setup", "--bare"])), 0);
    let after_bare = super::extensions::WrapperSettings::read(&roots.state_root).unwrap();
    assert_eq!(after_bare.addons_label(), "none");
    assert!(after_bare.signed_update);
    assert_eq!(run_public_main(os_vec(&["wrapper", "status", "--json"])), 0);

    drop(_env_guard);
    remove_temp_root(root);
}

#[test]
fn daemon_settings_corruption_and_shared_home_scope_in_isolated_roots() {
    const CHILD_ENV: &str = "CODEX_TEST_DAEMON_SCOPE_ARGS";
    if let Ok(encoded) = std::env::var(CHILD_ENV) {
        let args: Vec<OsString> = if encoded.is_empty() {
            Vec::new()
        } else {
            encoded.split('\x1f').map(OsString::from).collect()
        };
        std::process::exit(run_public_main(args));
    }

    let root = temp_root("daemon-scope");
    let syn_prefix = root.join("prefix");
    fs::create_dir_all(syn_prefix.join("bin")).unwrap();
    fs::create_dir_all(syn_prefix.join("etc/tls/certs")).unwrap();
    fs::write(syn_prefix.join("etc/resolv.conf"), b"nameserver 1.1.1.1\n").unwrap();
    fs::write(syn_prefix.join("etc/tls/cert.pem"), b"").unwrap();
    let syn_home = root.join("home");
    let shared_home = root.join("shared-home");
    fs::create_dir_all(syn_home.join(".codex/packages/standalone/releases")).unwrap();
    fs::create_dir_all(syn_home.join(".local/share/codex")).unwrap();
    fs::create_dir_all(&shared_home).unwrap();

    let _env = EnvRestoreGuard::new(&[
        ("HOME", Some(&syn_home)),
        ("PREFIX", Some(&syn_prefix)),
        ("TMPDIR", Some(&root)),
        ("CODEX_HOME", None),
        ("CODEX_SHARED_HOME", None),
        ("CODEX_SQLITE_HOME", None),
    ]);
    let roots = LocalWrapperRoots::from_environment().unwrap();
    let gen1 = roots.generation_root.join("gen-1");
    write_sample_generation(&gen1, "gen-1", false);
    let shell = if Path::new("/system/bin/sh").is_file() {
        "/system/bin/sh"
    } else {
        "/bin/sh"
    };
    fs::write(
            gen1.join("bin/codex"),
            format!("#!{shell}\nif [ \"${{1:-}}\" = --version ]; then echo codex-cli 1.2.3; else printf 'ARGS:%s|SQLITE:%s\\n' \"$*\" \"${{CODEX_SQLITE_HOME:-}}\"; fi\n"),
        )
        .unwrap();
    fs::set_permissions(gen1.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();
    let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
    generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();
    let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
    let mut io = generation_state::FsActivationIo;
    let s1 = generation_state::plan_initial_pointer_state("gen-1").unwrap();
    generation_state::activate_pointer_state_locked(
        &state_paths,
        generation_state::ActivationOperation::Update,
        None,
        &s1,
        &mut io,
        &lock,
    )
    .unwrap();
    drop(lock);

    let settings_path = syn_home.join(".codex/app-server-daemon/settings.json");
    fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    fs::write(&settings_path, "{broken-json").unwrap();

    let run_child = |args: &str, shared: Option<&Path>| {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "tests::daemon_settings_corruption_and_shared_home_scope_in_isolated_roots",
            "--nocapture",
        ])
        .env(CHILD_ENV, args);
        if let Some(dir) = shared {
            cmd.env("CODEX_SHARED_HOME", dir);
        }
        cmd.output().unwrap()
    };

    let fast_cache = root.join("fast-path-cache");
    let fast = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::daemon_settings_corruption_and_shared_home_scope_in_isolated_roots",
            "--nocapture",
        ])
        .env(CHILD_ENV, "--version")
        .env("TZ", ":Asia/Seoul")
        .env("XDG_CACHE_HOME", &fast_cache)
        .output()
        .unwrap();
    assert!(fast.status.success(), "{fast:?}");
    assert!(String::from_utf8_lossy(&fast.stdout).contains("codex-cli 1.2.3"));
    assert!(!fast_cache.exists(), "fast path must skip timezone cache");

    for (args, expected) in [
        ("exec\x1f--help", "ARGS:exec --help"),
        (
            "app-server\x1fdaemon\x1f--help",
            "ARGS:app-server daemon --help",
        ),
        (
            "-c\x1fmodel=doctor\x1fexec\x1fhi",
            "ARGS:-c model=doctor exec hi",
        ),
    ] {
        let result = run_child(args, None);
        assert!(result.status.success(), "{result:?}");
        assert!(String::from_utf8_lossy(&result.stdout).contains(expected));
    }
    for (args, expected) in [
        ("-c\x1ffoo=bar\x1fdoctor", "codex doctor"),
        ("-c\x1ffoo=bar\x1fwrapper\x1fstatus", "codex wrapper"),
    ] {
        let result = run_child(args, None);
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&result.stderr).contains(expected));
        assert!(!String::from_utf8_lossy(&result.stdout).contains("ARGS:"));
    }

    // Non-daemon routes succeed despite corrupt daemon settings and without overwriting them.
    let features = run_child("features\x1flist", None);
    assert!(features.status.success(), "{features:?}");
    assert!(String::from_utf8_lossy(&features.stdout).contains("ARGS:features list"));

    let no_daemon = run_child("--no-daemon\x1fexec\x1fhi", None);
    assert!(no_daemon.status.success(), "{no_daemon:?}");

    // Global flags before `update` fail closed with exit code 2 instead of reaching upstream.
    assert_eq!(run_child("--oss\x1fupdate", None).status.code(), Some(2));
    assert_eq!(run_child("--search\x1fupdate", None).status.code(), Some(2));

    // Daemon-dependent routes (default TUI & `app-server daemon`) fail closed on corrupt settings.
    assert_eq!(run_child("", None).status.code(), Some(1));
    assert_eq!(
        run_child("app-server\x1fdaemon", None).status.code(),
        Some(1)
    );
    assert_eq!(fs::read_to_string(&settings_path).unwrap(), "{broken-json");

    // Removing corrupt daemon settings lets `CODEX_SHARED_HOME` bind SQLite default cleanly.
    fs::remove_file(&settings_path).unwrap();
    let literal = run_child("--\x1fdoctor", None);
    assert!(literal.status.success(), "{literal:?}");
    assert!(String::from_utf8_lossy(&literal.stdout).contains("ARGS:-- doctor"));
    let with_shared = run_child("exec\x1fhi", Some(&shared_home));
    assert!(with_shared.status.success(), "{with_shared:?}");
    assert!(String::from_utf8_lossy(&with_shared.stdout)
        .contains(&format!("SQLITE:{}", shared_home.display())));

    drop(_env);
    remove_temp_root(root);
}

#[test]
fn upstream_launch_prepares_default_daemon_link_only_for_daemon_init_routes() {
    const CHILD_ENV: &str = "CODEX_TEST_DAEMON_LINK_CHILD";
    if let Ok(encoded) = std::env::var(CHILD_ENV) {
        let args: Vec<OsString> = if encoded.is_empty() {
            Vec::new()
        } else {
            encoded.split('\x1f').map(OsString::from).collect()
        };
        std::process::exit(run_public_main(args));
    }

    let root = temp_root("e2e-daemon-link");
    let syn_prefix = root.join("prefix");
    fs::create_dir_all(syn_prefix.join("bin")).unwrap();
    fs::create_dir_all(syn_prefix.join("etc/tls/certs")).unwrap();
    fs::write(syn_prefix.join("etc/resolv.conf"), b"nameserver 1.1.1.1\n").unwrap();
    fs::write(syn_prefix.join("etc/tls/cert.pem"), b"").unwrap();
    let syn_home = root.join("home");
    fs::create_dir_all(syn_home.join(".codex/packages/standalone/releases")).unwrap();
    fs::create_dir_all(syn_home.join(".local/share/codex")).unwrap();

    let _env = EnvRestoreGuard::new(&[
        ("HOME", Some(&syn_home)),
        ("PREFIX", Some(&syn_prefix)),
        ("TMPDIR", Some(&root)),
        ("CODEX_HOME", None),
        ("CODEX_SHARED_HOME", None),
        ("CODEX_SQLITE_HOME", None),
    ]);
    let roots = LocalWrapperRoots::from_environment().unwrap();
    let gen1 = roots.generation_root.join("gen-1");
    write_sample_generation(&gen1, "gen-1", false);
    let shell = if Path::new("/system/bin/sh").is_file() {
        "/system/bin/sh"
    } else {
        "/bin/sh"
    };
    fs::write(
            gen1.join("bin/codex"),
            format!("#!{shell}\nif [ \"${{1:-}}\" = --version ]; then echo codex-cli 1.2.3; else printf 'ARGS:%s\\n' \"$*\"; fi\n"),
        )
        .unwrap();
    fs::set_permissions(gen1.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();
    let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root)
        .unwrap()
        .with_generation_root(&roots.generation_root);
    generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();
    let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
    let mut io = generation_state::FsActivationIo;
    let s1 = generation_state::plan_initial_pointer_state("gen-1").unwrap();
    generation_state::activate_pointer_state_locked(
        &state_paths,
        generation_state::ActivationOperation::Update,
        None,
        &s1,
        &mut io,
        &lock,
    )
    .unwrap();
    drop(lock);

    let run_in = |codex_home: &Path, args: &str| {
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::upstream_launch_prepares_default_daemon_link_only_for_daemon_init_routes",
                "--nocapture",
            ])
            .env(CHILD_ENV, args)
            .env("CODEX_HOME", codex_home)
            .output()
            .unwrap()
    };

    // 1. Non-daemon / query paths must NOT create `packages/app-server-daemon/current`.
    let home_query = root.join("home-query");
    for args in [
        "exec\x1fhi",
        "login\x1fstatus",
        "app-server\x1fdaemon\x1fversion",
        "app-server\x1fdaemon\x1fupdate\x1f--from-cli\x1f--yes",
        "--no-daemon",
        "app-server\x1fdaemon\x1fstart\x1f--help",
    ] {
        let out = run_in(&home_query, args);
        assert!(out.status.success(), "{args}: {out:?}");
        assert!(
            !home_query
                .join("packages/app-server-daemon/current")
                .exists(),
            "query route {args} unexpectedly created daemon package link"
        );
    }

    // 2. Daemon initialization path creates `packages/app-server-daemon/current -> standalone/current`.
    let out_start = run_in(&home_query, "app-server\x1fdaemon\x1fstart");
    assert!(out_start.status.success(), "{out_start:?}");
    let expected_target = syn_home.join(".codex/packages/standalone/current");
    assert_eq!(
        fs::read_link(home_query.join("packages/app-server-daemon/current")).unwrap(),
        expected_target
    );

    // 3. Default TUI launch on a fresh custom CODEX_HOME creates link and preserves existing selection.
    let home_tui = root.join("home-tui");
    let out_tui = run_in(&home_tui, "");
    assert!(out_tui.status.success(), "{out_tui:?}");
    assert_eq!(
        fs::read_link(home_tui.join("packages/app-server-daemon/current")).unwrap(),
        expected_target
    );

    drop(_env);
    remove_temp_root(root);
}

fn execute_activated_route(
    route: PublicDispatchRoute,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
) -> Result<PublicDispatchCompletion, LocalProductError> {
    let scan = match &route {
        PublicDispatchRoute::Upstream(args) => UpstreamArgScan::new(args),
        _ => UpstreamArgScan::default(),
    };
    super::execute_activated_route(route, roots, process_env, &scan)
}

//! Shared source-runtime producer. Executors supply tools and disposable source;
//! this module owns compilation semantics and the raw artifact contract.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const TARGET: &str = "aarch64-unknown-linux-musl";
// The release-input JSON limit already enforced by CI admission.
const CONTROL_MAX_BYTES: usize = 16 * 1024;
const BUILD: [(&str, &str); 6] = [
    ("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z"),
    ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "16"),
    ("CARGO_PROFILE_RELEASE_LTO", "off"),
    ("CARGO_PROFILE_RELEASE_STRIP", "symbols"),
    ("CARGO_PROFILE_RELEASE_DEBUG", "0"),
    ("CARGO_PROFILE_RELEASE_INCREMENTAL", "false"),
];
const FILES: [&str; 6] = [
    "codex",
    "version.txt",
    "sha256.txt",
    "preparation.json",
    "lock-audit.json",
    "source.patch",
];
type Result<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upstream {
    version: String,
    source: String,
    archive_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Toolchains {
    core_rust: String,
    frontend_rust: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Qualification {
    focused_tests: u64,
    lock_audit: Audit,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Audit {
    external_packages_unchanged: u64,
    workspace_versions_changed: u64,
    original_lock_sha256: String,
    normalized_lock_sha256: String,
}

impl Audit {
    fn value(&self) -> Result<Value> {
        serde_json::to_value(self).map_err(|e| format!("runtime dependency audit: {e}"))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    schema: u8,
    upstream: Upstream,
    toolchains: Toolchains,
    qualification: Qualification,
    // This producer does not consume or admit optional frontend artifacts.
    #[serde(rename = "frontend")]
    _frontend: Option<Value>,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .is_file()
    {
        return Err(format!(
            "{}: expected a regular runtime input",
            path.display()
        ));
    }
    // These are release/control records, never executable payloads.
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| {
            file.take((CONTROL_MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
        })
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > CONTROL_MAX_BYTES {
        return Err(format!(
            "{}: runtime control input exceeds16 KiB",
            path.display()
        ));
    }
    Ok(bytes)
}

fn recipe(path: &Path) -> Result<Recipe> {
    let bytes = read(path)?;
    let value: Recipe =
        serde_json::from_slice(&bytes).map_err(|e| format!("runtime release input: {e}"))?;
    let audit = &value.qualification.lock_audit;
    if value.schema != 1
        || ![
            &value.upstream.version,
            &value.toolchains.core_rust,
            &value.toolchains.frontend_rust,
        ]
        .into_iter()
        .all(|v| super::valid_stable_version(v))
        || value.upstream.source.len() != 40
        || !value
            .upstream
            .source
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || !super::valid_lower_sha256(&value.upstream.archive_sha256)
        || !(1..=100000).contains(&value.qualification.focused_tests)
        || !(1..=100000).contains(&audit.external_packages_unchanged)
        || !(1..=100000).contains(&audit.workspace_versions_changed)
        || !super::valid_lower_sha256(&audit.original_lock_sha256)
        || !super::valid_lower_sha256(&audit.normalized_lock_sha256)
    {
        return Err("invalid runtime release input identity or dependency audit".into());
    }
    Ok(value)
}

pub(super) fn output(command: &mut Command, phase: &str) -> Result<Vec<u8>> {
    let result = command.output().map_err(|e| format!("{phase}: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "{phase}: {}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(result.stdout)
}

fn git(source: &Path, args: &[&str]) -> Result<Vec<u8>> {
    output(
        Command::new("git").arg("-C").arg(source).args(args),
        "runtime source Git",
    )
}

fn report(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, format!("{value}\n")).map_err(|e| format!("{}: {e}", path.display()))
}

fn verify(directory: &Path, recipe: &Recipe, openssl: &Path) -> Result<String> {
    let entries: BTreeSet<_> = fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .map(|entry| entry.map(|e| e.file_name()))
        .collect::<std::io::Result<_>>()
        .map_err(|e| e.to_string())?;
    if entries != FILES.map(OsString::from).into_iter().collect() {
        return Err("runtime artifact inventory differs".into());
    }
    for name in FILES {
        let path = directory.join(name);
        if !fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err(format!("runtime artifact is not a regular file: {name}"));
        }
    }
    let digest =
        super::openssl_sha256(openssl, &directory.join("codex")).map_err(|e| e.to_string())?;
    let preparation: Value = serde_json::from_slice(&read(&directory.join("preparation.json"))?)
        .map_err(|e| e.to_string())?;
    let audit: Value = serde_json::from_slice(&read(&directory.join("lock-audit.json"))?)
        .map_err(|e| e.to_string())?;
    if read(&directory.join("sha256.txt"))? != format!("{digest}  codex\n").as_bytes()
        || read(&directory.join("version.txt"))?
            != format!("codex-cli {}\n", recipe.upstream.version).as_bytes()
        || preparation["source"] != recipe.upstream.source
        || preparation["version"] != recipe.upstream.version
        || audit != recipe.qualification.lock_audit.value()?
    {
        return Err(
            "runtime artifact digest, version or source/dependency identity differs".into(),
        );
    }
    Ok(digest)
}

/// Verify transferred raw producer output; downstream Android admission is separate.
pub fn verify_artifact(directory: &Path, recipe_path: &Path, openssl: &Path) -> Result<String> {
    verify(directory, &recipe(recipe_path)?, openssl)
}

pub struct BuildRequest<'a> {
    pub source: &'a Path,
    pub output: &'a Path,
    pub recipe: &'a Path,
    pub target_dir: &'a Path,
    pub rustc: &'a Path,
    pub cargo: &'a Path,
    pub openssl: &'a Path,
}

pub(super) fn compile_target_probe(
    rustc: &Path,
    output_dir: &Path,
    toolchain: &str,
    linker: Option<&std::ffi::OsStr>,
) -> Result<()> {
    const EXPECTED: &str = "codex-runtime-target-ready\n";
    let directory = output_dir.join(".compiler-probe");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .map_err(|e| format!("reserve runtime compiler probe: {e}"))?;
    let source = directory.join("main.rs");
    let binary = directory.join("probe");
    let result = (|| {
        fs::write(&source, format!("fn main() {{ print!({EXPECTED:?}); }}\n"))
            .map_err(|e| format!("write runtime compiler probe: {e}"))?;
        let mut compiler = Command::new(rustc);
        compiler
            .args([
                "--crate-name",
                "codex_runtime_compiler_probe",
                "--crate-type",
                "bin",
                "--edition",
                "2021",
                "--target",
                TARGET,
                "--emit=link",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .env("RUSTUP_TOOLCHAIN", toolchain);
        if let Some(linker) = linker {
            let mut flag = OsString::from("linker=");
            flag.push(linker);
            compiler.arg("-C").arg(flag);
        }
        output(&mut compiler, "prepared Rust target compilation")?;
        super::validate_static_aarch64_elf(&binary).map_err(|e| e.to_string())?;
        probe(&binary, &directory, EXPECTED.as_bytes())
            .map_err(|e| format!("prepared Rust target execution: {e}"))?;
        Ok(())
    })();
    // A failed linker may leave rcgu objects beside its output too. This newly
    // reserved directory owns every probe artifact, including those diagnostics.
    let cleanup = fs::remove_dir_all(&directory);
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => {
            Err(format!("{error}; remove runtime compiler probe: {cleanup}"))
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(format!("remove runtime compiler probe: {error}")),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn probe(binary: &Path, directory: &Path, expected: &[u8]) -> Result<Vec<u8>> {
    let home = directory.join(".smoke-home");
    let codex_home = home.join(".codex");
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&home)
        .map_err(|e| format!("runtime smoke home: {e}"))?;
    let result = (|| {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&codex_home)
            .map_err(|e| format!("runtime smoke Codex home: {e}"))?;
        let mut child = Command::new(binary)
            .arg("--version")
            .env_clear()
            .env("HOME", &home)
            .env("CODEX_HOME", &codex_home)
            .current_dir(&home)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("runtime version probe: {e}"))?;
        // Versions are bounded by the stable-version contract; allow diagnostic
        // output up to the existing artifact-probe bound before rejecting it.
        let stdout = super::spawn_bounded_probe_reader(
            child.stdout.take().ok_or("missing runtime stdout")?,
            512,
        );
        let stderr = super::spawn_bounded_probe_reader(
            child.stderr.take().ok_or("missing runtime stderr")?,
            512,
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            match child
                .try_wait()
                .map_err(|e| format!("poll runtime version probe: {e}"))?
            {
                Some(status) => break Some(status),
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                None => std::thread::sleep(Duration::from_millis(25)),
            }
        };
        let stdout = stdout
            .join()
            .map_err(|_| "runtime stdout reader failed")?
            .map_err(|e| e.to_string())?;
        let stderr = stderr
            .join()
            .map_err(|_| "runtime stderr reader failed")?
            .map_err(|e| e.to_string())?;
        if status.is_none() {
            return Err("runtime version probe timed out".into());
        }
        if !status.is_some_and(|s| s.success()) || !stderr.is_empty() || stdout != expected {
            return Err(format!(
                "runtime version/streams differ: status={status:?}, stdout={:?}, stderr={:?}",
                String::from_utf8_lossy(&stdout),
                String::from_utf8_lossy(&stderr)
            ));
        }
        Ok(stdout)
    })();
    fs::remove_dir_all(&home).map_err(|e| format!("remove owned runtime smoke home: {e}"))?;
    result
}

/// Build only the source runtime. This grants no signing or installation authority.
pub fn build(request: &BuildRequest<'_>) -> Result<String> {
    let recipe = recipe(request.recipe)?;
    if !request.source.is_absolute()
        || request.source.is_symlink()
        || !request.source.is_dir()
        || !request.output.is_absolute()
        || fs::symlink_metadata(request.output).is_ok()
        || !request.target_dir.is_absolute()
    {
        return Err("runtime build requires an absolute disposable checkout, absent output and compiler cache".into());
    }
    if git(request.source, &["rev-parse", "HEAD"])?
        != format!("{}\n", recipe.upstream.source).as_bytes()
        || !git(request.source, &["status", "--porcelain"])?.is_empty()
    {
        return Err(
            "official runtime source differs or is not pristine; existing changes preserved".into(),
        );
    }
    for (key, value) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if !value.is_empty()
            && (matches!(
                name.as_ref(),
                "RUSTFLAGS"
                    | "CARGO_ENCODED_RUSTFLAGS"
                    | "RUSTC_WRAPPER"
                    | "RUSTC_WORKSPACE_WRAPPER"
                    | "CARGO_BUILD_RUSTFLAGS"
                    | "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS"
            ) || name.starts_with("CARGO_PROFILE_RELEASE_")
                && !BUILD.iter().any(|(k, _)| *k == name))
        {
            return Err(format!(
                "runtime build has an unrecorded compiler override: {name}"
            ));
        }
    }
    let compiler = |args: &[&str]| {
        output(
            Command::new(request.rustc)
                .args(args)
                .env("RUSTUP_TOOLCHAIN", &recipe.toolchains.frontend_rust),
            "prepared Rust compiler",
        )
    };
    let version = compiler(&["--version"])?;
    if std::str::from_utf8(&version)
        .map_err(|e| e.to_string())?
        .split_whitespace()
        .nth(1)
        != Some(recipe.toolchains.frontend_rust.as_str())
    {
        return Err("prepared Rust compiler does not match the selected runtime recipe".into());
    }
    let sysroot = compiler(&["--print", "sysroot"])?;
    let sysroot = PathBuf::from(
        std::str::from_utf8(&sysroot)
            .map_err(|e| e.to_string())?
            .trim(),
    );
    if !sysroot
        .join("lib/rustlib")
        .join(TARGET)
        .join("lib")
        .is_dir()
    {
        return Err("prepared Rust compiler is missing the musl target".into());
    }
    fs::DirBuilder::new()
        .mode(0o700)
        .create(request.output)
        .map_err(|e| format!("reserve runtime output: {e}"))?;
    println!("[runtime] compile, link and execute prepared Rust/target standard-library probe");
    compile_target_probe(
        request.rustc,
        request.output,
        &recipe.toolchains.frontend_rust,
        std::env::var_os("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER").as_deref(),
    )?;
    println!("[runtime] adapt exact official source");
    let preparation = super::source::prepare(
        request.source,
        &recipe.upstream.version,
        &recipe.upstream.source,
    )?;
    report(&request.output.join("preparation.json"), &preparation)?;
    println!("[runtime] audit unchanged external dependencies");
    let audit = super::source::normalize_lock(&request.source.join("codex-rs"), request.openssl)?;
    report(&request.output.join("lock-audit.json"), &audit)?;
    if audit != recipe.qualification.lock_audit.value()? {
        return Err("source runtime dependency audit differs from exact release input".into());
    }
    git(request.source, &["diff", "--check"])?;
    fs::write(
        request.output.join("source.patch"),
        git(request.source, &["diff", "--binary"])?,
    )
    .map_err(|e| e.to_string())?;
    super::source::preserve_times(request.source)?;
    println!("[runtime] compile adapted upstream executable");
    let mut cargo = Command::new(request.cargo);
    cargo
        .args([
            "build",
            "--release",
            "--locked",
            "--target",
            TARGET,
            "-p",
            "codex-cli",
            "--bin",
            "codex",
        ])
        .current_dir(request.source.join("codex-rs"))
        .envs(BUILD)
        .env("RUSTC", request.rustc)
        .env("RUSTUP_TOOLCHAIN", &recipe.toolchains.frontend_rust)
        .env("CARGO_TARGET_DIR", request.target_dir);
    if std::env::var_os("CARGO_BUILD_JOBS").is_none() {
        cargo.env("CARGO_BUILD_JOBS", "3");
    }
    let status = cargo
        .status()
        .map_err(|e| format!("runtime compilation: {e}"))?;
    if !status.success() {
        return Err(format!("runtime compilation failed: {status}"));
    }
    // Freeze output before any probe: Cargo's shared output may later be replaced.
    let binary = request.output.join("codex");
    fs::copy(
        request.target_dir.join(TARGET).join("release/codex"),
        &binary,
    )
    .map_err(|e| format!("snapshot compiled runtime: {e}"))?;
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    super::validate_static_aarch64_elf(&binary).map_err(|e| e.to_string())?;
    println!("[runtime] execute produced version");
    let version = probe(
        &binary,
        request.output,
        format!("codex-cli {}\n", recipe.upstream.version).as_bytes(),
    )?;
    fs::write(request.output.join("version.txt"), version).map_err(|e| e.to_string())?;
    let digest = super::openssl_sha256(request.openssl, &binary).map_err(|e| e.to_string())?;
    fs::write(
        request.output.join("sha256.txt"),
        format!("{digest}  codex\n"),
    )
    .map_err(|e| e.to_string())?;
    let digest = verify(request.output, &recipe, request.openssl)?;
    println!("[runtime] raw artifact complete; Android admission is still required");
    Ok(digest)
}

pub(super) fn command(args: &[OsString]) -> i32 {
    let result = (|| {
        let (action, args) = args.split_first().ok_or("missing runtime operation")?;
        let action = action.to_str().ok_or("invalid runtime operation")?;
        if !args.len().is_multiple_of(2) {
            return Err("runtime operation requires --flag value pairs".into());
        }
        let mut fields = BTreeMap::new();
        for pair in args.as_chunks::<2>().0 {
            if fields
                .insert(
                    pair[0].to_str().ok_or("invalid runtime build flag")?,
                    Path::new(&pair[1]),
                )
                .is_some()
            {
                return Err("duplicate runtime build flag".into());
            }
        }
        let flags: &[&str] = match action {
            "build-runtime" => &[
                "--source",
                "--output",
                "--recipe",
                "--target-dir",
                "--rustc",
                "--cargo",
                "--openssl",
            ],
            "verify-runtime" => &["--artifact", "--recipe", "--openssl"],
            _ => return Err("unsupported runtime operation".into()),
        };
        if fields.len() != flags.len() || !flags.iter().all(|f| fields.contains_key(f)) {
            return Err(format!("{action} requires {}", flags.join(" ")));
        }
        if action == "verify-runtime" {
            return verify_artifact(
                fields["--artifact"],
                fields["--recipe"],
                fields["--openssl"],
            );
        }
        build(&BuildRequest {
            source: fields["--source"],
            output: fields["--output"],
            recipe: fields["--recipe"],
            target_dir: fields["--target-dir"],
            rustc: fields["--rustc"],
            cargo: fields["--cargo"],
            openssl: fields["--openssl"],
        })
    })();
    match result {
        Ok(_) => 0,
        Err(error) => {
            eprintln!("codex-release-builder: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "runtime-producer-{}-{}",
            std::process::id(),
            super::super::STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }

    fn inputs() -> Value {
        json!({"schema":1,"upstream":{"version":"0.165.0","source":"a".repeat(40),"archive_sha256":"b".repeat(64)},
            "toolchains":{"core_rust":"1.98.0","frontend_rust":"1.99.1"},"frontend":null,
            "qualification":{"focused_tests":65,"lock_audit":{"external_packages_unchanged":1308,"workspace_versions_changed":167,"original_lock_sha256":"c".repeat(64),"normalized_lock_sha256":"d".repeat(64)}}})
    }

    fn openssl() -> PathBuf {
        std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"))
            .join("bin/openssl")
    }

    #[test]
    fn changed_or_ambiguous_runtime_inputs_fail_before_execution() {
        let root = root();
        let path = root.join("recipe.json");
        let original = inputs();
        for field in [
            "schema",
            "version",
            "source",
            "compiler",
            "count",
            "hash",
            "duplicate",
        ] {
            let mut value = original.clone();
            match field {
                "schema" => value["schema"] = json!(true),
                "version" => value["upstream"]["version"] = json!("0.0165.0"),
                "source" => value["upstream"]["source"] = json!("main"),
                "compiler" => value["toolchains"]["frontend_rust"] = json!("1.99.1\nEXTRA=yes"),
                "count" => {
                    value["qualification"]["lock_audit"]["workspace_versions_changed"] = json!(0)
                }
                "hash" => {
                    value["qualification"]["lock_audit"]["normalized_lock_sha256"] =
                        json!("A".repeat(64))
                }
                "duplicate" => {}
                _ => unreachable!(),
            }
            let text = value.to_string();
            fs::write(
                &path,
                if field == "duplicate" {
                    text.replace(
                        "\"workspace_versions_changed\":167",
                        "\"workspace_versions_changed\":1,\"workspace_versions_changed\":167",
                    )
                } else {
                    text
                },
            )
            .unwrap();
            assert!(recipe(&path).is_err(), "{field}");
        }
        report(&path, &original).unwrap();
        assert!(recipe(&path).is_ok());
        fs::write(&path, vec![b' '; CONTROL_MAX_BYTES + 1]).unwrap();
        assert!(recipe(&path).err().unwrap().contains("exceeds16 KiB"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn raw_transfer_inventory_identity_and_alias_faults_are_rejected() {
        for fault in [
            "none", "binary", "source", "version", "audit", "extra", "alias",
        ] {
            let root = root();
            let path = root.join("recipe.json");
            report(&path, &inputs()).unwrap();
            let output = root.join("artifact");
            fs::create_dir(&output).unwrap();
            fs::write(output.join("codex"), b"owned raw transfer bytes").unwrap();
            let digest = super::super::openssl_sha256(&openssl(), &output.join("codex")).unwrap();
            fs::write(output.join("sha256.txt"), format!("{digest}  codex\n")).unwrap();
            fs::write(output.join("version.txt"), "codex-cli 0.165.0\n").unwrap();
            report(
                &output.join("preparation.json"),
                &json!({"source":"a".repeat(40),"version":"0.165.0"}),
            )
            .unwrap();
            report(
                &output.join("lock-audit.json"),
                &inputs()["qualification"]["lock_audit"],
            )
            .unwrap();
            fs::write(output.join("source.patch"), "owned source adaptation").unwrap();
            match fault {
                "none" => {}
                "binary" => fs::write(output.join("codex"), "changed bytes").unwrap(),
                "source" => report(
                    &output.join("preparation.json"),
                    &json!({"source":"e".repeat(40),"version":"0.165.0"}),
                )
                .unwrap(),
                "version" => fs::write(output.join("version.txt"), "codex-cli other\n").unwrap(),
                "audit" => report(&output.join("lock-audit.json"), &json!({})).unwrap(),
                "extra" => fs::write(output.join("undeclared"), "extra").unwrap(),
                "alias" => {
                    fs::remove_file(output.join("source.patch")).unwrap();
                    std::os::unix::fs::symlink("version.txt", output.join("source.patch")).unwrap();
                }
                _ => unreachable!(),
            }
            let result = verify_artifact(&output, &path, &openssl());
            if fault == "none" {
                assert_eq!(result.unwrap(), digest);
            } else {
                assert!(result.is_err(), "{fault}");
            }
            fs::remove_dir_all(root).unwrap();
        }
    }

    fn checkout() -> (PathBuf, PathBuf, PathBuf) {
        let root = root();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        git(&source, &["init", "-q"]).unwrap();
        fs::write(source.join("protected"), "original").unwrap();
        git(&source, &["add", "."]).unwrap();
        git(
            &source,
            &[
                "-c",
                "user.name=Qualification",
                "-c",
                "user.email=test@localhost",
                "commit",
                "-qm",
                "Original",
            ],
        )
        .unwrap();
        let revision = String::from_utf8(git(&source, &["rev-parse", "HEAD"]).unwrap()).unwrap();
        let mut value = inputs();
        value["upstream"]["source"] = json!(revision.trim());
        let path = root.join("recipe.json");
        report(&path, &value).unwrap();
        (root, source, path)
    }

    #[test]
    fn dirty_official_source_is_preserved_without_starting_tools() {
        let (root, source, path) = checkout();
        fs::write(source.join("protected"), "existing user change").unwrap();
        let output = root.join("output");
        let cache = root.join("cache");
        let absent = root.join("must-not-run");
        let error = build(&BuildRequest {
            source: &source,
            output: &output,
            recipe: &path,
            target_dir: &cache,
            rustc: &absent,
            cargo: &absent,
            openssl: &absent,
        })
        .unwrap_err();
        assert!(error.contains("not pristine"), "{error}");
        assert!(!output.exists());
        assert_eq!(
            fs::read(source.join("protected")).unwrap(),
            b"existing user change"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unusable_target_std_fails_before_source_preparation_or_cargo() {
        let (root, source, path) = checkout();
        let shell = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"))
            .join("bin/sh");
        fs::create_dir_all(root.join("sysroot/lib/rustlib").join(TARGET).join("lib")).unwrap();
        let rustc = root.join("prepared-rustc");
        // Exercise real rustc with an incomplete prepared target, while the
        // prepared tool's version/sysroot queries satisfy the initial checks.
        fs::write(&rustc, format!("#!{}\ncase \"$1\" in\n--version) printf 'rustc 1.99.1\\n';;\n--print) printf '%s\\n' \"${{0%/*}}/sysroot\";;\n*) exec rustc --sysroot \"${{0%/*}}/sysroot\" \"$@\";;\nesac\n", shell.display())).unwrap();
        fs::set_permissions(&rustc, fs::Permissions::from_mode(0o700)).unwrap();
        let output = root.join("output");
        let error = build(&BuildRequest {
            source: &source,
            output: &output,
            recipe: &path,
            target_dir: &root.join("cache"),
            rustc: &rustc,
            cargo: &root.join("must-not-run-cargo"),
            openssl: &openssl(),
        })
        .unwrap_err();
        assert!(
            error.contains("prepared Rust target compilation"),
            "{error}"
        );
        assert!(
            error.contains("E0463"),
            "original rustc cause lost: {error}"
        );
        assert!(git(&source, &["status", "--porcelain"]).unwrap().is_empty());
        assert_eq!(fs::read_dir(output).unwrap().count(), 0);
        assert!(!root.join("cache").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unusable_linker_fails_before_source_preparation_or_cargo() {
        let (root, source, path) = checkout();
        let shell = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"))
            .join("bin/bash");
        fs::create_dir_all(root.join("sysroot/lib/rustlib").join(TARGET).join("lib")).unwrap();
        let rustc = root.join("prepared-rustc");
        // Compile with the real host rustc/std, but an unavailable linker. Only
        // target selection and initial SDK queries are adapted by this fixture.
        let version = inputs()["toolchains"]["frontend_rust"]
            .as_str()
            .unwrap()
            .to_owned();
        fs::write(&rustc, format!("#!{}\ncase \"$1\" in\n--version) printf 'rustc {version}\\n'; exit;;\n--print) printf '%s\\n' \"${{0%/*}}/sysroot\"; exit;;\nesac\nargs=()\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = --target ]; then shift 2; else args+=(\"$1\"); shift; fi\ndone\nexec rustc \"${{args[@]}}\" -C \"linker=${{0%/*}}/unavailable-probe-linker\"\n", shell.display())).unwrap();
        fs::set_permissions(&rustc, fs::Permissions::from_mode(0o700)).unwrap();
        let output = root.join("output");
        let cache = root.join("cache");
        let error = build(&BuildRequest {
            source: &source,
            output: &output,
            recipe: &path,
            target_dir: &cache,
            rustc: &rustc,
            cargo: &root.join("must-not-run-cargo"),
            openssl: &openssl(),
        })
        .unwrap_err();
        assert!(
            error.contains("prepared Rust target compilation"),
            "{error}"
        );
        assert!(
            error.contains("unavailable-probe-linker"),
            "linker cause lost: {error}"
        );
        assert!(git(&source, &["status", "--porcelain"]).unwrap().is_empty());
        assert_eq!(fs::read_dir(output).unwrap().count(), 0);
        assert!(!cache.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn actual_version_child_must_succeed_with_exact_stdout_and_empty_stderr() {
        let root = root();
        let shell = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"))
            .join("bin/sh");
        let child = root.join("version-child");
        for (body, accepted) in [
            (
                "test -d \"$CODEX_HOME\" || { printf 'CODEX_HOME missing\\n' >&2; exit 7; }; printf 'codex-cli 0.165.0\\n'",
                true,
            ),
            ("printf 'codex-cli other\\n'", false),
            (
                "printf 'codex-cli 0.165.0\\n'; printf 'diagnostic\\n' >&2",
                false,
            ),
            ("printf 'codex-cli 0.165.0\\n'; exit 7", false),
        ] {
            fs::write(&child, format!("#!{}\n{body}\n", shell.display())).unwrap();
            fs::set_permissions(&child, fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(
                probe(&child, &root, b"codex-cli 0.165.0\n").is_ok(),
                accepted,
                "{body}"
            );
            assert!(!root.join(".smoke-home").exists());
        }
        fs::remove_dir_all(root).unwrap();
    }
}

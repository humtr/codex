//! Local source construction feeds the ordinary signed-generation transaction.
//! Core owns acquisition/cache/package lifecycle; the shared builder owns builds.
use super::*;
use codex_release_builder::{native_dependencies, native_sdk, runtime, SourceRuntime};
use std::collections::BTreeMap;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, String>;

fn write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn step(command: &mut Command, phase: &str) -> Result<()> {
    println!("[local build] {phase}");
    let status = command.status().map_err(|e| format!("{phase}: {e}"))?;
    if !status.success() {
        return Err(format!("{phase}: {status}"));
    }
    Ok(())
}

fn private(path: &Path) -> Result<()> {
    ensure_real_directory_tree(
        path,
        "create local source cache",
        "local source cache must be a real directory",
    )
    .map_err(|e| e.to_string())?;
    ensure_remote_private_directory(path).map_err(|e| e.to_string())
}

fn cache_lease(cache: &Path) -> Result<fs::File> {
    let lease = fs::File::open(cache).map_err(|e| format!("open native build cache: {e}"))?;
    if unsafe { flock(lease.as_raw_fd(), FLOCK_EX | FLOCK_NB) } != 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.kind() == std::io::ErrorKind::WouldBlock {
            "another update owns this native source-build cache; retry after it finishes".into()
        } else {
            format!("lock native source-build cache: {error}")
        });
    }
    Ok(lease)
}

const REQUIRED_PACKAGES: [(&str, &str); 13] = [
    ("git", "git"),
    ("cargo", "rust"),
    ("make", "make"),
    ("perl", "perl"),
    ("clang", "clang"),
    ("llvm-ar", "llvm"),
    ("llvm-ranlib", "llvm"),
    ("ld.lld", "lld"),
    ("protoc", "protobuf"),
    ("patchelf", "patchelf"),
    ("tar", "tar"),
    ("xz", "xz-utils"),
    ("pkg-config", "pkg-config"),
];

fn packages(tools: &Path) -> Result<()> {
    let missing: std::collections::BTreeSet<_> = REQUIRED_PACKAGES
        .iter()
        .filter(|(tool, _)| !tools.join(tool).is_file())
        .map(|(_, package)| *package)
        .collect();
    if !missing.is_empty() {
        step(
            Command::new(tools.join("pkg"))
                .args(["install", "-y"])
                .args(&missing),
            "install missing source-build packages",
        )?;
    }
    for (tool, _) in REQUIRED_PACKAGES {
        if !tools.join(tool).is_file() {
            return Err(format!(
                "required local build tool unavailable after preparation: {tool}"
            ));
        }
    }
    Ok(())
}

fn tree(root: &Path, openssl: &Path) -> Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    fn walk(path: &Path, entries: &mut Vec<PathBuf>) -> Result<()> {
        let mut children = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let path = child.path();
            let info = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            entries.push(path.clone());
            if info.is_dir() {
                walk(&path, entries)?;
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    walk(root, &mut entries)?;
    let files = entries
        .iter()
        .filter(|path| fs::symlink_metadata(path).is_ok_and(|m| m.is_file()))
        .collect::<Vec<_>>();
    let mut hashes = BTreeMap::new();
    for group in files.chunks(64) {
        let result = Command::new(openssl)
            .args(["dgst", "-sha256", "-binary"])
            .args(group)
            .output()
            .map_err(|e| e.to_string())?;
        if !result.status.success() || result.stdout.len() != group.len() * 32 {
            return Err("native cache digest batch failed".into());
        }
        for (path, digest) in group.iter().zip(result.stdout.as_chunks::<32>().0) {
            hashes.insert((*path).clone(), digest.to_vec());
        }
    }
    let mut record = b"codex-native-tree-v1\n".to_vec();
    fn field(record: &mut Vec<u8>, bytes: &[u8]) {
        record.extend((bytes.len() as u64).to_be_bytes());
        record.extend(bytes);
    }
    for path in entries {
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        field(
            &mut record,
            path.strip_prefix(root).unwrap().as_os_str().as_bytes(),
        );
        record.extend((metadata.mode() & 0o777).to_be_bytes());
        if metadata.file_type().is_symlink() {
            record.push(b'L');
            field(
                &mut record,
                fs::read_link(&path)
                    .map_err(|e| e.to_string())?
                    .as_os_str()
                    .as_bytes(),
            );
        } else if metadata.is_dir() {
            record.push(b'D');
        } else if let Some(hash) = hashes.get(&path) {
            record.push(b'F');
            record.extend(metadata.len().to_be_bytes());
            record.extend(hash);
        } else {
            return Err("unsupported native cache entry or concurrent change".into());
        }
    }
    Ok(record)
}

fn cached(root: &Path, receipt: &Path, openssl: &Path) -> Result<bool> {
    if !root.exists() {
        return Ok(false);
    }
    if !fs::symlink_metadata(root)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err("native build cache root is not a real directory".into());
    }
    let expected = fs::read(receipt)
        .map_err(|e| format!("native cache completion record unavailable: {e}"))?;
    if expected != tree(root, openssl)? {
        return Err("native build cache changed; existing inputs preserved".into());
    }
    Ok(true)
}

fn seal(root: &Path, receipt: &Path, openssl: &Path) -> Result<()> {
    write(receipt, tree(root, openssl)?)
}

fn download(
    roots: &LocalCoreRoots,
    archive: &native_sdk::Archive,
    destination: &Path,
) -> Result<()> {
    println!("[local build] Acquire {}", archive.name);
    remote_download::payload(
        roots,
        &archive.url,
        destination,
        &ReleaseFileEntry {
            relative_path: archive.name.clone(),
            sha256: archive.sha256.clone(),
            mode: 0o600,
        },
        remote_download::PayloadOptions {
            limit: archive.max_bytes,
            expected_size: None,
            timeout: codex_release_builder::RELEASE_TRANSFER_TIMEOUT_SECONDS,
            progress: None,
        },
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub(super) fn prepare(
    roots: &LocalCoreRoots,
    selected: Option<runtime::LocalRecipe>,
    staging: &Path,
) -> Result<Option<SourceRuntime>> {
    let Some(selected) = selected else {
        return Ok(None);
    };
    let recipe = staging.join("runtime-recipe.json");
    write(&recipe, selected.bytes)?;
    let revision = selected.revision.as_str();
    let identity = staging.join("runtime-inputs");
    let mut inputs = runtime::input_bytes().to_vec();
    inputs.extend(selected.identity);
    // Core constructs the compiler environment consumed by the shared producer.
    // Bind those rules too; changing flags must not reuse an earlier raw output.
    inputs.extend(include_bytes!("local_source.rs"));
    write(&identity, inputs)?;
    let key = openssl_sha256(&roots.openssl, &identity).map_err(|e| e.to_string())?;
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or(
            required_absolute_env_path("HOME")
                .map_err(|e| e.to_string())?
                .join(".cache"),
        )
        .join("codex/source-build")
        .join(key);
    private(&cache)?;
    let _lease = cache_lease(&cache)?;
    let artifact = cache.join("artifact");
    let pending = cache.join("runtime-pending");
    let audit_openssl = fs::canonicalize(&roots.openssl)
        .map_err(|e| format!("resolve prepared OpenSSL executable: {e}"))?;
    if artifact.exists() {
        runtime::verify_artifact(&artifact, &recipe, &audit_openssl)?;
        println!("[local build] Reuse verified compiled source runtime.");
        return Ok(Some(SourceRuntime {
            path: artifact.join("codex"),
            revision: revision.into(),
        }));
    }
    // Incomplete output may still have a live compiler even before source
    // preparation becomes dirty. Never rename its output or reset its checkout.
    for process in fs::read_dir("/proc").map_err(|e| e.to_string())?.flatten() {
        if !process
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        if ["cwd", "exe"].iter().any(|kind| {
            fs::read_link(process.path().join(kind)).is_ok_and(|path| path.starts_with(&cache))
        }) {
            return Err("a native source-build process still owns this cache; retained".into());
        }
    }
    let tools = roots
        .curl
        .parent()
        .ok_or("Termux tool directory unavailable")?;
    packages(tools)?;
    let source = cache.join("official-source");
    if !source.exists() {
        private(&source)?;
    }
    let git = |args: &[&str]| {
        let mut command = Command::new(tools.join("git"));
        command
            .arg("-C")
            .arg(&source)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null");
        command
    };
    if !source.join(".git").exists() {
        step(&mut git(&["init"]), "initialize official source checkout")?;
    }
    let head = git(&["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    if !head.status.success() {
        step(
            &mut git(&[
                "fetch",
                "--no-tags",
                "--depth=1",
                "https://github.com/openai/codex.git",
                revision,
            ]),
            "fetch exact official source",
        )?;
        step(
            &mut git(&["checkout", "--detach", "FETCH_HEAD"]),
            "select exact official source",
        )?;
    } else if head.stdout != format!("{revision}\n").as_bytes() {
        return Err("cached official source revision differs".into());
    }
    let dirty = git(&["status", "--porcelain"])
        .output()
        .map_err(|e| e.to_string())?;
    if !dirty.status.success() {
        return Err("cannot inspect cached official source".into());
    }
    if !dirty.stdout.is_empty() {
        let patch = fs::read(pending.join("source.patch"))
            .map_err(|_| "previous source build checkout has unrecorded changes; preserved")?;
        let current = git(&["diff", "--binary"])
            .output()
            .map_err(|e| e.to_string())?;
        let extra = git(&["ls-files", "--others", "--exclude-standard"])
            .output()
            .map_err(|e| e.to_string())?;
        if !current.status.success()
            || !extra.status.success()
            || !extra.stdout.is_empty()
            || current.stdout != patch
        {
            return Err(
                "previous source build checkout changed beyond its recorded preparation; preserved"
                    .into(),
            );
        }
        step(
            &mut git(&["restore", "--worktree", "."]),
            "restore exact owned preparation for source-build retry",
        )?;
    }
    if pending.exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        fs::rename(&pending, cache.join(format!("failed-runtime-{stamp}")))
            .map_err(|e| e.to_string())?;
    }
    let archives = cache.join("archives");
    private(&archives)?;
    let sdk_root = cache.join("sdk");
    let sdk_record = cache.join("sdk-tree-v1");
    let sdk = if cached(&sdk_root, &sdk_record, &roots.openssl)? {
        native_sdk::Tools::at(&sdk_root)
    } else {
        for archive in &native_sdk::inputs()?.archives {
            download(roots, archive, &archives.join(&archive.name))?;
        }
        println!("[local build] Assemble and execute private compiler SDK.");
        let sdk = native_sdk::prepare(&native_sdk::Request {
            archives: &archives,
            output: &sdk_root,
            tools,
        })?;
        seal(&sdk_root, &sdk_record, &roots.openssl)?;
        sdk
    };
    let libraries_root = cache.join("libraries");
    let libraries_record = cache.join("libraries-tree-v1");
    let libraries = if cached(&libraries_root, &libraries_record, &roots.openssl)? {
        native_dependencies::Libraries {
            openssl: libraries_root.join("openssl"),
            libcap: libraries_root.join("libcap"),
        }
    } else {
        let inputs = native_dependencies::inputs(&source, revision)?;
        for archive in [&inputs.openssl.archive, &inputs.libcap.archive] {
            download(roots, archive, &archives.join(&archive.name))?;
        }
        let libraries = native_dependencies::prepare(&native_dependencies::Request {
            source: &source,
            revision,
            archives: &archives,
            output: &libraries_root,
            build_dir: &cache.join("dependency-build"),
            sdk: &sdk,
            tools,
        })?;
        seal(&libraries_root, &libraries_record, &roots.openssl)?;
        libraries
    };
    let mut environment: BTreeMap<OsString, OsString> =
        ["HOME", "PREFIX", "TMPDIR", "LANG", "TERM"]
            .into_iter()
            .filter_map(|name| std::env::var_os(name).map(|v| (name.into(), v)))
            .collect();
    for (name,value) in [("PATH",format!("{}:{}",sdk.rustc.parent().unwrap().display(),tools.display())),
        ("CARGO_HOME",std::env::var_os("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|| cache.parent().unwrap().join("cargo")).to_string_lossy().into_owned()),
        ("CARGO_BUILD_JOBS","1".into()),("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER",sdk.linker.to_string_lossy().into_owned()),
        ("CC",sdk.cc.to_string_lossy().into_owned()),("CXX",sdk.cxx.to_string_lossy().into_owned()),
        ("AR",tools.join("llvm-ar").to_string_lossy().into_owned()),("PROTOC",tools.join("protoc").to_string_lossy().into_owned()),
        ("SHELL",tools.join("sh").to_string_lossy().into_owned()),("CONFIG_SHELL",tools.join("sh").to_string_lossy().into_owned()),
        ("CFLAGS","-pthread -Wno-error=frame-larger-than".into()),("CXXFLAGS","-pthread -Wno-error=frame-larger-than".into()),
        ("CMAKE_C_COMPILER",sdk.cc.to_string_lossy().into_owned()),("CMAKE_CXX_COMPILER",sdk.cxx.to_string_lossy().into_owned()),
        ("CMAKE_ARGS","-DCMAKE_HAVE_THREADS_LIBRARY=1 -DCMAKE_USE_PTHREADS_INIT=1 -DCMAKE_THREAD_LIBS_INIT=-pthread -DTHREADS_PREFER_PTHREAD_FLAG=ON".into()),
        ("AARCH64_UNKNOWN_LINUX_MUSL_OPENSSL_DIR",libraries.openssl.to_string_lossy().into_owned()),
        ("AARCH64_UNKNOWN_LINUX_MUSL_OPENSSL_STATIC","1".into()),("AARCH64_UNKNOWN_LINUX_MUSL_OPENSSL_NO_VENDOR","1".into()),
        ("PKG_CONFIG_ALLOW_CROSS","1".into()),("PKG_CONFIG_PATH_aarch64_unknown_linux_musl",libraries.libcap.join("lib/pkgconfig").to_string_lossy().into_owned()),
        ("PKG_CONFIG_LIBDIR_aarch64_unknown_linux_musl",libraries.libcap.join("lib/pkgconfig").to_string_lossy().into_owned())]{environment.insert(name.into(),value.into());}
    println!("[local build] Compile selected source runtime; output follows.");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| cache.parent().unwrap().join("target"));
    if !target.is_absolute() {
        return Err("source build compiler cache must be absolute".into());
    }
    runtime::build(&runtime::BuildRequest {
        source: &source,
        output: &pending,
        recipe: &recipe,
        target_dir: &target,
        rustc: &sdk.rustc,
        cargo: &tools.join("cargo"),
        openssl: &audit_openssl,
        environment: Some(&environment),
    })?;
    fs::rename(&pending, &artifact)
        .map_err(|e| format!("publish complete source-runtime cache: {e}"))?;
    Ok(Some(SourceRuntime {
        path: artifact.join("codex"),
        revision: revision.into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_protobuf_requests_install_and_must_be_available_afterward() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "source-packages-{}-{}",
            std::process::id(),
            LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let native = required_absolute_env_path("PREFIX").unwrap().join("bin");
        // Only package planning/post-install admission is exercised here. The
        // actual SDK/dependency proof executes real tools separately.
        for (tool, _) in REQUIRED_PACKAGES {
            if tool != "protoc" {
                write(&root.join(tool), b"unexecuted package fixture\n").unwrap();
            }
        }
        let pkg = root.join("pkg");
        let installer = format!(
            "#!{}/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\n",
            native.display()
        );
        write(&pkg, &installer).unwrap();
        fs::set_permissions(&pkg, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(packages(&root)
            .unwrap_err()
            .contains("unavailable after preparation: protoc"));
        assert_eq!(
            fs::read(root.join("pkg.args")).unwrap(),
            b"install\n-y\nprotobuf\n"
        );
        write(&pkg, format!("{installer}: > \"${{0%/*}}/protoc\"\n")).unwrap();
        packages(&root).unwrap();
        assert!(root.join("protoc").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_tool_cache_rejects_changed_bytes_and_aliases_before_reuse() {
        let root = std::env::temp_dir().join(format!(
            "source-cache-{}-{}",
            std::process::id(),
            LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let lease = cache_lease(&root).unwrap();
        assert!(cache_lease(&root)
            .unwrap_err()
            .contains("another update owns"));
        drop(lease);
        drop(cache_lease(&root).unwrap());
        let sdk = root.join("sdk");
        fs::create_dir(&sdk).unwrap();
        let file = sdk.join("file with spaces");
        write(&file, b"first").unwrap();
        let receipt = root.join("tree");
        let openssl = required_absolute_env_path("PREFIX")
            .unwrap()
            .join("bin/openssl");
        seal(&sdk, &receipt, &openssl).unwrap();
        assert!(cached(&sdk, &receipt, &openssl).unwrap());
        write(&file, b"other").unwrap();
        assert!(cached(&sdk, &receipt, &openssl)
            .unwrap_err()
            .contains("cache changed"));
        write(&file, b"first").unwrap();
        let original = root.join("preserved");
        fs::rename(&sdk, &original).unwrap();
        std::os::unix::fs::symlink(&original, &sdk).unwrap();
        assert!(cached(&sdk, &receipt, &openssl)
            .unwrap_err()
            .contains("not a real directory"));
        assert_eq!(
            fs::read(original.join("file with spaces")).unwrap(),
            b"first"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

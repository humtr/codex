//! Assemble the owner-pinned native compiler SDK without changing host libraries.
//! Acquisition belongs to Core; only private authenticated snapshots are extracted.
use super::runtime::output;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub max_bytes: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub rust: String,
    pub zig: String,
    pub archives: Vec<Archive>,
}

pub fn inputs() -> Result<Inputs> {
    serde_json::from_str(include_str!("../../../release/native-sdk-inputs.json"))
        .map_err(|e| format!("native SDK input catalog: {e}"))
}

pub struct Request<'a> {
    pub archives: &'a Path,
    pub output: &'a Path,
    pub tools: &'a Path,
}

#[derive(Serialize)]
pub struct Tools {
    pub rustc: PathBuf,
    pub linker: PathBuf,
    pub zig: PathBuf,
    pub cc: PathBuf,
    pub cxx: PathBuf,
}

fn c_adapter(zig: &Path, bash: &Path) -> String {
    // Match the official Codex musl build contract: cc-rs supplies a Rust
    // target spelling, while Zig requires its own target spelling. Keep host
    // headers after target headers and normalize the GCC forwarding form.
    let executable = format!("'{}'", zig.to_string_lossy().replace('\'', "'\\''"));
    format!("#!{}\n", bash.display())
        + r#"set -euo pipefail
case "${0##*/}" in cxx) language=c++;; *) language=cc;; esac
args=()
while (($#)); do
  case "$1" in
    --target|-target) (($# >= 2)) || exit 2; shift 2;;
    --target=*|-target=*) shift;;
    -I)
      (($# >= 2)) || exit 2
      case "$2" in /usr/include|/usr/include/*) args+=(-idirafter "$2");; *) args+=(-I "$2");; esac
      shift 2;;
    -I/usr/include|-I/usr/include/*) args+=(-idirafter "${1#-I}"); shift;;
    -Wp,-U_FORTIFY_SOURCE) args+=(-U_FORTIFY_SOURCE); shift;;
    *) args+=("$1"); shift;;
  esac
done
exec "# + &executable
        + r#" "$language" -target aarch64-linux-musl "${args[@]}" -fno-sanitize=undefined
"#
}

fn regular(path: &Path, limit: u64) -> Result<()> {
    let info = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !info.is_file() || info.len() == 0 || info.len() > limit {
        return Err(format!(
            "{}: SDK archive must be a bounded regular file",
            path.display()
        ));
    }
    Ok(())
}

fn unpack(tar: &Path, archive: &Path, target: &Path, strip: &str, member: &str) -> Result<()> {
    output(
        Command::new(tar)
            .args(["--extract", "--file"])
            .arg(archive)
            .arg("--directory")
            .arg(target)
            .arg("--no-same-owner")
            .arg("--strip-components")
            .arg(strip)
            .arg(member),
        "extract admitted native SDK input",
    )?;
    Ok(())
}

fn library(tar: &Path, archive: &Path, member: &str, target: &Path) -> Result<()> {
    let bytes = output(
        Command::new(tar)
            .args(["--extract", "--to-stdout", "--file"])
            .arg(archive)
            .arg(member),
        "extract admitted native SDK library",
    )?;
    fs::write(target, bytes).map_err(|e| format!("write private SDK library: {e}"))?;
    fs::set_permissions(target, fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("set SDK library mode: {e}"))
}

/// Reserve a new private SDK, snapshot/admit every archive, assemble and execute
/// its compiler. Existing output is never merged, repaired or removed.
pub fn prepare(request: &Request<'_>) -> Result<Tools> {
    let catalog = inputs()?;
    for path in [request.archives, request.tools] {
        if !path.is_absolute() || fs::canonicalize(path).map_err(|e| e.to_string())? != path {
            return Err("native SDK requires canonical absolute archive/tool directories".into());
        }
    }
    let parent = request
        .output
        .parent()
        .ok_or("native SDK output has no parent")?;
    if !request.output.is_absolute()
        || fs::canonicalize(parent).map_err(|e| e.to_string())? != parent
        || request
            .output
            .file_name()
            .is_none_or(|n| n == "." || n == "..")
    {
        return Err("native SDK output requires a canonical existing parent".into());
    }
    fs::DirBuilder::new()
        .mode(0o700)
        .create(request.output)
        .map_err(|e| format!("reserve new private native SDK: {e}"))?;
    let result = (|| {
        let snapshots = request.output.join(".inputs");
        fs::create_dir(&snapshots).map_err(|e| e.to_string())?;
        for archive in &catalog.archives {
            let input = request.archives.join(&archive.name);
            regular(&input, archive.max_bytes)?;
            let snapshot = snapshots.join(&archive.name);
            let source = fs::File::open(&input).map_err(|e| format!("open SDK archive: {e}"))?;
            let mut destination =
                super::create_private_file(&snapshot).map_err(|e| e.to_string())?;
            let bytes = std::io::copy(&mut source.take(archive.max_bytes + 1), &mut destination)
                .map_err(|e| format!("snapshot SDK archive: {e}"))?;
            if bytes > archive.max_bytes {
                return Err(format!(
                    "native SDK archive exceeds bound: {}",
                    archive.name
                ));
            }
            regular(&snapshot, archive.max_bytes)?;
            if super::openssl_sha256(&request.tools.join("openssl"), &snapshot)
                .map_err(|e| e.to_string())?
                != archive.sha256
            {
                return Err(format!(
                    "native SDK archive digest differs: {}",
                    archive.name
                ));
            }
        }
        let tar = request.tools.join("tar");
        let rust = request.output.join("rust");
        let zig = request.output.join("zig");
        fs::create_dir(&rust)
            .and_then(|()| fs::create_dir(&zig))
            .map_err(|e| e.to_string())?;
        unpack(
            &tar,
            &snapshots.join("rustc.tar.xz"),
            &rust,
            "2",
            &format!("rustc-{}-{}/rustc", catalog.rust, super::runtime::TARGET),
        )?;
        unpack(
            &tar,
            &snapshots.join("rust-std.tar.xz"),
            &rust,
            "2",
            &format!(
                "rust-std-{}-{}/rust-std-{}",
                catalog.rust,
                super::runtime::TARGET,
                super::runtime::TARGET
            ),
        )?;
        unpack(
            &tar,
            &snapshots.join("zig.tar.xz"),
            &zig,
            "1",
            &format!("zig-linux-aarch64-{}", catalog.zig),
        )?;
        let lib = rust.join("lib");
        let loader = lib.join("ld-musl-aarch64.so.1");
        library(
            &tar,
            &snapshots.join("musl.apk"),
            "lib/ld-musl-aarch64.so.1",
            &loader,
        )?;
        library(
            &tar,
            &snapshots.join("libgcc.apk"),
            "usr/lib/libgcc_s.so.1",
            &lib.join("libgcc_s.so.1"),
        )?;
        for (name, target) in [
            ("libc.musl-aarch64.so.1", "ld-musl-aarch64.so.1"),
            ("libc.so", "ld-musl-aarch64.so.1"),
            ("libgcc_s.so", "libgcc_s.so.1"),
        ] {
            std::os::unix::fs::symlink(target, lib.join(name)).map_err(|e| e.to_string())?;
        }
        let patchelf = request.tools.join("patchelf");
        for name in [
            "bin/rustc",
            "bin/rustdoc",
            "lib/rustlib/aarch64-unknown-linux-musl/bin/rust-lld",
        ] {
            let binary = rust.join(name);
            output(
                Command::new(&patchelf)
                    .arg("--set-interpreter")
                    .arg(&loader)
                    .arg(&binary),
                "set private SDK interpreter",
            )?;
            output(
                Command::new(&patchelf)
                    .arg("--set-rpath")
                    .arg(&lib)
                    .arg(&binary),
                "set private SDK library search",
            )?;
        }
        let linker_directory = request.output.join("linker");
        fs::create_dir(&linker_directory).map_err(|e| e.to_string())?;
        // rustc chooses its linker argument dialect from this executable name.
        let linker = linker_directory.join("ld.lld");
        // The prepared native LLD receives only ELF objects: the shared release
        // profile disables dependency LTO as well as final-crate LTO.
        let quote = |value: &Path| format!("'{}'", value.to_string_lossy().replace('\'', "'\\''"));
        fs::write(
            &linker,
            format!(
                "#!{}\nexec {} -flavor gnu {} --threads=1 \"$@\"\n",
                request.tools.join("sh").display(),
                quote(&request.tools.join("ld.lld")),
                quote(&PathBuf::from(format!("-L{}", lib.display())))
            ),
        )
        .map_err(|e| e.to_string())?;
        fs::set_permissions(&linker, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        let tools = Tools {
            rustc: rust.join("bin/rustc"),
            linker,
            zig: zig.join("zig"),
            cc: request.output.join("cc"),
            cxx: request.output.join("cxx"),
        };
        fs::write(
            &tools.cc,
            c_adapter(&tools.zig, &request.tools.join("bash")),
        )
        .map_err(|e| format!("write native C/C++ adapter: {e}"))?;
        fs::set_permissions(&tools.cc, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        std::os::unix::fs::symlink("cc", &tools.cxx).map_err(|e| e.to_string())?;
        let version = output(
            Command::new(&tools.rustc).arg("--version"),
            "execute prepared native Rust compiler",
        )?;
        if std::str::from_utf8(&version)
            .map_err(|e| e.to_string())?
            .split_whitespace()
            .nth(1)
            != Some(catalog.rust.as_str())
        {
            return Err("native SDK compiler version differs".into());
        }
        if output(
            Command::new(&tools.zig).arg("version"),
            "execute prepared native Zig",
        )? != format!("{}\n", catalog.zig).as_bytes()
        {
            return Err("native SDK Zig version differs".into());
        }
        super::runtime::compile_target_probe(
            &tools.rustc,
            request.output,
            &catalog.rust,
            Some(tools.linker.as_os_str()),
        )?;
        fs::remove_dir_all(&snapshots).map_err(|e| format!("remove SDK input snapshots: {e}"))?;
        Ok(tools)
    })();
    match result {
        Ok(tools) => Ok(tools),
        Err(error) => match fs::remove_dir_all(request.output) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!("{error}; remove incomplete owned SDK: {cleanup}")),
        },
    }
}

pub(super) fn command(args: &[OsString]) -> i32 {
    let result = (|| {
        let mut fields = BTreeMap::new();
        if args.len() != 7 {
            return Err("prepare-native-sdk requires --archives, --output and --tools".into());
        }
        for pair in args[1..].as_chunks::<2>().0 {
            let name = pair[0].to_str().ok_or("invalid native SDK flag")?;
            if !["--archives", "--output", "--tools"].contains(&name)
                || fields.insert(name, Path::new(&pair[1])).is_some()
            {
                return Err("invalid or duplicate native SDK flag".into());
            }
        }
        prepare(&Request {
            archives: fields["--archives"],
            output: fields["--output"],
            tools: fields["--tools"],
        })
    })();
    match result {
        Ok(tools) => {
            println!(
                "{}",
                serde_json::to_string(&tools).expect("native SDK paths serialize")
            );
            0
        }
        Err(error) => {
            eprintln!("codex-release-builder: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_c_and_cxx_adapters_preserve_arguments_and_normalize_the_target() {
        let root = std::env::temp_dir().join(format!(
            "sdk-cc-{}-{}",
            std::process::id(),
            super::super::STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let prefix = PathBuf::from(std::env::var_os("PREFIX").unwrap_or_else(|| "/usr".into()));
        let zig = root.join("compiler's quoted path");
        fs::write(
            &zig,
            format!(
                "#!{}\nprintf '<%s>\\n' \"$@\"\n",
                prefix.join("bin/sh").display()
            ),
        )
        .unwrap();
        fs::set_permissions(&zig, fs::Permissions::from_mode(0o700)).unwrap();
        let cc = root.join("cc");
        fs::write(&cc, c_adapter(&zig, &prefix.join("bin/bash"))).unwrap();
        fs::set_permissions(&cc, fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink("cc", root.join("cxx")).unwrap();
        for language in ["cc", "cxx"] {
            let result = Command::new(root.join(language))
                .args([
                    "--target=aarch64-unknown-linux-musl",
                    "-I",
                    "/usr/include",
                    "-I/a/quoted path",
                    "-Wp,-U_FORTIFY_SOURCE",
                    "-c",
                    "file with spaces.c",
                ])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let compiler = if language == "cc" { "cc" } else { "c++" };
            assert_eq!(String::from_utf8(result.stdout).unwrap(),format!(
                "<{compiler}>\n<-target>\n<aarch64-linux-musl>\n<-idirafter>\n</usr/include>\n<-I/a/quoted path>\n<-U_FORTIFY_SOURCE>\n<-c>\n<file with spaces.c>\n<-fno-sanitize=undefined>\n"));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bad_archive_is_rejected_before_extraction_and_existing_sdk_is_preserved() {
        let root = std::env::temp_dir().join(format!(
            "sdk-admission-{}-{}",
            std::process::id(),
            super::super::STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let archives = root.join("archives");
        fs::create_dir(&archives).unwrap();
        fs::write(archives.join("rustc.tar.xz"), b"not the admitted archive").unwrap();
        let prefix = PathBuf::from(std::env::var_os("PREFIX").unwrap_or_else(|| "/usr".into()));
        let output = root.join("sdk");
        let request = Request {
            archives: &archives,
            output: &output,
            tools: &prefix.join("bin"),
        };
        let error = prepare(&request).err().unwrap();
        assert!(error.contains("digest differs: rustc.tar.xz"), "{error}");
        assert!(!output.exists());
        fs::create_dir(&output).unwrap();
        fs::write(output.join("user-file"), b"preserve").unwrap();
        assert!(prepare(&request)
            .err()
            .unwrap()
            .contains("reserve new private native SDK"));
        assert_eq!(fs::read(output.join("user-file")).unwrap(), b"preserve");
        fs::remove_dir_all(root).unwrap();
    }
}

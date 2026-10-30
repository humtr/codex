//! Native target C libraries selected by the exact upstream musl recipe.
//! Downloading/system package installation belong to Core, not these builders.
use super::native_sdk::{Archive, Tools};
use super::runtime::output;
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, String>;

#[derive(Serialize)]
pub struct LibraryInput {
    pub version: String,
    pub archive: Archive,
}

#[derive(Serialize)]
pub struct Inputs {
    pub openssl: LibraryInput,
    pub libcap: LibraryInput,
    pub openssl_options: Vec<String>,
}

fn literal(script: &str, name: &str) -> Result<String> {
    let prefix = format!("{name}=");
    let values: Vec<_> = script
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .collect();
    if values.len() != 1 {
        return Err(format!("upstream musl recipe: missing/ambiguous {name}"));
    }
    values[0]
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| format!("upstream musl recipe: nonliteral {name}"))
}

fn parse(openssl: &str, libcap: &str) -> Result<Inputs> {
    let ssl_version = literal(openssl, "openssl_version")?;
    let ssl_hash = literal(openssl, "openssl_sha256")?;
    let cap_version = literal(libcap, "libcap_version")?;
    let cap_hash = literal(libcap, "libcap_sha256")?;
    if !super::valid_stable_version(&ssl_version)
        || cap_version.split('.').count() != 2
        || !super::valid_stable_version(&format!("{cap_version}.0"))
        || !super::valid_lower_sha256(&ssl_hash)
        || !super::valid_lower_sha256(&cap_hash)
    {
        return Err("upstream musl recipe: invalid dependency version/digest".into());
    }
    let marker = "CC=\"${OPENSSL_CC}\" perl ./Configure \"${openssl_target}\" \\";
    let mut starts = openssl
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim() == marker);
    let start = starts
        .next()
        .ok_or("upstream musl recipe: unsupported OpenSSL Configure invocation")?
        .0;
    if starts.next().is_some() {
        return Err("upstream musl recipe: ambiguous OpenSSL Configure invocation".into());
    }
    let mut arguments = Vec::new();
    let mut ended = false;
    for line in openssl.lines().skip(start + 1) {
        let line = line.trim();
        let continued = line.ends_with('\\');
        arguments.extend(
            line.trim_end_matches('\\')
                .split_whitespace()
                .map(str::to_owned),
        );
        if !continued {
            ended = true;
            break;
        }
    }
    if !ended || arguments.first().map(String::as_str) != Some("\"--prefix=${openssl_prefix}\"") {
        return Err("upstream musl recipe: unsupported private OpenSSL prefix".into());
    }
    arguments.remove(0); // The actual private prefix is selected by this build.
    if arguments.iter().any(|a| {
        !a.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_=/.".contains(&b))
            || a.starts_with("--prefix=")
            || a.starts_with("--libdir=") && a != "--libdir=lib"
            || a.starts_with("--openssldir=") && a != "--openssldir=/usr/local/ssl"
    }) || [
        "--libdir=lib",
        "--openssldir=/usr/local/ssl",
        "no-shared",
        "no-module",
        "no-tests",
    ]
    .iter()
    .any(|required| !arguments.iter().any(|a| a == required))
    {
        return Err("upstream musl recipe: unsupported OpenSSL build options".into());
    }
    Ok(Inputs {
        openssl: LibraryInput { archive: Archive {
            name:format!("openssl-{ssl_version}.tar.gz"),
            url:format!("https://github.com/openssl/openssl/releases/download/openssl-{ssl_version}/openssl-{ssl_version}.tar.gz"),
            sha256:ssl_hash, max_bytes:super::ARCHIVE_MAX_BYTES }, version:ssl_version },
        libcap: LibraryInput { archive: Archive {
            name:format!("libcap-{cap_version}.tar.xz"),
            url:format!("https://mirrors.edge.kernel.org/pub/linux/libs/security/linux-privs/libcap2/libcap-{cap_version}.tar.xz"),
            sha256:cap_hash, max_bytes:super::ARCHIVE_MAX_BYTES }, version:cap_version },
        openssl_options:arguments,
    })
}

pub fn inputs(source: &Path, revision: &str) -> Result<Inputs> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("native dependencies require exact official source commit".into());
    }
    let git = |args: &[&str]| {
        output(
            Command::new("git").arg("-C").arg(source).args(args),
            "read upstream musl recipe",
        )
    };
    if git(&["rev-parse", "HEAD"])? != format!("{revision}\n").as_bytes() {
        return Err("native dependency source commit differs".into());
    }
    let read = |name: &str| -> Result<String> {
        let bytes = git(&["show", &format!("{revision}:.github/scripts/{name}")])?;
        // Same16-KiB acceptance bound as runtime recipe controls.
        if bytes.len() > 16 * 1024 {
            return Err(format!("upstream musl recipe exceeds16 KiB: {name}"));
        }
        String::from_utf8(bytes).map_err(|e| e.to_string())
    };
    parse(
        &read("install-musl-openssl.sh")?,
        &read("install-musl-build-tools.sh")?,
    )
}

pub struct Request<'a> {
    pub source: &'a Path,
    pub revision: &'a str,
    pub archives: &'a Path,
    pub output: &'a Path,
    pub build_dir: &'a Path,
    pub sdk: &'a Tools,
    pub tools: &'a Path,
}

#[derive(Serialize)]
pub struct Libraries {
    pub openssl: PathBuf,
    pub libcap: PathBuf,
}

fn build(command: &mut Command, phase: &str) -> Result<()> {
    println!("[dependencies] {phase}");
    let status = command.status().map_err(|e| format!("{phase}: {e}"))?;
    if !status.success() {
        return Err(format!("{phase}: {status}"));
    }
    Ok(())
}

pub fn prepare(request: &Request<'_>) -> Result<Libraries> {
    let selected = inputs(request.source, request.revision)?;
    for path in [request.output, request.build_dir] {
        let parent = path
            .parent()
            .ok_or("native dependency root has no parent")?;
        if !path.is_absolute() || fs::canonicalize(parent).map_err(|e| e.to_string())? != parent {
            return Err("native dependency roots require canonical absolute parents".into());
        }
    }
    fs::DirBuilder::new()
        .mode(0o700)
        .create(request.output)
        .map_err(|e| format!("reserve private native library prefix: {e}"))?;
    let result = (|| {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(request.build_dir)
            .map_err(|e| format!("reserve private native dependency build directory: {e}"))?;
        let snapshots = request.build_dir.join(".inputs");
        fs::create_dir(&snapshots).map_err(|e| e.to_string())?;
        let ssl_archive = selected.openssl.archive.snapshot(
            request.archives,
            &snapshots,
            &request.tools.join("openssl"),
        )?;
        let cap_archive = selected.libcap.archive.snapshot(
            request.archives,
            &snapshots,
            &request.tools.join("openssl"),
        )?;
        let ssl_source = request.build_dir.join("openssl-source");
        let cap_source = request.build_dir.join("libcap-source");
        fs::create_dir(&ssl_source)
            .and_then(|()| fs::create_dir(&cap_source))
            .map_err(|e| e.to_string())?;
        super::native_sdk::unpack(
            &request.tools.join("tar"),
            &ssl_archive,
            &ssl_source,
            "1",
            &format!("openssl-{}", selected.openssl.version),
        )?;
        super::native_sdk::unpack(
            &request.tools.join("tar"),
            &cap_archive,
            &cap_source,
            "1",
            &format!("libcap-{}", selected.libcap.version),
        )?;
        fs::remove_dir_all(&snapshots).map_err(|e| e.to_string())?;
        let libraries = Libraries {
            openssl: request.output.join("openssl"),
            libcap: request.output.join("libcap"),
        };
        build(
            Command::new(request.tools.join("perl"))
                .arg(ssl_source.join("Configure"))
                .arg("linux-aarch64")
                .arg(format!("--prefix={}", libraries.openssl.display()))
                .args(&selected.openssl_options)
                .env("CC", &request.sdk.cc)
                .env("AR", request.tools.join("llvm-ar"))
                .env("RANLIB", request.tools.join("llvm-ranlib"))
                .current_dir(&ssl_source),
            "configure target OpenSSL",
        )?;
        for target in ["build_libs", "install_dev"] {
            build(
                Command::new(request.tools.join("make"))
                    .args(["-j1", target])
                    .arg(format!("SHELL={}", request.tools.join("sh").display()))
                    .current_dir(&ssl_source),
                "build/install target OpenSSL",
            )?;
        }
        let cap_lib = cap_source.join("libcap");
        build(
            Command::new(request.tools.join("make"))
                .arg("-C")
                .arg(&cap_lib)
                .args(["-j1", "libcap.a", "libcap.pc", "lib=lib", "GOLANG=no"])
                .arg(format!("prefix={}", libraries.libcap.display()))
                .arg(format!("CC={}", request.sdk.cc.display()))
                .arg(format!(
                    "BUILD_CC={}",
                    request.tools.join("clang").display()
                ))
                .arg(format!("AR={}", request.tools.join("llvm-ar").display()))
                .arg(format!(
                    "RANLIB={}",
                    request.tools.join("llvm-ranlib").display()
                ))
                .arg(format!("SHELL={}", request.tools.join("sh").display())),
            "build target libcap and native generator",
        )?;
        for (from, to) in [
            ("libcap.a", "lib/libcap.a"),
            ("libcap.pc", "lib/pkgconfig/libcap.pc"),
            ("include/sys/capability.h", "include/sys/capability.h"),
            (
                "include/uapi/linux/capability.h",
                "include/linux/capability.h",
            ),
        ] {
            let target = libraries.libcap.join(to);
            fs::create_dir_all(target.parent().unwrap())
                .and_then(|()| fs::copy(cap_lib.join(from), target).map(|_| ()))
                .map_err(|e| format!("install private libcap artifact: {e}"))?;
        }
        let source = request.build_dir.join("libraries.c");
        fs::write(&source,format!(r#"#include <openssl/evp.h>
#include <openssl/ssl.h>
#include <openssl/crypto.h>
#include <sys/capability.h>
#include <stdio.h>
#include <string.h>
int main(void) {{
 const unsigned char expected[32]={{0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad}};
 unsigned char actual[32];unsigned int count=0;
 if(strcmp(OpenSSL_version(OPENSSL_VERSION_STRING),"{}"))return 1;
 if(!EVP_Digest("abc",3,actual,&count,EVP_sha256(),0)||count!=sizeof(expected)||memcmp(actual,expected,sizeof(expected)))return 2;
 SSL_CTX *ctx=SSL_CTX_new(TLS_method());if(!ctx)return 3;SSL_CTX_free(ctx);
 cap_t caps=cap_get_proc();if(!caps)return 4;if(cap_free(caps))return 5;
 puts("codex-native-libraries-ready");return 0;
}}
"#,selected.openssl.version)).map_err(|e| e.to_string())?;
        let probe = request.build_dir.join("libraries");
        build(
            Command::new(&request.sdk.cc)
                .arg(&source)
                .arg("-I")
                .arg(libraries.openssl.join("include"))
                .arg("-I")
                .arg(libraries.libcap.join("include"))
                .arg(libraries.openssl.join("lib/libssl.a"))
                .arg(libraries.openssl.join("lib/libcrypto.a"))
                .arg(libraries.libcap.join("lib/libcap.a"))
                .args(["-ldl", "-pthread", "-o"])
                .arg(&probe),
            "link native target C-library probe",
        )?;
        super::validate_static_aarch64_elf(&probe).map_err(|e| e.to_string())?;
        super::runtime::probe(&probe, request.build_dir, b"codex-native-libraries-ready\n")?;
        Ok(libraries)
    })();
    match result {
        Ok(libraries) => Ok(libraries),
        Err(error) => match fs::remove_dir_all(request.output) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!(
                "{error}; remove incomplete private libraries: {cleanup}"
            )),
        },
    }
}

pub(super) fn command(args: &[OsString]) -> i32 {
    let result = (|| -> Result<String> {
        let action = args
            .first()
            .and_then(|v| v.to_str())
            .ok_or("missing native dependency operation")?;
        let required: &[&str] = if action == "native-dependency-inputs" {
            &["--source", "--source-sha"]
        } else {
            &[
                "--source",
                "--source-sha",
                "--archives",
                "--output",
                "--build-dir",
                "--sdk",
                "--tools",
            ]
        };
        if args.len() != required.len() * 2 + 1 {
            return Err("native dependency operation has missing/extra arguments".into());
        }
        let mut fields = BTreeMap::new();
        for pair in args[1..].as_chunks::<2>().0 {
            let name = pair[0].to_str().ok_or("invalid native dependency flag")?;
            if !required.contains(&name) || fields.insert(name, Path::new(&pair[1])).is_some() {
                return Err("invalid/duplicate native dependency flag".into());
            }
        }
        let revision = fields["--source-sha"]
            .to_str()
            .ok_or("invalid native dependency source SHA")?;
        if action == "native-dependency-inputs" {
            return serde_json::to_string(&inputs(fields["--source"], revision)?)
                .map_err(|e| e.to_string());
        }
        serde_json::to_string(&prepare(&Request {
            source: fields["--source"],
            revision,
            archives: fields["--archives"],
            output: fields["--output"],
            build_dir: fields["--build-dir"],
            sdk: &Tools::at(fields["--sdk"]),
            tools: fields["--tools"],
        })?)
        .map_err(|e| e.to_string())
    })();
    match result {
        Ok(value) => {
            println!("{value}");
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
    fn dependency_recipe_comes_from_literal_upstream_values_and_rejects_changed_contracts() {
        let ssl=format!("openssl_version=\"9.2.1\"\nopenssl_sha256=\"{}\"\nCC=\"${{OPENSSL_CC}}\" perl ./Configure \"${{openssl_target}}\" \\\n  \"--prefix=${{openssl_prefix}}\" --openssldir=/usr/local/ssl --libdir=lib \\\n  no-shared no-module no-tests no-async -DOWNED=1\n", "a".repeat(64));
        let cap = format!(
            "libcap_version=\"4.7\"\nlibcap_sha256=\"{}\"\n",
            "b".repeat(64)
        );
        let inputs = parse(&ssl, &cap).unwrap();
        assert_eq!(inputs.openssl.archive.name, "openssl-9.2.1.tar.gz");
        assert_eq!(inputs.libcap.archive.name, "libcap-4.7.tar.xz");
        assert_eq!(inputs.openssl.archive.sha256, "a".repeat(64));
        assert!(inputs.openssl_options.contains(&"-DOWNED=1".into()));
        for bad in [
            ssl.replace("9.2.1", "$(hostname)"),
            ssl.clone() + "openssl_version=\"9.2.1\"\n",
            ssl.replace("--libdir=lib", "--libdir=/system/lib"),
            ssl.replace("no-shared", "shared"),
            ssl.replace("-DOWNED=1", "-DOTHER=$(id)"),
            ssl.replace("perl ./Configure", "perl ./other"),
        ] {
            assert!(parse(&bad, &cap).is_err(), "{bad}");
        }
    }
}

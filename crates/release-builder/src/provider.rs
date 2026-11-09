//! Independent optional Provider payload: one codec for producer and Core.
use super::*;

pub const FORMAT: &str = "codex-provider-payload-v1";
pub const API: &str = "provider-payload-v1";
pub const PROBE: &[u8] =
    b"codex-provider-artifact-v1\nprovider=chatgpt\ncontrol=provider-payload-v1\n";
pub const DEPENDENCIES: &str =
    "native-core;managed=proot+xvnc+explicit-rootfs;viewer=optional-assets";
pub const FILES: [&str; 6] = [
    "Cargo.lock",
    "NOTICE.md",
    "codex-chatgpt-provider",
    "licenses/novnc-LICENSE.txt",
    "licenses/proot-LICENSE",
    "source-inputs.json",
];

// Retain previously signed eight-file generations for verification and rollback.
// New packages use only FILES, without the retired adapter or summary template.
const LEGACY_FILES: [&str; 8] = [
    "Cargo.lock",
    "NOTICE.md",
    "codex-chatgpt-provider",
    "licenses/bridge-LICENSE",
    "licenses/novnc-LICENSE.txt",
    "licenses/proot-LICENSE",
    "licenses/upstream-codex-LICENSE",
    "source-inputs.json",
];

pub fn file_limit(name: &str) -> u64 {
    if name == "codex-chatgpt-provider" {
        128 * 1024 * 1024
    } else {
        1024 * 1024
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub generation: String,
    pub sequence: u64,
    pub manifest_digest: String,
}
impl Index {
    pub fn release_base(&self) -> String {
        format!(
            "https://github.com/humtr/codex/releases/download/{}/",
            self.generation
        )
    }
    pub fn render(&self) -> Result<Vec<u8>, &'static str> {
        if !valid_id(&self.generation)
            || self.sequence == 0
            || !valid_lower_sha256(&self.manifest_digest)
        {
            return Err("invalid Provider index identity");
        }
        Ok(format!("codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\ngeneration_id\t{}\nrelease_sequence\t{}\nrelease_base\t{}\nmanifest_sha256\t{}\n", self.generation,self.sequence,self.release_base(),self.manifest_digest).into_bytes())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 4096 || bytes.contains(&b'\r') || !bytes.ends_with(b"\n") {
            return Err("invalid Provider index bounds/text");
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "invalid Provider index text")?;
        let rows: Vec<_> = text.lines().collect();
        if rows.len() != 7
            || rows[..3]
                != [
                    "codex-provider-index-v1",
                    "provider\tchatgpt",
                    "channel\tstable",
                ]
        {
            return Err("invalid Provider index contract");
        }
        let field = |i: usize, name: &str| {
            rows[i]
                .strip_prefix(name)
                .ok_or("invalid Provider index field")
        };
        let result = Self {
            generation: field(3, "generation_id\t")?.into(),
            sequence: field(4, "release_sequence\t")?
                .parse()
                .map_err(|_| "invalid Provider index sequence")?,
            manifest_digest: field(6, "manifest_sha256\t")?.into(),
        };
        if result.render()? != bytes {
            return Err("noncanonical Provider index");
        }
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub digest: String,
    pub bytes: u64,
    pub mode: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub generation: String,
    pub sequence: u64,
    pub source: String,
    pub source_inputs: String,
    pub files: Vec<Entry>,
}

pub fn valid_id(value: &str) -> bool {
    (value.starts_with("provider-chatgpt-")
        || value.starts_with("provider-gpt-")
        || value.starts_with("chatgpt-"))
        && value.len() <= 120
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && !value.ends_with('-')
}

/// New payloads use the component name. The codec above retains signed legacy IDs.
fn valid_new_id(value: &str) -> bool {
    valid_id(value)
        && value
            .bytes()
            .last()
            .is_some_and(|last| last.is_ascii_alphanumeric())
        && value
            .strip_prefix("provider-chatgpt-v")
            .and_then(|suffix| suffix.bytes().next())
            .is_some_and(|first| first.is_ascii_digit())
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 8192 || bytes.contains(&b'\r') || !bytes.ends_with(b"\n") {
            return Err("invalid Provider manifest bounds");
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "invalid Provider manifest text")?;
        let mut lines = text.lines();
        if lines.next() != Some(FORMAT) || lines.next() != Some("provider\tchatgpt") {
            return Err("unsupported Provider payload");
        }
        fn field<'a>(lines: &mut std::str::Lines<'a>, name: &str) -> Result<&'a str, &'static str> {
            lines
                .next()
                .and_then(|line| line.split_once('\t'))
                .filter(|(key, value)| {
                    *key == name && !value.is_empty() && !value.contains(['\t', '\r'])
                })
                .map(|(_, value)| value)
                .ok_or("invalid ordered Provider field")
        }
        let generation = field(&mut lines, "generation_id")?.to_string();
        let sequence = field(&mut lines, "release_sequence")?;
        if sequence.starts_with('0') || !sequence.bytes().all(|b| b.is_ascii_digit()) {
            return Err("invalid Provider sequence");
        }
        let sequence = sequence
            .parse::<u64>()
            .map_err(|_| "invalid Provider sequence")?;
        let source = field(&mut lines, "source")?.to_string();
        let source_inputs = field(&mut lines, "source_inputs_sha256")?.to_string();
        if !valid_id(&generation)
            || source.len() != 40
            || !source
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !valid_lower_sha256(&source_inputs)
        {
            return Err("invalid Provider provenance");
        }
        if field(&mut lines, "target")? != "aarch64-linux-android"
            || field(&mut lines, "core_api")? != API
            || field(&mut lines, "dependencies")? != DEPENDENCIES
        {
            return Err("unsupported Provider contract");
        }
        let count = field(&mut lines, "file_count")?;
        let ordered_files = [FILES.as_slice(), LEGACY_FILES.as_slice()]
            .into_iter()
            .find(|layout| count == layout.len().to_string())
            .ok_or("unsupported Provider contract")?;
        let mut files = Vec::new();
        for &expected in ordered_files {
            let row: Vec<_> = lines
                .next()
                .ok_or("missing Provider file")?
                .split('\t')
                .collect();
            if row.len() != 5
                || row[0] != "file"
                || row[1] != expected
                || !valid_lower_sha256(row[2])
            {
                return Err("invalid Provider file inventory");
            }
            let bytes = row[3]
                .parse::<u64>()
                .map_err(|_| "invalid Provider file size")?;
            let mode = if expected == "codex-chatgpt-provider" {
                0o755
            } else {
                0o644
            };
            if row[3] != bytes.to_string()
                || bytes == 0
                || bytes > file_limit(expected)
                || row[4] != format!("{mode:04o}")
            {
                return Err("invalid Provider file size/mode");
            }
            files.push(Entry {
                name: expected.into(),
                digest: row[2].into(),
                bytes,
                mode,
            });
        }
        if lines.next().is_some() || files.last().unwrap().digest != source_inputs {
            return Err("invalid Provider source binding");
        }
        Ok(Self {
            generation,
            sequence,
            source,
            source_inputs,
            files,
        })
    }

    pub fn render(&self) -> Result<Vec<u8>, &'static str> {
        let mut text=format!("{FORMAT}\nprovider\tchatgpt\ngeneration_id\t{}\nrelease_sequence\t{}\nsource\t{}\nsource_inputs_sha256\t{}\ntarget\taarch64-linux-android\ncore_api\t{API}\ndependencies\t{DEPENDENCIES}\nfile_count\t{}\n",self.generation,self.sequence,self.source,self.source_inputs,self.files.len());
        for file in &self.files {
            text.push_str(&format!(
                "file\t{}\t{}\t{}\t{:04o}\n",
                file.name, file.digest, file.bytes, file.mode
            ));
        }
        let bytes = text.into_bytes();
        Self::parse(&bytes)?;
        Ok(bytes)
    }
}

pub fn exact_layout(root: &Path, manifest: &Manifest) -> Result<(), &'static str> {
    if root.symlink_metadata().is_err() || root.is_symlink() || !root.is_dir() {
        return Err("Provider package directory required");
    }
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(root).map_err(|_| "read Provider package")? {
        let entry = entry.map_err(|_| "read Provider entry")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid Provider filename")?;
        let kind = entry.file_type().map_err(|_| "inspect Provider entry")?;
        if name == "licenses" && kind.is_dir() {
            for child in std::fs::read_dir(entry.path()).map_err(|_| "read Provider licenses")? {
                let child = child.map_err(|_| "read Provider license")?;
                if !child
                    .file_type()
                    .map_err(|_| "inspect Provider license")?
                    .is_file()
                {
                    return Err("nonregular Provider entry");
                }
                names.insert(format!(
                    "licenses/{}",
                    child
                        .file_name()
                        .to_str()
                        .ok_or("invalid Provider license name")?
                ));
            }
        } else if kind.is_file() {
            names.insert(name);
        } else {
            return Err("nonregular Provider entry");
        }
    }
    // The signed manifest is the inventory authority. An otherwise valid
    // eight-file directory must not extend a signed six-file package.
    let expected: BTreeSet<String> = manifest
        .files
        .iter()
        .map(|entry| entry.name.clone())
        .chain(["payload.manifest".into(), "payload.sig".into()])
        .collect();
    if names != expected {
        return Err("Provider inventory differs from signed manifest");
    }
    Ok(())
}

fn package(args: &[OsString]) -> Result<(), BuilderError> {
    let mut fields = std::collections::BTreeMap::new();
    if !(args.len() - 1).is_multiple_of(2) {
        return Err(BuilderError::Usage);
    }
    for pair in args[1..].as_chunks::<2>().0 {
        let key = pair[0].to_str().ok_or(BuilderError::Usage)?;
        if ![
            "--binary",
            "--assets",
            "--source-inputs",
            "--source-sha",
            "--generation-id",
            "--release-sequence",
            "--private-key",
            "--openssl",
            "--output",
        ]
        .contains(&key)
            || fields.insert(key, &pair[1]).is_some()
        {
            return Err(BuilderError::Usage);
        }
    }
    if fields.len() != 9 {
        return Err(BuilderError::Usage);
    }
    let text = |key: &str| fields[key].to_str().ok_or(BuilderError::Usage);
    if !valid_new_id(text("--generation-id")?) {
        return Err(BuilderError::Invalid(
            "new Provider generation must use provider-chatgpt-v...",
        ));
    }
    let path = |key: &str| {
        let p = PathBuf::from(fields[key]);
        if canonical_absolute_path(&p) {
            Ok(p)
        } else {
            Err(BuilderError::Invalid(
                "canonical absolute Provider package path required",
            ))
        }
    };
    let output = path("--output")?;
    let openssl = path("--openssl")?;
    let key = path("--private-key")?;
    openssl_public_key(&openssl, &key)?;
    let assets = path("--assets")?;
    let binary = path("--binary")?;
    let source_inputs = path("--source-inputs")?;
    let sequence = text("--release-sequence")?
        .parse()
        .map_err(|_| BuilderError::Invalid("invalid Provider sequence"))?;
    let mut manifest = Manifest {
        generation: text("--generation-id")?.into(),
        sequence,
        source: text("--source-sha")?.into(),
        source_inputs: String::new(),
        files: Vec::new(),
    };
    if std::fs::symlink_metadata(&output).is_ok() {
        return Err(BuilderError::Invalid("Provider output already exists"));
    }
    let parent = output
        .parent()
        .ok_or(BuilderError::Invalid("Provider output has no parent"))?;
    if std::fs::canonicalize(parent).map_err(|e| io_error("resolve Provider output parent", e))?
        != parent
    {
        return Err(BuilderError::Invalid(
            "Provider output parent contains a symlink",
        ));
    }
    let staging = output.with_file_name(format!(
        ".provider-stage-{}-{}",
        std::process::id(),
        STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    create_private_dir(&staging)?;
    let result = (|| {
        create_private_dir(&staging.join("licenses"))?;
        for name in FILES {
            let source = match name {
                "codex-chatgpt-provider" => binary.clone(),
                "source-inputs.json" => source_inputs.clone(),
                _ => assets.join(name),
            };
            let metadata = ensure_regular_file(
                &source,
                "inspect Provider asset",
                "Provider asset must be regular",
            )?;
            if metadata.len() == 0 || metadata.len() > file_limit(name) {
                return Err(BuilderError::Invalid("Provider asset size invalid"));
            }
            let destination = staging.join(name);
            let input = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&source)
                .map_err(|e| io_error("open Provider asset", e))?;
            let opened = input
                .metadata()
                .map_err(|e| io_error("inspect opened Provider asset", e))?;
            if !opened.is_file() || opened.len() != metadata.len() {
                return Err(BuilderError::Invalid(
                    "Provider asset changed before snapshot",
                ));
            }
            if name == "codex-chatgpt-provider" {
                validate_publish_file_mode(&opened, true, "Provider executable mode is unsafe")?;
            }
            let mut file = create_private_file(&destination)?;
            io::copy(&mut input.take(file_limit(name) + 1), &mut file)
                .map_err(|e| io_error("copy Provider asset", e))?;
            if file
                .metadata()
                .map_err(|e| io_error("stat Provider asset", e))?
                .len()
                != metadata.len()
            {
                return Err(BuilderError::Invalid("Provider asset changed"));
            }
            if name == "codex-chatgpt-provider" {
                validate_android_aarch64_elf(&destination)?;
            }
            let mode = if name == "codex-chatgpt-provider" {
                0o755
            } else {
                0o644
            };
            set_mode(&destination, mode, "set Provider asset mode")?;
            file.sync_all()
                .map_err(|e| io_error("sync Provider asset", e))?;
            manifest.files.push(Entry {
                name: name.into(),
                digest: openssl_sha256(&openssl, &destination)?,
                bytes: metadata.len(),
                mode,
            });
        }
        manifest.source_inputs = manifest.files.last().unwrap().digest.clone();
        let bytes = manifest.render().map_err(BuilderError::Invalid)?;
        write_published_file(
            &staging.join("payload.manifest"),
            &bytes,
            0o644,
            "write Provider manifest",
        )?;
        sign_published_file(
            &openssl,
            &key,
            &staging.join("payload.manifest"),
            &staging.join("payload.sig"),
        )?;
        exact_layout(&staging, &manifest).map_err(BuilderError::Invalid)?;
        File::open(&staging)
            .and_then(|f| f.sync_all())
            .map_err(|e| io_error("sync Provider package", e))?;
        rename_noreplace(&staging, &output)?;
        File::open(output.parent().unwrap())
            .and_then(|f| f.sync_all())
            .map_err(|e| io_error("sync Provider parent", e))?;
        Ok(())
    })();
    finish_staging(&staging, result)
}

pub(crate) fn command(args: &[OsString]) -> i32 {
    match package(args) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("codex-release-builder Provider package: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_admits_android_pie_and_rejects_malformed_or_unsafe_inputs() {
        let openssl = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"))
            .join("bin/openssl");
        for fault in [
            "none",
            "type",
            "program-bound",
            "linker",
            "missing-linker",
            "legacy-magic",
            "mode",
        ] {
            let root = std::env::temp_dir().join(format!(
                "provider-producer-{}-{}",
                std::process::id(),
                STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            let assets = root.join("assets");
            std::fs::create_dir_all(assets.join("licenses")).unwrap();
            for name in FILES
                .into_iter()
                .filter(|name| !matches!(*name, "codex-chatgpt-provider" | "source-inputs.json"))
            {
                std::fs::write(assets.join(name), b"owned fixture input\n").unwrap();
            }
            let binary = root.join("provider");
            let mut bytes = super::super::tests::fake_core_elf(ANDROID_AARCH64_INTERPRETER);
            match fault {
                "none" | "mode" => {}
                "type" => bytes[16..18].copy_from_slice(&2u16.to_le_bytes()),
                "program-bound" => bytes[32..40].copy_from_slice(&u64::MAX.to_le_bytes()),
                "linker" => *bytes.last_mut().unwrap() = b'x',
                "missing-linker" => bytes[64..68].copy_from_slice(&1u32.to_le_bytes()),
                "legacy-magic" => bytes.truncate(64),
                _ => unreachable!(),
            }
            std::fs::write(&binary, &bytes).unwrap();
            set_mode(
                &binary,
                if fault == "mode" { 0o777 } else { 0o755 },
                "set Provider fixture mode",
            )
            .unwrap();
            let inputs = root.join("source-inputs.json");
            std::fs::write(&inputs, b"{\"fixture\":true}\n").unwrap();
            let private = root.join("private.pem");
            assert!(Command::new(&openssl)
                .args(["genpkey", "-algorithm", "Ed25519", "-out"])
                .arg(&private)
                .output()
                .unwrap()
                .status
                .success());
            let output = root.join("package");
            let mut args = vec![OsString::from("provider-package")];
            for (flag, value) in [
                ("--binary", binary.as_os_str().to_owned()),
                ("--assets", assets.into_os_string()),
                ("--source-inputs", inputs.into_os_string()),
                ("--source-sha", "a".repeat(40).into()),
                ("--generation-id", "provider-chatgpt-v0.0.0-fixture".into()),
                ("--release-sequence", "1".into()),
                ("--private-key", private.into_os_string()),
                ("--openssl", openssl.as_os_str().to_owned()),
                ("--output", output.as_os_str().to_owned()),
            ] {
                args.extend([flag.into(), value]);
            }
            let result = package(&args);
            assert_eq!(result.is_ok(), fault == "none", "{fault}: {result:?}");
            assert_eq!(output.exists(), fault == "none");
            assert_eq!(std::fs::read(&binary).unwrap(), bytes);
            assert!(!std::fs::read_dir(&root).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".provider-stage-")));
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn new_generation_names_preserve_legacy_manifest_and_index_identity() {
        for generation in [
            "provider-chatgpt-v0.1.0-54767f7-s27",
            "provider-gpt-v0.1.0-54767f7-s27",
            "chatgpt-test-1",
        ] {
            let mut payload = manifest();
            payload.generation = generation.into();
            let bytes = payload.render().unwrap();
            assert_eq!(Manifest::parse(&bytes).unwrap().generation, generation);
            let index = Index {
                generation: generation.into(),
                sequence: 27,
                manifest_digest: "a".repeat(64),
            };
            let bytes = index.render().unwrap();
            assert_eq!(Index::parse(&bytes).unwrap().render().unwrap(), bytes);
            assert_eq!(
                valid_new_id(generation),
                generation.starts_with("provider-chatgpt-")
            );
        }
        for invalid in [
            "provider-chatgpt-v",
            "provider-chatgpt-version",
            "provider-chatgpt-v1/escape",
            "provider-chatgpt-v1.",
        ] {
            assert!(!valid_new_id(invalid));
        }
    }

    #[test]
    fn producer_rejects_historical_ids_before_accessing_inputs() {
        for generation in ["provider-gpt-v0.1.0-54767f7-s27", "chatgpt-test-1"] {
            let args: Vec<OsString> = [
                "provider-package",
                "--binary",
                "/nonexistent/provider",
                "--assets",
                "/nonexistent/assets",
                "--source-inputs",
                "/nonexistent/source-inputs",
                "--source-sha",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "--generation-id",
                generation,
                "--release-sequence",
                "1",
                "--private-key",
                "/nonexistent/private-key",
                "--openssl",
                "/nonexistent/openssl",
                "--output",
                "/nonexistent/output",
            ]
            .into_iter()
            .map(OsString::from)
            .collect();
            assert!(matches!(
                package(&args),
                Err(BuilderError::Invalid(
                    "new Provider generation must use provider-chatgpt-v..."
                ))
            ));
        }
    }

    fn manifest() -> Manifest {
        Manifest {
            generation: "chatgpt-test-1".into(),
            sequence: 1,
            source: "a".repeat(40),
            source_inputs: "b".repeat(64),
            files: FILES
                .into_iter()
                .map(|name| Entry {
                    name: name.into(),
                    digest: "b".repeat(64),
                    bytes: 64,
                    mode: if name == "codex-chatgpt-provider" {
                        0o755
                    } else {
                        0o644
                    },
                })
                .collect(),
        }
    }
    #[test]
    fn stable_index_canonical_binding_and_negative_cases() {
        let index = Index {
            generation: "chatgpt-a".into(),
            sequence: 7,
            manifest_digest: "a".repeat(64),
        };
        let bytes = index.render().unwrap();
        assert_eq!(Index::parse(&bytes).unwrap(), index);
        let text = String::from_utf8(bytes).unwrap();
        for invalid in [
            text.replace("https://github.com/humtr/codex/", "https://other.example/"),
            text.replace("release_sequence\t7", "release_sequence\t07"),
            text.replace("release_sequence\t7", "release_sequence\t0"),
            text.replace("channel\tstable", "channel\tbeta"),
            text.replace("\n", "\r\n"),
            format!("{text}\n"),
            text.trim_end().into(),
            text.replace("chatgpt-a", "../escape"),
        ] {
            assert!(Index::parse(invalid.as_bytes()).is_err());
        }
    }
    #[test]
    fn ordered_manifest_roundtrip_and_rejections() {
        let valid = manifest();
        let bytes = valid.render().unwrap();
        assert_eq!(Manifest::parse(&bytes).unwrap(), valid);
        let mut legacy = valid.clone();
        legacy.files.insert(
            3,
            Entry {
                name: "licenses/bridge-LICENSE".into(),
                digest: "b".repeat(64),
                bytes: 64,
                mode: 0o644,
            },
        );
        legacy.files.insert(
            6,
            Entry {
                name: "licenses/upstream-codex-LICENSE".into(),
                digest: "b".repeat(64),
                bytes: 64,
                mode: 0o644,
            },
        );
        let old_bytes = legacy.render().unwrap();
        assert_eq!(Manifest::parse(&old_bytes).unwrap(), legacy);
        let mut mismatched = old_bytes.clone();
        let marker = b"file_count\t8\n";
        let offset = mismatched
            .windows(marker.len())
            .position(|p| p == marker)
            .unwrap();
        mismatched[offset + marker.len() - 2] = b'6';
        assert!(Manifest::parse(&mismatched).is_err());
        let text = String::from_utf8(bytes).unwrap();
        for bad in [
            text.replace("\n", "\r\n"),
            text.trim_end().into(),
            format!("{text}\n"),
            text.replace("chatgpt-test-1", "../escape"),
            text.replace("release_sequence\t1", "release_sequence\t01"),
            text.replace("release_sequence\t1", "release_sequence\t0"),
            text.replace("aarch64-linux-android", "x86_64-linux-gnu"),
            text.replace(
                "provider-payload-v1\ndependencies",
                "provider-payload-v2\ndependencies",
            ),
            text.replace("\t64\t0755", "\t64\t0777"),
            text.replace("\t64\t0644", "\t134217729\t0644"),
            text.replace("file\tNOTICE.md", "file\tother"),
        ] {
            assert!(Manifest::parse(bad.as_bytes()).is_err(), "{bad}");
        }
        let mut wrong = valid.clone();
        wrong.source_inputs = "c".repeat(64);
        assert!(wrong.render().is_err());
        wrong = valid;
        wrong.files.reverse();
        assert!(wrong.render().is_err());
    }
    #[test]
    fn package_layout_rejects_extras_aliases_and_missing_files() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "provider-layout-{}-{}",
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        create_private_dir(&root).unwrap();
        create_private_dir(&root.join("licenses")).unwrap();
        let manifest = manifest();
        for name in FILES.into_iter().chain(["payload.manifest", "payload.sig"]) {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        exact_layout(&root, &manifest).unwrap();
        std::fs::write(root.join("extra"), b"x").unwrap();
        assert!(exact_layout(&root, &manifest).is_err());
        std::fs::remove_file(root.join("extra")).unwrap();
        std::fs::remove_file(root.join("NOTICE.md")).unwrap();
        assert!(exact_layout(&root, &manifest).is_err());
        symlink(root.join("Cargo.lock"), root.join("NOTICE.md")).unwrap();
        assert!(exact_layout(&root, &manifest).is_err());
        std::fs::remove_file(root.join("NOTICE.md")).unwrap();
        std::fs::write(root.join("NOTICE.md"), b"x").unwrap();
        std::fs::write(root.join("licenses/bridge-LICENSE"), b"x").unwrap();
        assert!(
            exact_layout(&root, &manifest).is_err(),
            "an undeclared legacy license cannot extend a signed current manifest"
        );
        let mut legacy = manifest.clone();
        legacy.files.insert(
            3,
            Entry {
                name: "licenses/bridge-LICENSE".into(),
                digest: "b".repeat(64),
                bytes: 64,
                mode: 0o644,
            },
        );
        std::fs::write(root.join("licenses/upstream-codex-LICENSE"), b"x").unwrap();
        legacy.files.insert(
            6,
            Entry {
                name: "licenses/upstream-codex-LICENSE".into(),
                digest: "a".repeat(64),
                bytes: 1,
                mode: 0o644,
            },
        );
        exact_layout(&root, &legacy).unwrap();
        std::fs::remove_file(root.join("licenses/bridge-LICENSE")).unwrap();
        assert!(
            exact_layout(&root, &legacy).is_err(),
            "a signed eight-file manifest requires all eight files"
        );
        std::fs::remove_file(root.join("licenses/upstream-codex-LICENSE")).unwrap();
        exact_layout(&root, &manifest).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

//! Optional signed payload selection. No Manager, host or inference side effects.
use super::*;
use codex_release_builder::provider::{Index, Manifest, FILES, PROBE};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const USAGE: &str = "usage: codex provider install chatgpt [--local ABSOLUTE_DIRECTORY|--remote HTTPS_DIRECTORY_URL] | update chatgpt | status chatgpt | rollback chatgpt | chatgpt ARGS...";

fn owned_directory(path: &Path, create: bool) -> Result<()> {
    // Reject aliases even in parents; never chmod an existing user directory.
    if let Some(parent) = path.parent() {
        if parent != path {
            owned_directory(parent, create)?;
        }
    }
    let m = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if create && e.kind() == std::io::ErrorKind::NotFound => {
            fs::DirBuilder::new().mode(0o700).create(path)?;
            fs::symlink_metadata(path)?
        }
        Err(e) => return Err(e.into()),
    };
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err("Provider directory contains an alias/non-directory".into());
    }
    Ok(())
}
fn private_directory(path: &Path, create: bool) -> Result<()> {
    owned_directory(path, create)?;
    let m = fs::metadata(path)?;
    if m.uid() != fs::metadata("/proc/self")?.uid() || m.mode() & 0o077 != 0 {
        return Err("Provider private directory has unsafe owner/mode".into());
    }
    Ok(())
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.len() > limit {
        return Err("Provider file is not bounded/regular".into());
    }
    let mut data = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 != m.len() {
        return Err("Provider file changed while reading".into());
    }
    Ok(data)
}
fn fresh(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?)
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Selected {
    id: String,
    key: ReleasePublicKey,
    digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    current: Selected,
    previous: Option<Selected>,
    floor: u64,
}
impl State {
    fn encode(&self) -> String {
        let previous = self.previous.as_ref();
        format!("codex-provider-state-v1\ncurrent\t{}\ncurrent_key\t{}\ncurrent_digest\t{}\nprevious\t{}\nprevious_key\t{}\nprevious_digest\t{}\nhigh_water\t{}\n",
            self.current.id,self.current.key.to_hex(),self.current.digest,
            previous.map_or("-",|v|v.id.as_str()),previous.map_or_else(||"-".into(),|v|v.key.to_hex()),
            previous.map_or("-",|v|v.digest.as_str()),self.floor)
    }
    fn parse(bytes: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(bytes)?;
        let rows: Vec<_> = text.lines().collect();
        if rows.len() != 8 || rows[0] != "codex-provider-state-v1" {
            return Err("invalid Provider selection state".into());
        }
        let field = |i: usize, name: &str| -> Result<&str> {
            rows[i]
                .strip_prefix(name)
                .ok_or_else(|| "invalid Provider state field".into())
        };
        let selected = |id: &str, key: &str, digest: &str| -> Result<Selected> {
            if !codex_release_builder::provider::valid_id(id)
                || ReleasePublicKey::parse_hex(digest).is_none()
            {
                return Err("invalid Provider state identity/digest".into());
            }
            Ok(Selected {
                id: id.into(),
                key: ReleasePublicKey::parse_hex(key).ok_or("invalid Provider admitted key")?,
                digest: digest.into(),
            })
        };
        let current = selected(
            field(1, "current\t")?,
            field(2, "current_key\t")?,
            field(3, "current_digest\t")?,
        )?;
        let prior = [
            field(4, "previous\t")?,
            field(5, "previous_key\t")?,
            field(6, "previous_digest\t")?,
        ];
        let previous = if prior == ["-", "-", "-"] {
            None
        } else {
            Some(selected(prior[0], prior[1], prior[2])?)
        };
        let floor = field(7, "high_water\t")?.parse::<u64>()?;
        let state = Self {
            current,
            previous,
            floor,
        };
        if floor == 0
            || state
                .previous
                .as_ref()
                .is_some_and(|v| v.id == state.current.id)
            || state.encode() != text
        {
            return Err("noncanonical Provider selection state".into());
        }
        Ok(state)
    }
}
struct Store {
    root: PathBuf,
    generations: PathBuf,
}
impl Store {
    fn new(roots: &LocalCoreRoots) -> Result<Self> {
        let root = roots.state_root.join("providers/chatgpt");
        let generations = roots
            .generation_root
            .parent()
            .and_then(Path::parent)
            .ok_or("Core payload root missing")?
            .join("providers/chatgpt/generations");
        Ok(Self { root, generations })
    }
    fn state(&self) -> Result<Option<State>> {
        match fs::symlink_metadata(self.root.join("selection")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
            Ok(_) => Ok(Some(State::parse(&read(
                &self.root.join("selection"),
                4096,
            )?)?)),
        }
    }
    fn lease(&self, exclusive: bool) -> Result<File> {
        let file = File::open(&self.root)?;
        if unsafe { flock(file.as_raw_fd(), if exclusive { 2 | 4 } else { 1 | 4 }) } != 0 {
            return Err("Provider is busy; selection unchanged".into());
        }
        Ok(file)
    }
    fn commit(&self, state: &State) -> Result<()> {
        let temp = self.root.join(format!(".selection-{}", std::process::id()));
        let mut file = fresh(&temp)?;
        let result = (|| {
            file.write_all(state.encode().as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, self.root.join("selection"))?;
            File::open(&self.root)?.sync_all()?;
            Ok(())
        })();
        // Creation succeeded above, so this invocation owns the temporary.
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}
fn verify(roots: &LocalCoreRoots, path: &Path, key: ReleasePublicKey) -> Result<Manifest> {
    let manifest = Manifest::parse(&read(&path.join("payload.manifest"), 8192)?)?;
    codex_release_builder::provider::exact_layout(path, &manifest)?;
    for name in ["payload.manifest", "payload.sig"] {
        if fs::symlink_metadata(path.join(name))?.mode() & 0o7777 != 0o644 {
            return Err("Provider envelope mode mismatch".into());
        }
    }
    if read(&path.join("payload.sig"), 64)?.len() != 64 {
        return Err("Provider signature length invalid".into());
    }
    verify_release_signature_with_key(
        &roots.openssl,
        key,
        &path.join("payload.manifest"),
        &path.join("payload.sig"),
    )?;
    for entry in &manifest.files {
        let file = path.join(&entry.name);
        let m = fs::symlink_metadata(&file)?;
        if !m.is_file()
            || m.len() != entry.bytes
            || m.mode() & 0o7777 != entry.mode
            || openssl_sha256(&roots.openssl, &file)? != entry.digest
        {
            return Err("Provider inventory hash/size/mode mismatch".into());
        }
    }
    let mut header = [0u8; 64];
    File::open(path.join("codex-chatgpt-provider"))?.read_exact(&mut header)?;
    if header[..6] != *b"\x7fELF\x02\x01" || header[18..20] != [0xb7, 0] {
        return Err("Android/AArch64 Provider ELF required".into());
    }
    Ok(manifest)
}
fn selected(
    roots: &LocalCoreRoots,
    store: &Store,
    value: &Selected,
    floor: u64,
) -> Result<Manifest> {
    let path = store.generations.join(&value.id);
    private_directory(&path, false)?;
    let manifest = verify(roots, &path, value.key)?;
    if manifest.generation != value.id
        || manifest.sequence > floor
        || openssl_sha256(&roots.openssl, &path.join("payload.manifest"))? != value.digest
    {
        return Err("Provider selection binding mismatch".into());
    }
    Ok(manifest)
}
fn no_processes(root: &Path) -> Result<()> {
    let uid = fs::metadata("/proc/self")?.uid();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if !entry
            .file_name()
            .as_encoded_bytes()
            .iter()
            .all(u8::is_ascii_digit)
        {
            continue;
        }
        let m = match fs::metadata(entry.path()) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        if m.uid() != uid {
            continue;
        }
        match fs::read_link(entry.path().join("exe")) {
            Ok(exe) if exe.starts_with(root) => {
                return Err("Provider process is running; selection unchanged".into())
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
fn probe(path: &Path, store: &Store) -> Result<()> {
    let output = store.root.join(format!(".probe-{}", std::process::id()));
    let file = fresh(&output)?;
    let result = (|| {
        let mut child = std::process::Command::new(path)
            .arg("--artifact-probe")
            .env_clear()
            .stdin(std::process::Stdio::null())
            .stdout(file)
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let begin = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err("Provider artifact probe rejected".into());
                }
                break;
            }
            if begin.elapsed() > Duration::from_secs(5) || fs::metadata(&output)?.len() > 1024 {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Provider artifact probe exceeded bound".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if read(&output, 1024)? != PROBE {
            return Err("Provider artifact identity mismatch".into());
        }
        Ok(())
    })();
    let _ = fs::remove_file(output);
    result
}
fn install(
    roots: &LocalCoreRoots,
    store: &Store,
    source: &Path,
    key: ReleasePublicKey,
    state: Option<State>,
) -> Result<()> {
    if !source.is_absolute() {
        return Err("absolute Provider package path required".into());
    }
    owned_directory(source, false)?;
    let manifest = verify(roots, source, key)?;
    let digest = openssl_sha256(&roots.openssl, &source.join("payload.manifest"))?;
    let value = Selected {
        id: manifest.generation.clone(),
        key,
        digest,
    };
    if let Some(before) = state.as_ref() {
        selected(roots, store, &before.current, before.floor)?;
        if before.current == value {
            println!("Provider chatgpt unchanged: {} (selection only)", value.id);
            return Ok(());
        }
    }
    let destination = store.generations.join(&value.id);
    let exists = fs::symlink_metadata(&destination).is_ok();
    if let Some(before) = state.as_ref() {
        if manifest.sequence < before.floor
            || (manifest.sequence == before.floor
                && (!exists || before.previous.as_ref() != Some(&value)))
        {
            return Err("Provider sequence rollback/reuse rejected".into());
        }
    }
    no_processes(&store.generations)?;
    private_directory(&store.generations, true)?;
    if exists {
        private_directory(&destination, false)?;
        let old = verify(roots, &destination, key)?;
        if old != manifest
            || openssl_sha256(&roots.openssl, &destination.join("payload.manifest"))?
                != value.digest
        {
            return Err("Provider generation identity collision".into());
        }
    } else {
        let staging = store
            .generations
            .join(format!(".stage-{}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&staging)?;
        let result = (|| -> Result<()> {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(staging.join("licenses"))?;
            for name in FILES.into_iter().chain(["payload.manifest", "payload.sig"]) {
                let input = source.join(name);
                let m = fs::symlink_metadata(&input)?;
                let bound = if name == "payload.manifest" {
                    8192
                } else if name == "payload.sig" {
                    64
                } else {
                    manifest
                        .files
                        .iter()
                        .find(|v| v.name == name)
                        .ok_or("Provider file missing")?
                        .bytes
                };
                if !m.is_file() || m.len() != bound && name != "payload.manifest" || m.len() > bound
                {
                    return Err("Provider staging input changed".into());
                }
                let mut file = fresh(&staging.join(name))?;
                let count = std::io::copy(&mut File::open(input)?.take(bound + 1), &mut file)?;
                if count != m.len() {
                    return Err("Provider staging input changed".into());
                }
                let mode = if name == "codex-chatgpt-provider" {
                    0o755
                } else {
                    0o644
                };
                file.set_permissions(fs::Permissions::from_mode(mode))?;
                file.sync_all()?;
            }
            if verify(roots, &staging, key)? != manifest {
                return Err("Provider staged identity changed".into());
            }
            probe(&staging.join("codex-chatgpt-provider"), store)?;
            File::open(staging.join("licenses"))?.sync_all()?;
            File::open(&staging)?.sync_all()?;
            rename_noreplace(&staging, &destination)?;
            File::open(&store.generations)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result?;
    }
    probe(&destination.join("codex-chatgpt-provider"), store)?;
    let floor = manifest.sequence;
    store.commit(&State {
        current: value,
        previous: state.map(|v| v.current),
        floor,
    })?;
    println!(
        "Provider chatgpt selected: {} (selection only; run prepare explicitly to update profiles)",
        manifest.generation
    );
    Ok(())
}
fn install_remote(
    roots: &LocalCoreRoots,
    store: &Store,
    url: &OsStr,
    key: ReleasePublicKey,
    state: Option<State>,
    expected: Option<&Index>,
) -> Result<()> {
    let base = RemoteReleaseBase::parse(url)?;
    let acquisition = store.root.join(format!(".download-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&acquisition)?;
    let result = (|| -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(240);
        let mut total = 0u64;
        let mut fetch = |name: &str, bound: u64, control: bool| -> Result<()> {
            let remaining = deadline.saturating_duration_since(Instant::now()).as_secs();
            if remaining == 0 {
                return Err("Provider acquisition deadline exceeded".into());
            }
            let timeout = remaining.min(if control { 20 } else { 120 }).to_string();
            let output = fresh(&acquisition.join(name))?;
            let bytes = fetch_remote_file_with_timeout(
                roots,
                &format!("{}{}", base.value, name.replace('/', "--")),
                &output,
                bound,
                &timeout,
            )?;
            total = total
                .checked_add(bytes)
                .filter(|n| *n <= 136 * 1024 * 1024)
                .ok_or("Provider acquisition aggregate bound exceeded")?;
            output.sync_all()?;
            fs::set_permissions(acquisition.join(name), fs::Permissions::from_mode(0o644))?;
            Ok(())
        };
        fetch("payload.manifest", 8192, true)?;
        fetch("payload.sig", 64, true)?;
        let manifest = Manifest::parse(&read(&acquisition.join("payload.manifest"), 8192)?)?;
        if read(&acquisition.join("payload.sig"), 64)?.len() != 64 {
            return Err("Provider signature length invalid".into());
        }
        verify_release_signature_with_key(
            &roots.openssl,
            key,
            &acquisition.join("payload.manifest"),
            &acquisition.join("payload.sig"),
        )?;
        if let Some(index) = expected {
            if index.generation != manifest.generation
                || index.sequence != manifest.sequence
                || index.manifest_digest
                    != openssl_sha256(&roots.openssl, &acquisition.join("payload.manifest"))?
            {
                return Err("Provider stable index/package binding mismatch".into());
            }
        }
        if state.as_ref().is_some_and(|v| manifest.sequence < v.floor) {
            return Err("Provider sequence rollback rejected before payload acquisition".into());
        }
        fs::DirBuilder::new()
            .mode(0o700)
            .create(acquisition.join("licenses"))?;
        for file in &manifest.files {
            fetch(&file.name, file.bytes, false)?;
            let path = acquisition.join(&file.name);
            if fs::metadata(&path)?.len() != file.bytes
                || openssl_sha256(&roots.openssl, &path)? != file.digest
            {
                return Err("Provider downloaded file mismatch".into());
            }
            fs::set_permissions(path, fs::Permissions::from_mode(file.mode))?;
        }
        install(roots, store, &acquisition, key, state)
    })();
    // Creation succeeded above; this is solely our acquisition, never user data.
    let cleanup = fs::remove_dir_all(&acquisition);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn install_stable(
    roots: &LocalCoreRoots,
    store: &Store,
    key: ReleasePublicKey,
    state: Option<State>,
) -> Result<()> {
    let acquisition = store.root.join(format!(".index-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&acquisition)?;
    let result = (|| -> Result<()> {
        for (name, bound) in [("provider-index-v1", 4096), ("provider-index-v1.sig", 64)] {
            let output = fresh(&acquisition.join(name))?;
            fetch_remote_file_with_timeout(
                roots,
                &format!("https://raw.githubusercontent.com/humtr/codex/main/{name}"),
                &output,
                bound,
                "20",
            )?;
        }
        if read(&acquisition.join("provider-index-v1.sig"), 64)?.len() != 64 {
            return Err("Provider index signature length invalid".into());
        }
        verify_release_signature_with_key(
            &roots.openssl,
            key,
            &acquisition.join("provider-index-v1"),
            &acquisition.join("provider-index-v1.sig"),
        )?;
        let index = Index::parse(&read(&acquisition.join("provider-index-v1"), 4096)?)?;
        if state.as_ref().is_some_and(|v| index.sequence < v.floor) {
            return Err("Provider stable index sequence rollback rejected".into());
        }
        install_remote(
            roots,
            store,
            OsStr::new(&index.release_base()),
            key,
            state,
            Some(&index),
        )
    })();
    let cleanup = fs::remove_dir_all(&acquisition);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn action(roots: &LocalCoreRoots, args: &[OsString]) -> Result<()> {
    let management = matches!(
        args.first().and_then(|v| v.to_str()),
        Some("install" | "update" | "rollback" | "status")
    );
    let valid = if management {
        match args.first().and_then(|v| v.to_str()) {
            Some("install") if args.len() == 2 => args[1] == "chatgpt",
            Some("install") => {
                args.len() == 4
                    && args[1] == "chatgpt"
                    && (args[2] == "--local" || args[2] == "--remote")
            }
            _ => args.len() == 2 && args[1] == "chatgpt",
        }
    } else {
        args.first().is_some_and(|v| v == "chatgpt")
    };
    if !valid {
        return Err(USAGE.into());
    }
    let paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)?;
    let _core_lock = m2_generation_state::acquire_activation_lock(&paths)?;
    let authority =
        m2_generation_state::read_pointer_state(&paths)?.ok_or("Core is not admitted")?;
    let loaded = load_activated_generation(roots)?;
    verify_active_core_entrypoint_pair_with_state(roots, &loaded, &authority)?;
    let store = Store::new(roots)?;
    let install_requested = args[0] == "install" || args[0] == "update";
    if fs::symlink_metadata(&store.root).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        && !install_requested
    {
        if args[0] == "status" {
            println!("Provider chatgpt not installed");
            return Ok(());
        }
        return Err("Provider chatgpt not installed".into());
    }
    private_directory(&store.root, install_requested)?;
    let exclusive = install_requested || args[0] == "rollback";
    let lease = store.lease(exclusive)?;
    let state = store.state()?;
    match args[0].to_str() {
        Some("update") => install_stable(roots, &store, authority.update_key, state),
        Some("install") if args.len() == 2 => {
            install_stable(roots, &store, authority.update_key, state)
        }
        Some("install") if args[2] == "--remote" => {
            install_remote(roots, &store, &args[3], authority.update_key, state, None)
        }
        Some("install") => install(
            roots,
            &store,
            Path::new(&args[3]),
            authority.update_key,
            state,
        ),
        Some("status") => {
            if let Some(state) = state {
                let manifest = selected(roots, &store, &state.current, state.floor)?;
                println!("Provider chatgpt selected: {}\nrelease_sequence: {}\nhigh_water: {}\nprevious: {}\nprofile_binding: explicit prepare\nreadiness: not queried",state.current.id,manifest.sequence,state.floor,state.previous.map_or_else(||"-".into(),|v|v.id));
            } else {
                println!("Provider chatgpt not installed");
            }
            Ok(())
        }
        Some("rollback") => {
            let state = state.ok_or("Provider chatgpt not installed")?;
            selected(roots, &store, &state.current, state.floor)?;
            let previous = state.previous.ok_or("no previous Provider generation")?;
            selected(roots, &store, &previous, state.floor)?;
            no_processes(&store.generations)?;
            probe(
                &store
                    .generations
                    .join(&previous.id)
                    .join("codex-chatgpt-provider"),
                &store,
            )?;
            let id = previous.id.clone();
            store.commit(&State {
                current: previous,
                previous: Some(state.current),
                floor: state.floor,
            })?;
            println!("Provider chatgpt rollback selection: {id} (profiles unchanged)");
            Ok(())
        }
        _ => {
            let state = state.ok_or("Provider chatgpt not installed")?;
            selected(roots, &store, &state.current, state.floor)?;
            let safe = unsafe { fcntl(lease.as_raw_fd(), F_DUPFD_CLOEXEC, 64) };
            if safe < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let inherited = unsafe { File::from_raw_fd(safe) };
            if unsafe { fcntl(safe, 2, 0) } < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            drop(lease);
            drop(_core_lock);
            let error = std::process::Command::new(
                store
                    .generations
                    .join(&state.current.id)
                    .join("codex-chatgpt-provider"),
            )
            .args(&args[1..])
            .env_remove(CORE_ONLY_ENV)
            .exec();
            drop(inherited);
            Err(error.into())
        }
    }
}
pub(super) fn run(roots: &LocalCoreRoots, args: &[OsString]) -> i32 {
    match action(roots, args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("codex provider: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_selection_binds_keys_digests_and_floor() {
        let state = State {
            current: Selected {
                id: "chatgpt-test-2".into(),
                key: ReleasePublicKey([1; 32]),
                digest: "a".repeat(64),
            },
            previous: Some(Selected {
                id: "chatgpt-test-1".into(),
                key: ReleasePublicKey([2; 32]),
                digest: "b".repeat(64),
            }),
            floor: 2,
        };
        let encoded = state.encode();
        assert_eq!(State::parse(encoded.as_bytes()).unwrap(), state);
        for bad in [
            encoded.replace("\n", "\r\n"),
            format!("{encoded}\n"),
            encoded.trim_end().into(),
            encoded.replace("high_water\t2", "high_water\t02"),
            encoded.replace("high_water\t2", "high_water\t0"),
            encoded.replace("chatgpt-test-1", "chatgpt-test-2"),
            encoded.replace("previous_key\t0202", "previous_key\t-0202"),
            encoded.replace("chatgpt-test-2", "../escape"),
        ] {
            assert!(State::parse(bad.as_bytes()).is_err());
        }
        let no_previous = State {
            previous: None,
            ..state
        };
        assert_eq!(
            State::parse(no_previous.encode().as_bytes()).unwrap(),
            no_previous
        );
    }
    #[test]
    fn provider_dispatch_preserves_opaque_arguments() {
        use std::os::unix::ffi::OsStringExt;
        let opaque = OsString::from_vec(vec![0xff]);
        let args = vec![
            "provider".into(),
            "chatgpt".into(),
            "--home".into(),
            opaque.clone(),
        ];
        assert_eq!(
            plan_public_dispatch(args).unwrap(),
            PublicDispatchRoute::Provider(vec!["chatgpt".into(), "--home".into(), opaque])
        );
        assert!(matches!(
            plan_public_dispatch(["--", "provider"]).unwrap(),
            PublicDispatchRoute::Upstream(_)
        ));
    }
    #[test]
    fn directory_lease_and_private_path_guard() {
        use std::os::unix::fs::symlink;
        let root = super::super::tests::temp_root("provider-private");
        let private = root.join("state");
        private_directory(&private, true).unwrap();
        let store = Store {
            root: private.clone(),
            generations: root.join("payloads"),
        };
        let shared = store.lease(false).unwrap();
        assert!(store.lease(true).is_err());
        let peer = store.lease(false).unwrap();
        drop(shared);
        assert!(store.lease(true).is_err());
        drop(peer);
        let exclusive = store.lease(true).unwrap();
        assert!(store.lease(false).is_err());
        drop(exclusive);
        fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(private_directory(&private, false).is_err());
        let alias = root.join("alias");
        symlink(&private, &alias).unwrap();
        assert!(private_directory(&alias, false).is_err());
        let before = fs::metadata(&private).unwrap().mode();
        assert_eq!(before & 0o777, 0o755);
        fs::remove_dir_all(root).unwrap();
    }
}

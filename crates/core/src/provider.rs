//! Optional signed payload selection. No Manager, host or inference side effects.
use super::*;
use codex_release_builder::provider::{Index, Manifest, PROBE};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const USAGE: &str = "usage: codex provider install chatgpt [--local ABSOLUTE_DIRECTORY|--remote HTTPS_DIRECTORY_URL] | update chatgpt | status chatgpt | rollback chatgpt | chatgpt ARGS...";

// Manual all-component update may advance an existing selection; explicit
// Provider commands additionally own enrollment and direct dispatch.
pub(super) enum Request<'a> {
    Command(&'a [OsString]),
    UpdateSelected,
}

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
    fn lease(&self) -> Result<File> {
        let file = File::open(&self.root)?;
        if unsafe { flock(file.as_raw_fd(), 1 | 4) } != 0 {
            return Err("legacy Provider selection writer is busy; selection unchanged".into());
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
fn verify_index_binding(index: &Index, manifest: &Manifest, digest: &str) -> Result<()> {
    if index.generation != manifest.generation
        || index.sequence != manifest.sequence
        || index.manifest_digest != digest
    {
        return Err("Provider stable index/package binding mismatch".into());
    }
    Ok(())
}

fn install(
    roots: &LocalCoreRoots,
    store: &Store,
    source: &Path,
    key: ReleasePublicKey,
    state: Option<State>,
    expected: Option<&Index>,
) -> Result<()> {
    if !source.is_absolute() {
        return Err("absolute Provider package path required".into());
    }
    owned_directory(source, false)?;
    let manifest = verify(roots, source, key)?;
    let digest = openssl_sha256(&roots.openssl, &source.join("payload.manifest"))?;
    if let Some(index) = expected {
        verify_index_binding(index, &manifest, &digest)?;
        if let Some(details) = release_display::provider_details(
            roots,
            &index.release_base(),
            &manifest.generation,
            manifest.sequence,
        ) {
            println!("{details}");
        }
    }
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
            for (name, bound) in manifest
                .files
                .iter()
                .map(|entry| (entry.name.as_str(), entry.bytes))
                .chain([("payload.manifest", 8192), ("payload.sig", 64)])
            {
                let input = source.join(name);
                let m = fs::symlink_metadata(&input)?;
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
        let mut total = 0u64;
        let mut fetch_control = |name: &str, bound: u64| -> Result<()> {
            let output = fresh(&acquisition.join(name))?;
            let bytes = fetch_remote_file_with_timeout(
                roots,
                &format!("{}{}", base.value, name.replace('/', "--")),
                &output,
                bound,
                "20",
            )?;
            total = total
                .checked_add(bytes)
                .filter(|n| *n <= 136 * 1024 * 1024)
                .ok_or("Provider acquisition aggregate bound exceeded")?;
            output.sync_all()?;
            fs::set_permissions(acquisition.join(name), fs::Permissions::from_mode(0o644))?;
            Ok(())
        };
        fetch_control("payload.manifest", 8192)?;
        fetch_control("payload.sig", 64)?;
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
            verify_index_binding(
                index,
                &manifest,
                &openssl_sha256(&roots.openssl, &acquisition.join("payload.manifest"))?,
            )?;
        }
        if state.as_ref().is_some_and(|v| manifest.sequence < v.floor) {
            return Err("Provider sequence rollback rejected before payload acquisition".into());
        }
        #[cfg(all(
            any(target_os = "android", target_os = "linux"),
            any(target_arch = "aarch64", target_arch = "x86_64")
        ))]
        let retained = match (expected, state.as_ref()) {
            (Some(_), Some(current)) => Some((
                store.generations.join(&current.current.id),
                selected(roots, store, &current.current, current.floor)?,
            )),
            _ => None,
        };
        if expected.is_none() {
            if let Some(details) = release_display::provider_details(
                roots,
                &base.value,
                &manifest.generation,
                manifest.sequence,
            ) {
                println!("{details}");
            }
        }
        fs::DirBuilder::new()
            .mode(0o700)
            .create(acquisition.join("licenses"))?;
        let mut presentation = UpdatePresentation::new();
        for file in &manifest.files {
            #[cfg(all(
                any(target_os = "android", target_os = "linux"),
                any(target_arch = "aarch64", target_arch = "x86_64")
            ))]
            if let Some((path, _)) = retained
                .as_ref()
                .filter(|(_, current)| current.files.contains(file))
            {
                // Adapt the Provider's signed inventory to Core's existing
                // target-bound copier; no second filesystem reuse policy.
                let entry = ReleaseFileEntry {
                    relative_path: file.name.clone(),
                    sha256: file.digest.clone(),
                    mode: file.mode,
                };
                if reuse_remote_release_file(roots, &acquisition, path, &entry, file.bytes)?
                    .is_some()
                {
                    continue;
                }
            }
            let path = acquisition.join(&file.name);
            let entry = ReleaseFileEntry {
                relative_path: file.name.clone(),
                sha256: file.digest.clone(),
                mode: file.mode,
            };
            let label = format!("Downloading Provider {}...", file.name);
            let mut progress = |bytes| {
                presentation.transient(
                    "⠙",
                    &render_update_download_progress(&label, bytes, file.bytes),
                );
            };
            let bytes = remote_download::payload(
                roots,
                &format!("{}{}", base.value, file.name.replace('/', "--")),
                &path,
                &entry,
                remote_download::PayloadOptions {
                    limit: file.bytes,
                    expected_size: Some(file.bytes),
                    timeout: "120",
                    progress: Some(&mut progress),
                },
            )?;
            total = total
                .checked_add(bytes)
                .filter(|n| *n <= 136 * 1024 * 1024)
                .ok_or("Provider acquisition aggregate bound exceeded")?;
            fs::set_permissions(path, fs::Permissions::from_mode(file.mode))?;
        }
        presentation.clear_transient();
        install(roots, store, &acquisition, key, state, expected)
    })();
    // Creation succeeded above; this is solely our acquisition, never user data.
    let cleanup = fs::remove_dir_all(&acquisition);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn with_stable_index<T>(
    roots: &LocalCoreRoots,
    store: &Store,
    key: ReleasePublicKey,
    floor: Option<u64>,
    timeout_seconds: &str,
    consume: impl FnOnce(Index) -> Result<T>,
) -> Result<T> {
    let acquisition = store.root.join(format!(".index-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&acquisition)?;
    let result = (|| -> Result<T> {
        for (name, bound) in [("provider-index-v1", 4096), ("provider-index-v1.sig", 64)] {
            let output = fresh(&acquisition.join(name))?;
            fetch_remote_file_with_timeout(
                roots,
                &format!("https://raw.githubusercontent.com/humtr/codex/main/{name}"),
                &output,
                bound,
                timeout_seconds,
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
        if floor.is_some_and(|floor| index.sequence < floor) {
            return Err("Provider stable index sequence rollback rejected".into());
        }
        consume(index)
    })();
    let cleanup = fs::remove_dir_all(&acquisition);
    match (result, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(value), Ok(())) => Ok(value),
    }
}

fn install_stable(
    roots: &LocalCoreRoots,
    store: &Store,
    key: ReleasePublicKey,
    state: Option<State>,
    show_current_details: bool,
) -> Result<()> {
    with_stable_index(
        roots,
        store,
        key,
        state.as_ref().map(|v| v.floor),
        "20",
        |index| {
            if let Some(current) = state.as_ref().filter(|current| {
                current.current.id == index.generation
                    && current.current.digest == index.manifest_digest
            }) {
                let installed = selected(roots, store, &current.current, current.floor)?;
                verify_index_binding(&index, &installed, &current.current.digest)?;
                if show_current_details {
                    if let Some(details) = release_display::provider_details(
                        roots,
                        &index.release_base(),
                        &index.generation,
                        index.sequence,
                    ) {
                        println!("{details}");
                    }
                }
                println!(
                    "Provider chatgpt unchanged: {} (selection only)",
                    installed.generation
                );
                return Ok(());
            }
            let cached = store.generations.join(&index.generation);
            match fs::symlink_metadata(&cached) {
                Ok(_) => return install(roots, store, &cached, key, state, Some(&index)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
            install_remote(
                roots,
                store,
                OsStr::new(&index.release_base()),
                key,
                state,
                Some(&index),
            )
        },
    )
}

pub(super) fn execute(roots: &LocalCoreRoots, request: Request<'_>) -> Result<()> {
    let update_args = [OsString::from("update"), OsString::from("chatgpt")];
    let (args, update_selected) = match request {
        Request::Command(args) => (args, false),
        Request::UpdateSelected => (update_args.as_slice(), true),
    };
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
    let store = Store::new(roots)?;
    if update_selected
        && fs::symlink_metadata(store.root.join("selection"))
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    let paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)?;
    let _core_lock = m2_generation_state::acquire_activation_lock(&paths)?;
    let authority =
        m2_generation_state::read_pointer_state(&paths)?.ok_or("Core is not admitted")?;
    let (_, loaded) = verify_installed_local_release(
        roots,
        &authority.current,
        authority.current_key,
        "Provider authority generation does not match current Core",
    )?;
    verify_active_core_entrypoint_pair_with_state(roots, &loaded, &authority)?;
    let install_requested = args[0] == "install" || args[0] == "update";
    if fs::symlink_metadata(&store.root).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        && (!install_requested || update_selected)
    {
        if update_selected {
            return Ok(());
        }
        if args[0] == "status" {
            println!("Provider chatgpt not installed");
            return Ok(());
        }
        return Err("Provider chatgpt not installed".into());
    }
    private_directory(&store.root, install_requested && !update_selected)?;
    // Core's activation lock serializes selection writes. Shared store leases
    // coexist with old running payloads and still exclude legacy Core writers.
    let lease = store.lease()?;
    let state = store.state()?;
    if update_selected && state.is_none() {
        return Ok(());
    }
    match args[0].to_str() {
        Some("update") => {
            install_stable(roots, &store, authority.update_key, state, update_selected)
        }
        Some("install") if args.len() == 2 => {
            install_stable(roots, &store, authority.update_key, state, false)
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
            None,
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
pub(super) fn run(roots: &LocalCoreRoots, request: Request<'_>) -> i32 {
    match execute(roots, request) {
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
        let shared = store.lease().unwrap();
        let peer = store.lease().unwrap();
        let legacy_writer = File::open(&private).unwrap();
        assert_ne!(unsafe { flock(legacy_writer.as_raw_fd(), 2 | 4) }, 0);
        drop(shared);
        assert_ne!(unsafe { flock(legacy_writer.as_raw_fd(), 2 | 4) }, 0);
        drop(peer);
        assert_eq!(unsafe { flock(legacy_writer.as_raw_fd(), 2 | 4) }, 0);
        assert!(store.lease().is_err());
        drop(legacy_writer);
        fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(private_directory(&private, false).is_err());
        let alias = root.join("alias");
        symlink(&private, &alias).unwrap();
        assert!(private_directory(&alias, false).is_err());
        let before = fs::metadata(&private).unwrap().mode();
        assert_eq!(before & 0o777, 0o755);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stable_update_verifies_selection_and_reuses_only_matching_payloads() {
        use codex_release_builder::provider::{Entry, FILES};
        // This is a signed, pre-admitted selection fixture. The actual index
        // and installed-envelope verifiers run. A changed package must reach
        // the real kernel probe, which rejects this deliberately incomplete ELF.
        for defect in [
            "none",
            "index-signature",
            "sequence",
            "local-bytes",
            "new-generation",
            "manifest-digest",
            "changed-file",
            "cached-target",
        ] {
            let root = super::super::tests::temp_root("provider-unchanged");
            let prefix = std::env::var_os("PREFIX")
                .map(PathBuf::from)
                .unwrap_or_else(|| "/usr".into());
            let roots = LocalCoreRoots {
                generation_root: root.join("core/generations"),
                state_root: root.join("state"),
                config_dir: root.join("config"),
                resolver_path: root.join("unused-resolver"),
                cert_file: root.join("unused-ca"),
                cert_dir: root.join("unused-certs"),
                openssl: prefix.join("bin/openssl"),
                curl: root.join("curl"),
            };
            let store = Store {
                root: root.join("provider-state"),
                generations: root.join("provider-generations"),
            };
            private_directory(&store.root, true).unwrap();
            private_directory(&store.generations, true).unwrap();
            private_directory(&roots.state_root, true).unwrap();
            let generation = "chatgpt-unchanged-fixture";
            let package = store.generations.join(generation);
            private_directory(&package, true).unwrap();
            private_directory(&package.join("licenses"), true).unwrap();
            let key = root.join("key.pem");
            assert!(std::process::Command::new(&roots.openssl)
                .args(["genpkey", "-algorithm", "ED25519", "-out"])
                .arg(&key)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
                .success());
            let authority = release_public_key_from_private_pem(&roots.openssl, &key).unwrap();
            let sign = |input: &Path, signature: &Path| {
                assert!(std::process::Command::new(&roots.openssl)
                    .args(["pkeyutl", "-sign", "-inkey"])
                    .arg(&key)
                    .args(["-rawin", "-in"])
                    .arg(input)
                    .arg("-out")
                    .arg(signature)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .unwrap()
                    .success());
            };
            let mut files = Vec::new();
            for name in FILES {
                let path = package.join(name);
                if name == "codex-chatgpt-provider" {
                    let mut header = vec![0; 64];
                    header[..6].copy_from_slice(b"\x7fELF\x02\x01");
                    header[18] = 0xb7;
                    fs::write(&path, header).unwrap();
                } else {
                    fs::write(&path, b"owned signed fixture input\n").unwrap();
                }
                let mode = if name == "codex-chatgpt-provider" {
                    0o755
                } else {
                    0o644
                };
                fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
                files.push(Entry {
                    name: name.into(),
                    digest: openssl_sha256(&roots.openssl, &path).unwrap(),
                    bytes: fs::metadata(&path).unwrap().len(),
                    mode,
                });
            }
            let manifest = Manifest {
                generation: generation.into(),
                sequence: 7,
                source: "a".repeat(40),
                source_inputs: files.last().unwrap().digest.clone(),
                files,
            };
            fs::write(package.join("payload.manifest"), manifest.render().unwrap()).unwrap();
            sign(
                &package.join("payload.manifest"),
                &package.join("payload.sig"),
            );
            for name in ["payload.manifest", "payload.sig"] {
                fs::set_permissions(package.join(name), fs::Permissions::from_mode(0o644)).unwrap();
            }
            let digest = openssl_sha256(&roots.openssl, &package.join("payload.manifest")).unwrap();
            let state = State {
                current: Selected {
                    id: generation.into(),
                    key: authority,
                    digest: digest.clone(),
                },
                previous: None,
                floor: 7,
            };
            store.commit(&state).unwrap();
            let before = read(&store.root.join("selection"), 4096).unwrap();
            let changed = if defect == "cached-target" {
                store.generations.join("chatgpt-changed-fixture")
            } else {
                root.join("changed-package")
            };
            let mut next = manifest.clone();
            if matches!(defect, "changed-file" | "cached-target") {
                private_directory(&changed, true).unwrap();
                if defect == "cached-target" {
                    private_directory(&changed.join("licenses"), true).unwrap();
                    for entry in &manifest.files {
                        fs::copy(package.join(&entry.name), changed.join(&entry.name)).unwrap();
                    }
                }
                next.generation = "chatgpt-changed-fixture".into();
                next.sequence = 8;
                let notice = changed.join("NOTICE.md");
                fs::write(&notice, b"changed signed fixture notice\n").unwrap();
                let entry = next
                    .files
                    .iter_mut()
                    .find(|f| f.name == "NOTICE.md")
                    .unwrap();
                entry.digest = openssl_sha256(&roots.openssl, &notice).unwrap();
                entry.bytes = fs::metadata(&notice).unwrap().len();
                fs::write(changed.join("payload.manifest"), next.render().unwrap()).unwrap();
                sign(
                    &changed.join("payload.manifest"),
                    &changed.join("payload.sig"),
                );
                for name in ["payload.manifest", "payload.sig"] {
                    fs::set_permissions(changed.join(name), fs::Permissions::from_mode(0o644))
                        .unwrap();
                }
            }
            let index = Index {
                generation: if defect == "new-generation" {
                    "chatgpt-new-fixture".into()
                } else {
                    next.generation.clone()
                },
                sequence: if defect == "sequence" {
                    8
                } else {
                    next.sequence
                },
                manifest_digest: if defect == "manifest-digest" {
                    "b".repeat(64)
                } else if matches!(defect, "changed-file" | "cached-target") {
                    openssl_sha256(&roots.openssl, &changed.join("payload.manifest")).unwrap()
                } else {
                    digest
                },
            };
            let index_file = root.join("index");
            let index_sig = root.join("index.sig");
            fs::write(&index_file, index.render().unwrap()).unwrap();
            sign(&index_file, &index_sig);
            if defect == "index-signature" {
                fs::write(&index_sig, [0; 64]).unwrap();
            }
            if defect == "local-bytes" {
                fs::write(package.join("NOTICE.md"), b"changed local input").unwrap();
            }
            let log = root.join("requests");
            let quote = |p: &Path| format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"));
            let package_routes = ["payload.manifest", "payload.sig", "NOTICE.md"]
                .into_iter()
                .map(|name| {
                    format!(
                        "*/releases/download/chatgpt-changed-fixture/{name}) cat {} ;;\n",
                        quote(&changed.join(name))
                    )
                })
                .collect::<String>();
            fs::write(&roots.curl,format!("#!{}\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --url ]; then shift; url=$1; fi; shift; done\nprintf '%s\\n' \"$url\" >> {}\ncase \"$url\" in\n*/provider-index-v1) cat {} ;;\n*/provider-index-v1.sig) cat {} ;;\n{package_routes}*) exit 22 ;;\nesac\n",prefix.join("bin/sh").display(),quote(&log),quote(&index_file),quote(&index_sig))).unwrap();
            fs::set_permissions(&roots.curl, fs::Permissions::from_mode(0o755)).unwrap();
            let result = install_stable(&roots, &store, authority, Some(state), false);
            assert_eq!(result.is_ok(), defect == "none", "{defect}: {result:?}");
            if matches!(defect, "changed-file" | "cached-target") {
                let rejected = result.unwrap_err();
                // Hosts may reject exec with ENOEXEC or start a child that exits
                // unsuccessfully. Both are the artifact-probe boundary;
                // acquisition, signature or inventory failures are not accepted.
                assert!(
                    rejected.to_string() == "Provider artifact probe rejected"
                        || rejected
                            .downcast_ref::<std::io::Error>()
                            .and_then(std::io::Error::raw_os_error)
                            == Some(8),
                    "changed input must reach the kernel ELF probe: {rejected}"
                );
            }
            let requests = fs::read_to_string(log).unwrap();
            assert_eq!(
                requests
                    .lines()
                    .any(|url| url.contains("/releases/download/")),
                matches!(defect, "new-generation" | "changed-file"),
                "{defect}: {requests}"
            );
            if defect == "changed-file" {
                let payload: Vec<_> = requests
                    .lines()
                    .filter(|url| url.contains("/releases/download/"))
                    .collect();
                assert_eq!(payload.len(), 3, "{requests}");
                assert!(
                    payload
                        .iter()
                        .all(|url| ["/payload.manifest", "/payload.sig", "/NOTICE.md"]
                            .iter()
                            .any(|name| url.ends_with(name))),
                    "{requests}"
                );
            }
            assert_eq!(read(&store.root.join("selection"), 4096).unwrap(), before);
            fs::remove_dir_all(root).unwrap();
        }
    }
}

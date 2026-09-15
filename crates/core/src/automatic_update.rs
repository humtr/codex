use super::*;
use std::ffi::OsStr;
use std::io::{Read as _, Write as _};
use std::os::unix::fs::PermissionsExt as _;

const PAGES_WORKFLOW: &str = "publish-termux-update-pages.yml";
const PAGES_RUN_PREFIX: &str = "Publish Termux update Pages ";
const PUBLICATION_TIMEOUT_SECONDS: u64 = 900;

static AUTO_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn publication_overrides_absent(index_override: bool, version_override: bool) -> bool {
    !index_override && !version_override
}

fn maintainer_publication_prerequisites(
    default_channel: bool,
    signing_key_present: bool,
    signing_key_matches: bool,
    github_cli_present: bool,
    github_authenticated: bool,
) -> bool {
    default_channel
        && signing_key_present
        && signing_key_matches
        && github_cli_present
        && github_authenticated
}

pub(super) fn automatic_publication_is_default_channel() -> bool {
    publication_overrides_absent(
        std::env::var_os(UPDATE_INDEX_URL_ENV).is_some(),
        std::env::var_os(UPDATE_VERSION_ENV).is_some(),
    )
}

fn active_state(
    roots: &LocalCoreRoots,
) -> Result<m2_generation_state::GenerationPointerState, LocalProductError> {
    let paths = m2_generation_state::CoreStatePaths::new(&roots.state_root)
        .map_err(LocalProductError::StateFormat)?;
    m2_generation_state::recover_activation_state(&paths)
        .map_err(LocalProductError::State)?
        .ok_or(LocalProductError::NoCurrentGeneration)
}

pub(super) fn maintainer_publication_enabled(
    roots: &LocalCoreRoots,
) -> Result<bool, LocalProductError> {
    if !automatic_publication_is_default_channel() {
        return Ok(false);
    }
    let home = required_absolute_env_path("HOME")?;
    let key_path = std::env::var_os(UPDATE_PRIVATE_KEY_ENV)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".config/codex/termux/update-private-key.pem"));
    match std::fs::symlink_metadata(&key_path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect automatic update signing key",
                source,
            })
        }
        Ok(_) => {}
    }
    let private_key = configured_update_private_key(&home)?;
    let derived = release_public_key_from_private_pem(&roots.openssl, &private_key)?;
    if derived != active_state(roots)?.update_key {
        return Err(LocalProductError::LocalUpdate(
            "automatic update signing key does not match the active update authority",
        ));
    }
    let Some(gh) = github_cli_path(roots) else {
        return Ok(false);
    };
    Ok(maintainer_publication_prerequisites(
        true,
        true,
        true,
        true,
        github_authenticated(&gh, &home),
    ))
}

fn publication_staging_root(
    roots: &LocalCoreRoots,
) -> Result<std::path::PathBuf, LocalProductError> {
    use std::os::unix::fs::DirBuilderExt as _;
    for _ in 0..32 {
        let sequence = AUTO_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = roots.state_root.join(format!(
            ".auto-publication-{}-{sequence}",
            std::process::id()
        ));
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "create automatic publication staging root",
                    source,
                })
            }
        }
    }
    Err(LocalProductError::LocalUpdate(
        "automatic publication staging namespace is exhausted",
    ))
}

fn copy_asset(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), LocalProductError> {
    let metadata = std::fs::symlink_metadata(source).map_err(|source| LocalProductError::Io {
        operation: "inspect automatic publication asset",
        source,
    })?;
    if !metadata.file_type().is_file() || metadata.len() > REMOTE_RELEASE_FILE_MAX_BYTES {
        return Err(LocalProductError::LocalUpdate(
            "automatic publication asset is outside its type or byte bound",
        ));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LocalProductError::Io {
            operation: "create automatic publication asset parent",
            source,
        })?;
    }
    std::fs::copy(source, destination).map_err(|source| LocalProductError::Io {
        operation: "copy automatic publication asset",
        source,
    })?;
    let mut permissions = std::fs::metadata(destination)
        .map_err(|source| LocalProductError::Io {
            operation: "inspect copied automatic publication asset",
            source,
        })?
        .permissions();
    permissions.set_mode(metadata.permissions().mode() & 0o7777);
    std::fs::set_permissions(destination, permissions).map_err(|source| LocalProductError::Io {
        operation: "set automatic publication asset mode",
        source,
    })?;
    Ok(())
}

fn prepare_staging_assets(
    roots: &LocalCoreRoots,
    publication: &std::path::Path,
    generation_id: &str,
    staging: &std::path::Path,
) -> Result<u64, LocalProductError> {
    let state = active_state(roots)?;
    let release = publication.join("releases").join(generation_id);
    let (manifest, loaded) =
        verify_local_release_bundle_with_key(&release, &roots.openssl, state.update_key)?;
    if loaded.generation_id != generation_id
        || !r10_browser_helper_bridge(&loaded.manifest)?
        || loaded.core_path.is_none()
        || loaded.manager_path.is_none()
    {
        return Err(LocalProductError::LocalUpdate(
            "automatic public release is not an exact R10-readable bridge bundle",
        ));
    }
    let expected = [
        "codex-code-mode-host",
        "core",
        "generation.meta",
        "helpers/0",
        "helpers/1",
        "manager",
        "runtime",
    ];
    if manifest.files.len() != expected.len()
        || !manifest
            .files
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .eq(expected)
    {
        return Err(LocalProductError::LocalUpdate(
            "automatic public release inventory is not the exact bridge inventory",
        ));
    }
    for (source_name, destination_name) in [
        ("codex-code-mode-host", "codex-code-mode-host"),
        ("core", "core"),
        ("generation.meta", "generation.meta"),
        ("helpers/0", "helper-0"),
        ("helpers/1", "helper-1"),
        ("manager", "manager"),
        ("release.manifest", "release.manifest"),
        ("release.sig", "release.sig"),
        ("runtime", "runtime"),
    ] {
        copy_asset(&release.join(source_name), &staging.join(destination_name))?;
    }
    copy_asset(
        &publication.join("update-index-v1"),
        &staging.join("candidate-update-index-v1"),
    )?;
    copy_asset(
        &publication.join("update-index-v1.sig"),
        &staging.join("candidate-update-index-v1.sig"),
    )?;
    copy_asset(
        &bootstrap_public_key_path()?,
        &staging.join("update-public-key.pem"),
    )?;
    verify_release_signature_with_key(
        &roots.openssl,
        state.update_key,
        &staging.join("candidate-update-index-v1"),
        &staging.join("candidate-update-index-v1.sig"),
    )?;
    let index_bytes = read_bounded_regular_file(
        &staging.join("candidate-update-index-v1"),
        UPDATE_INDEX_MAX_BYTES,
        "read automatic candidate index",
        LocalProductError::UpdateIndex("automatic candidate index exceeds its byte bound"),
        LocalProductError::UpdateIndex("automatic candidate index is not a regular file"),
    )?;
    let index = parse_signed_update_index(&index_bytes)?;
    let expected_base = format!("https://humtr.github.io/codex/{generation_id}/");
    if index.generation_id != generation_id || index.release_base.value != expected_base {
        return Err(LocalProductError::UpdateIndex(
            "automatic candidate index does not bind the Pages generation",
        ));
    }
    Ok(manifest.release_sequence)
}

fn finish_github_output_bounded(
    child: &mut std::process::Child,
    operation: &'static str,
    max_bytes: usize,
) -> Result<Vec<u8>, LocalProductError> {
    wait_for_github_child_with_timeout(
        child,
        std::time::Duration::from_secs(PUBLICATION_TIMEOUT_SECONDS),
    )
    .map_err(|_| LocalProductError::LocalUpdate(operation))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(LocalProductError::LocalUpdate(operation))?;
    let mut output = Vec::new();
    stdout
        .take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut output)
        .map_err(|_| LocalProductError::LocalUpdate(operation))?;
    if output.len() > max_bytes {
        return Err(LocalProductError::LocalUpdate(operation));
    }
    Ok(output)
}

fn github_output_bounded(
    mut command: std::process::Command,
    operation: &'static str,
    max_bytes: usize,
) -> Result<Vec<u8>, LocalProductError> {
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| LocalProductError::LocalUpdate(operation))?;
    finish_github_output_bounded(&mut child, operation, max_bytes)
}

fn github_main_head(
    gh: &std::path::Path,
    home: &std::path::Path,
) -> Result<String, LocalProductError> {
    let endpoint = format!("repos/{GITHUB_REPOSITORY}/branches/{GITHUB_BRANCH}");
    let mut command = github_command(gh, home);
    command.args([
        "api",
        "--hostname",
        GITHUB_HOST,
        &endpoint,
        "--jq",
        ".commit.sha",
    ]);
    let output = github_output_bounded(
        command,
        "read GitHub main head failed",
        GITHUB_RESPONSE_MAX_BYTES,
    )?;
    let head = std::str::from_utf8(&output)
        .map_err(|_| LocalProductError::LocalUpdate("GitHub main head is not UTF-8"))?
        .trim()
        .to_owned();
    if !valid_github_content_sha(&head) {
        return Err(LocalProductError::LocalUpdate(
            "GitHub main head is invalid",
        ));
    }
    Ok(head)
}

fn run_github_child(
    mut command: std::process::Command,
    operation: &'static str,
) -> Result<(), LocalProductError> {
    let mut child = command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| LocalProductError::LocalUpdate(operation))?;
    wait_for_github_child_with_timeout(
        &mut child,
        std::time::Duration::from_secs(PUBLICATION_TIMEOUT_SECONDS),
    )
    .map_err(|_| LocalProductError::LocalUpdate(operation))
}

fn create_staging_release(
    gh: &std::path::Path,
    home: &std::path::Path,
    generation_id: &str,
    staging: &std::path::Path,
) -> Result<(), LocalProductError> {
    let title = format!("Codex Termux generation {generation_id}");
    let mut create = github_command(gh, home);
    create.args([
        "release",
        "create",
        generation_id,
        "--repo",
        GITHUB_REPOSITORY,
        "--target",
        GITHUB_BRANCH,
        "--title",
        &title,
        "--notes",
        "Signed Termux-adapted Codex generation staged for verified Pages publication.",
        "--draft",
        "--latest=false",
    ]);
    run_github_child(create, "create GitHub staging release failed")?;

    let mut upload = github_command(gh, home);
    upload
        .args([
            "release",
            "upload",
            generation_id,
            "--repo",
            GITHUB_REPOSITORY,
        ])
        .args([
            staging.join("candidate-update-index-v1"),
            staging.join("candidate-update-index-v1.sig"),
            staging.join("codex-code-mode-host"),
            staging.join("core"),
            staging.join("generation.meta"),
            staging.join("helper-0"),
            staging.join("helper-1"),
            staging.join("manager"),
            staging.join("release.manifest"),
            staging.join("release.sig"),
            staging.join("runtime"),
            staging.join("update-public-key.pem"),
        ]);
    run_github_child(upload, "upload GitHub staging release assets failed")?;

    let mut publish = github_command(gh, home);
    publish.args([
        "release",
        "edit",
        generation_id,
        "--repo",
        GITHUB_REPOSITORY,
        "--draft=false",
        "--latest=false",
    ]);
    run_github_child(publish, "publish GitHub staging release failed")
}

fn dispatch_pages(
    gh: &std::path::Path,
    home: &std::path::Path,
    generation_id: &str,
    release_sequence: u64,
) -> Result<(), LocalProductError> {
    let expected_head = github_main_head(gh, home)?;
    let mut dispatch = github_command(gh, home);
    dispatch.args([
        "workflow",
        "run",
        PAGES_WORKFLOW,
        "--repo",
        GITHUB_REPOSITORY,
        "--ref",
        GITHUB_BRANCH,
        "-f",
        &format!("generation_id={generation_id}"),
        "-f",
        &format!("release_sequence={release_sequence}"),
    ]);
    run_github_child(dispatch, "dispatch Pages workflow failed")?;
    let expected_title = format!("{PAGES_RUN_PREFIX}{generation_id}");
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(PUBLICATION_TIMEOUT_SECONDS);
    let run_id = loop {
        if std::time::Instant::now() >= deadline {
            return Err(LocalProductError::LocalUpdate(
                "Pages workflow run did not appear",
            ));
        }
        let jq = format!(
            ".[] | select(.displayTitle == \"{expected_title}\") | [.databaseId,.headSha] | @tsv"
        );
        let mut list = github_command(gh, home);
        list.args([
            "run",
            "list",
            "--repo",
            GITHUB_REPOSITORY,
            "--workflow",
            PAGES_WORKFLOW,
            "--event",
            "workflow_dispatch",
            "--limit",
            "20",
            "--json",
            "databaseId,displayTitle,headSha",
            "--jq",
            &jq,
        ]);
        let output = github_output_bounded(
            list,
            "list Pages workflow runs failed",
            GITHUB_RESPONSE_MAX_BYTES,
        )?;
        let text = std::str::from_utf8(&output)
            .map_err(|_| LocalProductError::LocalUpdate("Pages workflow run list is not UTF-8"))?;
        let matches: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
        if matches.len() == 1 {
            let mut fields = matches[0].split('\t');
            let id = fields.next().ok_or(LocalProductError::LocalUpdate(
                "Pages workflow run identity is invalid",
            ))?;
            let head = fields.next().ok_or(LocalProductError::LocalUpdate(
                "Pages workflow run identity is invalid",
            ))?;
            if fields.next().is_some()
                || head != expected_head
                || id.is_empty()
                || !id.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(LocalProductError::LocalUpdate(
                    "Pages workflow run identity is invalid",
                ));
            }
            break id.to_owned();
        }
        if matches.len() > 1 {
            return Err(LocalProductError::LocalUpdate(
                "Pages workflow run identity is ambiguous",
            ));
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    };
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(LocalProductError::LocalUpdate("Pages workflow timed out"));
        }
        let mut view = github_command(gh, home);
        view.args([
            "run",
            "view",
            &run_id,
            "--repo",
            GITHUB_REPOSITORY,
            "--json",
            "status,conclusion,headSha",
            "--jq",
            "[.status,.conclusion,.headSha] | @tsv",
        ]);
        let output = github_output_bounded(
            view,
            "inspect Pages workflow run failed",
            GITHUB_RESPONSE_MAX_BYTES,
        )?;
        let text = std::str::from_utf8(&output)
            .map_err(|_| LocalProductError::LocalUpdate("Pages workflow result is not UTF-8"))?
            .trim();
        let mut fields = text.split('\t');
        let status = fields.next().unwrap_or_default();
        let conclusion = fields.next().unwrap_or_default();
        let head = fields.next().unwrap_or_default();
        if fields.next().is_some() || head != expected_head {
            return Err(LocalProductError::LocalUpdate(
                "Pages workflow result identity is invalid",
            ));
        }
        if status == "completed" {
            return (conclusion == "success")
                .then_some(())
                .ok_or(LocalProductError::LocalUpdate("Pages workflow failed"));
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn verify_pages_publication(
    roots: &LocalCoreRoots,
    publication: &std::path::Path,
    generation_id: &str,
) -> Result<String, LocalProductError> {
    let state = active_state(roots)?;
    let base_value = format!("https://humtr.github.io/codex/{generation_id}/");
    let base = RemoteReleaseBase::parse(OsStr::new(&base_value))?;
    let acquisition_root = publication_staging_root(roots)?;
    let result = (|| {
        acquire_remote_release_source(roots, &base, &acquisition_root, state.update_key)?;
        let local_release = publication.join("releases").join(generation_id);
        for name in ["release.manifest", "release.sig"] {
            if std::fs::read(acquisition_root.join(name)).map_err(|source| {
                LocalProductError::Io {
                    operation: "read Pages release control",
                    source,
                }
            })? != std::fs::read(local_release.join(name)).map_err(|source| {
                LocalProductError::Io {
                    operation: "read local release control",
                    source,
                }
            })? {
                return Err(LocalProductError::LocalUpdate(
                    "Pages release control does not byte-match local signed publication",
                ));
            }
        }
        let remote_index = acquisition_root.join("candidate-update-index-v1");
        let remote_sig = acquisition_root.join("candidate-update-index-v1.sig");
        let index_output = create_remote_output(&remote_index)?;
        fetch_remote_file(
            roots,
            &format!("{base_value}update-index-v1"),
            &index_output,
            UPDATE_INDEX_MAX_BYTES as u64,
        )?;
        let sig_output = create_remote_output(&remote_sig)?;
        fetch_remote_file(
            roots,
            &format!("{base_value}update-index-v1.sig"),
            &sig_output,
            LOCAL_RELEASE_SIGNATURE_MAX_BYTES,
        )?;
        verify_release_signature_with_key(
            &roots.openssl,
            state.update_key,
            &remote_index,
            &remote_sig,
        )?;
        if std::fs::read(&remote_index).map_err(|source| LocalProductError::Io {
            operation: "read Pages candidate index",
            source,
        })? != std::fs::read(publication.join("update-index-v1")).map_err(|source| {
            LocalProductError::Io {
                operation: "read local candidate index",
                source,
            }
        })? || std::fs::read(&remote_sig).map_err(|source| LocalProductError::Io {
            operation: "read Pages candidate signature",
            source,
        })? != std::fs::read(publication.join("update-index-v1.sig")).map_err(
            |source| LocalProductError::Io {
                operation: "read local candidate signature",
                source,
            },
        )? {
            return Err(LocalProductError::LocalUpdate(
                "Pages candidate index does not byte-match local signed publication",
            ));
        }
        Ok(format!("{base_value}update-index-v1"))
    })();
    let cleanup =
        std::fs::remove_dir_all(&acquisition_root).map_err(|source| LocalProductError::Io {
            operation: "remove Pages readback staging",
            source,
        });
    match (result, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

fn copy_generation_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
    copied_bytes: &mut u64,
) -> Result<(), LocalProductError> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};

    let metadata = std::fs::symlink_metadata(source).map_err(|source| LocalProductError::Io {
        operation: "inspect smoke generation source",
        source,
    })?;
    if !metadata.file_type().is_dir() {
        return Err(LocalProductError::LocalUpdate(
            "smoke generation source is not a directory",
        ));
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(destination)
        .map_err(|source| LocalProductError::Io {
            operation: "create smoke generation directory",
            source,
        })?;
    for entry in std::fs::read_dir(source).map_err(|source| LocalProductError::Io {
        operation: "read smoke generation source",
        source,
    })? {
        let entry = entry.map_err(|source| LocalProductError::Io {
            operation: "read smoke generation entry",
            source,
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata =
            std::fs::symlink_metadata(&source_path).map_err(|source| LocalProductError::Io {
                operation: "inspect smoke generation entry",
                source,
            })?;
        if metadata.file_type().is_dir() {
            copy_generation_tree(&source_path, &destination_path, copied_bytes)?;
        } else if metadata.file_type().is_file() {
            if metadata.len() > REMOTE_RELEASE_FILE_MAX_BYTES {
                return Err(LocalProductError::LocalUpdate(
                    "smoke generation file exceeds its byte bound",
                ));
            }
            let next_total = copied_bytes
                .checked_add(metadata.len())
                .filter(|total| *total <= REMOTE_RELEASE_TOTAL_MAX_BYTES)
                .ok_or(LocalProductError::LocalUpdate(
                    "smoke generation snapshot exceeds its total byte bound",
                ))?;
            let copied = std::fs::copy(&source_path, &destination_path).map_err(|source| {
                LocalProductError::Io {
                    operation: "copy smoke generation file",
                    source,
                }
            })?;
            if copied != metadata.len() {
                return Err(LocalProductError::LocalUpdate(
                    "smoke generation file copy was incomplete",
                ));
            }
            let mut permissions = std::fs::metadata(&destination_path)
                .map_err(|source| LocalProductError::Io {
                    operation: "inspect copied smoke generation file",
                    source,
                })?
                .permissions();
            permissions.set_mode(metadata.permissions().mode() & 0o7777);
            std::fs::set_permissions(&destination_path, permissions).map_err(|source| {
                LocalProductError::Io {
                    operation: "set copied smoke generation file mode",
                    source,
                }
            })?;
            *copied_bytes = next_total;
        } else {
            return Err(LocalProductError::LocalUpdate(
                "smoke generation contains an unsafe file type",
            ));
        }
    }
    Ok(())
}

fn copy_smoke_material(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), LocalProductError> {
    let metadata = std::fs::metadata(source).map_err(|source| LocalProductError::Io {
        operation: "inspect smoke material",
        source,
    })?;
    if !metadata.is_file() || metadata.len() > REMOTE_RELEASE_FILE_MAX_BYTES {
        return Err(LocalProductError::LocalUpdate(
            "smoke material is not a bounded file",
        ));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LocalProductError::Io {
            operation: "create smoke material parent",
            source,
        })?;
    }
    std::fs::copy(source, destination).map_err(|source| LocalProductError::Io {
        operation: "copy smoke material",
        source,
    })?;
    let mut permissions = std::fs::metadata(destination)
        .map_err(|source| LocalProductError::Io {
            operation: "inspect copied smoke material",
            source,
        })?
        .permissions();
    permissions.set_mode(metadata.permissions().mode() & 0o7777);
    std::fs::set_permissions(destination, permissions).map_err(|source| LocalProductError::Io {
        operation: "set smoke material mode",
        source,
    })?;
    Ok(())
}

fn public_update_smoke(
    roots: &LocalCoreRoots,
    candidate_index_url: &str,
    generation_id: &str,
) -> Result<(), LocalProductError> {
    let state = active_state(roots)?;
    let root = publication_staging_root(roots)?;
    let home = root.join("home");
    let prefix = root.join("prefix");
    let tmp = root.join("tmp");
    let smoke_state = home.join(".local/share/codex/core");
    let smoke_generations = home.join(".local/lib/codex/core/generations");
    let result = (|| {
        std::fs::create_dir_all(smoke_state.join("config")).map_err(|source| {
            LocalProductError::Io {
                operation: "create smoke state",
                source,
            }
        })?;
        std::fs::create_dir_all(&smoke_generations).map_err(|source| LocalProductError::Io {
            operation: "create smoke generation root",
            source,
        })?;
        std::fs::create_dir_all(&tmp).map_err(|source| LocalProductError::Io {
            operation: "create smoke temporary root",
            source,
        })?;
        copy_smoke_material(
            &roots.state_root.join("activation-state"),
            &smoke_state.join("activation-state"),
        )?;
        let config = roots.config_dir.join("config.toml");
        if config.exists() {
            copy_smoke_material(&config, &smoke_state.join("config/config.toml"))?;
        }
        let mut copied_generation_bytes = 0u64;
        copy_generation_tree(
            &roots.generation_root.join(&state.current),
            &smoke_generations.join(&state.current),
            &mut copied_generation_bytes,
        )?;
        if let Some(previous) = state.previous.as_deref() {
            copy_generation_tree(
                &roots.generation_root.join(previous),
                &smoke_generations.join(previous),
                &mut copied_generation_bytes,
            )?;
        }
        let (_, current) = verify_installed_local_release(
            roots,
            &state.current,
            state.current_key,
            "smoke current generation descriptor id does not match current",
        )?;
        let core = current.core_path.ok_or(LocalProductError::LocalUpdate(
            "smoke baseline generation has no coordinated Core",
        ))?;
        copy_smoke_material(&core, &prefix.join("bin/codex"))?;
        copy_smoke_material(&roots.curl, &prefix.join("bin/curl"))?;
        copy_smoke_material(&roots.openssl, &prefix.join("bin/openssl"))?;
        copy_smoke_material(&roots.resolver_path, &prefix.join("etc/resolv.conf"))?;
        copy_smoke_material(&roots.cert_file, &prefix.join("etc/tls/cert.pem"))?;
        std::fs::create_dir_all(prefix.join("etc/tls/certs")).map_err(|source| {
            LocalProductError::Io {
                operation: "create smoke certificate directory",
                source,
            }
        })?;
        copy_smoke_material(
            &bootstrap_public_key_path()?,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        )?;
        let output = std::process::Command::new(prefix.join("bin/codex"))
            .arg("update")
            .env_clear()
            .env("HOME", &home)
            .env("PREFIX", &prefix)
            .env("TMPDIR", &tmp)
            .env(
                "PATH",
                format!("{}:/system/bin:/system/xbin", prefix.join("bin").display()),
            )
            .env(
                "ANDROID_DATA",
                std::env::var_os("ANDROID_DATA").unwrap_or_else(|| "/data".into()),
            )
            .env(
                "ANDROID_ROOT",
                std::env::var_os("ANDROID_ROOT").unwrap_or_else(|| "/system".into()),
            )
            .env(
                "TERMUX_VERSION",
                std::env::var_os("TERMUX_VERSION").unwrap_or_else(|| "0.118.0".into()),
            )
            .env("SSL_CERT_FILE", prefix.join("etc/tls/cert.pem"))
            .env("CODEX_TERMUX_UPDATE_INDEX_URL", candidate_index_url)
            .env(
                "OPENAI_API_KEY",
                "sk-disposable-automatic-publication-smoke",
            )
            .output()
            .map_err(|source| LocalProductError::Io {
                operation: "run disposable public update smoke",
                source,
            })?;
        if !output.status.success() {
            return Err(LocalProductError::LocalUpdate(
                "disposable public no-argument update smoke failed",
            ));
        }
        let smoke_paths = m2_generation_state::CoreStatePaths::new(&smoke_state)
            .map_err(LocalProductError::StateFormat)?;
        let after = m2_generation_state::recover_activation_state(&smoke_paths)
            .map_err(LocalProductError::State)?
            .ok_or(LocalProductError::NoCurrentGeneration)?;
        if after.current != generation_id {
            return Err(LocalProductError::LocalUpdate(
                "disposable public update smoke activated the wrong generation",
            ));
        }
        Ok(())
    })();
    let cleanup = std::fs::remove_dir_all(&root).map_err(|source| LocalProductError::Io {
        operation: "remove disposable public update smoke",
        source,
    });
    match (result, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn github_raw_file_at_head(
    gh: &std::path::Path,
    home: &std::path::Path,
    head: &str,
    path: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, LocalProductError> {
    let endpoint = format!("repos/{GITHUB_REPOSITORY}/contents/{path}?ref={head}");
    let mut command = github_command(gh, home);
    command.args([
        "api",
        "--hostname",
        GITHUB_HOST,
        "-H",
        "Accept: application/vnd.github.raw+json",
        &endpoint,
    ]);
    github_output_bounded(command, "read GitHub content failed", max_bytes)
}

fn parse_and_verify_stable_at_head(
    roots: &LocalCoreRoots,
    gh: &std::path::Path,
    home: &std::path::Path,
    head: &str,
    expected_generation: &str,
) -> Result<(), LocalProductError> {
    let root = publication_staging_root(roots)?;
    let result = (|| {
        let index =
            github_raw_file_at_head(gh, home, head, "update-index-v1", UPDATE_INDEX_MAX_BYTES)?;
        let sig = github_raw_file_at_head(
            gh,
            home,
            head,
            "update-index-v1.sig",
            LOCAL_RELEASE_SIGNATURE_MAX_BYTES as usize,
        )?;
        let index_path = root.join("update-index-v1");
        let sig_path = root.join("update-index-v1.sig");
        std::fs::write(&index_path, &index).map_err(|source| LocalProductError::Io {
            operation: "write stable index verification file",
            source,
        })?;
        std::fs::write(&sig_path, &sig).map_err(|source| LocalProductError::Io {
            operation: "write stable signature verification file",
            source,
        })?;
        let state = active_state(roots)?;
        verify_release_signature_with_key(
            &roots.openssl,
            state.update_key,
            &index_path,
            &sig_path,
        )?;
        let parsed = parse_signed_update_index(&index)?;
        if parsed.generation_id != expected_generation {
            return Err(LocalProductError::LocalUpdate(
                "public stable generation changed during automatic publication",
            ));
        }
        Ok(())
    })();
    let cleanup = std::fs::remove_dir_all(&root).map_err(|source| LocalProductError::Io {
        operation: "remove stable index verification staging",
        source,
    });
    match (result, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn github_create_blob(
    gh: &std::path::Path,
    home: &std::path::Path,
    file: &std::path::Path,
) -> Result<String, LocalProductError> {
    let endpoint = format!("repos/{GITHUB_REPOSITORY}/git/blobs");
    let mut command = github_command(gh, home);
    let mut child = command
        .args([
            "api",
            "--hostname",
            GITHUB_HOST,
            "--method",
            "POST",
            &endpoint,
            "--input",
            "-",
            "--jq",
            ".sha",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| LocalProductError::LocalUpdate("create GitHub blob failed"))?;
    {
        let mut input = child
            .stdin
            .take()
            .ok_or(LocalProductError::LocalUpdate("create GitHub blob failed"))?;
        input
            .write_all(b"{\"content\":\"")
            .map_err(|_| LocalProductError::LocalUpdate("create GitHub blob failed"))?;
        write_github_base64(&mut input, file)
            .map_err(|_| LocalProductError::LocalUpdate("create GitHub blob failed"))?;
        input
            .write_all(b"\",\"encoding\":\"base64\"}")
            .map_err(|_| LocalProductError::LocalUpdate("create GitHub blob failed"))?;
    }
    let output = finish_github_output_bounded(
        &mut child,
        "create GitHub blob failed",
        GITHUB_RESPONSE_MAX_BYTES,
    )?;
    let sha = std::str::from_utf8(&output)
        .map_err(|_| LocalProductError::LocalUpdate("GitHub blob identity is invalid"))?
        .trim()
        .to_owned();
    if !valid_github_content_sha(&sha) {
        return Err(LocalProductError::LocalUpdate(
            "GitHub blob identity is invalid",
        ));
    }
    Ok(sha)
}

fn github_api_sha(
    gh: &std::path::Path,
    home: &std::path::Path,
    method: &str,
    endpoint: &str,
    json: &str,
) -> Result<String, LocalProductError> {
    let mut child = github_command(gh, home)
        .args([
            "api",
            "--hostname",
            GITHUB_HOST,
            "--method",
            method,
            endpoint,
            "--input",
            "-",
            "--jq",
            ".sha",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| LocalProductError::LocalUpdate("GitHub Git data operation failed"))?;
    child
        .stdin
        .take()
        .ok_or(LocalProductError::LocalUpdate(
            "GitHub Git data operation failed",
        ))?
        .write_all(json.as_bytes())
        .map_err(|_| LocalProductError::LocalUpdate("GitHub Git data operation failed"))?;
    let output = finish_github_output_bounded(
        &mut child,
        "GitHub Git data operation failed",
        GITHUB_RESPONSE_MAX_BYTES,
    )?;
    let sha = std::str::from_utf8(&output)
        .map_err(|_| LocalProductError::LocalUpdate("GitHub Git data identity is invalid"))?
        .trim()
        .to_owned();
    if !valid_github_content_sha(&sha) {
        return Err(LocalProductError::LocalUpdate(
            "GitHub Git data identity is invalid",
        ));
    }
    Ok(sha)
}

fn github_tree_sha(
    gh: &std::path::Path,
    home: &std::path::Path,
    head: &str,
) -> Result<String, LocalProductError> {
    let endpoint = format!("repos/{GITHUB_REPOSITORY}/git/commits/{head}");
    let mut command = github_command(gh, home);
    command.args([
        "api",
        "--hostname",
        GITHUB_HOST,
        &endpoint,
        "--jq",
        ".tree.sha",
    ]);
    let output = github_output_bounded(
        command,
        "read GitHub tree identity failed",
        GITHUB_RESPONSE_MAX_BYTES,
    )?;
    let sha = std::str::from_utf8(&output)
        .map_err(|_| LocalProductError::LocalUpdate("GitHub tree identity is invalid"))?
        .trim()
        .to_owned();
    if !valid_github_content_sha(&sha) {
        return Err(LocalProductError::LocalUpdate(
            "GitHub tree identity is invalid",
        ));
    }
    Ok(sha)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefAdvanceDisposition {
    Committed,
    NotCommitted,
    Indeterminate,
}

fn classify_ref_advance(
    old_head: &str,
    candidate_commit: &str,
    observed_head: Option<&str>,
) -> RefAdvanceDisposition {
    match observed_head {
        Some(observed) if observed == candidate_commit => RefAdvanceDisposition::Committed,
        Some(observed) if observed == old_head => RefAdvanceDisposition::NotCommitted,
        _ => RefAdvanceDisposition::Indeterminate,
    }
}

fn promote_stable_atomically(
    roots: &LocalCoreRoots,
    gh: &std::path::Path,
    home: &std::path::Path,
    publication: &std::path::Path,
    generation_id: &str,
    expected_stable_generation: &str,
) -> Result<(), LocalProductError> {
    let head = github_main_head(gh, home)?;
    parse_and_verify_stable_at_head(roots, gh, home, &head, expected_stable_generation)?;
    let index_blob = github_create_blob(gh, home, &publication.join("update-index-v1"))?;
    let sig_blob = github_create_blob(gh, home, &publication.join("update-index-v1.sig"))?;
    let base_tree = github_tree_sha(gh, home, &head)?;
    let tree_endpoint = format!("repos/{GITHUB_REPOSITORY}/git/trees");
    let tree_json = format!(
        "{{\"base_tree\":\"{base_tree}\",\"tree\":[{{\"path\":\"update-index-v1\",\"mode\":\"100644\",\"type\":\"blob\",\"sha\":\"{index_blob}\"}},{{\"path\":\"update-index-v1.sig\",\"mode\":\"100644\",\"type\":\"blob\",\"sha\":\"{sig_blob}\"}}]}}"
    );
    let tree = github_api_sha(gh, home, "POST", &tree_endpoint, &tree_json)?;
    let commit_endpoint = format!("repos/{GITHUB_REPOSITORY}/git/commits");
    let message = format!("codex update: promote {generation_id}");
    let commit_json =
        format!("{{\"message\":\"{message}\",\"tree\":\"{tree}\",\"parents\":[\"{head}\"]}}");
    let commit = github_api_sha(gh, home, "POST", &commit_endpoint, &commit_json)?;
    let ref_endpoint = format!("repos/{GITHUB_REPOSITORY}/git/refs/heads/{GITHUB_BRANCH}");
    let ref_json = format!("{{\"sha\":\"{commit}\",\"force\":false}}");
    let mut child = github_command(gh, home)
        .args([
            "api",
            "--hostname",
            GITHUB_HOST,
            "--method",
            "PATCH",
            &ref_endpoint,
            "--input",
            "-",
            "--silent",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| LocalProductError::LocalUpdate("advance stable Git ref failed"))?;
    child
        .stdin
        .take()
        .ok_or(LocalProductError::LocalUpdate(
            "advance stable Git ref failed",
        ))?
        .write_all(ref_json.as_bytes())
        .map_err(|_| LocalProductError::LocalUpdate("advance stable Git ref failed"))?;
    match wait_for_github_child_with_timeout(
        &mut child,
        std::time::Duration::from_secs(PUBLICATION_TIMEOUT_SECONDS),
    ) {
        Ok(()) => Ok(()),
        Err(()) => {
            let observed = github_main_head(gh, home).ok();
            match classify_ref_advance(&head, &commit, observed.as_deref()) {
                RefAdvanceDisposition::Committed => Ok(()),
                RefAdvanceDisposition::NotCommitted => Err(LocalProductError::LocalUpdate(
                    "advance stable Git ref failed before commit",
                )),
                RefAdvanceDisposition::Indeterminate => Err(LocalProductError::LocalUpdate(
                    "automatic stable promotion outcome is indeterminate; rerun codex update to re-resolve the signed channel",
                )),
            }
        }
    }
}

fn publish_candidate(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    expected_stable_generation: &str,
    generation_id: String,
    publication: std::path::PathBuf,
) -> Result<(SignedUpdateOutcome, GithubPublicationOutcome), LocalProductError> {
    let gh = github_cli_path(roots).ok_or(LocalProductError::LocalUpdate(
        "GitHub CLI is unavailable for automatic stable publication",
    ))?;
    let home = required_absolute_env_path("HOME")?;
    if !github_authenticated(&gh, &home) {
        return Err(LocalProductError::LocalUpdate(
            "GitHub authentication is unavailable for automatic stable publication",
        ));
    }
    let staging = publication_staging_root(roots)?;
    let pre_promotion = (|| {
        let release_sequence =
            prepare_staging_assets(roots, &publication, &generation_id, &staging)?;
        create_staging_release(&gh, &home, &generation_id, &staging)?;
        dispatch_pages(&gh, &home, &generation_id, release_sequence)?;
        let candidate_index_url = verify_pages_publication(roots, &publication, &generation_id)?;
        public_update_smoke(roots, &candidate_index_url, &generation_id)?;
        promote_stable_atomically(
            roots,
            &gh,
            &home,
            &publication,
            &generation_id,
            expected_stable_generation,
        )?;
        Ok(())
    })();
    let cleanup = std::fs::remove_dir_all(&staging).map_err(|source| LocalProductError::Io {
        operation: "remove automatic publication staging",
        source,
    });
    match (pre_promotion, cleanup) {
        (_, Err(error)) => return Err(error),
        (Err(error), Ok(())) => return Err(error),
        (Ok(()), Ok(())) => {}
    }

    let release = publication.join("releases").join(&generation_id);
    match activate_signed_local_release_outcome_with_hold_policy(
        &release,
        roots,
        process_env,
        UpdateHoldPolicy::Enforce,
    ) {
        Ok(outcome) if outcome.generation_id() == generation_id => {
            Ok((outcome, GithubPublicationOutcome::Published))
        }
        Ok(outcome) => {
            eprintln!(
                "codex update: public stable generation {generation_id} was promoted; local activation returned unexpected generation {}; local activation is deferred",
                outcome.generation_id()
            );
            Ok((
                SignedUpdateOutcome::Promoted(generation_id),
                GithubPublicationOutcome::Published,
            ))
        }
        Err(error) => {
            eprintln!(
                "codex update: public stable generation {generation_id} was promoted; local activation is deferred: {error}"
            );
            Ok((
                SignedUpdateOutcome::Promoted(generation_id),
                GithubPublicationOutcome::Published,
            ))
        }
    }
}

pub(super) fn maybe_publish_newer_official_generation(
    roots: &LocalCoreRoots,
    process_env: &TermuxProcessEnvSnapshot,
    expected_stable_generation: &str,
) -> Result<Option<(SignedUpdateOutcome, GithubPublicationOutcome)>, LocalProductError> {
    if !maintainer_publication_enabled(roots)? {
        return Ok(None);
    }
    let Some((generation_id, publication)) = build_newer_official_publication(roots)? else {
        return Ok(None);
    };
    publish_candidate(
        roots,
        process_env,
        expected_stable_generation,
        generation_id,
        publication,
    )
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_generation_snapshot_uses_bounded_regular_file_copies() {
        use std::os::unix::fs::{symlink, PermissionsExt as _};

        let sequence = AUTO_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-auto-smoke-copy-{}-{sequence}",
            std::process::id()
        ));
        let source = root.join("source");
        let destination = root.join("destination");
        std::fs::create_dir_all(source.join("nested")).unwrap();
        std::fs::write(source.join("runtime"), b"runtime-bytes").unwrap();
        std::fs::write(source.join("nested/helper"), b"helper-bytes").unwrap();
        let mut helper_permissions = std::fs::metadata(source.join("nested/helper"))
            .unwrap()
            .permissions();
        helper_permissions.set_mode(0o755);
        std::fs::set_permissions(source.join("nested/helper"), helper_permissions).unwrap();

        let mut copied_bytes = 0u64;
        copy_generation_tree(&source, &destination, &mut copied_bytes).unwrap();
        assert_eq!(
            copied_bytes,
            b"runtime-bytes".len() as u64 + b"helper-bytes".len() as u64
        );
        assert_eq!(
            std::fs::read(destination.join("runtime")).unwrap(),
            b"runtime-bytes"
        );
        assert_eq!(
            std::fs::read(destination.join("nested/helper")).unwrap(),
            b"helper-bytes"
        );
        assert_eq!(
            std::fs::metadata(destination.join("nested/helper"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o755
        );

        let unsafe_source = root.join("unsafe-source");
        std::fs::create_dir_all(&unsafe_source).unwrap();
        symlink(source.join("runtime"), unsafe_source.join("runtime-link")).unwrap();
        let mut unsafe_bytes = 0u64;
        assert!(matches!(
            copy_generation_tree(
                &unsafe_source,
                &root.join("unsafe-destination"),
                &mut unsafe_bytes
            ),
            Err(LocalProductError::LocalUpdate(
                "smoke generation contains an unsafe file type"
            ))
        ));
        assert_eq!(unsafe_bytes, 0);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn automatic_publication_requires_the_unmodified_default_channel() {
        assert!(publication_overrides_absent(false, false));
        assert!(!publication_overrides_absent(true, false));
        assert!(!publication_overrides_absent(false, true));
        assert!(!publication_overrides_absent(true, true));
    }

    #[test]
    fn automatic_publication_requires_every_maintainer_prerequisite() {
        assert!(maintainer_publication_prerequisites(
            true, true, true, true, true
        ));
        for denied in 0..5 {
            let mut conditions = [true; 5];
            conditions[denied] = false;
            assert!(!maintainer_publication_prerequisites(
                conditions[0],
                conditions[1],
                conditions[2],
                conditions[3],
                conditions[4],
            ));
        }
    }

    #[test]
    fn automatic_publication_rejects_github_auth_without_signing_authority() {
        assert!(!maintainer_publication_prerequisites(
            true, false, false, true, true
        ));
    }

    #[test]
    fn automatic_publication_rejects_mismatched_signing_authority() {
        assert!(!maintainer_publication_prerequisites(
            true, true, false, true, true
        ));
    }

    #[test]
    fn automatic_publication_allows_only_matching_signing_authority_and_github_auth() {
        assert!(maintainer_publication_prerequisites(
            true, true, true, true, true
        ));
        assert!(!maintainer_publication_prerequisites(
            true, true, true, true, false
        ));
    }

    #[test]
    fn stable_ref_advance_disambiguation_is_fail_closed() {
        let old = "1111111111111111111111111111111111111111";
        let candidate = "2222222222222222222222222222222222222222";
        let other = "3333333333333333333333333333333333333333";
        assert_eq!(
            classify_ref_advance(old, candidate, Some(candidate)),
            RefAdvanceDisposition::Committed
        );
        assert_eq!(
            classify_ref_advance(old, candidate, Some(old)),
            RefAdvanceDisposition::NotCommitted
        );
        assert_eq!(
            classify_ref_advance(old, candidate, Some(other)),
            RefAdvanceDisposition::Indeterminate
        );
        assert_eq!(
            classify_ref_advance(old, candidate, None),
            RefAdvanceDisposition::Indeterminate
        );
    }
}

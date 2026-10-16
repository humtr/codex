//! Upstream loads configuration; Core binds its resolved catalog to the backend.
use codex_utils_absolute_path::AbsolutePathBuf;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const LIMIT: usize = 4 * 1024 * 1024;

/// Upstream resolves configuration; this adapter owns local catalog selection.
pub(crate) async fn bind(
    target: &mut crate::AppServerTarget,
    config: &crate::legacy_core::config::Config,
) -> color_eyre::Result<()> {
    if crate::termux_permissions::enabled()
        && matches!(target, crate::AppServerTarget::LocalDaemon { .. })
        && let Some(catalog) = config.model_catalog.as_ref()
    {
        let core = std::env::var_os("CODEX_PROFILE_CORE")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| color_eyre::eyre::eyre!("qualified Core is unavailable"))?;
        let bytes = serde_json::to_vec(catalog)?;
        let socket_path = prepare(&core, &config.codex_home, &bytes).await?;
        *target = crate::AppServerTarget::LocalDaemon {
            endpoint: codex_app_server_client::RemoteAppServerEndpoint::UnixSocket { socket_path },
            allow_embedded_fallback: false,
        };
    }
    Ok(())
}

pub(crate) async fn prepare(
    core: &Path,
    home: &Path,
    catalog: &[u8],
) -> color_eyre::Result<AbsolutePathBuf> {
    let metadata = std::fs::symlink_metadata(core)?;
    if !core.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
        || catalog.is_empty()
        || catalog.len() > LIMIT
    {
        color_eyre::eyre::bail!("qualified catalog startup is unavailable");
    }
    tokio::time::timeout(Duration::from_secs(20), async {
        let mut child = Command::new(core)
            .arg("__app-server-catalog-v1")
            .env("CODEX_HOME", home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let mut input = child.stdin.take().expect("piped catalog stdin");
        input.write_all(catalog).await?;
        input.shutdown().await?;
        drop(input);
        let mut output = Vec::new();
        child
            .stdout
            .take()
            .expect("piped catalog stdout")
            .take(109)
            .read_to_end(&mut output)
            .await?;
        if output.len() > 108 || !child.wait().await?.success() {
            color_eyre::eyre::bail!("qualified backend catalog startup failed");
        }
        decode_socket(&output)
    })
    .await
    .map_err(|_| color_eyre::eyre::eyre!("qualified backend catalog startup timed out"))?
}

fn decode_socket(bytes: &[u8]) -> color_eyre::Result<AbsolutePathBuf> {
    let path = std::str::from_utf8(bytes)?
        .strip_suffix('\n')
        .filter(|s| Path::new(s).is_absolute() && s.len() <= 107 && !s.contains(['\n', '\r', '\0']))
        .ok_or_else(|| color_eyre::eyre::eyre!("invalid qualified backend socket"))?;
    Ok(AbsolutePathBuf::from_absolute_path_checked(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn termux_backend_catalog_rejects_non_socket_output() {
        assert!(decode_socket(b"/owned/s\n").is_ok());
        for value in [
            &b"relative\n"[..],
            b"~/owned/s\n",
            b"/owned/s",
            b"/owned/s\nextra\n",
            b"/owned/\0s\n",
            b"/owned/s\r\n",
            b"\xff\n",
        ] {
            assert!(
                decode_socket(value).is_err(),
                "accepted invalid socket output: {value:?}"
            );
        }
        assert!(decode_socket(format!("/{}\n", "s".repeat(107)).as_bytes()).is_err());
    }

    #[tokio::test]
    async fn termux_backend_catalog_transfers_exact_resolved_bytes_and_home()
    -> color_eyre::Result<()> {
        let root = tempfile::tempdir()?;
        let core = root.path().join("core");
        let home = root.path().join("home");
        std::fs::create_dir(&home)?;
        std::fs::write(
            &core,
            "#!/bin/sh\n[ \"$#\" = 1 ] && [ \"$1\" = __app-server-catalog-v1 ] || exit 2\ncat > \"$CODEX_HOME/catalog\"\nprintf '/owned/s\\n'\n",
        )?;
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700))?;
        let bytes = br#"{"models":[{"slug":"owned"}]}"#;
        let socket = prepare(&core, &home, bytes).await?;
        assert_eq!(socket.as_path(), Path::new("/owned/s"));
        assert_eq!(std::fs::read(home.join("catalog"))?, bytes);
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o777))?;
        assert!(prepare(&core, &home, bytes).await.is_err());
        Ok(())
    }
}

//! Optional same-generation Manager frontend; the official runtime owns execution.
use super::{GenerationManifest, LocalProductError, RuntimeFdSources, TermuxBaseEnvPlan};
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub(super) const PREFIX: &str = "termux-manager-tui-v1:";

pub(super) fn index(manifest: &GenerationManifest) -> Result<Option<usize>, LocalProductError> {
    let matches: Vec<_> = manifest
        .helper_digests
        .iter()
        .enumerate()
        .filter(|(_, helper)| helper.identity.starts_with(PREFIX))
        .collect();
    if matches.is_empty() {
        return Ok(None);
    }
    if matches.len() != 1
        || matches[0].0 != 2
        || manifest.helper_digests.len() != 3
        || matches[0].1.identity.strip_prefix(PREFIX)
            != Some(manifest.upstream_package_version.as_str())
        || manifest.manager_artifact_digest.is_none()
        || manifest.helper_digests[0].identity != super::TERMUX_BROWSER_OPEN_HELPER_IDENTITY
        || manifest.helper_digests[1].identity != super::TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
    {
        return Err(LocalProductError::Descriptor(
            "Manager frontend must match its generation version, Manager and helper layout",
        ));
    }
    Ok(Some(2))
}

pub(super) fn supported(args: &[OsString]) -> bool {
    !args.iter().any(|arg| {
        arg.to_str().is_some_and(|arg| {
            arg == "--add-dir"
                || arg.starts_with("--add-dir=")
                || arg == "--worktree"
                || arg.starts_with("--worktree=")
        })
    })
}

fn native_args(socket: &Path, original: &[OsString]) -> io::Result<Vec<OsString>> {
    let socket = socket
        .to_str()
        .filter(|_| socket.is_absolute())
        .ok_or_else(|| io::Error::other("qualified local frontend socket is invalid"))?;
    let mut args = vec!["--remote".into(), format!("unix://{socket}").into()];
    args.extend_from_slice(original);
    Ok(args)
}

pub(super) fn launch(
    frontend: &OsStr,
    socket: &Path,
    profile: &Path,
    original: &[OsString],
    resolver: &Path,
    config: &Path,
    plan: &TermuxBaseEnvPlan,
) -> io::Error {
    let prepare = || -> io::Result<_> {
        let core = super::validated_core_entrypoint()?;
        let fds = RuntimeFdSources::open(resolver, config)?;
        let mut command = Command::new(frontend);
        command
            .args(native_args(socket, original)?)
            .env("CODEX_HOME", profile)
            .env("CODEX_PROFILE_CORE", core);
        super::apply_child_env_plan_and_fence(&mut command, Some(plan));
        fds.configure(&mut command);
        Ok((command, fds))
    };
    match prepare() {
        Ok((mut command, _fds)) => command.exec(),
        Err(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GenerationHelperDigest;
    use std::os::unix::ffi::OsStringExt;

    fn manifest() -> GenerationManifest {
        GenerationManifest {
            upstream_package_identity: "owned".into(),
            upstream_package_version: "0.160.0".into(),
            source_artifact_digest: "owned".into(),
            expected_platform: "android".into(),
            expected_architecture: "aarch64".into(),
            patch_policy_id: "owned".into(),
            patch_report: "owned".into(),
            runtime_digest: "owned".into(),
            helper_digests: [
                super::super::TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
                super::super::TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
                "termux-manager-tui-v1:0.160.0",
            ]
            .map(|identity| GenerationHelperDigest {
                identity: identity.into(),
                digest: "owned".into(),
            })
            .into(),
            core_artifact_digest: "owned".into(),
            manager_artifact_digest: Some("owned".into()),
            core_api_identity: "owned".into(),
            persistent_schema_identity: "owned".into(),
            creation_metadata: "owned".into(),
        }
    }

    #[test]
    fn manager_tui_declaration_requires_one_matching_version_manager_and_layout() {
        let valid = manifest();
        assert_eq!(index(&valid).unwrap(), Some(2));
        let mut absent = valid.clone();
        absent.helper_digests.pop();
        assert_eq!(index(&absent).unwrap(), None);
        let mut invalid = manifest();
        invalid.upstream_package_version = "0.161.0".into();
        assert!(index(&invalid).is_err());
        invalid = manifest();
        invalid.manager_artifact_digest = None;
        assert!(index(&invalid).is_err());
        invalid = manifest();
        invalid.helper_digests.swap(0, 1);
        assert!(index(&invalid).is_err());
        invalid = manifest();
        invalid.creation_metadata = super::super::R10_BROWSER_HELPER_BRIDGE_METADATA.into();
        assert_eq!(index(&invalid).unwrap(), Some(2));
        assert!(super::super::r10_browser_helper_bridge(&invalid).unwrap());
        invalid.helper_digests[2].identity = "unqualified-third-helper".into();
        assert!(super::super::r10_browser_helper_bridge(&invalid).is_err());
        invalid = manifest();
        invalid
            .helper_digests
            .push(invalid.helper_digests[2].clone());
        assert!(index(&invalid).is_err());
    }

    #[test]
    fn manager_tui_socket_route_preserves_non_utf8_and_upstream_workspace_options() {
        let args = vec!["resume".into(), OsString::from_vec(vec![0xff])];
        let planned = native_args(Path::new("/owned/s"), &args).unwrap();
        assert_eq!(&planned[2..], args);
        assert_eq!(planned[0], "--remote");
        assert_eq!(planned[1], "unix:///owned/s");
        assert!(native_args(Path::new("relative"), &args).is_err());
        assert!(supported(&args));
        for arg in [
            "--add-dir",
            "--add-dir=/owned",
            "--worktree",
            "--worktree=owned",
        ] {
            assert!(!supported(&[arg.into()]));
        }
    }
}

use super::*;
use std::os::unix::ffi::OsStringExt;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "preview-pin-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn asset(&self) -> PathBuf {
        let path = self.0.join("asset");
        fs::write(&path, b"preview-owned").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
    fn openssl() -> PathBuf {
        super::super::LocalCoreRoots::from_environment()
            .unwrap()
            .openssl
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn live_preview_remote_route_preserves_raw_argv() {
    let original = vec![
        "resume".into(),
        OsString::from_vec(vec![b'x', 0xff]),
        "--all".into(),
    ];
    let args = native_args(Path::new("/owned/server/s"), &original).unwrap();
    assert_eq!(args[0], "--remote");
    assert_eq!(args[1], "unix:///owned/server/s");
    assert_eq!(&args[2..], &original);
    assert!(native_args(Path::new("relative"), &original).is_err());
}

#[test]
fn live_preview_pinned_owned_asset_rejects_drift_and_unsafe_modes() {
    let fixture = Fixture::new();
    let asset = fixture.asset();
    let openssl = Fixture::openssl();
    let digest = super::super::openssl_sha256(&openssl, &asset).unwrap();
    assert!(executable(&asset, &digest, &openssl).is_ok());
    fs::write(&asset, b"changed").unwrap();
    assert!(executable(&asset, &digest, &openssl).is_err());
    fs::write(&asset, b"preview-owned").unwrap();
    fs::set_permissions(&asset, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(executable(&asset, &digest, &openssl).is_err());
    fs::set_permissions(&asset, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(executable(&asset, &digest, &openssl).is_err());
    fs::set_permissions(&asset, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(executable(&asset, &digest, &openssl).is_err());
}

#[test]
fn live_preview_missing_and_substituted_assets_are_rejected() {
    let fixture = Fixture::new();
    let asset = fixture.asset();
    let openssl = Fixture::openssl();
    let digest = super::super::openssl_sha256(&openssl, &asset).unwrap();
    let link = fixture.0.join("link");
    std::os::unix::fs::symlink(&asset, &link).unwrap();
    assert!(executable(&link, &digest, &openssl).is_err());
    assert!(executable(&fixture.0.join("missing"), &digest, &openssl).is_err());
}

#[test]
fn live_preview_ordinary_routes_and_extra_snapshot_arguments_keep_core_dispatch() {
    for args in [
        vec!["--version".into()],
        vec!["termux".into(), "profile".into(), "list".into()],
        vec![
            "termux".into(),
            "__profile-snapshot-v1".into(),
            "extra".into(),
        ],
    ] {
        assert!(handle(&args).is_none());
    }
}

#[test]
fn live_preview_workspace_options_retain_original_installed_route() {
    for args in [
        vec!["--add-dir".into(), "owned".into()],
        vec!["--add-dir=owned".into()],
        vec!["resume".into(), "--worktree".into()],
    ] {
        assert!(!supported_options(&args));
    }
    assert!(supported_options(&["resume".into(), "--all".into()]));
    assert!(supported_options(&["-C".into(), "owned".into()]));
}

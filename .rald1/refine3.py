from pathlib import Path

p = Path("crates/core/src/main.rs")
s = p.read_text()
start_marker = '''    #[cfg(unix)]
    #[test]
    fn test_r7_bare_update_transport_fallback_builds_and_activates_local_release() {
'''
end_marker = '''    #[cfg(unix)]
    fn write_github_release_manifest(release: &std::path::Path, manager: bool) {
'''
start = s.find(start_marker)
if start < 0:
    raise SystemExit("R7 fallback test start marker missing")
end = s.find(end_marker, start)
if end < 0:
    raise SystemExit("R7 fallback test end marker missing")
replacement = r'''    #[cfg(unix)]
    #[test]
    fn test_rald1_bare_update_transport_fallback_builds_and_activates_local_derived_release() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("rald1-local-fallback");
        let openssl = b4_termux_openssl();
        let (home, prefix, tmp) = b4_prepare_public_environment(&root, &openssl, true);
        let prefix_openssl = prefix.join("bin/openssl");
        std::fs::remove_file(&prefix_openssl).unwrap();
        std::fs::copy(&openssl, &prefix_openssl).unwrap();
        let live_gzip =
            std::path::PathBuf::from(std::env::var_os("PREFIX").unwrap()).join("bin/gzip");
        let fixture_gzip = prefix.join("bin/gzip");
        std::fs::copy(&live_gzip, &fixture_gzip).unwrap();

        let private_key = root.join("keys/private.pem");
        let public_key = root.join("keys/public.pem");
        b4_generate_release_keypair(&openssl, &private_key, &public_key);
        b4_install_trusted_release_key(&home, &public_key);
        let source_roots = b4_source_roots(&root.join("source"), &openssl);
        std::fs::create_dir_all(&source_roots.generation_root).unwrap();
        let current = b2_write_generation(&source_roots, "r7-current", false, "unsupported");
        b4_write_signed_release(&current, 1, &openssl, &private_key);
        b7_seed_initial_release(
            &current,
            &home,
            &prefix,
            &home.join(".local/lib/codex/core/release-public-key.pem"),
        );
        let roots = b7_public_roots(&home, &prefix);
        let paths = CoreStatePaths::new(&roots.state_root).unwrap();
        let before = read_pointer_state(&paths).unwrap().unwrap();
        assert_eq!(before.current, "r7-current");
        assert_eq!(before.current_key, before.update_key);

        let stable_entrypoint = prefix.join("bin/codex");
        std::fs::copy(std::env::current_exe().unwrap(), &stable_entrypoint).unwrap();
        let mut stable_mode = std::fs::metadata(&stable_entrypoint).unwrap().permissions();
        stable_mode.set_mode(0o755);
        std::fs::set_permissions(&stable_entrypoint, stable_mode).unwrap();
        std::fs::remove_file(home.join(".local/lib/codex/core/release-public-key.pem")).unwrap();

        let runtime_source = b8_compile_static_probe_runtime(&root);
        let archive = root.join(UPSTREAM_PACKAGE_ASSET);
        let raw_runtime = std::fs::read(&runtime_source).unwrap();
        b6_write_official_shape_archive_with_runtime(&fixture_gzip, &archive, &raw_runtime);
        let archive_digest = openssl_sha256(&openssl, &archive).unwrap();
        let metadata_path = root.join("upstream-release.json");
        std::fs::write(
            &metadata_path,
            format!(
                "{{\"tag_name\":\"rust-v0.150.1\",\"assets\":[{{\"name\":\"{UPSTREAM_PACKAGE_ASSET}\",\"digest\":\"sha256:{archive_digest}\"}}]}}"
            ),
        )
        .unwrap();
        let index_url = "https://updates.example.invalid/codex/update-index-v1";
        let metadata_url = DEFAULT_UPSTREAM_LATEST_URL.to_owned();
        let archive_url =
            format!("{UPSTREAM_RELEASE_METADATA_BASE}/0.150.1/{UPSTREAM_PACKAGE_ASSET}");
        let curl_log = root.join("r7-curl-log");
        b7_write_local_update_curl(
            &prefix.join("bin/curl"),
            &curl_log,
            index_url,
            &metadata_url,
            &metadata_path,
            &archive_url,
            &archive,
        );
        let gh_log = root.join("r7-gh-log");
        b7_write_fake_github_cli(
            &prefix.join("bin/gh"),
            &gh_log,
            &root.join("r7-gh-body"),
            77,
        );
        assert!(prefix.join("bin/gh").is_file());

        let mut private_mode = std::fs::metadata(&private_key).unwrap().permissions();
        private_mode.set_mode(0o000);
        std::fs::set_permissions(&private_key, private_mode).unwrap();
        assert_eq!(
            std::fs::metadata(&private_key).unwrap().permissions().mode() & 0o7777,
            0
        );

        let output =
            b7_run_public_local_fallback(index_url, None, &private_key, &home, &prefix, &tmp);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stdout={:?} stderr={:?}",
            output.stdout,
            output.stderr
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("activated local-derived generation local-"));
        assert!(output.stderr.is_empty(), "stderr={:?}", output.stderr);
        assert!(
            !gh_log.exists(),
            "local-derived transport fallback must never invoke gh"
        );
        assert!(
            !home.join(LOCAL_PUBLICATION_ROOT_RELATIVE).exists(),
            "local-derived transport fallback must not persist an official publication tree"
        );
        let calls = std::fs::read_to_string(&curl_log).unwrap();
        assert!(calls.contains(index_url));
        assert!(calls.contains(&metadata_url));
        assert!(calls.contains(&archive_url));

        let state = read_pointer_state(&paths).unwrap().unwrap();
        assert!(state.current.starts_with("local-"));
        assert_eq!(state.previous.as_deref(), Some("r7-current"));
        assert_eq!(state.update_key, before.update_key);
        assert_eq!(state.previous_key, Some(before.current_key));
        assert_ne!(state.current_key, state.update_key);
        let (release, loaded) = verify_installed_local_release(
            &roots,
            &state.current,
            state.current_key,
            "RALD-1 local fallback generation id mismatch",
        )
        .unwrap();
        assert_eq!(release.release_sequence, 1);
        assert_eq!(loaded.manifest.upstream_package_version, "0.150.1");
        assert_eq!(loaded.manifest.source_artifact_digest, archive_digest);
        assert_eq!(loaded.generation_layout, GenerationLayout::RootCodeModeHost);
        let provenance = local_derived::parse_provenance(&loaded.manifest.creation_metadata)
            .unwrap()
            .expect("local-derived generation must carry signed provenance");
        assert_eq!(provenance.upstream_version, "0.150.1");
        assert_eq!(provenance.archive_sha256, loaded.manifest.source_artifact_digest);
        assert_eq!(provenance.baseline.generation_id, before.current);
        assert_eq!(provenance.baseline.release_sequence, 1);
        assert_eq!(provenance.baseline.update_key, before.current_key);
        assert!(local_derived::current_is_local_derived(
            &roots,
            &state,
            &release,
            &loaded,
        )
        .unwrap());
        assert!(std::fs::read_dir(&roots.state_root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".local-update-")
        }));
        remove_temp_root(root);
    }

'''
s = s[:start] + replacement + s[end:]
p.write_text(s)

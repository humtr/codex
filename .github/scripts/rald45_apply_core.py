from pathlib import Path

path = Path("crates/core/src/main.rs")
text = path.read_text()

old_gate = '''        if loaded.doctor_capability == UpstreamDoctorCapability::Supported
            && probe_qualified_upstream_doctor(
                assets,
                process_env,
                roots.cert_file.as_os_str(),
                Some(roots.cert_dir.as_os_str()),
                &roots.resolver_path,
                &roots.config_dir,
            )
            .map_err(|_| LocalProductError::CandidateProbe("candidate doctor probe failed"))?
                != UpstreamDoctorStatus::Healthy
        {
            return Err(LocalProductError::CandidateProbe(
                "candidate doctor probe was unhealthy",
            ));
        }
'''
if text.count(old_gate) != 1:
    raise SystemExit("expected one activation-time doctor gate")
text = text.replace(old_gate, "", 1)

old_name = "fn test_m2_b4_activation_version_and_doctor_probe_failures_preserve_old_current() {"
new_name = "fn test_m2_b4_activation_version_probe_failure_preserves_old_current_and_doctor_health_does_not_gate() {"
if text.count(old_name) != 1:
    raise SystemExit("expected activation probe test name")
text = text.replace(old_name, new_name, 1)

old_assertion = '''        b4_assert_public_update_rejected(
            &doctor_failure,
            &home,
            &prefix,
            &tmp,
            b"candidate doctor probe was unhealthy",
        );
        assert_eq!(
            std::fs::read(&state_paths.activation_state).unwrap(),
            state_before
        );
'''
new_assertion = '''        b4_assert_public_update_activated(
            &doctor_failure,
            &home,
            &prefix,
            &tmp,
            "doctor-failure",
        );
        let state_after_doctor = read_pointer_state(&state_paths).unwrap().unwrap();
        assert_eq!(state_after_doctor.current, "doctor-failure");
        assert_eq!(state_after_doctor.previous.as_deref(), Some("probe-current"));
'''
if text.count(old_assertion) != 1:
    raise SystemExit("expected doctor rejection assertion block")
text = text.replace(old_assertion, new_assertion, 1)

path.write_text(text)

#!/usr/bin/env python3
from pathlib import Path

PRODUCT_SHA = "37fbbd8033b8cc2d508689ab1d6637b4c4f5d516"


def once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


workflow_path = Path(".github/workflows/auto-release-termux.yml")
workflow = workflow_path.read_text()
workflow = once(
    workflow,
    "  CODEX_SOURCE_SHA: '1cdcb44d035ec5b1ce6339f2aa7b0e95831a6f0a'\n",
    f"  CODEX_SOURCE_SHA: '{PRODUCT_SHA}'\n",
    "pin corrected Core source",
)
workflow = once(
    workflow,
    """      rald5_same_version_acceptance:
        description: 'Bounded RALD-5 acceptance: republish exact official 0.154.0 through the full public path'
        required: false
        type: boolean
        default: false
      rald5_negative_gate:
""",
    """      rald5_same_version_acceptance:
        description: 'Bounded RALD-5 acceptance: republish exact official 0.154.0 through the full public path'
        required: false
        type: boolean
        default: false
      rald45_transition_stage:
        description: 'Stage/prove the backward-compatible RALD-4.5 transition generation without stable promotion'
        required: false
        type: boolean
        default: false
      rald5_negative_gate:
""",
    "transition dispatch input",
)
workflow = once(
    workflow,
    "      same_version_acceptance: ${{ steps.decision.outputs.same_version_acceptance }}\n      negative_gate:",
    "      same_version_acceptance: ${{ steps.decision.outputs.same_version_acceptance }}\n      transition_stage: ${{ steps.decision.outputs.transition_stage }}\n      negative_gate:",
    "producer transition output",
)
workflow = once(
    workflow,
    "          RALD5_SAME_VERSION_ACCEPTANCE: ${{ github.event_name == 'workflow_dispatch' && inputs.rald5_same_version_acceptance }}\n          RALD5_NEGATIVE_GATE:",
    "          RALD5_SAME_VERSION_ACCEPTANCE: ${{ github.event_name == 'workflow_dispatch' && inputs.rald5_same_version_acceptance }}\n          RALD45_TRANSITION_STAGE: ${{ github.event_name == 'workflow_dispatch' && inputs.rald45_transition_stage }}\n          RALD5_NEGATIVE_GATE:",
    "transition decision environment",
)
workflow = once(
    workflow,
    '          test "$RALD5_SAME_VERSION_ACCEPTANCE" = true -o "$RALD5_SAME_VERSION_ACCEPTANCE" = false\n',
    '          test "$RALD5_SAME_VERSION_ACCEPTANCE" = true -o "$RALD5_SAME_VERSION_ACCEPTANCE" = false\n'
    '          test "$RALD45_TRANSITION_STAGE" = true -o "$RALD45_TRANSITION_STAGE" = false\n',
    "transition boolean validation",
)
workflow = once(
    workflow,
    '            test "$RALD5_SAME_VERSION_ACCEPTANCE" != true\n            test "$RALD5_NEGATIVE_GATE" != true\n',
    '            test "$RALD5_SAME_VERSION_ACCEPTANCE" != true\n'
    '            test "$RALD45_TRANSITION_STAGE" != true\n'
    '            test "$RALD5_NEGATIVE_GATE" != true\n',
    "RALD-4 exclusion of transition mode",
)
transition_branch = '''          if test "$RALD45_TRANSITION_STAGE" = true; then
            test "$GITHUB_EVENT_NAME" = workflow_dispatch
            test "$GITHUB_REF" = refs/heads/rewrite/rust-core
            test "$RALD4_POSITIVE_GATE" != true
            test "$RALD5_PUBLICATION_AUTHORIZED" = true
            test "$RALD5_SAME_VERSION_ACCEPTANCE" = true
            test "$RALD5_NEGATIVE_GATE" != true
            test '${{ steps.stable.outputs.current_version }}' = "$RALD5_SAME_VERSION_ACCEPTANCE_VERSION"
            test "$upstream_version" = "$RALD5_SAME_VERSION_ACCEPTANCE_VERSION"
            test '${{ steps.stable.outputs.current_version }}' = "$upstream_version"
            test "$candidate" = true
            acceptance_suffix='-rald45-transition'
            printf 'RALD-4.5 transition stage selected from authenticated public stable %s to corrected Core source %s\\n' \\
              "$upstream_version" "$CODEX_SOURCE_SHA"
          fi
'''
workflow = once(
    workflow,
    '          if test "$RALD5_NEGATIVE_GATE" = true; then\n',
    transition_branch + '          if test "$RALD5_NEGATIVE_GATE" = true; then\n',
    "insert bounded transition decision",
)
workflow = once(
    workflow,
    '            test "$RALD4_POSITIVE_GATE" != true\n            test "$RALD5_PUBLICATION_AUTHORIZED" = true\n            test "$candidate" = true\n          fi\n          suffix=',
    '            test "$RALD4_POSITIVE_GATE" != true\n'
    '            test "$RALD45_TRANSITION_STAGE" != true\n'
    '            test "$RALD5_PUBLICATION_AUTHORIZED" = true\n'
    '            test "$candidate" = true\n'
    '          fi\n'
    '          suffix=',
    "negative gate excludes transition",
)
workflow = once(
    workflow,
    "          printf 'same_version_acceptance=%s\\n' \"$RALD5_SAME_VERSION_ACCEPTANCE\" >> \"$GITHUB_OUTPUT\"\n          printf 'negative_gate=%s",
    "          printf 'same_version_acceptance=%s\\n' \"$RALD5_SAME_VERSION_ACCEPTANCE\" >> \"$GITHUB_OUTPUT\"\n          printf 'transition_stage=%s\\n' \"$RALD45_TRANSITION_STAGE\" >> \"$GITHUB_OUTPUT\"\n          printf 'negative_gate=%s",
    "transition decision output",
)
workflow = once(
    workflow,
    "          generation_id='${{ steps.decision.outputs.generation_id }}'\n          fetch_output=",
    "          generation_id='${{ steps.decision.outputs.generation_id }}'\n"
    "          transition_args=()\n"
    "          if test '${{ steps.decision.outputs.transition_stage }}' = true; then\n"
    "            transition_args+=(--legacy-activation-doctor-unsupported)\n"
    "          fi\n"
    "          fetch_output=",
    "transition builder args",
)
workflow = once(
    workflow,
    "            --defer-manager-probe \\\n            --creation-metadata r10-browser-helper-bridge-v1 \\\n",
    "            --defer-manager-probe \\\n            \"${transition_args[@]}\" \\\n            --creation-metadata r10-browser-helper-bridge-v1 \\\n",
    "bounded legacy activation flag",
)
workflow = once(
    workflow,
    "          python3 source/.github/scripts/rald3_preflight.py candidate \\\n            \"$work/candidate\" \"$upstream_version\"\n          find \"$work/candidate\"",
    "          python3 source/.github/scripts/rald3_preflight.py candidate \\\n            \"$work/candidate\" \"$upstream_version\"\n"
    "          test \"$(awk -F '\\t' '$1==\"creation_metadata\"{print $2}' \"$work/candidate/generation.meta\")\" = r10-browser-helper-bridge-v1\n"
    "          doctor_capability=\"$(awk -F '\\t' '$1==\"upstream_doctor\"{print $2}' \"$work/candidate/generation.meta\")\"\n"
    "          if test '${{ steps.decision.outputs.transition_stage }}' = true; then\n"
    "            test \"$doctor_capability\" = unsupported\n"
    "          else\n"
    "            test \"$doctor_capability\" = supported\n"
    "          fi\n"
    "          find \"$work/candidate\"",
    "transition descriptor assertion",
)

bootstrap_old = '''          /bin/sh publication-source/bootstrap/codex-bootstrap \\
            "$root/$current/core" "$root/$current" "$root/update-public-key.pem"
          test -x "$PREFIX/bin/codex"

          export CODEX_TERMUX_UPDATE_INDEX_URL="https://humtr.github.io/codex/$candidate/update-index-v1"
'''
bootstrap_new = '''          if test '${{ needs.producer.outputs.transition_stage }}' = true; then
            generation_root="$HOME/.local/lib/codex/core/generations"
            current_dir="$generation_root/$current"
            state_root="$HOME/.local/share/codex/core"
            mkdir -p "$generation_root" "$state_root"
            python3 - "$root/$current" "$current_dir" <<'PY'
          import hashlib
          import os
          from pathlib import Path, PurePosixPath
          import shutil
          import stat
          import sys

          source = Path(sys.argv[1])
          destination = Path(sys.argv[2])
          if destination.exists():
              raise SystemExit("current generation destination already exists")
          manifest = source / "release.manifest"
          data = manifest.read_bytes()
          if len(data) > 128 * 1024 or not data.endswith(b"\\n"):
              raise SystemExit("current manifest is not bounded canonical text")
          lines = data.decode("utf-8").splitlines()
          if not lines or lines[0] != "codex-release-v4":
              raise SystemExit("current manifest format is not v4")
          headers = {}
          for line in lines[1:10]:
              parts = line.split("\\t")
              if len(parts) != 2 or parts[0] in headers:
                  raise SystemExit("current manifest header is invalid")
              headers[parts[0]] = parts[1]
          if headers.get("generation_id") != destination.name:
              raise SystemExit("current manifest generation id mismatch")
          count = int(headers.get("file_count", "0"))
          records = lines[10:]
          if count != len(records) or count != 7:
              raise SystemExit("current manifest inventory size mismatch")
          destination.mkdir(mode=0o755)
          seen = set()
          for line in records:
              parts = line.split("\\t")
              if len(parts) != 4 or parts[0] != "file":
                  raise SystemExit("current manifest file record is invalid")
              rel, digest, mode = parts[1:]
              pure = PurePosixPath(rel)
              if pure.is_absolute() or ".." in pure.parts or rel in seen:
                  raise SystemExit("current manifest path is unsafe")
              if mode not in {"0644", "0755"}:
                  raise SystemExit("current manifest mode is invalid")
              seen.add(rel)
              src = source / rel
              st = src.lstat()
              if not stat.S_ISREG(st.st_mode) or src.is_symlink():
                  raise SystemExit(f"current source is not a regular file: {rel}")
              if hashlib.sha256(src.read_bytes()).hexdigest() != digest:
                  raise SystemExit(f"current source digest mismatch: {rel}")
              if f"{stat.S_IMODE(st.st_mode):04o}" != mode:
                  raise SystemExit(f"current source mode mismatch: {rel}")
              target = destination / rel
              target.parent.mkdir(parents=True, exist_ok=True)
              shutil.copyfile(src, target)
              os.chmod(target, int(mode, 8))
          for name in ("release.manifest", "release.sig"):
              src = source / name
              st = src.lstat()
              if not stat.S_ISREG(st.st_mode) or src.is_symlink():
                  raise SystemExit(f"current authority file is unsafe: {name}")
              shutil.copyfile(src, destination / name)
              os.chmod(destination / name, 0o644)
          PY
            manifest_key="$(awk -F '\t' '$1=="release_public_key"{print $2}' "$current_dir/release.manifest")"
            test "${#manifest_key}" -eq 64
            case "$manifest_key" in (*[!0-9a-f]*) exit 1;; esac
            openssl pkey -pubin -in "$root/update-public-key.pem" -pubout -outform DER \\
              > "$RUNNER_TEMP/rald45-public.der"
            test "$(wc -c < "$RUNNER_TEMP/rald45-public.der")" -eq 44
            authority_key="$(tail -c 32 "$RUNNER_TEMP/rald45-public.der" | od -An -vtx1 | tr -d ' \\n')"
            test "$authority_key" = "$manifest_key"
            install -m 0644 "$root/update-public-key.pem" \\
              "$HOME/.local/lib/codex/core/release-public-key.pem"
            cat > "$state_root/activation-state" <<EOF
          format=codex-activation-state-v3
          update_key=$manifest_key
          current=$current
          current_key=$manifest_key
          previous_present=0
          previous=
          previous_key=
          EOF
            chmod 0644 "$state_root/activation-state"
            test ! -e "$state_root/activation-journal"
            test ! -e "$state_root/activation-journal.tmp"
            test ! -e "$state_root/activation-state.tmp"
            install -m 0755 "$current_dir/core" "$PREFIX/bin/codex"
            cmp "$PREFIX/bin/codex" "$current_dir/core"
            if cmp -s "$PREFIX/bin/codex" "$root/$candidate/core"; then
              printf 'transition fixture did not preserve the old public stable Core\\n' >&2
              exit 1
            fi
            "$PREFIX/bin/codex" --version > "$RUNNER_TEMP/rald45-pre-update-version.out"
            printf 'codex-cli %s\\n' "$version" > "$RUNNER_TEMP/rald45-pre-update-version.expected"
            cmp "$RUNNER_TEMP/rald45-pre-update-version.expected" "$RUNNER_TEMP/rald45-pre-update-version.out"
          else
            /bin/sh publication-source/bootstrap/codex-bootstrap \\
              "$root/$current/core" "$root/$current" "$root/update-public-key.pem"
          fi
          test -x "$PREFIX/bin/codex"

          export CODEX_TERMUX_UPDATE_INDEX_URL="https://humtr.github.io/codex/$candidate/update-index-v1"
'''
workflow = once(workflow, bootstrap_old, bootstrap_new, "transition pre-installed stable fixture")
workflow = once(
    workflow,
    '''          cmp "$RUNNER_TEMP/rald5-first-update.expected" "$RUNNER_TEMP/rald5-first-update.out"
          "$PREFIX/bin/codex" --version > "$RUNNER_TEMP/rald5-version.out"
''',
    '''          cmp "$RUNNER_TEMP/rald5-first-update.expected" "$RUNNER_TEMP/rald5-first-update.out"
          if test '${{ needs.producer.outputs.transition_stage }}' = true; then
            test "$(sed -n 's/^current=//p' "$HOME/.local/share/codex/core/activation-state")" = "$candidate"
            test "$(sed -n 's/^previous=//p' "$HOME/.local/share/codex/core/activation-state")" = "$current"
            cmp "$PREFIX/bin/codex" "$root/$candidate/core"
          fi
          "$PREFIX/bin/codex" --version > "$RUNNER_TEMP/rald5-version.out"
''',
    "transition post-update authority proof",
)
doctor_old = '''          "$PREFIX/bin/codex" doctor > "$RUNNER_TEMP/rald5-doctor.out"
          test -s "$RUNNER_TEMP/rald5-doctor.out"
          test "$(wc -c < "$RUNNER_TEMP/rald5-doctor.out")" -lt 65536
'''
doctor_new = '''          set +e
          "$PREFIX/bin/codex" doctor --json > "$RUNNER_TEMP/rald5-doctor.out"
          doctor_rc=$?
          set -e
          test "$doctor_rc" -eq 0 -o "$doctor_rc" -eq 1
          test -s "$RUNNER_TEMP/rald5-doctor.out"
          test "$(wc -c < "$RUNNER_TEMP/rald5-doctor.out")" -lt 65536
          python3 - "$RUNNER_TEMP/rald5-doctor.out" "$doctor_rc" "$candidate" <<'PY'
          import json
          from pathlib import Path
          import sys

          report = json.loads(Path(sys.argv[1]).read_text())
          rc = int(sys.argv[2])
          candidate = sys.argv[3]
          if report.get("schema_version") != 2:
              raise SystemExit("doctor schema is not v2")
          upstream = report.get("upstream")
          core = report.get("termux_core")
          manager = report.get("manager")
          summary = report.get("summary")
          if not isinstance(upstream, dict) or not isinstance(core, dict):
              raise SystemExit("doctor report sections are invalid")
          if core.get("status") != "healthy" or core.get("generation_id") != candidate:
              raise SystemExit("Termux Core health is not healthy for the activated candidate")
          if core.get("layout") != "root-code-mode-host-v2":
              raise SystemExit("Termux Core layout is unexpected")
          if core.get("runtime", {}).get("status") != "healthy":
              raise SystemExit("Termux runtime health is not healthy")
          if core.get("code_mode_host", {}).get("status") != "healthy":
              raise SystemExit("code-mode host health is not healthy")
          upstream_status = upstream.get("status")
          if upstream_status not in {"healthy", "unhealthy"}:
              raise SystemExit("actual upstream doctor did not run")
          upstream_output = upstream.get("output")
          if not isinstance(upstream_output, str) or len(upstream_output.encode()) >= 32768:
              raise SystemExit("upstream diagnostic output is invalid or unbounded")
          manager_status = manager.get("status") if isinstance(manager, dict) else None
          if manager_status not in {"healthy", "unavailable", "unhealthy"}:
              raise SystemExit("manager doctor status is invalid for this proof")
          if upstream_status == "unhealthy" or manager_status == "unhealthy":
              expected_summary = "unhealthy"
          elif manager_status == "unavailable":
              expected_summary = "degraded"
          else:
              expected_summary = "healthy"
          actual_summary = summary.get("status") if isinstance(summary, dict) else None
          if actual_summary != expected_summary:
              raise SystemExit("doctor summary is inconsistent with component health")
          expected_rc = 0 if expected_summary == "healthy" else 1
          if rc != expected_rc:
              raise SystemExit("doctor exit status is inconsistent with diagnostic report")
          print(f"doctor_upstream_status={upstream_status}")
          print("doctor_termux_core_status=healthy")
          PY
'''
workflow = once(workflow, doctor_old, doctor_new, "credential-free doctor semantic proof")
workflow = once(
    workflow,
    "needs.producer.outputs.publication_authorized == 'true' && needs.producer.outputs.negative_gate != 'true'\n",
    "needs.producer.outputs.publication_authorized == 'true' && needs.producer.outputs.negative_gate != 'true' && needs.producer.outputs.transition_stage != 'true'\n",
    "transition promotion guard",
)
workflow_path.write_text(workflow)


test_path = Path(".github/scripts/test_rald5_workflow.py")
test = test_path.read_text()
test = once(
    test,
    'RALD5_SOURCE_SHA = "fa1b887b7e726202e309a2eb731aad303a6c7e03"\n',
    f'CODEX_SOURCE_SHA = "{PRODUCT_SHA}"\nRALD5_SOURCE_SHA = "fa1b887b7e726202e309a2eb731aad303a6c7e03"\n',
    "workflow contract source pin constant",
)
test = once(
    test,
    '        self.assertIn(f"RALD5_SOURCE_SHA: \'{RALD5_SOURCE_SHA}\'", self.auto)\n',
    '        self.assertIn(f"CODEX_SOURCE_SHA: \'{CODEX_SOURCE_SHA}\'", self.auto)\n'
    '        self.assertIn(f"RALD5_SOURCE_SHA: \'{RALD5_SOURCE_SHA}\'", self.auto)\n',
    "workflow source pin assertion",
)
insert_before = '''    def test_write_authority_is_job_local_and_after_signing(self) -> None:
'''
new_test = '''    def test_rald45_transition_stage_is_bounded_and_cannot_promote(self) -> None:
        self.assertIn("rald45_transition_stage:", self.header)
        transition_input = self.header.split("rald45_transition_stage:", 1)[1].split(
            "rald5_negative_gate:", 1
        )[0]
        self.assertIn("type: boolean", transition_input)
        self.assertIn("default: false", transition_input)
        self.assertIn(
            "RALD45_TRANSITION_STAGE: ${{ github.event_name == 'workflow_dispatch' && inputs.rald45_transition_stage }}",
            self.decision,
        )
        transition = self.decision.split('if test "$RALD45_TRANSITION_STAGE" = true; then', 1)[1].split(
            'if test "$RALD5_NEGATIVE_GATE" = true; then', 1
        )[0]
        for required in [
            'test "$GITHUB_EVENT_NAME" = workflow_dispatch',
            'test "$GITHUB_REF" = refs/heads/rewrite/rust-core',
            'test "$RALD4_POSITIVE_GATE" != true',
            'test "$RALD5_PUBLICATION_AUTHORIZED" = true',
            'test "$RALD5_SAME_VERSION_ACCEPTANCE" = true',
            'test "$RALD5_NEGATIVE_GATE" != true',
            "RALD5_SAME_VERSION_ACCEPTANCE_VERSION",
            "-rald45-transition",
        ]:
            self.assertIn(required, transition)
        self.assertIn("transition_stage=%s", self.decision)
        self.assertIn("transition_stage: ${{ steps.decision.outputs.transition_stage }}", self.auto)
        candidate_build = self.auto.split("- name: Fetch, adapt, and qualify unsigned candidate", 1)[1].split(
            "- name: Upload unsigned candidate only", 1
        )[0]
        self.assertIn("--legacy-activation-doctor-unsupported", candidate_build)
        self.assertIn("--creation-metadata r10-browser-helper-bridge-v1", candidate_build)
        self.assertIn("test \"$doctor_capability\" = unsupported", candidate_build)
        self.assertIn("needs.producer.outputs.transition_stage != 'true'", self.promote)
        self.assertNotIn("OPENAI_API_KEY", self.verify)
        self.assertNotIn("CODEX_API_KEY", self.verify)

    def test_transition_proof_uses_old_stable_update_and_semantic_doctor_report(self) -> None:
        verify = self.verify
        self.assertIn("activation-state", verify)
        self.assertIn("codex-activation-state-v3", verify)
        self.assertIn('cmp "$PREFIX/bin/codex" "$current_dir/core"', verify)
        self.assertIn("transition fixture did not preserve the old public stable Core", verify)
        self.assertIn('"$PREFIX/bin/codex" update', verify)
        self.assertIn('cmp "$PREFIX/bin/codex" "$root/$candidate/core"', verify)
        self.assertIn('doctor --json', verify)
        self.assertIn("doctor_rc=$?", verify)
        self.assertIn('test "$doctor_rc" -eq 0 -o "$doctor_rc" -eq 1', verify)
        self.assertIn('core.get("status") != "healthy"', verify)
        self.assertIn('upstream_status not in {"healthy", "unhealthy"}', verify)
        self.assertIn("actual upstream doctor did not run", verify)
        self.assertIn("doctor exit status is inconsistent with diagnostic report", verify)
        self.assertIn("publication-source/bootstrap/codex-bootstrap", verify)

'''
test = once(test, insert_before, new_test + insert_before, "transition workflow contract tests")
test_path.write_text(test)

#!/usr/bin/env python3
"""Exercise the actual one-shot workflow admission and component qualifier."""

import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).parents[1] / "workflows/auto-release-termux.yml"
TEXT = WORKFLOW.read_text()
SOURCE = "7285b91adbad0824bbb8eb4bf7bf8e336fe532c1"
PARENT = "07d625561283bad5d25b778782c16d4929b4939f"
MESSAGE = "release-boundary-alignment-seq35-source-7285b91a"
CURRENT = "local-hosted-0-160-0-4b00b8d46193-notify-line"
NEW = "local-hosted-0-160-0-7285b91adbad-boundary-release"
OLD_CORE = "b083753451042c861acc6d22153d57eb9df6fe3938567eec2ec9f74e9a9978cd"
OLD_MANAGER = "cc0a38c5af308702318a2c9575c1efccd846e816183071d5b43a0dafc0f7544d"


def qualifier():
    return textwrap.dedent(TEXT.split("<<'PYBOUNDARY'\n", 1)[1].split("          PYBOUNDARY\n", 1)[0])


class BoundaryReleaseTests(unittest.TestCase):
    def test_push_authorization_is_exact_and_consistent(self):
        expected = (
            "github.event_name == 'push' && github.ref == 'refs/heads/main' && "
            f"github.event.before == '{PARENT}' && github.event.head_commit.message == '{MESSAGE}'"
        )
        self.assertEqual(TEXT.count(expected), 3)
        expression = next(line for line in TEXT.splitlines() if "BOUNDARY_RELEASE_PUSH_BRIDGE: ${{" in line)
        expression = expression.split("${{", 1)[1].split("}}", 1)[0].strip()
        fields = ["github.event_name", "github.ref", "github.event.before", "github.event.head_commit.message"]
        def admitted(values):
            code = expression
            for name, value in zip(fields, values, strict=True):
                code = code.replace(name, repr(value))
            return eval(code.replace("&&", "and"), {"__builtins__": {}}, {})
        valid = ["push", "refs/heads/main", PARENT, MESSAGE]
        self.assertTrue(admitted(valid))
        for index, invalid in enumerate(["workflow_dispatch", "refs/heads/rewrite/rust-core", "0" * 40, "another commit"]):
            with self.subTest(field=fields[index]):
                values = list(valid)
                values[index] = invalid
                self.assertFalse(admitted(values))
        self.assertNotIn("NOTIFY_LINE_PUSH_BRIDGE", TEXT)
        for key in ["CODEX_SOURCE_SHA", "RALD5_SOURCE_SHA", "BOUNDARY_RELEASE_ACCEPTED_SOURCE_SHA"]:
            self.assertIn(f"  {key}: '{SOURCE}'", TEXT)

    def test_actual_shell_decision_requires_exact_authenticated_baseline(self):
        start = TEXT.index('          test "$BOUNDARY_RELEASE_PUSH_BRIDGE" = true -o')
        end = TEXT.index('          if test "$RALD4_POSITIVE_GATE" = true; then', start)
        script = textwrap.dedent(TEXT[start:end])
        stable = {"current_version": "0.160.0", "current_generation": CURRENT, "current_release_sequence": "34", "release_sequence": "35"}
        for key in stable:
            script = script.replace("${{ steps.stable.outputs." + key + " }}", "${STABLE_" + key + "}")
        # The production expressions are single-quoted. Expand only fixture values,
        # matching GitHub's interpolation before bash sees the actual run block.
        for key, value in stable.items():
            script = script.replace("'${STABLE_" + key + "}'", '"$STABLE_' + key + '"')
        script = "set -euo pipefail\n" + script + '\ntest "$candidate" = true\ntest "$acceptance_suffix" = -boundary-release\n'
        env = dict(os.environ)
        for key in ["RALD4_POSITIVE_GATE", "RALD5_SAME_VERSION_ACCEPTANCE", "RALD45_TRANSITION_STAGE", "RALD45_TRANSITION_PROMOTE", "RALD5_NEGATIVE_GATE", "LEGACY_LAG_JUMP_REMEDIATION", "UX1_SAME_VERSION_DEPLOY", "UPDATE_PROGRESS_SAME_VERSION_DEPLOY", "EXACT_CURRENT_FASTPATH_SAME_VERSION_DEPLOY", "NO_EMOJI_SAME_VERSION_DEPLOY", "DOWNLOAD_SIZE_SAME_VERSION_DEPLOY"]:
            env[key] = "false"
        env.update(BOUNDARY_RELEASE_PUSH_BRIDGE="true", GITHUB_EVENT_NAME="push", GITHUB_REF="refs/heads/main", RALD5_PUBLICATION_AUTHORIZED="true", CODEX_SOURCE_SHA=SOURCE, BOUNDARY_RELEASE_ACCEPTED_SOURCE_SHA=SOURCE, BOUNDARY_RELEASE_TARGET_VERSION="0.160.0", BOUNDARY_RELEASE_CURRENT_GENERATION=CURRENT, BOUNDARY_RELEASE_CURRENT_SEQUENCE="34", BOUNDARY_RELEASE_TARGET_SEQUENCE="35", BOUNDARY_RELEASE_TARGET_ARCHIVE_SHA256="7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c", upstream_version="0.160.0", archive_sha256="7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c", candidate="false")
        env.update({"STABLE_" + key: value for key, value in stable.items()})
        self.assertEqual(subprocess.run(["bash", "-c", script], env=env, capture_output=True).returncode, 0)
        invalid = {"BOUNDARY_RELEASE_PUSH_BRIDGE": "false", "CODEX_SOURCE_SHA": "0" * 40, "STABLE_current_generation": "another-generation", "STABLE_current_release_sequence": "33", "STABLE_release_sequence": "36", "STABLE_current_version": "0.159.0", "upstream_version": "0.161.0", "archive_sha256": "0" * 64, "RALD5_PUBLICATION_AUTHORIZED": "false", "RALD5_NEGATIVE_GATE": "true", "candidate": "true"}
        for key, value in invalid.items():
            with self.subTest(field=key):
                self.assertNotEqual(subprocess.run(["bash", "-c", script], env=dict(env, **{key: value}), capture_output=True).returncode, 0)

    def run_component(self, mutations=None, old_mutations=None, corrupt=None, duplicate=None, reverse=False):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "core").write_bytes(b"new Core artifact")
            (root / "manager").write_bytes(b"new Manager artifact")
            old = [("generation_id", CURRENT), ("runtime_digest", "b" * 64), ("core_artifact_digest", OLD_CORE), ("manager_artifact_digest", OLD_MANAGER), ("patch_policy_id", "termux-fd-remap-v2"), ("patch_report", "qualified UDS report"), ("helper", "first"), ("helper", "second")]
            new_values = {"generation_id": NEW, "core_artifact_digest": hashlib.sha256((root / "core").read_bytes()).hexdigest(), "manager_artifact_digest": hashlib.sha256((root / "manager").read_bytes()).hexdigest()}
            new_values.update(mutations or {})
            new = [(key, new_values.get(key, value)) for key, value in old]
            old = [(key, (old_mutations or {}).get(key, value)) for key, value in old]
            if duplicate:
                old.append((duplicate, dict(old)[duplicate]))
                new.append((duplicate, dict(new)[duplicate]))
            if reverse:
                new.reverse()
            for name, rows in [("old", old), ("new", new)]:
                (root / name).write_text("codex-local-generation-v2\n" + "".join(key + "\t" + value + "\n" for key, value in rows))
            if corrupt:
                (root / corrupt).write_bytes(b"wrong downloaded component")
            script = root / "qualifier.py"
            script.write_text(qualifier())
            return subprocess.run([sys.executable, str(script), str(root / "old"), str(root / "new"), NEW, str(root / "core"), str(root / "manager")], capture_output=True).returncode

    def test_changed_core_and_manager_pass(self):
        self.assertEqual(self.run_component(), 0)

    def test_core_only_or_manager_only_rejected(self):
        for key, digest in [("core_artifact_digest", OLD_CORE), ("manager_artifact_digest", OLD_MANAGER)]:
            with self.subTest(component=key):
                self.assertNotEqual(self.run_component({key: digest}), 0)

    def test_downloaded_component_mismatch_rejected(self):
        for component in ["core", "manager"]:
            with self.subTest(component=component):
                self.assertNotEqual(self.run_component(corrupt=component), 0)

    def test_unrelated_fields_or_identity_change_rejected(self):
        for key in ["generation_id", "runtime_digest", "patch_policy_id", "patch_report", "helper"]:
            with self.subTest(field=key):
                self.assertNotEqual(self.run_component({key: "unexpected"}), 0)
        self.assertNotEqual(self.run_component(reverse=True), 0)

    def test_wrong_baseline_or_duplicate_scalar_rejected(self):
        for key in ["generation_id", "core_artifact_digest", "manager_artifact_digest"]:
            with self.subTest(field=key):
                self.assertNotEqual(self.run_component(old_mutations={key: "unexpected"}), 0)
                self.assertNotEqual(self.run_component(duplicate=key), 0)

    def test_runtime_inventory_and_noop_gate_remain_bound(self):
        step = TEXT.split("      - name: Bind boundary release to changed Core and Manager only", 1)[1].split("      - name:", 1)[0]
        body = textwrap.dedent(step.split("        run: |\n", 1)[1].split("          python3 - ", 1)[0])
        paths = ["runtime", "codex-code-mode-host", "helpers/0", "helpers/1"]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            candidate = root / "rald3-candidate/candidate"
            stable = root / "rald3-stable"
            helper = root / "source/.github/scripts/rald3_preflight.py"
            helper.parent.mkdir(parents=True)
            helper.write_bytes(subprocess.check_output(["git", "show", SOURCE + ":.github/scripts/rald3_preflight.py"], cwd=WORKFLOW.parent))
            stable.mkdir()
            inventory = []
            for rel in sorted(paths):
                path = candidate / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(rel.encode())
                path.chmod(0o755)
                inventory.append(f"file\t{rel}\t{hashlib.sha256(path.read_bytes()).hexdigest()}\t0755\n")
            headers = [("generation_id", CURRENT), ("release_sequence", "34"), ("channel", "stable"), ("expected_platform", "android"), ("expected_architecture", "aarch64"), ("core_api_identity", "core-api-v1"), ("persistent_schema_identity", "schema-v1"), ("release_public_key", "11" * 32), ("file_count", "4")]
            (stable / "release.manifest").write_text("codex-release-v4\n" + "".join(key + "\t" + value + "\n" for key, value in headers) + "".join(inventory))
            env = dict(os.environ, RUNNER_TEMP=tmp)
            self.assertEqual(subprocess.run(["bash", "-c", body], cwd=root, env=env, capture_output=True).returncode, 0)
            for rel in paths:
                path = candidate / rel
                with self.subTest(path=rel, fault="bytes"):
                    path.write_bytes(b"changed upstream artifact")
                    self.assertNotEqual(subprocess.run(["bash", "-c", body], cwd=root, env=env, capture_output=True).returncode, 0)
                    path.write_bytes(rel.encode())
                with self.subTest(path=rel, fault="mode"):
                    path.chmod(0o644)
                    self.assertNotEqual(subprocess.run(["bash", "-c", body], cwd=root, env=env, capture_output=True).returncode, 0)
                    path.chmod(0o755)
        self.assertIn("needs.producer.outputs.boundary_release_push_bridge", TEXT)


if __name__ == "__main__":
    unittest.main()

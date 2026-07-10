#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'doctor: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B - <<'PYTHON'
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory

from codex_termux import doctor, registry, schemas


def write_executable(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    path.chmod(0o755)


def make_runtime(runtime_dir: Path, builder: Path, runtime_bytes: bytes, raw_sha256: str, code_host_bytes: bytes) -> str:
    runtime_dir.mkdir(parents=True, exist_ok=True)
    runtime = runtime_dir / "codex"
    runtime.write_bytes(runtime_bytes)
    runtime.chmod(0o755)
    code_host = runtime_dir / "codex-code-mode-host"
    code_host.write_bytes(code_host_bytes)
    code_host.chmod(0o755)
    runtime_sha = hashlib.sha256(runtime_bytes).hexdigest()
    manifest = {
        "patch_policy": "termux-fd-remap-v1",
        "builder_sha256": hashlib.sha256(builder.read_bytes()).hexdigest(),
        "raw_sha256": raw_sha256,
        "runtime_sha256": runtime_sha,
        "code_mode_host_sha256": hashlib.sha256(code_host_bytes).hexdigest(),
    }
    (runtime_dir / "runtime-build.json").write_text(json.dumps(manifest) + "\n", encoding="utf-8")
    return runtime_sha


with TemporaryDirectory() as tmp:
    root = Path(tmp)
    manager = root / "manager"
    runtime_store = root / "runtime-store"
    raw_store = root / "raw-store"
    state_file = root / "state.json"
    registry_file = root / "registry.json"
    builder = manager / "build-runtime.py"

    write_executable(manager / "managed.sh", "#!/bin/sh\nexit 0\n")
    (manager / "lib.sh").write_text("# lib\n", encoding="utf-8")
    write_executable(builder, "#!/usr/bin/env python3\n")
    write_executable(manager / "bwrap-termux-compat.py", "#!/bin/sh\nexit 0\n")
    write_executable(manager / "rg-termux-shim.sh", "#!/bin/sh\nexit 0\n")
    write_executable(manager / "termux-notify.sh", "#!/bin/sh\nexit 0\n")
    write_executable(manager / "codex-turn-notify.sh", "#!/bin/sh\nexit 0\n")
    (manager / "wrapper-version.env").write_text(
        "\n".join(
            [
                "CODEX_TERMUX_WRAPPER_VERSION=260627-36",
                "CODEX_TERMUX_WRAPPER_CHANNEL=termux",
                "CODEX_TERMUX_WRAPPER_REPO=humtr/codex",
                "CODEX_TERMUX_WRAPPER_COMMIT=2497a22aadc2",
                "CODEX_TERMUX_WRAPPER_INSTALLED_AT=2026-06-27T23:22:48+09:00",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    current_runtime = runtime_store / "runtime-a"
    verified_runtime = runtime_store / "runtime-a"
    raw_runtime = raw_store / "raw-a"
    raw_vendor = raw_runtime / "vendor/aarch64-unknown-linux-musl"
    raw_bytes = (
        b"prefix /etc/resolv.conf middle /etc/codex/config.toml "
        b"middle /etc/codex/requirements.toml middle /etc/codex/managed_config.toml suffix"
    )
    runtime_bytes = (
        raw_bytes.replace(b"/etc/resolv.conf", b"/proc/self/fd/33")
        .replace(b"/etc/codex/config.toml", b"/dev/fd/34/config.toml")
        .replace(b"/etc/codex/requirements.toml", b"/dev/fd/34/requirements.toml")
        .replace(b"/etc/codex/managed_config.toml", b"/dev/fd/34/managed_config.toml")
    )
    code_host_bytes = b"upstream code mode host"

    runtime_sha = make_runtime(current_runtime, builder, runtime_bytes, hashlib.sha256(raw_bytes).hexdigest(), code_host_bytes)
    write_executable(current_runtime / "codex-path/bwrap", "#!/bin/sh\nexit 0\n")
    write_executable(current_runtime / "codex-path/rg", "#!/bin/sh\nexit 0\n")
    write_executable(current_runtime / "codex-path/rg.real", "#!/bin/sh\nexit 0\n")
    write_executable(current_runtime / "codex-resources/bwrap", "#!/bin/sh\nexit 0\n")
    write_executable(current_runtime / "codex-resources/zsh/bin/zsh", "#!/bin/sh\nexit 0\n")
    raw_bin = raw_vendor / "bin/codex"
    raw_bin.parent.mkdir(parents=True, exist_ok=True)
    raw_bin.write_bytes(raw_bytes)
    raw_code_host = raw_vendor / "bin/codex-code-mode-host"
    raw_code_host.write_bytes(code_host_bytes)
    raw_code_host.chmod(0o755)
    raw_sha = hashlib.sha256(raw_bytes).hexdigest()
    (root / "resolv.conf").write_text("nameserver 1.1.1.1\n", encoding="utf-8")
    (root / "cert.pem").write_text("CERT\n", encoding="utf-8")

    tuple_id = registry.record(
        registry_file=registry_file,
        version="0.142.3-linux-arm64",
        raw_sha256=raw_sha,
        runtime_sha256=runtime_sha,
        package_spec="@openai/codex@0.142.3-linux-arm64",
        runtime_path=str(current_runtime),
        wrapper_version="260627-36",
        wrapper_commit="2497a22aadc2",
        runtime_store_dir=runtime_store,
        updated_at="2026-06-27T23:22:48+09:00",
        smoke_tested_at="2026-06-27T23:22:48+09:00",
        raw_path=str(raw_runtime),
    )
    registry.activate_existing_tuple(registry_file, tuple_id)

    state_file.write_text(
        json.dumps(
            schemas.build_state_v3(
                version="0.142.3-linux-arm64",
                raw_sha256=raw_sha,
                runtime_sha256=runtime_sha,
                package_spec="@openai/codex@0.142.3-linux-arm64",
                active_tuple_id=tuple_id,
                wrapper_version="260627-36",
                wrapper_commit="2497a22aadc2",
                updated_at="2026-06-27T23:22:48+09:00",
                verified_tuple_id=tuple_id,
                verified_at="2026-06-27T23:22:48+09:00",
            )
        )
        + "\n",
        encoding="utf-8",
    )

    current_link = root / "current"
    verified_link = root / "verified"
    raw_link = root / "raw"
    current_link.symlink_to(current_runtime)
    verified_link.symlink_to(verified_runtime)
    raw_link.symlink_to(raw_runtime)

    report = doctor.build_report(
        doctor.DoctorInputs(
            runtime=current_runtime / "codex",
            current_link=current_link,
            verified_link=verified_link,
            raw_link=raw_link,
            manager_dir=manager,
            runtime_store=runtime_store,
            raw_store=raw_store,
            raw_vendor=raw_vendor,
            resolv_conf=root / "resolv.conf",
            cert_file=root / "cert.pem",
            state_file=state_file,
            registry_file=registry_file,
            version="0.142.3-linux-arm64",
            raw_sha256=raw_sha,
            runtime_sha256=runtime_sha,
            prefix=Path("/data/data/com.termux/files/usr"),
            runtime_builder=builder,
            patch_policy="termux-fd-remap-v1",
        )
    )

    wrapper = report["wrapper"]
    assert report["schema"] == 2, report
    assert report["overallStatus"] == "ok", report
    assert report["activeTupleId"] == tuple_id, report
    assert report["verifiedTupleId"] == tuple_id, report
    for key in (
        "raw_code_mode_host",
        "code_mode_host",
        "code_mode_host_hash",
        "raw_hash",
        "runtime_hash",
        "registry_active_tuple",
        "registry_current_match",
        "registry_verified_match",
        "current_verified_match",
    ):
        assert report["checks"][key] is True, (key, report["checks"])
    assert wrapper["version"] == "260627-36", wrapper
    assert wrapper["repo"] == "humtr/codex", wrapper
    assert wrapper["commit"] == "2497a22aadc2", wrapper
    assert wrapper["installedAt"] == "2026-06-27T23:22:48+09:00", wrapper
    assert wrapper["channel"] == "termux", wrapper

    code_host = current_runtime / "codex-code-mode-host"
    missing_code_host = current_runtime / ".codex-code-mode-host.missing"
    code_host.rename(missing_code_host)
    missing_host = doctor.build_report(
        doctor.DoctorInputs(
            runtime=current_runtime / "codex",
            current_link=current_link,
            verified_link=verified_link,
            raw_link=raw_link,
            manager_dir=manager,
            runtime_store=runtime_store,
            raw_store=raw_store,
            raw_vendor=raw_vendor,
            resolv_conf=root / "resolv.conf",
            cert_file=root / "cert.pem",
            state_file=state_file,
            registry_file=registry_file,
            version="0.142.3-linux-arm64",
            raw_sha256=raw_sha,
            runtime_sha256=runtime_sha,
            prefix=Path("/data/data/com.termux/files/usr"),
            runtime_builder=builder,
            patch_policy="termux-fd-remap-v1",
        )
    )
    missing_code_host.rename(code_host)
    assert missing_host["overallStatus"] == "fail", missing_host
    assert missing_host["checks"]["code_mode_host"] is False, missing_host["checks"]
    assert missing_host["checks"]["code_mode_host_hash"] is False, missing_host["checks"]

    mismatched = doctor.build_report(
        doctor.DoctorInputs(
            runtime=current_runtime / "codex",
            current_link=current_link,
            verified_link=verified_link,
            raw_link=raw_link,
            manager_dir=manager,
            runtime_store=runtime_store,
            raw_store=raw_store,
            raw_vendor=raw_vendor,
            resolv_conf=root / "resolv.conf",
            cert_file=root / "cert.pem",
            state_file=state_file,
            registry_file=registry_file,
            version="0.142.3-linux-arm64",
            raw_sha256=raw_sha,
            runtime_sha256="0" * 64,
            prefix=Path("/data/data/com.termux/files/usr"),
            runtime_builder=builder,
            patch_policy="termux-fd-remap-v1",
        )
    )
    assert mismatched["overallStatus"] == "fail", mismatched
    assert mismatched["checks"]["runtime_hash"] is False, mismatched["checks"]

    buffer = io.StringIO()
    assert doctor.render_human(report, buffer) == 0
    output = buffer.getvalue()
    assert "Wrapper" in output, output
    assert "260627-36" in output, output
    assert "2497a22aadc2" in output, output
    assert "humtr/codex" in output, output
    assert "2026-06-27T23:22:48+09:00" in output, output
    assert "installedAt" not in output, output

print("doctor: ok")
PYTHON

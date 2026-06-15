"""Wrapper doctor machine-report builder."""

from __future__ import annotations

import filecmp
import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from . import migration, registry, schemas
from .hashing import sha256_file

@dataclass(frozen=True)
class DoctorReportInputs:
    runtime: Path
    current_link: Path
    verified_link: Path
    raw_link: Path
    manager_dir: Path
    runtime_store: Path
    raw_store: Path
    raw_vendor: Path
    resolv_conf: Path
    cert_file: Path
    state_file: Path
    registry_file: Path
    migration_report: Path
    legacy_store: Path
    version: str
    raw_sha256: str
    runtime_sha256: str
    prefix: Path
    runtime_builder: Path
    patch_policy: str
    network_boundary: schemas.NetworkBoundaryReport

def build_doctor_report(inputs: DoctorReportInputs) -> schemas.DoctorReportV4:
    manifest = _read_json_dict(inputs.current_link / "runtime-build.json")
    checks = _base_checks(inputs, manifest)
    active_tuple_id, verified_tuple_id = _registry_checks(inputs, checks)
    migration_status = _migration_status(
        inputs.migration_report, inputs.legacy_store, inputs.runtime_store.parent
    )
    report = {
        "schema": 4,
        "overallStatus": "ok" if all(checks.values()) else "fail",
        "version": inputs.version,
        "raw_sha256": inputs.raw_sha256,
        "runtime_sha256": inputs.runtime_sha256,
        "paths": _report_paths(inputs),
        "activeTupleId": active_tuple_id,
        "verifiedTupleId": verified_tuple_id,
        "migration": migration_status,
        "termuxDelta": _termux_delta(),
        "networkBoundary": inputs.network_boundary,
        "buildManifest": manifest,
        "checks": checks,
    }
    return report


def _base_checks(
    inputs: DoctorReportInputs,
    manifest: dict[str, Any],
) -> dict[str, bool]:
    runtime_dir = inputs.current_link
    path_bwrap = runtime_dir / "codex-path/bwrap"
    bundled_bwrap = runtime_dir / "codex-resources/bwrap"
    rg = runtime_dir / "codex-path/rg"
    manager_required = _manager_required(inputs.manager_dir)
    actual_raw_sha = _safe_hash(inputs.raw_vendor / "bin/codex")
    actual_runtime_sha = _safe_hash(inputs.runtime)
    checks = {
        "runtime": inputs.runtime.exists() and os.access(inputs.runtime, os.X_OK),
        "raw": (inputs.raw_vendor / "bin/codex").exists(),
        "manager": _manager_ok(inputs.manager_dir, manager_required),
        "runtime_store": inputs.runtime_store.is_dir(),
        "raw_store": inputs.raw_store.is_dir(),
        "current_pointer": inputs.current_link.is_symlink(),
        "verified_pointer": inputs.verified_link.is_symlink(),
        "raw_pointer": inputs.raw_link.is_symlink(),
        "current_in_store": _managed_target(inputs.current_link, inputs.runtime_store),
        "verified_in_store": _managed_target(inputs.verified_link, inputs.runtime_store),
        "raw_in_store": _managed_target(inputs.raw_link, inputs.raw_store),
        "current_verified_match": bool(_resolved(inputs.current_link) and _resolved(inputs.current_link) == _resolved(inputs.verified_link)),
        "path_bwrap": path_bwrap.exists() and os.access(path_bwrap, os.X_OK),
        "bundled_bwrap": bundled_bwrap.exists() and os.access(bundled_bwrap, os.X_OK),
        "rg": rg.exists() and os.access(rg, os.X_OK),
        "rg_real": (runtime_dir / "codex-path/rg.real").exists(),
        "support_bwrap_match": _files_match(inputs.manager_dir / "bwrap-termux-compat.py", path_bwrap),
        "support_rg_match": _files_match(inputs.manager_dir / "rg-termux-shim.sh", rg),
        "zsh": (runtime_dir / "codex-resources/zsh/bin/zsh").exists(),
        "resolv": inputs.resolv_conf.exists() and os.access(inputs.resolv_conf, os.R_OK),
        "cert": inputs.cert_file.exists() and os.access(inputs.cert_file, os.R_OK),
        "state": inputs.state_file.exists(),
        "registry": inputs.registry_file.exists(),
        "raw_hash": bool(actual_raw_sha and actual_raw_sha == inputs.raw_sha256 == manifest.get("raw_sha256")),
        "runtime_hash": bool(actual_runtime_sha and actual_runtime_sha == inputs.runtime_sha256 and actual_runtime_sha == manifest.get("runtime_sha256")),
        "build_manifest": bool(manifest.get("patch_policy") == inputs.patch_policy and manifest.get("builder_sha256") == _safe_hash(inputs.runtime_builder)),
        "dns_only_patch": _dns_only_patch(inputs.raw_vendor, inputs.runtime),
        "network_boundary": inputs.network_boundary.get("overallStatus") != "fail",
        "bwrap_exec": _run_ok([str(path_bwrap), "--ro-bind", "/", "/", "--", str(inputs.prefix / "bin/true")]),
        "rg_exec": _run_ok([str(rg), "--version"]),
        "dns_patch": _dns_patch_check(inputs.runtime),
    }
    return checks


def _registry_checks(
    inputs: DoctorReportInputs,
    checks: dict[str, bool],
) -> tuple[str, str]:
    registry_data = _validated_registry(inputs.registry_file)
    state_data = _validated_state(inputs.state_file)
    active_tuple_id = registry_data.get("active_tuple_id", "")
    verified_tuple_id = state_data.get("verified_tuple_id", "") or registry_data.get("verified_tuple_id", "")
    active_runtime_path = registry_data.get("runtime", {}).get(active_tuple_id, {}).get("path", "")
    verified_runtime_path = registry_data.get("runtime", {}).get(verified_tuple_id, {}).get("path", "")
    checks["registry_active_tuple"] = bool(active_tuple_id and active_tuple_id in registry_data.get("runtime", {}))
    checks["registry_current_match"] = bool(active_runtime_path and _resolved(Path(active_runtime_path)) == _resolved(inputs.current_link))
    checks["registry_verified_match"] = bool(verified_runtime_path and _resolved(Path(verified_runtime_path)) == _resolved(inputs.verified_link))
    return active_tuple_id, verified_tuple_id


def _report_paths(inputs: DoctorReportInputs) -> dict[str, str]:
    return {
        "runtime": str(inputs.runtime),
        "manager": str(inputs.manager_dir),
        "current": str(inputs.current_link),
        "current_target": _resolved(inputs.current_link),
        "verified": str(inputs.verified_link),
        "verified_target": _resolved(inputs.verified_link),
        "raw": str(inputs.raw_link),
        "raw_target": _resolved(inputs.raw_link),
        "runtime_store": str(inputs.runtime_store),
        "raw_store": str(inputs.raw_store),
        "raw_vendor": str(inputs.raw_vendor),
        "state": str(inputs.state_file),
        "registry": str(inputs.registry_file),
    }


def _termux_delta() -> dict[str, str]:
    return {
        "browserLogin": "termux-open-url when available",
        "bwrap": "runtime-private quiet no-namespace compatibility launcher",
        "codexSelfExe": "managed runtime",
        "ldLibraryPath": "sanitized before runtime execution",
        "runtimePatch": "official linux-arm64 raw package rebuilt into Termux-managed runtime",
    }


def _manager_required(manager_dir: Path) -> tuple[Path, ...]:
    return (
        manager_dir / "managed.sh",
        manager_dir / "lib.sh",
        manager_dir / "build-runtime.py",
        manager_dir / "bwrap-termux-compat.py",
        manager_dir / "rg-termux-shim.sh",
        manager_dir / "wrapper-version.env",
    )

def build_network_boundary_report(
    *,
    baseline: dict[str, Any],
    network_off: dict[str, Any],
    network_on: dict[str, Any],
    network_reset: dict[str, Any],
    exit_codes: tuple[int, int, int, int],
) -> schemas.NetworkBoundaryReport:
    baseline_ok = exit_codes[0] == 0 and baseline.get("socket_allowed") is True
    off_ok = (
        exit_codes[1] == 0
        and network_off.get("socket_allowed") is False
        and network_off.get("socket_errno") == 1
        and network_off.get("no_new_privs") == 1
        and network_off.get("seccomp") == 2
    )
    on_ok = exit_codes[2] == 0 and network_on.get("socket_allowed") is True
    reset_ok = (
        exit_codes[3] == 0
        and network_reset.get("socket_allowed") is False
        and network_reset.get("socket_errno") == 1
    )
    if not baseline_ok:
        overall = "inconclusive"
    elif off_ok and on_ok and reset_ok:
        overall = "ok"
    else:
        overall = "fail"
    return {
        "overallStatus": overall,
        "checks": {
            "baseline_socket": baseline_ok,
            "network_off": off_ok,
            "network_on": on_ok,
            "network_reset": reset_ok,
        },
        "reports": {
            "baseline": baseline,
            "off": network_off,
            "on": network_on,
            "reset": network_reset,
        },
    }

def socket_probe() -> dict[str, Any]:
    import socket

    status: dict[str, Any] = {}
    for line in Path("/proc/self/status").read_text(encoding="utf-8").splitlines():
        if line.startswith("NoNewPrivs:"):
            status["no_new_privs"] = int(line.split()[1])
        elif line.startswith("Seccomp:"):
            status["seccomp"] = int(line.split()[1])
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.close()
        status["socket_allowed"] = True
        status["socket_errno"] = None
    except OSError as exc:
        status["socket_allowed"] = False
        status["socket_errno"] = exc.errno
    return status

def _read_json_dict(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}

def _validated_registry(path: Path) -> schemas.RegistryV3 | dict[str, Any]:
    try:
        return registry.load(path)
    except Exception:
        return {}

def _validated_state(path: Path) -> schemas.StateV3 | dict[str, Any]:
    try:
        return schemas.validate_state_v3(schemas.load_json_object(path))
    except Exception:
        return {}

def _resolved(path: Path) -> str:
    try:
        return str(path.resolve())
    except Exception:
        return ""

def _managed_target(link: Path, store: Path) -> bool:
    try:
        target = link.resolve()
        return link.is_symlink() and target.is_dir() and target.parent == store.resolve()
    except Exception:
        return False

def _safe_hash(path: Path) -> str:
    try:
        return sha256_file(path)
    except Exception:
        return ""

def _migration_status(
    report_file: Path,
    legacy_store: Path,
    store_root: Path,
) -> dict[str, Any]:
    try:
        return migration.migration_status(report_file, legacy_store, store_root)
    except Exception as exc:
        return {
            "status": "issues",
            "report": str(report_file),
            "legacyStore": str(legacy_store),
            "imported": [],
            "skipped": [],
            "error": str(exc),
        }

def _manager_ok(manager_dir: Path, required: tuple[Path, ...]) -> bool:
    return bool(
        manager_dir.is_dir()
        and all(path.exists() for path in required)
        and all(
            os.access(path, os.X_OK)
            for path in required
            if path.name
            in {"managed.sh", "build-runtime.py", "bwrap-termux-compat.py", "rg-termux-shim.sh"}
        )
        and all(
            os.access(path, os.R_OK)
            for path in required
            if path.name in {"lib.sh", "wrapper-version.env"}
        )
    )

def _files_match(left: Path, right: Path) -> bool:
    try:
        return left.exists() and right.exists() and filecmp.cmp(left, right, shallow=False)
    except Exception:
        return False

def _dns_only_patch(raw_vendor: Path, runtime: Path) -> bool:
    try:
        raw_bytes = (raw_vendor / "bin/codex").read_bytes()
        runtime_bytes = runtime.read_bytes()
        return (
            raw_bytes.replace(b"/etc/resolv.conf", b"/proc/self/fd/33")
            == runtime_bytes
        )
    except Exception:
        return False

def _run_ok(argv: list[str]) -> bool:
    try:
        return (
            subprocess.run(
                argv,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=10,
            ).returncode
            == 0
        )
    except Exception:
        return False

def _dns_patch_check(runtime: Path) -> bool:
    try:
        strings = subprocess.check_output(
            ["strings", str(runtime)], text=True, errors="replace", timeout=10
        )
        return "/proc/self/fd/33" in strings and "/etc/resolv.conf" not in strings
    except Exception:
        return False

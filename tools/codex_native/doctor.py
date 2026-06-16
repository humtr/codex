"""Focused diagnostics for the Codex Termux native wrapper."""

from __future__ import annotations

import filecmp
import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TextIO

from . import registry, schemas
from .hashing import sha256_file


DETAIL_LABEL_WIDTH = 24
SEPARATOR_WIDTH = 61


@dataclass(frozen=True)
class DoctorInputs:
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
    version: str
    raw_sha256: str
    runtime_sha256: str
    prefix: Path
    runtime_builder: Path
    patch_policy: str


def build_report(inputs: DoctorInputs) -> dict[str, Any]:
    manifest = _read_json(inputs.current_link / "runtime-build.json")
    checks = _checks(inputs, manifest)
    active_tuple_id, verified_tuple_id = _registry_checks(inputs, checks)
    return {
        "schema": 2,
        "overallStatus": "ok" if all(checks.values()) else "fail",
        "version": inputs.version,
        "raw_sha256": inputs.raw_sha256,
        "runtime_sha256": inputs.runtime_sha256,
        "activeTupleId": active_tuple_id,
        "verifiedTupleId": verified_tuple_id,
        "paths": _paths(inputs),
        "termuxDelta": {
            "runtimePatch": "replace /etc/resolv.conf with /proc/self/fd/33",
            "bwrap": "Termux no-namespace compatibility launcher",
            "rg": "managed shim that preserves the upstream rg binary",
            "env": "sanitize LD_* and npm/bun management variables before launch",
        },
        "buildManifest": manifest,
        "checks": checks,
    }


def _checks(inputs: DoctorInputs, manifest: dict[str, Any]) -> dict[str, bool]:
    runtime_dir = inputs.current_link
    path_bwrap = runtime_dir / "codex-path/bwrap"
    bundled_bwrap = runtime_dir / "codex-resources/bwrap"
    rg = runtime_dir / "codex-path/rg"
    rg_real = runtime_dir / "codex-path/rg.real"
    raw_binary = inputs.raw_vendor / "bin/codex"
    actual_raw_sha = _safe_hash(raw_binary)
    actual_runtime_sha = _safe_hash(inputs.runtime)
    return {
        "runtime": inputs.runtime.exists() and os.access(inputs.runtime, os.X_OK),
        "raw": raw_binary.exists(),
        "manager": _manager_ok(inputs.manager_dir),
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
        "rg_real": rg_real.exists(),
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
        "dns_only_patch": _dns_only_patch(raw_binary, inputs.runtime),
        "bwrap_exec": _run_ok([str(path_bwrap), "--ro-bind", "/", "/", "--", str(inputs.prefix / "bin/true")]),
        "rg_exec": _run_ok([str(rg), "--version"]),
        "dns_patch": _dns_patch_check(inputs.runtime),
    }


def _registry_checks(inputs: DoctorInputs, checks: dict[str, bool]) -> tuple[str, str]:
    registry_data = _load_registry(inputs.registry_file)
    state_data = _load_state(inputs.state_file)
    active_tuple_id = str(registry_data.get("active_tuple_id", ""))
    verified_tuple_id = str(state_data.get("verified_tuple_id", "") or registry_data.get("verified_tuple_id", ""))
    active_runtime_path = registry_data.get("runtime", {}).get(active_tuple_id, {}).get("path", "")
    verified_runtime_path = registry_data.get("runtime", {}).get(verified_tuple_id, {}).get("path", "")
    checks["registry_active_tuple"] = bool(active_tuple_id and active_tuple_id in registry_data.get("runtime", {}))
    checks["registry_current_match"] = bool(active_runtime_path and _resolved(Path(active_runtime_path)) == _resolved(inputs.current_link))
    checks["registry_verified_match"] = bool(verified_runtime_path and _resolved(Path(verified_runtime_path)) == _resolved(inputs.verified_link))
    return active_tuple_id, verified_tuple_id


def _paths(inputs: DoctorInputs) -> dict[str, str]:
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


def render_human(report: dict[str, Any], output: TextIO | None = None) -> int:
    import sys

    out = output or sys.stdout
    checks = report.get("checks", {})
    paths = report.get("paths", {})
    color = os.environ.get("NO_COLOR") is None and hasattr(out, "isatty") and out.isatty()
    counts = {"ok": 0, "fail": 0, "idle": 0, "notes": 0, "warn": 0}

    def row(ok: bool, name: str, summary: str) -> None:
        counts["ok" if ok else "fail"] += 1
        print(
            f"  {_status_marker_slot(ok, color)}{name:<14} {_style_description(summary, ok, color)}",
            file=out,
        )

    def detail(name: str, value: object) -> None:
        label = f"{name:<{DETAIL_LABEL_WIDTH}}"
        print(f"    {_detail_marker(False, color)} {_detail_label(label, color)} {_detail_value(str(value), color)}", file=out)

    header_suffix = f"version {report.get('version', 'unknown')} · status {report.get('overallStatus', 'unknown')}"
    print(f"{_bold('Codex Termux Wrapper Doctor', color)} {_dim(header_suffix, color)}", file=out)

    print(file=out)
    print(_bold("Runtime", color), file=out)
    row(bool(checks.get("runtime")), "runtime", "patched runtime executable exists")
    detail("path", paths.get("runtime", "missing"))
    row(bool(checks.get("raw")), "raw", "official raw binary cache exists")
    detail("vendor", paths.get("raw_vendor", "missing"))
    row(bool(checks.get("runtime_hash")) and bool(checks.get("raw_hash")), "hashes", "state, raw, runtime, and manifest hashes agree")
    row(bool(checks.get("dns_only_patch")) and bool(checks.get("dns_patch")), "dns patch", "only the resolver fd33 patch is present")
    row(bool(checks.get("build_manifest")), "manifest", "runtime-build.json matches builder and patch policy")

    print(file=out)
    print(_bold("Support", color), file=out)
    row(bool(checks.get("manager")), "manager", "managed support files are installed")
    detail("path", paths.get("manager", "missing"))
    row(bool(checks.get("path_bwrap")) and bool(checks.get("bwrap_exec")), "bwrap", "Termux compatibility launcher works")
    row(bool(checks.get("rg")) and bool(checks.get("rg_real")) and bool(checks.get("rg_exec")), "rg", "ripgrep shim and upstream rg work")
    row(bool(checks.get("support_bwrap_match")) and bool(checks.get("support_rg_match")), "support copy", "runtime shims match installed manager files")
    row(bool(checks.get("zsh")), "zsh", "bundled zsh resource exists")

    print(file=out)
    print(_bold("State", color), file=out)
    row(bool(checks.get("resolv")), "resolver", "Termux resolver source is readable")
    row(bool(checks.get("cert")), "cert", "Termux CA bundle is readable")
    row(bool(checks.get("state")) and bool(checks.get("registry")) and bool(checks.get("registry_active_tuple")), "metadata", "state and registry contain an active tuple")
    detail("state", paths.get("state", "missing"))
    detail("registry", paths.get("registry", "missing"))

    print(file=out)
    print(_bold("Store", color), file=out)
    row(bool(checks.get("current_pointer")) and bool(checks.get("current_in_store")) and bool(checks.get("registry_current_match")), "current", "current pointer matches registry")
    detail("target", paths.get("current_target", "missing"))
    row(bool(checks.get("verified_pointer")) and bool(checks.get("verified_in_store")) and bool(checks.get("registry_verified_match")), "verified", "verified pointer matches registry")
    detail("target", paths.get("verified_target", "missing"))
    row(bool(checks.get("raw_pointer")) and bool(checks.get("raw_in_store")), "raw", "raw pointer targets the managed store")
    detail("target", paths.get("raw_target", "missing"))
    row(bool(checks.get("runtime_store")) and bool(checks.get("raw_store")), "stores", "runtime and raw stores exist")
    row(bool(checks.get("current_verified_match")), "alignment", "current and verified runtime pointers agree")
    detail("active tuple", report.get("activeTupleId", "missing"))
    detail("verified tuple", report.get("verifiedTupleId", "missing"))

    print(file=out)
    print(_dim(_separator(), color), file=out)
    print(_summary_line(counts, color), file=out)
    return 0 if counts["fail"] == 0 else 1


def _separator() -> str:
    return "─" * SEPARATOR_WIDTH


def _summary_line(counts: dict[str, int], enabled: bool) -> str:
    status_ok = counts["fail"] == 0
    status = "ok" if status_ok else "failed"
    separator = _dim(" · ", enabled)
    parts = [_count_label(counts["ok"], "ok", "ok", enabled)]
    if counts["idle"] > 0:
        parts.append(_count_label(counts["idle"], "idle", "idle", enabled))
    if counts["notes"] > 0:
        parts.append(_count_label(counts["notes"], "notes", "notes", enabled))
    parts.append(_count_label(counts["warn"], "warn", "warn", enabled))
    parts.append(_count_label(counts["fail"], "fail", "fail", enabled))
    return separator.join(parts) + " " + _styled_overall_status(status, status_ok, enabled)


def _count_label(count: int, label: str, status: str, enabled: bool) -> str:
    count_text = _dim(str(count), enabled)
    if status == "ok":
        label_text = _green(label, enabled)
    elif status == "idle":
        label_text = _dim(label, enabled)
    elif status in {"notes", "warn"}:
        label_text = _orange(label, enabled)
    elif status == "fail":
        label_text = _red(label, enabled)
    else:
        label_text = label
    return f"{count_text} {label_text}"


def _styled_overall_status(status: str, ok: bool, enabled: bool) -> str:
    return _bold(_green(status, enabled) if ok else _red(status, enabled), enabled)


def _status_marker_slot(ok: bool, enabled: bool) -> str:
    return f"{_status_marker(ok, enabled)} "


def _status_marker(ok: bool, enabled: bool) -> str:
    return _green("✓", enabled) if ok else _red("✗", enabled)


def _style_description(description: str, ok: bool, enabled: bool) -> str:
    highlighted = _highlight_actions(description, enabled)
    return _dim(highlighted, enabled) if ok else highlighted


def _detail_marker(is_issue: bool, enabled: bool) -> str:
    return _orange("▸", enabled) if is_issue else " "


def _detail_label(text: str, enabled: bool) -> str:
    return _color256(text, 240, enabled)


def _detail_value(text: str, enabled: bool) -> str:
    if not enabled:
        return text
    return _style_detail_text(text, enabled)


def _style_detail_text(text: str, enabled: bool) -> str:
    parts = text.split("`")
    out = [_style_detail_plain_text(parts[0], enabled)]
    in_code = True
    for part in parts[1:]:
        out.append(_cyan(part, enabled) if in_code else _style_detail_plain_text(part, enabled))
        in_code = not in_code
    return "".join(out)


def _style_detail_plain_text(text: str, enabled: bool) -> str:
    return "".join(_style_detail_token(token, enabled) for token in _split_inclusive_whitespace(text))


def _style_detail_token(token: str, enabled: bool) -> str:
    trimmed = token.rstrip()
    suffix = token[len(trimmed):]
    bare = trimmed.rstrip(",.:;)" )
    punctuation = trimmed[len(bare):]
    return f"{_style_detail_bare_token(bare, enabled)}{punctuation}{suffix}"


def _style_detail_bare_token(bare: str, enabled: bool) -> str:
    if not bare:
        return ""
    if bare == "<redacted>":
        return _color256(_italic(bare, enabled), 244, enabled)
    if "(missing)" in bare or _is_falsy(bare):
        return _color256(bare, 240, enabled)
    if ":" in bare:
        label, value = bare.split(":", 1)
        if _is_falsy(value):
            return f"{label}:{_color256(value, 240, enabled)}"
    if bare == "ok":
        return _green(bare, enabled)
    if bare.startswith("--") or _looks_copyable(bare):
        return _cyan(bare, enabled)
    if bare in {"B", "KB", "MB", "GB", "TB", "files", "file"}:
        return _dim(bare, enabled)
    return bare


def _highlight_actions(text: str, enabled: bool) -> str:
    if not enabled:
        return text
    parts = text.split("`")
    out = [_highlight_flags(parts[0], enabled)]
    in_code = True
    for part in parts[1:]:
        out.append(_cyan(part, enabled) if in_code else _highlight_flags(part, enabled))
        in_code = not in_code
    return "".join(out)


def _highlight_flags(text: str, enabled: bool) -> str:
    return "".join(_highlight_flag_token(token, enabled) for token in _split_inclusive_whitespace(text))


def _highlight_flag_token(token: str, enabled: bool) -> str:
    trimmed = token.rstrip()
    suffix = token[len(trimmed):]
    bare = trimmed.rstrip(",.:;)" )
    punctuation = trimmed[len(bare):]
    if bare.startswith("--"):
        return f"{_cyan(bare, enabled)}{punctuation}{suffix}"
    return token


def _split_inclusive_whitespace(text: str) -> list[str]:
    if not text:
        return []
    tokens: list[str] = []
    start = 0
    for index, char in enumerate(text):
        if char.isspace():
            tokens.append(text[start:index + 1])
            start = index + 1
    if start < len(text):
        tokens.append(text[start:])
    return tokens


def _is_falsy(value: str) -> bool:
    return value.strip().lower() in {"false", "no", "absent", "missing", "not", "not-set", "none", "null", "0", ""}


def _looks_copyable(text: str) -> bool:
    return text.startswith(("http://", "https://", "wss://", "~/", "/", "./", "../"))


def _manager_ok(manager_dir: Path) -> bool:
    required = (
        manager_dir / "managed.sh",
        manager_dir / "lib.sh",
        manager_dir / "build-runtime.py",
        manager_dir / "bwrap-termux-compat.py",
        manager_dir / "rg-termux-shim.sh",
        manager_dir / "wrapper-version.env",
    )
    executable = {"managed.sh", "build-runtime.py", "bwrap-termux-compat.py", "rg-termux-shim.sh"}
    readable = {"lib.sh", "wrapper-version.env"}
    return bool(
        manager_dir.is_dir()
        and all(path.exists() for path in required)
        and all(os.access(path, os.X_OK) for path in required if path.name in executable)
        and all(os.access(path, os.R_OK) for path in required if path.name in readable)
    )


def _read_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def _load_registry(path: Path) -> dict[str, Any]:
    try:
        return registry.load(path)
    except Exception:
        return {}


def _load_state(path: Path) -> dict[str, Any]:
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


def _files_match(left: Path, right: Path) -> bool:
    try:
        return left.exists() and right.exists() and filecmp.cmp(left, right, shallow=False)
    except Exception:
        return False


def _dns_only_patch(raw_binary: Path, runtime: Path) -> bool:
    try:
        raw = raw_binary.read_bytes()
        return raw.replace(b"/etc/resolv.conf", b"/proc/self/fd/33") == runtime.read_bytes()
    except Exception:
        return False


def _run_ok(argv: list[str]) -> bool:
    try:
        return subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10, check=False).returncode == 0
    except Exception:
        return False


def _dns_patch_check(runtime: Path) -> bool:
    try:
        data = runtime.read_bytes()
        return b"/proc/self/fd/33" in data and b"/etc/resolv.conf" not in data
    except Exception:
        return False


def _bold(text: str, enabled: bool) -> str:
    return f"\033[1m{text}\033[0m" if enabled else text


def _italic(text: str, enabled: bool) -> str:
    return f"\033[3m{text}\033[0m" if enabled else text


def _dim(text: str, enabled: bool) -> str:
    return f"\033[2m{text}\033[0m" if enabled else text


def _green(text: str, enabled: bool) -> str:
    return _color256(text, 10, enabled)


def _amber(text: str, enabled: bool) -> str:
    return _color256(text, 220, enabled)


def _orange(text: str, enabled: bool) -> str:
    return _color256(text, 214, enabled)


def _red(text: str, enabled: bool) -> str:
    return _color256(text, 196, enabled)


def _cyan(text: str, enabled: bool) -> str:
    return _color256(text, 117, enabled)


def _color256(text: str, code: int, enabled: bool) -> str:
    return f"\033[38;5;{code}m{text}\033[39m" if enabled else text

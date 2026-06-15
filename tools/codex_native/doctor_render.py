"""Wrapper doctor human renderer."""

from __future__ import annotations

import json
import os
import sys
from typing import TextIO

from . import schemas


def render_human_doctor(
    report: schemas.DoctorReportV4,
    *,
    stream: TextIO | None = None,
    use_color: bool | None = None,
) -> int:
    output = stream if stream is not None else sys.stdout
    checks = report.get("checks", {})
    paths = report.get("paths", {})
    network = report.get("networkBoundary", {})
    manifest = report.get("buildManifest", {})
    migration = report.get("migration", {})
    status = report.get("overallStatus", "fail")
    color_enabled = output.isatty() and not os.environ.get("NO_COLOR")
    if use_color is not None:
        color_enabled = use_color
    line = "─" * 61
    counts = {"ok": 0, "idle": 0, "warn": 0, "fail": 0}
    _render_header(output, report, network, migration, status, color_enabled, line)
    row_status = _row_status_printer(output, color_enabled, counts)
    detail = _detail_printer(output, color_enabled)
    path_detail = _path_detail_printer(output, color_enabled)
    probe_detail = _probe_detail_printer(output, counts)
    _render_runtime(output, report, checks, paths, manifest, color_enabled, row_status, detail, path_detail)
    _render_support(output, checks, color_enabled, row_status, detail)
    _render_environment(output, checks, paths, color_enabled, row_status, detail, path_detail)
    _render_storage(output, report, checks, paths, color_enabled, row_status, detail, path_detail)
    _render_migration(output, migration, color_enabled, row_status, detail, path_detail)
    _render_sandbox(output, network, color_enabled, row_status, probe_detail)
    _render_summary(output, counts, status, color_enabled, line)
    return 0 if status == "ok" else 1


def render_json_report(
    report: schemas.DoctorReportV4,
    *,
    stream: TextIO | None = None,
) -> int:
    output = stream if stream is not None else sys.stdout
    print(json.dumps(report, ensure_ascii=True, sort_keys=True), file=output)
    return 0


def _render_header(
    output: TextIO,
    report: schemas.DoctorReportV4,
    network: dict[str, object],
    migration: dict[str, object],
    status: str,
    color_enabled: bool,
    line: str,
) -> None:
    version_text = f"v{report.get('version', 'unknown')}"
    print(f"{_bold('Termux Wrapper Doctor', color_enabled)} {_dim(version_text, color_enabled)}", file=output)
    notes: list[tuple[str, str]] = []
    if network.get("overallStatus") == "inconclusive":
        notes.append(("sandbox", "network boundary baseline was blocked by the outer environment; restricted probes still passed."))
    if _migration_row_status(migration) == "warn":
        notes.append(("migration", "legacy store migration needs attention; active runtime may still be healthy."))
    if status != "ok":
        notes.append(("wrapper", "one or more wrapper checks failed."))
    if not notes:
        return
    print(file=output)
    print(_bold("Notes", color_enabled), file=output)
    for name, summary in notes:
        print(f"   {_fg('214', '⚠', color_enabled)} {name:<12} {summary}", file=output)
    print(_dim(line, color_enabled), file=output)


def _render_runtime(output: TextIO, report: schemas.DoctorReportV4, checks: dict[str, object], paths: dict[str, object], manifest: dict[str, object], color_enabled: bool, row_status: object, detail: object, path_detail: object) -> None:
    _section(output, "Runtime", color_enabled)
    _row(row_status, bool(checks.get("runtime")), "runtime", f"managed executable · {_compact_hash(str(report.get('runtime_sha256', '')))}")
    path_detail("executable", str(paths.get("runtime", "missing")))
    _row(row_status, bool(checks.get("raw")), "raw", f"official linux-arm64 package · {_compact_hash(str(report.get('raw_sha256', '')))}")
    path_detail("vendor", str(paths.get("raw_vendor", "missing")))
    _row(row_status, bool(checks.get("build_manifest")), "manifest", str(manifest.get("patch_policy", "missing")))
    detail("builder hash", _compact_hash(str(manifest.get("builder_sha256", ""))))
    _row(row_status, bool(checks.get("dns_only_patch")), "patch", "DNS resolver path redirects to fd 33 only")
    detail("changed bytes", manifest.get("changed_byte_count", "unknown"))
    _row(row_status, bool(checks.get("runtime_hash")) and bool(checks.get("raw_hash")), "integrity", "runtime and raw hashes match recorded state")
    detail("active tuple", report.get("activeTupleId", "missing"))


def _render_support(output: TextIO, checks: dict[str, object], color_enabled: bool, row_status: object, detail: object) -> None:
    _section(output, "Support", color_enabled)
    _row(row_status, bool(checks.get("path_bwrap")) and bool(checks.get("bundled_bwrap")) and bool(checks.get("bwrap_exec")), "bwrap", "Termux compatibility launcher is executable")
    detail("launcher", "codex-path/bwrap")
    _row(row_status, bool(checks.get("rg")) and bool(checks.get("rg_real")) and bool(checks.get("rg_exec")), "search", "ripgrep shim and original rg are executable")
    detail("provider", "managed rg shim")
    _row(row_status, bool(checks.get("support_bwrap_match")) and bool(checks.get("support_rg_match")), "support", "installed support files match wrapper files")
    _row(row_status, bool(checks.get("zsh")), "zsh", "bundled zsh resource is present")


def _render_environment(output: TextIO, checks: dict[str, object], paths: dict[str, object], color_enabled: bool, row_status: object, detail: object, path_detail: object) -> None:
    _section(output, "Environment", color_enabled)
    _row(row_status, bool(checks.get("resolv")), "resolver", "fd 33 source is readable")
    detail("source", "/proc/self/fd/33")
    _row(row_status, bool(checks.get("cert")), "cert", "Termux CA bundle is readable")
    _row(row_status, bool(checks.get("state")) and bool(checks.get("registry")) and bool(checks.get("registry_active_tuple")), "state", "state and registry point at active runtime")
    path_detail("state", str(paths.get("state", "missing")))
    path_detail("registry", str(paths.get("registry", "missing")))


def _render_storage(output: TextIO, report: schemas.DoctorReportV4, checks: dict[str, object], paths: dict[str, object], color_enabled: bool, row_status: object, detail: object, path_detail: object) -> None:
    _section(output, "Storage", color_enabled)
    _row(row_status, bool(checks.get("manager")), "manager", "managed support files are installed")
    path_detail("manager", str(paths.get("manager", "missing")))
    _row(row_status, bool(checks.get("current_pointer")) and bool(checks.get("current_in_store")) and bool(checks.get("registry_current_match")), "current", "active runtime pointer is aligned with registry")
    path_detail("link", str(paths.get("current", "missing")))
    path_detail("target", str(paths.get("current_target", "missing")))
    _row(row_status, bool(checks.get("verified_pointer")) and bool(checks.get("verified_in_store")) and bool(checks.get("registry_verified_match")), "verified", "last-known-good pointer is aligned with registry")
    path_detail("link", str(paths.get("verified", "missing")))
    path_detail("target", str(paths.get("verified_target", "missing")))
    _row(row_status, bool(checks.get("raw_pointer")) and bool(checks.get("raw_in_store")), "raw cache", "active raw pointer targets the managed store")
    path_detail("link", str(paths.get("raw", "missing")))
    path_detail("target", str(paths.get("raw_target", "missing")))
    _row(row_status, bool(checks.get("runtime_store")) and bool(checks.get("raw_store")), "stores", "runtime and raw stores are present")
    path_detail("runtime", str(paths.get("runtime_store", "missing")))
    path_detail("raw", str(paths.get("raw_store", "missing")))
    _row(row_status, bool(checks.get("current_verified_match")), "alignment", "current and verified runtime pointers agree")
    detail("active tuple", report.get("activeTupleId", "missing"))
    detail("verified tuple", report.get("verifiedTupleId", "missing"))


def _render_migration(output: TextIO, migration: dict[str, object], color_enabled: bool, row_status: object, detail: object, path_detail: object) -> None:
    _section(output, "Migration", color_enabled)
    row_status(_migration_row_status(migration), "legacy store", f"legacy store cache migration {migration.get('status', 'unknown')}")
    path_detail("report", str(migration.get("report", "missing")))
    path_detail("legacy store", str(migration.get("legacyStore", "missing")))
    detail("imported count", len(migration.get("imported", [])))
    detail("skipped count", len(migration.get("skipped", [])))
    if migration.get("error"):
        detail("error", migration.get("error"))


def _render_sandbox(output: TextIO, network: dict[str, object], color_enabled: bool, row_status: object, probe_detail: object) -> None:
    _section(output, "Sandbox", color_enabled)
    net_status = str(network.get("overallStatus", "missing"))
    if net_status == "ok":
        network_row_status = "ok"
    elif net_status == "inconclusive":
        network_row_status = "warn"
    else:
        network_row_status = "fail"
    row_status(network_row_status, "network", f"boundary probe {net_status}")
    for name in ("baseline_socket", "network_off", "network_on", "network_reset"):
        probe_ok = bool(network.get("checks", {}).get(name, False))
        probe_status = "ok" if probe_ok else ("warn" if net_status == "inconclusive" else "fail")
        probe_detail(name.replace("_", " "), probe_status, probe_ok)


def _render_summary(output: TextIO, counts: dict[str, int], status: str, color_enabled: bool, line: str) -> None:
    print(file=output)
    print(_dim(line, color_enabled), file=output)
    summary_status = "fail" if counts["fail"] else ("degraded" if counts["warn"] else "ok")
    if summary_status == "ok":
        summary_tail = _bold(_std_fg("32", "ok", color_enabled), color_enabled)
    elif summary_status == "degraded":
        summary_tail = _bold(_std_fg("33", "degraded", color_enabled), color_enabled)
    else:
        summary_tail = _bold(_std_fg("31", "fail", color_enabled), color_enabled)
    print(_dim(str(counts["ok"]), color_enabled) + " " + _fg("10", "ok", color_enabled) + _dim(" · ", color_enabled) + _dim(str(counts["idle"]), color_enabled) + _dim(" idle · ", color_enabled) + _dim(str(counts["warn"]), color_enabled) + " " + _fg("214", "warn", color_enabled) + _dim(" · ", color_enabled) + _dim(str(counts["fail"]), color_enabled) + " " + _fg("196", "fail", color_enabled) + " " + summary_tail, file=output)
    _ = status


def _section(output: TextIO, text: str, color_enabled: bool) -> None:
    print(file=output)
    print(_bold(text, color_enabled), file=output)


def _row_status_printer(output: TextIO, color_enabled: bool, counts: dict[str, int]):
    def printer(item_status: str, name: str, summary: str) -> None:
        counts[item_status] += 1
        print(f"  {_status_mark(item_status, color_enabled)} {name:<12} {_dim(summary, color_enabled)}", file=output)
    return printer


def _detail_printer(output: TextIO, color_enabled: bool):
    def printer(name: str, value: object) -> None:
        print(f"      {_fg('240', f'{name:<24}', color_enabled)} {value}", file=output)
    return printer


def _path_detail_printer(output: TextIO, color_enabled: bool):
    def printer(name: str, value: str) -> None:
        print(f"      {_fg('240', f'{name:<24}', color_enabled)} {_fg('117', value, color_enabled)}", file=output)
    return printer


def _probe_detail_printer(output: TextIO, counts: dict[str, int]):
    def printer(name: str, item_status: str, value: object) -> None:
        counts[item_status] += 1
        print(f"      {name:<24} {value}", file=output)
    return printer


def _row(row_status: object, ok: bool, name: str, summary: str) -> None:
    row_status("ok" if ok else "fail", name, summary)


def _migration_row_status(migration: dict[str, object]) -> str:
    migration_status = migration.get("status", "unknown")
    if migration_status == "not-needed":
        return "idle"
    if migration_status == "completed" and not migration.get("skipped") and not migration.get("error"):
        return "ok"
    return "warn"


def _compact_hash(value: str) -> str:
    return value[:12] if value else "missing"


def _color(code: str, text: str, enabled: bool) -> str:
    return f"\033[{code}m{text}\033[0m" if enabled else text


def _fg(code: str, text: str, enabled: bool) -> str:
    return f"\033[38;5;{code}m{text}\033[39m" if enabled else text


def _std_fg(code: str, text: str, enabled: bool) -> str:
    return f"\033[{code}m{text}\033[39m" if enabled else text


def _dim(text: str, enabled: bool) -> str:
    return _color("2", text, enabled)


def _bold(text: str, enabled: bool) -> str:
    return _color("1", text, enabled)


def _status_mark(status: str, enabled: bool) -> str:
    if status == "ok":
        return _fg("10", "✓", enabled)
    if status == "warn":
        return _fg("214", "⚠", enabled)
    if status == "idle":
        return _dim("○", enabled)
    return _fg("196", "✗", enabled)

#!/data/data/com.termux/files/usr/bin/python3
"""Build a Termux-compatible Codex runtime from an official npm vendor tree."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path


RESOLV_CONF_SOURCE = b"/etc/resolv.conf"
RESOLV_CONF_TARGET = b"/proc/self/fd/33"
SANDBOX_WARNING_STRINGS = (
    b"Codex could not find bubblewrap on PATH. Install bubblewrap with your OS package manager. See the sandbox prerequisites: https://developers.openai.com/codex/concepts/sandboxing#prerequisites. Codex will use the bundled bubblewrap in the meantime.",
    b"Codex's Linux sandbox uses bubblewrap and needs access to create user namespaces.",
    b"Codex's Linux sandbox uses bubblewrap, which is not supported on WSL1 because WSL1 cannot create the required user namespaces. Use WSL2 for sandboxed shell commands.",
)
CHUNK_SIZE = 1024 * 1024
TOOL_DIR = Path(__file__).resolve().parent
BWRAP_COMPAT_SOURCE = TOOL_DIR / "bwrap-termux-compat.py"
RG_SHIM_SOURCE = TOOL_DIR / "rg-termux-shim.sh"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(CHUNK_SIZE), b""):
            digest.update(chunk)
    return digest.hexdigest()


def copy_tree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst, symlinks=True)


def install_termux_compat_tools(runtime_dir: Path) -> None:
    bundled_bwrap = runtime_dir / "codex-resources" / "bwrap"
    bwrap_real = runtime_dir / "codex-resources" / "bwrap.real"
    bwrap = runtime_dir / "codex-path" / "bwrap"
    rg = runtime_dir / "codex-path" / "rg"
    rg_real = runtime_dir / "codex-path" / "rg.real"

    for source in (BWRAP_COMPAT_SOURCE, RG_SHIM_SOURCE):
        if not source.exists():
            raise RuntimeError(f"missing compat tool source: {source}")

    if bundled_bwrap.exists() and not bwrap_real.exists():
        shutil.copy2(bundled_bwrap, bwrap_real)
    shutil.copy2(BWRAP_COMPAT_SOURCE, bwrap)

    if rg.exists() and not rg_real.exists():
        os.replace(rg, rg_real)
    shutil.copy2(RG_SHIM_SOURCE, rg)

    for executable in (bundled_bwrap, bwrap_real, bwrap, rg, rg_real):
        executable.chmod(executable.stat().st_mode | 0o755)


def patch_codex_binary(src: Path, dst: Path) -> dict[str, object]:
    data = bytearray(src.read_bytes())
    source_count = data.count(RESOLV_CONF_SOURCE)
    target_count_before = data.count(RESOLV_CONF_TARGET)
    if len(RESOLV_CONF_SOURCE) != len(RESOLV_CONF_TARGET):
        raise RuntimeError("resolver rewrite must preserve byte length")
    if source_count < 1:
        raise RuntimeError(
            f"expected at least one {RESOLV_CONF_SOURCE!r}; found {source_count}"
        )
    if target_count_before != 0:
        raise RuntimeError(
            f"raw binary already contains {RESOLV_CONF_TARGET!r}; refusing to patch"
        )
    data[:] = data.replace(RESOLV_CONF_SOURCE, RESOLV_CONF_TARGET)
    warning_counts: dict[str, int] = {}
    for warning in SANDBOX_WARNING_STRINGS:
        count = data.count(warning)
        warning_counts[warning.decode("utf-8", errors="replace")[:80]] = count
        if count:
            data[:] = data.replace(warning, b" " * len(warning))
    tmp = dst.with_name(f".{dst.name}.building")
    tmp.write_bytes(data)
    os.chmod(tmp, 0o755)
    os.replace(tmp, dst)
    return {
        "resolver_source_count": source_count,
        "resolver_target_count_before": target_count_before,
        "resolver_target_count_after": data.count(RESOLV_CONF_TARGET),
        "suppressed_sandbox_warning_counts": warning_counts,
    }


def build(raw_vendor: Path, runtime_dir: Path) -> dict[str, object]:
    raw_bin = raw_vendor / "bin" / "codex"
    raw_resources = raw_vendor / "codex-resources"
    raw_path_tools = raw_vendor / "codex-path"
    raw_package = raw_vendor / "codex-package.json"
    required = [
        raw_bin,
        raw_resources / "bwrap",
        raw_resources / "zsh" / "bin" / "zsh",
        raw_path_tools / "rg",
        raw_package,
    ]
    missing = [str(path) for path in required if not path.exists()]
    if missing:
        raise RuntimeError("missing required upstream package paths: " + ", ".join(missing))

    runtime_dir.mkdir(parents=True, exist_ok=True)
    tmp_dir = runtime_dir.with_name(f".{runtime_dir.name}.building")
    if tmp_dir.exists():
        shutil.rmtree(tmp_dir)
    tmp_dir.mkdir(parents=True)

    patch_report = patch_codex_binary(raw_bin, tmp_dir / "codex")
    copy_tree(raw_resources, tmp_dir / "codex-resources")
    copy_tree(raw_path_tools, tmp_dir / "codex-path")
    shutil.copy2(raw_package, tmp_dir / "codex-package.json")
    install_termux_compat_tools(tmp_dir)

    for executable in [
        tmp_dir / "codex",
        tmp_dir / "codex-resources" / "bwrap",
        tmp_dir / "codex-resources" / "bwrap.real",
        tmp_dir / "codex-resources" / "zsh" / "bin" / "zsh",
        tmp_dir / "codex-path" / "bwrap",
        tmp_dir / "codex-path" / "rg",
        tmp_dir / "codex-path" / "rg.real",
    ]:
        executable.chmod(executable.stat().st_mode | 0o755)

    runtime_dir.mkdir(parents=True, exist_ok=True)
    for name in ("codex", "codex-resources", "codex-path", "codex-package.json"):
        target = runtime_dir / name
        source = tmp_dir / name
        old = runtime_dir / f".{name}.old"
        if old.exists():
            if old.is_dir():
                shutil.rmtree(old)
            else:
                old.unlink()
        if target.exists():
            os.replace(target, old)
        os.replace(source, target)
        if old.exists():
            if old.is_dir():
                shutil.rmtree(old)
            else:
                old.unlink()
    shutil.rmtree(tmp_dir, ignore_errors=True)

    return {
        "schema": 1,
        "raw_vendor": str(raw_vendor),
        "runtime_dir": str(runtime_dir),
        "raw_sha256": sha256(raw_bin),
        "runtime_sha256": sha256(runtime_dir / "codex"),
        **patch_report,
        "resources": {
            "bwrap": str(runtime_dir / "codex-path" / "bwrap"),
            "bundled_bwrap": str(runtime_dir / "codex-resources" / "bwrap"),
            "bwrap_real": str(runtime_dir / "codex-resources" / "bwrap.real"),
            "zsh": str(runtime_dir / "codex-resources" / "zsh" / "bin" / "zsh"),
            "rg": str(runtime_dir / "codex-path" / "rg"),
            "rg_real": str(runtime_dir / "codex-path" / "rg.real"),
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_vendor", help="vendor/aarch64-unknown-linux-musl directory")
    parser.add_argument("--runtime-dir", required=True, help="managed runtime directory")
    parser.add_argument("--report-json", help="write build report JSON")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        report = build(Path(args.raw_vendor).expanduser(), Path(args.runtime_dir).expanduser())
        print(json.dumps(report, sort_keys=True))
        if args.report_json:
            out = Path(args.report_json).expanduser()
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(json.dumps(report, sort_keys=True) + "\n")
        return 0
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

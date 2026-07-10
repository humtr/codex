#!/data/data/com.termux/files/usr/bin/python3
"""Build a Termux-compatible Codex runtime from an official npm vendor tree."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
import sys
from pathlib import Path


REWRITES = {
    b"/etc/resolv.conf": b"/proc/self/fd/33",
    b"/etc/codex/config.toml": b"/dev/fd/34/config.toml",
    b"/etc/codex/requirements.toml": b"/dev/fd/34/requirements.toml",
    b"/etc/codex/managed_config.toml": b"/dev/fd/34/managed_config.toml",
}
PATCH_POLICY = "termux-fd-remap-v1"
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


def tree_digest(root: Path) -> str:
    """Hash an upstream tree without following symlinks."""
    digest = hashlib.sha256()
    root_mode = root.lstat().st_mode
    digest.update(b".\0" + f"{stat.S_IMODE(root_mode):04o}".encode("ascii") + b"\0D\0")
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        mode = path.lstat().st_mode
        digest.update(path.relative_to(root).as_posix().encode("utf-8") + b"\0")
        digest.update(f"{stat.S_IMODE(mode):04o}".encode("ascii") + b"\0")
        if stat.S_ISLNK(mode):
            digest.update(b"L\0" + os.readlink(path).encode("utf-8") + b"\0")
        elif stat.S_ISDIR(mode):
            digest.update(b"D\0")
        elif stat.S_ISREG(mode):
            digest.update(b"F\0" + path.read_bytes())
        else:
            raise RuntimeError(f"unsupported upstream tree entry: {path}")
    return digest.hexdigest()


def copy_tree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst, symlinks=True)


def install_termux_compat_tools(runtime_dir: Path) -> None:
    bundled_bwrap = runtime_dir / "codex-resources" / "bwrap"
    bwrap = runtime_dir / "codex-path" / "bwrap"
    rg = runtime_dir / "codex-path" / "rg"
    rg_real = runtime_dir / "codex-path" / "rg.real"
    overlay_dir = runtime_dir / "overlay"
    overlay_path_tools = overlay_dir / "codex-path"
    overlay_path_tools.mkdir(parents=True, exist_ok=True)

    for source in (BWRAP_COMPAT_SOURCE, RG_SHIM_SOURCE):
        if not source.exists():
            raise RuntimeError(f"missing compat tool source: {source}")

    shutil.copy2(BWRAP_COMPAT_SOURCE, overlay_path_tools / "bwrap")
    shutil.copy2(BWRAP_COMPAT_SOURCE, bwrap)

    if rg.exists() and not rg_real.exists():
        os.replace(rg, rg_real)
    shutil.copy2(RG_SHIM_SOURCE, overlay_path_tools / "rg")
    shutil.copy2(RG_SHIM_SOURCE, rg)

    for executable in (bundled_bwrap, bwrap, rg, rg_real):
        executable.chmod(executable.stat().st_mode | 0o755)


def patch_codex_binary(src: Path, dst: Path) -> dict[str, object]:
    raw = src.read_bytes()
    data = bytearray(raw)
    rewrite_report: dict[str, dict[str, int]] = {}
    for source, target in REWRITES.items():
        if len(source) != len(target):
            raise RuntimeError(f"rewrite must preserve byte length: {source!r}")
        source_count = data.count(source)
        target_count_before = data.count(target)
        if source_count < 1:
            raise RuntimeError(f"expected at least one {source!r}; found {source_count}")
        if target_count_before != 0:
            raise RuntimeError(f"raw binary already contains {target!r}; refusing to patch")
        data[:] = data.replace(source, target)
        rewrite_report[source.decode("ascii")] = {
            "source_count": source_count,
            "target_count_before": target_count_before,
            "target_count_after": data.count(target),
        }
    expected = raw
    for source, target in REWRITES.items():
        expected = expected.replace(source, target)
    if data != expected:
        raise RuntimeError("runtime binary differs from the fd remap patch policy")
    tmp = dst.with_name(f".{dst.name}.building")
    tmp.write_bytes(data)
    os.chmod(tmp, 0o755)
    os.replace(tmp, dst)
    return {
        "rewrites": rewrite_report,
        "changed_byte_count": sum(left != right for left, right in zip(raw, data)),
    }


def build(raw_vendor: Path, runtime_dir: Path) -> dict[str, object]:
    raw_bin = raw_vendor / "bin" / "codex"
    raw_code_host = raw_vendor / "bin" / "codex-code-mode-host"
    raw_resources = raw_vendor / "codex-resources"
    raw_path_tools = raw_vendor / "codex-path"
    raw_package = raw_vendor / "codex-package.json"
    required = [
        raw_bin,
        raw_code_host,
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

    copy_tree(raw_vendor, tmp_dir / "upstream")
    overlay_dir = tmp_dir / "overlay"
    overlay_dir.mkdir(parents=True, exist_ok=True)
    patch_report = patch_codex_binary(raw_bin, overlay_dir / "codex")
    shutil.copy2(overlay_dir / "codex", tmp_dir / "codex")
    shutil.copy2(raw_code_host, tmp_dir / "codex-code-mode-host")
    copy_tree(raw_resources, tmp_dir / "codex-resources")
    copy_tree(raw_path_tools, tmp_dir / "codex-path")
    shutil.copy2(raw_package, tmp_dir / "codex-package.json")
    install_termux_compat_tools(tmp_dir)
    runtime_sha = sha256(tmp_dir / "codex")
    raw_sha = sha256(raw_bin)
    code_host_sha = sha256(tmp_dir / "codex-code-mode-host")
    overlay_sha = tree_digest(overlay_dir)
    build_manifest = {
        "schema": 3,
        "patch_policy": PATCH_POLICY,
        "builder_sha256": sha256(Path(__file__)),
        "raw_sha256": raw_sha,
        "runtime_sha256": runtime_sha,
        "code_mode_host_sha256": code_host_sha,
        "upstream_tree_sha256": tree_digest(raw_vendor),
        "overlay_tree_sha256": overlay_sha,
        "overlay_entries": [
            "codex",
            "codex-path/bwrap",
            "codex-path/rg",
        ],
        **patch_report,
    }
    (tmp_dir / "runtime-build.json").write_text(
        json.dumps(build_manifest, ensure_ascii=True, sort_keys=True) + "\n"
    )

    for executable in [
        tmp_dir / "codex",
        tmp_dir / "codex-code-mode-host",
        tmp_dir / "codex-resources" / "bwrap",
        tmp_dir / "codex-resources" / "zsh" / "bin" / "zsh",
        tmp_dir / "codex-path" / "bwrap",
        tmp_dir / "codex-path" / "rg",
        tmp_dir / "codex-path" / "rg.real",
    ]:
        executable.chmod(executable.stat().st_mode | 0o755)

    runtime_dir.mkdir(parents=True, exist_ok=True)
    for name in ("codex", "codex-code-mode-host", "codex-resources", "codex-path", "codex-package.json", "runtime-build.json", "upstream", "overlay"):
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
        "raw_sha256": raw_sha,
        "runtime_sha256": runtime_sha,
        "build_manifest": build_manifest,
        **patch_report,
        "resources": {
            "bwrap": str(runtime_dir / "codex-path" / "bwrap"),
            "bundled_bwrap": str(runtime_dir / "codex-resources" / "bwrap"),
            "code_mode_host": str(runtime_dir / "codex-code-mode-host"),
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

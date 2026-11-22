#!/usr/bin/env python3
"""Immutable generation lifecycle & isolated CDP probe for Linux ChatGPT AppHost.

Canonical XDG `codex/` namespace paths (zero `~/.cache`):
  - Generations root: ~/.local/lib/codex/host/chatgpt-desktop/
      generations/<ver>/rootfs/
      generations/<ver>/generation.json
      current -> generations/<ver>
      previous -> generations/<old-ver>
      viewer/
      downloads/
  - Persistent profile (never deleted on generation upgrade/rollback):
      ~/.local/share/codex/chatgpt-desktop/profile/
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import secrets
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

DEFAULT_REPO_URL = "https://persistent.oaistatic.com/codex-app-prod/linux/deb"
PINNED_KEYRING_SHA256 = (
    "23e2cfbdef6afe95505f9e95a2cb63585da7ffe9b06a51ec08a32407c847d596"
)
PINNED_KEYRING_FINGERPRINT = "3BFA0E4AE8B8CC16A2D9BA684A3B4A566C4660E4"
DEFAULT_KEYRING_PATH = Path(__file__).resolve().with_name("openai-apt-keyring.gpg")
VERSION_RE = re.compile(r"^[0-9A-Za-z][0-9A-Za-z.+:~-]{0,63}$")


def default_install_root(home: Path) -> Path:
    return home / ".local" / "lib" / "codex" / "host" / "chatgpt-desktop"


def default_profile_dir(home: Path) -> Path:
    return home / ".local" / "share" / "codex" / "chatgpt-desktop" / "profile"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            chunk = f.read(1024 * 1024)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def validate_version(version: str) -> str:
    if not VERSION_RE.match(version) or "/" in version or ".." in version:
        raise ValueError(f"invalid generation version: {version!r}")
    return version


def verify_keyring(keyring_path: Path, expected_sha256: str = PINNED_KEYRING_SHA256) -> str:
    if not keyring_path.is_file():
        raise RuntimeError(f"APT keyring not found: {keyring_path}")
    actual = sha256_file(keyring_path)
    if actual != expected_sha256:
        raise RuntimeError(
            f"APT keyring SHA-256 mismatch: expected {expected_sha256}, got {actual}"
        )
    return actual


def verify_inrelease_gpg(inrelease_bytes: bytes, keyring_path: Path) -> None:
    verify_keyring(keyring_path)
    with tempfile.NamedTemporaryFile(prefix="chatgpt-inrelease-", delete=False) as tf:
        tf.write(inrelease_bytes)
        tmp_path = Path(tf.name)
    try:
        proc = subprocess.run(
            ["gpgv", "--keyring", str(keyring_path), str(tmp_path)],
            capture_output=True,
            text=True,
            timeout=20,
        )
        if proc.returncode != 0:
            raise RuntimeError(
                f"InRelease GPG verification failed (rc={proc.returncode}): {proc.stderr.strip()}"
            )
    finally:
        tmp_path.unlink(missing_ok=True)


def parse_inrelease(inrelease_text: str) -> dict[str, Any]:
    """Extract Date and SHA256 file entries from a clearsigned InRelease document."""
    date_val: str | None = None
    sha256_entries: dict[str, tuple[str, int]] = {}
    in_sha256 = False
    for raw_line in inrelease_text.splitlines():
        line = raw_line.rstrip("\r\n")
        if line.startswith("-----BEGIN PGP SIGNATURE-----"):
            break
        if line.startswith("Date:"):
            date_val = line.split(":", 1)[1].strip()
            in_sha256 = False
            continue
        if line == "SHA256:":
            in_sha256 = True
            continue
        if in_sha256:
            if line.startswith(" "):
                parts = line.split()
                if len(parts) == 3:
                    digest, size_str, rel_path = parts
                    if re.fullmatch(r"[0-9a-f]{64}", digest):
                        sha256_entries[rel_path] = (digest, int(size_str))
            else:
                in_sha256 = False
    if not sha256_entries:
        raise RuntimeError("InRelease missing SHA256 section")
    return {"date": date_val, "sha256": sha256_entries}


def parse_arm64_chatgpt_package(packages_text: str) -> dict[str, Any]:
    """Parse Debian Packages stanzas and return the highest arm64 `chatgpt` entry."""
    stanzas: list[dict[str, str]] = []
    cur: dict[str, str] = {}
    last_key: str | None = None
    for raw_line in packages_text.splitlines():
        if not raw_line.strip():
            if cur:
                stanzas.append(cur)
                cur = {}
                last_key = None
            continue
        if raw_line[0].isspace() and last_key is not None:
            cur[last_key] = cur[last_key] + "\n" + raw_line.strip()
        elif ":" in raw_line:
            k, v = raw_line.split(":", 1)
            last_key = k.strip()
            cur[last_key] = v.strip()
    if cur:
        stanzas.append(cur)

    matches = [
        s
        for s in stanzas
        if s.get("Package") == "chatgpt" and s.get("Architecture") == "arm64"
    ]
    if not matches:
        raise RuntimeError("no arm64 'chatgpt' package stanza found in Packages")

    def ver_key(stanza: dict[str, str]) -> list[int]:
        return [int(x) for x in re.findall(r"\d+", stanza.get("Version", "0"))]

    best = max(matches, key=ver_key)
    for req in ("Version", "Filename", "SHA256", "Size"):
        if req not in best:
            raise RuntimeError(f"Packages stanza missing required field '{req}'")
    digest = best["SHA256"].lower()
    if not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise RuntimeError(f"invalid SHA256 in Packages: {digest!r}")
    return {
        "version": validate_version(best["Version"]),
        "filename": best["Filename"],
        "sha256": digest,
        "size": int(best["Size"]),
    }


def http_get_bytes(url: str, timeout: int = 30) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "codex-chatgpt-desktop/0.1"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.read()
    except urllib.error.HTTPError as e:
        try:
            code = e.code
        finally:
            e.close()
        raise RuntimeError(f"HTTP {code} fetching {url}") from None
    except urllib.error.URLError as e:
        raise RuntimeError(f"network error fetching {url}: {e.reason}") from None


def discover_latest_deb(
    repo_url: str = DEFAULT_REPO_URL,
    keyring_path: Path = DEFAULT_KEYRING_PATH,
) -> dict[str, Any]:
    base = repo_url.rstrip("/")
    inrelease_url = f"{base}/dists/stable/InRelease"
    inrelease_bytes = http_get_bytes(inrelease_url)
    verify_inrelease_gpg(inrelease_bytes, keyring_path)
    parsed_ir = parse_inrelease(inrelease_bytes.decode("utf-8", errors="replace"))

    rel_pkg = "main/binary-arm64/Packages"
    if rel_pkg not in parsed_ir["sha256"]:
        raise RuntimeError(f"InRelease missing SHA256 entry for {rel_pkg}")
    expected_pkg_sha, expected_pkg_size = parsed_ir["sha256"][rel_pkg]

    packages_bytes = http_get_bytes(f"{base}/dists/stable/{rel_pkg}")
    actual_pkg_sha = sha256_bytes(packages_bytes)
    if actual_pkg_sha != expected_pkg_sha or len(packages_bytes) != expected_pkg_size:
        raise RuntimeError(
            f"Packages integrity mismatch: expected {expected_pkg_sha} ({expected_pkg_size}B), "
            f"got {actual_pkg_sha} ({len(packages_bytes)}B)"
        )
    pkg = parse_arm64_chatgpt_package(packages_bytes.decode("utf-8", errors="replace"))
    pkg["url"] = f"{base}/{pkg['filename'].lstrip('/')}"
    pkg["inrelease_date"] = parsed_ir["date"]
    pkg["keyring_fingerprint"] = PINNED_KEYRING_FINGERPRINT
    pkg["keyring_sha256"] = PINNED_KEYRING_SHA256
    return pkg


def download_verified_deb(url: str, dest: Path, expected_sha256: str, expected_size: int) -> Path:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.stat().st_size == expected_size and sha256_file(dest) == expected_sha256:
        return dest

    part = dest.with_suffix(dest.suffix + f".part-{os.getpid()}")
    try:
        proc = subprocess.run(
            ["curl", "-fL", "--retry", "3", "--connect-timeout", "20", "-o", str(part), url],
            timeout=900,
        )
        if proc.returncode != 0:
            raise RuntimeError(f"curl failed downloading {url} (rc={proc.returncode})")
        actual_size = part.stat().st_size
        actual_sha = sha256_file(part)
        if actual_size != expected_size or actual_sha != expected_sha256:
            raise RuntimeError(
                f"deb integrity mismatch: expected {expected_sha256} ({expected_size}B), "
                f"got {actual_sha} ({actual_size}B)"
            )
        os.chmod(part, 0o600)
        os.replace(part, dest)
        return dest
    finally:
        part.unlink(missing_ok=True)


def generation_dir(install_root: Path, version: str) -> Path:
    return install_root / "generations" / validate_version(version)


def is_valid_generation(gen_path: Path) -> bool:
    rootfs = gen_path / "rootfs"
    return (
        gen_path.is_dir()
        and not gen_path.is_symlink()
        and (gen_path / "generation.json").is_file()
        and (rootfs / "usr/bin/chatgpt").is_file()
        and (rootfs / "usr/lib/chatgpt/resources/app.asar").is_file()
    )


def read_link_version(link: Path) -> str | None:
    if not link.is_symlink():
        return None
    target = os.readlink(link)
    parts = Path(target).parts
    if len(parts) == 2 and parts[0] == "generations":
        return parts[1]
    return Path(target).name


def atomic_symlink(link: Path, target: str) -> None:
    tmp = link.with_name(f".{link.name}.tmp-{os.getpid()}-{secrets.token_hex(4)}")
    try:
        os.symlink(target, tmp)
        os.replace(tmp, link)
    finally:
        if tmp.is_symlink() or tmp.exists():
            tmp.unlink(missing_ok=True)


def activate_generation(install_root: Path, version: str) -> dict[str, Any]:
    ver = validate_version(version)
    target_dir = generation_dir(install_root, ver)
    if not is_valid_generation(target_dir):
        raise RuntimeError(f"generation '{ver}' is missing or incomplete: {target_dir}")
    cur_link = install_root / "current"
    prev_link = install_root / "previous"
    old_cur = read_link_version(cur_link)
    if old_cur and old_cur != ver and is_valid_generation(generation_dir(install_root, old_cur)):
        atomic_symlink(prev_link, f"generations/{old_cur}")
    atomic_symlink(cur_link, f"generations/{ver}")
    return status_report(install_root, default_profile_dir(Path.home()))


def rollback_generation(install_root: Path) -> dict[str, Any]:
    cur_link = install_root / "current"
    prev_link = install_root / "previous"
    prev_ver = read_link_version(prev_link)
    if not prev_ver:
        raise RuntimeError("no previous generation available to roll back to")
    prev_dir = generation_dir(install_root, prev_ver)
    if not is_valid_generation(prev_dir):
        raise RuntimeError(f"previous generation '{prev_ver}' is invalid: {prev_dir}")
    old_cur = read_link_version(cur_link)
    atomic_symlink(cur_link, f"generations/{prev_ver}")
    if old_cur and old_cur != prev_ver and is_valid_generation(generation_dir(install_root, old_cur)):
        atomic_symlink(prev_link, f"generations/{old_cur}")
    return status_report(install_root, default_profile_dir(Path.home()))


def status_report(install_root: Path, profile_dir: Path) -> dict[str, Any]:
    gens_root = install_root / "generations"
    installed: list[dict[str, Any]] = []
    if gens_root.is_dir():
        for entry in sorted(gens_root.iterdir()):
            if entry.name.startswith(".") or not entry.is_dir() or entry.is_symlink():
                continue
            meta_file = entry / "generation.json"
            meta = {}
            if meta_file.is_file():
                try:
                    meta = json.loads(meta_file.read_text(encoding="utf-8"))
                except json.JSONDecodeError:
                    meta = {}
            installed.append(
                {
                    "version": entry.name,
                    "valid": is_valid_generation(entry),
                    "deb_sha256": meta.get("deb_sha256"),
                    "installed_at": meta.get("installed_at"),
                }
            )
    cur_ver = read_link_version(install_root / "current")
    prev_ver = read_link_version(install_root / "previous")
    return {
        "install_root": str(install_root),
        "current": cur_ver,
        "current_valid": bool(cur_ver and is_valid_generation(generation_dir(install_root, cur_ver))),
        "previous": prev_ver,
        "previous_valid": bool(prev_ver and is_valid_generation(generation_dir(install_root, prev_ver))),
        "generations": installed,
        "profile_dir": str(profile_dir),
        "profile_exists": profile_dir.is_dir() and not profile_dir.is_symlink(),
    }


def provision_rootfs_generation(
    install_root: Path,
    version: str,
    deb_path: Path,
    deb_meta: dict[str, Any],
    prefix: Path,
) -> Path:
    ver = validate_version(version)
    final_dir = generation_dir(install_root, ver)
    if is_valid_generation(final_dir):
        return final_dir

    gens_root = install_root / "generations"
    gens_root.mkdir(parents=True, mode=0o700, exist_ok=True)
    stage_dir = gens_root / f".staging-{ver}-{os.getpid()}-{secrets.token_hex(3)}"
    stage_dir.mkdir(mode=0o700)

    try:
        # Build an isolated proot-distro TERMUX__PREFIX so global Termux containers are untouched.
        pd_prefix = stage_dir / "pd-prefix"
        pd_prefix.mkdir(mode=0o700)
        for sub in ("bin", "etc", "lib", "libexec", "share", "tmp"):
            src = prefix / sub
            if src.exists():
                os.symlink(src, pd_prefix / sub)
        (pd_prefix / "var/lib/proot-distro").mkdir(parents=True, mode=0o700)

        pd_env = os.environ.copy()
        pd_env["TERMUX__PREFIX"] = str(pd_prefix)
        pd_env["PD_FORCE_NO_COLORS"] = "true"
        pd_env["PD_PROOT_BIN"] = str(prefix / "bin/proot")

        print(f"[chatgpt-desktop] Installing base debian:13-slim rootfs for {ver}...", file=sys.stderr)
        subprocess.run(
            ["proot-distro", "install", "--override-alias", "chatgpt-gen", "debian:13-slim"],
            env=pd_env,
            check=True,
            timeout=600,
        )
        extracted_rootfs = pd_prefix / "var/lib/proot-distro/containers/chatgpt-gen/rootfs"
        if not extracted_rootfs.is_dir():
            raise RuntimeError(f"proot-distro did not produce rootfs at {extracted_rootfs}")

        # Install the verified chatgpt .deb + Debian dependencies via `proot-distro login --isolated`
        # so Android link2symlink/groupadd/fake-proc handling matches Termux's canonical container runtime.
        guest_tmp = stage_dir / "guest-tmp"
        guest_tmp.mkdir(mode=0o700)
        deb_name = "chatgpt.deb"
        shutil.copy2(deb_path, guest_tmp / deb_name)

        install_script = (
            "set -eu\n"
            "export DEBIAN_FRONTEND=noninteractive\n"
            "apt-get update\n"
            "apt-get install -y --no-install-recommends "
            f"/tmp/{deb_name} ca-certificates dbus-x11\n"
            "apt-get clean\n"
            "rm -rf /var/lib/apt/lists/* /tmp/chatgpt.deb\n"
        )
        print(f"[chatgpt-desktop] Installing chatgpt {ver} + dependencies in isolated PRoot...", file=sys.stderr)
        subprocess.run(
            [
                "proot-distro",
                "login",
                "--isolated",
                "--bind",
                f"{guest_tmp}:/tmp",
                "chatgpt-gen",
                "--",
                "/bin/sh",
                "-c",
                install_script,
            ],
            env=pd_env,
            check=True,
            timeout=1200,
        )

        rootfs = stage_dir / "rootfs"
        os.replace(extracted_rootfs, rootfs)
        shutil.rmtree(pd_prefix, ignore_errors=True)
        shutil.rmtree(guest_tmp, ignore_errors=True)

        manifest = {
            "schema": "chatgpt-desktop-generation",
            "version": ver,
            "deb_url": deb_meta.get("url"),
            "deb_sha256": deb_meta["sha256"],
            "deb_bytes": deb_meta["size"],
            "inrelease_date": deb_meta.get("inrelease_date"),
            "keyring_fingerprint": deb_meta.get("keyring_fingerprint", PINNED_KEYRING_FINGERPRINT),
            "installed_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        }
        meta_path = stage_dir / "generation.json"
        meta_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        os.chmod(meta_path, 0o600)

        if not is_valid_generation(stage_dir):
            raise RuntimeError("installed generation failed validation (missing chatgpt binary or app.asar)")

        if final_dir.exists():
            shutil.rmtree(final_dir)
        os.replace(stage_dir, final_dir)
        return final_dir
    except Exception:
        shutil.rmtree(stage_dir, ignore_errors=True)
        raise


def _write_xauthority(path: Path, display: int) -> None:
    cookie = secrets.token_bytes(16)
    def field(b: bytes) -> bytes:
        return struct.pack("!H", len(b)) + b
    data = (
        struct.pack("!H", 65535)
        + field(b"")
        + field(str(display).encode("ascii"))
        + field(b"MIT-MAGIC-COOKIE-1")
        + field(cookie)
    )
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as f:
        f.write(data)


def _cdp_pipe_call(sock: socket.socket, req_id: int, method: str, params: dict[str, Any], timeout: float) -> dict[str, Any]:
    payload = json.dumps({"id": req_id, "method": method, "params": params}).encode("utf-8") + b"\0"
    sock.settimeout(timeout)
    sock.sendall(payload)
    buf = bytearray()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        remaining = max(0.1, deadline - time.monotonic())
        sock.settimeout(remaining)
        chunk = sock.recv(65536)
        if not chunk:
            raise RuntimeError(f"CDP pipe closed while waiting for {method}")
        buf.extend(chunk)
        while b"\0" in buf:
            idx = buf.index(0)
            raw = bytes(buf[:idx])
            del buf[: idx + 1]
            if not raw:
                continue
            msg = json.loads(raw.decode("utf-8", errors="replace"))
            if msg.get("id") == req_id:
                if "error" in msg:
                    raise RuntimeError(f"CDP error for {method}: {msg['error']}")
                return msg.get("result", {})
    raise RuntimeError(f"CDP timeout waiting for {method}")


def probe_generation(
    install_root: Path,
    version: str | None,
    prefix: Path,
    timeout_secs: int = 35,
) -> dict[str, Any]:
    """Isolated, account-free PRoot/Xvnc + --remote-debugging-pipe CDP boot probe."""
    ver = validate_version(version) if version else read_link_version(install_root / "current")
    if not ver:
        raise RuntimeError("no active generation selected; pass --version")
    gen_path = generation_dir(install_root, ver)
    if not is_valid_generation(gen_path):
        raise RuntimeError(f"generation '{ver}' is invalid: {gen_path}")
    rootfs = gen_path / "rootfs"

    socket_root = prefix / "tmp/.X11-unix"
    display = next(
        (
            n
            for n in range(130, 1900)
            if not (socket_root / f"X{n}").exists() and not (prefix / f"tmp/.X{n}-lock").exists()
        ),
        None,
    )
    if display is None:
        raise RuntimeError("no free X11 display available")

    tmp_base = Path(tempfile.mkdtemp(prefix=f"cg-probe-{ver}-", dir=str(prefix / "tmp")))
    os.chmod(tmp_base, 0o700)
    procs: list[subprocess.Popen[Any]] = []
    parent_sock: socket.socket | None = None
    child_sock: socket.socket | None = None
    x_lock = prefix / f"tmp/.X{display}-lock"
    x_sock = socket_root / f"X{display}"
    xvnc_pid: int | None = None

    try:
        for name in ("home", "home/profile", "tmp", "shm", "links"):
            d = tmp_base / name
            d.mkdir(parents=True, mode=0o700, exist_ok=True)
        auth = tmp_base / "Xauthority"
        _write_xauthority(auth, display)

        xvnc_env = {
            "HOME": str(tmp_base / "home"),
            "PATH": str(prefix / "bin"),
            "PREFIX": str(prefix),
            "TMPDIR": str(tmp_base),
            "LANG": "C.UTF-8",
        }
        xvnc = subprocess.Popen(
            [
                str(prefix / "bin/Xvnc"),
                f":{display}",
                "-geometry",
                "480x785",
                "-depth",
                "24",
                "-nolisten",
                "tcp",
                "-auth",
                str(auth),
                "-rfbport",
                "-1",
                "-rfbunixpath",
                str(tmp_base / "viewer"),
                "-rfbunixmode",
                "0600",
                "-SecurityTypes",
                "None",
            ],
            env=xvnc_env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        procs.append(xvnc)
        xvnc_pid = xvnc.pid

        deadline = time.monotonic() + 10.0
        while time.monotonic() < deadline:
            if x_sock.exists():
                break
            if xvnc.poll() is not None:
                raise RuntimeError(f"Xvnc exited prematurely (rc={xvnc.returncode})")
            time.sleep(0.05)
        else:
            raise RuntimeError("Xvnc startup timeout")

        wm_env = dict(xvnc_env, DISPLAY=f":{display}", XAUTHORITY=str(auth))
        if (prefix / "bin/openbox").is_file():
            procs.append(
                subprocess.Popen(
                    [str(prefix / "bin/openbox")],
                    env=wm_env,
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True,
                )
            )

        parent_sock, child_sock = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        child_fd = child_sock.fileno()

        def setup_child() -> None:
            os.setsid()
            # Move child_fd above FD 4 first so dup2(..., 3/4) always clears O_CLOEXEC,
            # then close all other inherited FDs (avoiding Python's post-preexec close_fds!).
            hi_fd = os.dup(child_fd)
            while hi_fd <= 4:
                next_fd = os.dup(hi_fd)
                os.close(hi_fd)
                hi_fd = next_fd
            os.dup2(hi_fd, 3)
            os.dup2(hi_fd, 4)
            os.set_inheritable(3, True)
            os.set_inheritable(4, True)
            os.closerange(5, 4096)

        app_log = tmp_base / "app.log"
        proot_env = {
            "PATH": str(prefix / "bin"),
            "HOME": str(tmp_base / "home"),
            "TMPDIR": str(tmp_base),
            "PROOT_TMP_DIR": str(tmp_base),
            "PROOT_L2S_DIR": str(tmp_base / "links"),
        }
        with app_log.open("wb") as log_f:
            app = subprocess.Popen(
                [
                    str(prefix / "bin/proot"),
                    "--kill-on-exit",
                    "--link2symlink",
                    "--sysvipc",
                    "-L",
                    "--change-id=0:0",
                    f"--rootfs={rootfs}",
                    "--cwd=/root",
                    "--bind=/dev",
                    "--bind=/proc",
                    "--bind=/sys",
                    "--bind=/dev/urandom:/dev/random",
                    f"--bind={tmp_base / 'home'}:/root",
                    f"--bind={tmp_base / 'tmp'}:/tmp",
                    f"--bind={tmp_base / 'shm'}:/dev/shm",
                    f"--bind={socket_root}:/tmp/.X11-unix",
                    f"--bind={auth}:/root/.Xauthority",
                    "/usr/bin/env",
                    "-i",
                    "HOME=/root",
                    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                    "LANG=C.UTF-8",
                    "XAUTHORITY=/root/.Xauthority",
                    f"DISPLAY=:{display}",
                    "/usr/bin/dbus-run-session",
                    "--",
                    "/usr/bin/chatgpt",
                    "--no-sandbox",
                    "--disable-gpu",
                    "--disable-dev-shm-usage",
                    "--user-data-dir=/root/profile",
                    "--remote-debugging-pipe",
                ],
                env=proot_env,
                stdin=subprocess.DEVNULL,
                stdout=log_f,
                stderr=log_f,
                close_fds=False,
                preexec_fn=setup_child,
            )
        procs.append(app)
        child_sock.close()
        child_sock = None

        t0 = time.monotonic()
        try:
            browser_version = _cdp_pipe_call(parent_sock, 1, "Browser.getVersion", {}, float(timeout_secs))
        except Exception as exc:
            tail = ""
            if app_log.is_file():
                tail = app_log.read_text(encoding="utf-8", errors="replace")[-1200:]
            raise RuntimeError(f"{exc}; app_rc={app.poll()}; log_tail={tail!r}") from None
        # Give Electron a brief moment to register its initial renderer target.
        targets_res: dict[str, Any] = {"targetInfos": []}
        poll_end = time.monotonic() + min(12.0, float(timeout_secs))
        while time.monotonic() < poll_end:
            targets_res = _cdp_pipe_call(parent_sock, 2, "Target.getTargets", {}, 8.0)
            infos = targets_res.get("targetInfos", [])
            if any(t.get("type") == "page" and t.get("url", "").startswith(("http", "app:", "file:")) for t in infos):
                break
            time.sleep(0.4)

        safe_targets = [
            {
                "type": t.get("type"),
                "title": t.get("title"),
                "url_origin": t.get("url", "").split("?", 1)[0].split("#", 1)[0],
            }
            for t in targets_res.get("targetInfos", [])
        ]
        try:
            _cdp_pipe_call(parent_sock, 3, "Browser.close", {}, 3.0)
        except Exception:
            pass
        return {
            "state": "ok",
            "version": ver,
            "account_free": True,
            "display": display,
            "boot_ms": int((time.monotonic() - t0) * 1000),
            "browser_version": browser_version,
            "targets": safe_targets,
        }
    finally:
        if parent_sock is not None:
            parent_sock.close()
        if child_sock is not None:
            child_sock.close()
        for p in reversed(procs):
            if p.poll() is None:
                try:
                    os.killpg(p.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    p.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(p.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    try:
                        p.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        pass
        # Only unlink X lock/socket if we still own them.
        if xvnc_pid is not None and x_lock.is_file():
            try:
                if x_lock.read_text(encoding="utf-8", errors="replace").strip() == str(xvnc_pid):
                    x_lock.unlink(missing_ok=True)
                    x_sock.unlink(missing_ok=True)
            except OSError:
                pass
        shutil.rmtree(tmp_base, ignore_errors=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="chatgpt-desktop",
        description="Manage immutable Linux ChatGPT AppHost rootfs generations.",
    )
    parser.add_argument("--install-root", type=Path, default=None)
    parser.add_argument("--profile-dir", type=Path, default=None)
    parser.add_argument("--repo-url", default=DEFAULT_REPO_URL)
    parser.add_argument("--keyring", type=Path, default=DEFAULT_KEYRING_PATH)
    parser.add_argument("--prefix", type=Path, default=None)

    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check", help="Check latest official GPG-verified arm64 chatgpt .deb")
    sub.add_parser("status", help="Show installed generations, active/previous links, and profile status")

    p_inst = sub.add_parser("install", help="Download/verify .deb and provision immutable rootfs generation")
    p_inst.add_argument("--version", default=None, help="Require exact version match")
    p_inst.add_argument("--deb", type=Path, default=None, help="Use a pre-downloaded local .deb")
    p_inst.add_argument("--activate", action="store_true", help="Activate this generation after install")
    p_inst.add_argument("--keep-deb", action="store_true", help="Retain cached .deb after install")

    p_act = sub.add_parser("activate", help="Atomically activate an installed generation")
    p_act.add_argument("--version", required=True)

    sub.add_parser("rollback", help="Atomically roll back to the previous generation")

    p_probe = sub.add_parser("probe", help="Run isolated account-free PRoot/Xvnc CDP boot probe")
    p_probe.add_argument("--version", default=None)
    p_probe.add_argument("--timeout", type=int, default=35)

    args = parser.parse_args(argv)
    home = Path.home()
    install_root = args.install_root or default_install_root(home)
    profile_dir = args.profile_dir or default_profile_dir(home)
    prefix = args.prefix or Path(os.environ.get("PREFIX", "/data/data/com.termux/files/usr"))

    try:
        if args.cmd == "check":
            latest = discover_latest_deb(args.repo_url, args.keyring)
            out = {"latest": latest, "installed": status_report(install_root, profile_dir)}
            print(json.dumps(out, indent=2))
            return 0

        if args.cmd == "status":
            print(json.dumps(status_report(install_root, profile_dir), indent=2))
            return 0

        if args.cmd == "install":
            latest = discover_latest_deb(args.repo_url, args.keyring)
            if args.version and latest["version"] != args.version:
                raise RuntimeError(
                    f"requested version {args.version!r} does not match official repo version {latest['version']!r}"
                )
            ver = latest["version"]
            if args.deb:
                deb_path = args.deb
                if not deb_path.is_file():
                    raise RuntimeError(f"local deb not found: {deb_path}")
                if sha256_file(deb_path) != latest["sha256"]:
                    raise RuntimeError("local deb SHA-256 does not match GPG-verified Packages digest")
            else:
                dl_dir = install_root / "downloads"
                deb_path = download_verified_deb(
                    latest["url"],
                    dl_dir / f"chatgpt_{ver}_arm64.deb",
                    latest["sha256"],
                    latest["size"],
                )
            gen_path = provision_rootfs_generation(install_root, ver, deb_path, latest, prefix)
            if not args.keep_deb and not args.deb and deb_path.is_file():
                deb_path.unlink(missing_ok=True)
            if args.activate:
                st = activate_generation(install_root, ver)
            else:
                st = status_report(install_root, profile_dir)
            print(json.dumps({"installed_path": str(gen_path), "version": ver, "status": st}, indent=2))
            return 0

        if args.cmd == "activate":
            print(json.dumps(activate_generation(install_root, args.version), indent=2))
            return 0

        if args.cmd == "rollback":
            print(json.dumps(rollback_generation(install_root), indent=2))
            return 0

        if args.cmd == "probe":
            print(json.dumps(probe_generation(install_root, args.version, prefix, args.timeout), indent=2))
            return 0
    except Exception as exc:
        print(json.dumps({"state": "error", "error": str(exc)}), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

"""Runtime process environment planning."""

from __future__ import annotations

import shlex
from pathlib import Path


UNSET_NAMES = (
    "CODEX_MANAGED_BY_NPM",
    "CODEX_MANAGED_BY_BUN",
    "CODEX_MANAGED_PACKAGE_ROOT",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
)


def shell_exports(
    *,
    runtime_dir: str,
    runtime_exe: str,
    set_home: bool,
    home: str,
    tmpdir: str,
    cert_file: str,
    cert_dir: str,
    prefix: str,
    path: str,
    browser: str,
    ssl_cert_file: str,
    ssl_cert_dir: str,
    xdg_config_home: str,
    xdg_cache_home: str,
    xdg_data_home: str,
    godebug: str,
    bwrap_quiet: str,
    termux_open_url_available: bool,
) -> str:
    exports = {
        "TMPDIR": tmpdir,
        "TMP": tmpdir,
        "TEMP": tmpdir,
        "SQLITE_TMPDIR": tmpdir,
        "SSL_CERT_FILE": ssl_cert_file or cert_file,
        "CODEX_SELF_EXE": runtime_exe,
        "CODEX_CODE_MODE_HOST_PATH": f"{runtime_dir}/codex-code-mode-host",
        "CODEX_TERMUX_BWRAP_COMPAT_QUIET": bwrap_quiet or "1",
        "PATH": f"{runtime_dir}/codex-path:{runtime_dir}/codex-resources:{prefix}/bin:{path}",
    }
    if set_home:
        exports.update({
            "HOME": home,
            "XDG_CONFIG_HOME": xdg_config_home or f"{home}/.config",
            "XDG_CACHE_HOME": xdg_cache_home or f"{home}/.cache",
            "XDG_DATA_HOME": xdg_data_home or f"{home}/.local/share",
            "GODEBUG": godebug or "netdns=go",
        })
    if Path(cert_dir).is_dir():
        exports["SSL_CERT_DIR"] = ssl_cert_dir or cert_dir
    if not browser and termux_open_url_available:
        exports["BROWSER"] = "termux-open-url"
    lines = [f"export {key}={shlex.quote(value)}" for key, value in exports.items()]
    lines.append("unset " + " ".join(UNSET_NAMES))
    return "\n".join(lines)

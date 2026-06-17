#!/data/data/com.termux/files/usr/bin/python3
"""Termux bubblewrap compatibility entrypoint for Codex.

Android kernels used by Termux commonly deny the namespace setup that
bubblewrap needs. This shim preserves the command-launch contract Codex
expects from bwrap, applies the environment and cwd options that matter for
process execution, and then execs the command after the `--` separator without
creating a Linux mount namespace.
"""

from __future__ import annotations

import os
import sys


TERMUX_DEFAULT_PATH = "/data/data/com.termux/files/usr/bin:/system/bin"


def verbose() -> bool:
    return os.environ.get("CODEX_TERMUX_BWRAP_COMPAT_VERBOSE") == "1"


def warn(message: str) -> None:
    if verbose():
        print(f"bwrap-termux-compat: {message}", file=sys.stderr)


def die(message: str, code: int = 1) -> None:
    print(f"bwrap-termux-compat: {message}", file=sys.stderr)
    raise SystemExit(code)


def read_fd_args(fd_text: str) -> list[str]:
    try:
        fd = int(fd_text)
    except ValueError:
        die(f"invalid --args fd: {fd_text}", 2)
    try:
        with os.fdopen(fd, "rb", closefd=False) as handle:
            data = handle.read()
    except OSError as exc:
        die(f"failed to read --args fd {fd}: {exc}", 1)
    if not data:
        return []
    return [os.fsdecode(part) for part in data.split(b"\0") if part]


def expand_args(argv: list[str]) -> list[str]:
    out: list[str] = [argv[0]]
    i = 1
    while i < len(argv):
        arg = argv[i]
        if arg == "--args":
            if i + 1 >= len(argv):
                die("--args requires an fd", 2)
            out.extend(read_fd_args(argv[i + 1]))
            i += 2
            continue
        out.append(arg)
        i += 1
    return out


def find_separator(argv: list[str]) -> int:
    try:
        return argv.index("--", 1)
    except ValueError:
        die("bubblewrap argv is missing command separator '--'", 2)


def parse_launch_options(options: list[str]) -> tuple[dict[str, str], str | None, str | None]:
    env = dict(os.environ)
    chdir: str | None = None
    argv0: str | None = None
    i = 0
    while i < len(options):
        arg = options[i]
        if arg == "--clearenv":
            env = {}
            i += 1
        elif arg == "--setenv" and i + 2 < len(options):
            env[options[i + 1]] = options[i + 2]
            i += 3
        elif arg == "--unsetenv" and i + 1 < len(options):
            env.pop(options[i + 1], None)
            i += 2
        elif arg == "--chdir" and i + 1 < len(options):
            chdir = options[i + 1]
            i += 2
        elif arg == "--argv0" and i + 1 < len(options):
            argv0 = options[i + 1]
            i += 2
        else:
            i += 1
    env.setdefault("PATH", TERMUX_DEFAULT_PATH)
    return env, chdir, argv0


def main(argv: list[str]) -> int:
    if len(argv) == 2 and argv[1] in {"--version", "version"}:
        print("bubblewrap termux compat for Codex")
        return 0
    if len(argv) == 2 and argv[1] in {"--help", "help", "-h"}:
        print("Usage: bwrap [BWRAP-OPTIONS] -- COMMAND [ARGS...]")
        print("  --argv0 ARGV0")
        print("  --perms OCTAL")
        print("Termux compat mode ignores namespace/mount options and execs COMMAND.")
        return 0

    expanded = expand_args(argv)
    sep = find_separator(expanded)
    command = expanded[sep + 1 :]
    if not command:
        die("bubblewrap argv is missing inner command after '--'", 2)

    env, chdir, argv0 = parse_launch_options(expanded[1:sep])
    if chdir:
        try:
            os.chdir(chdir)
        except OSError as exc:
            if os.environ.get("CODEX_TERMUX_BWRAP_COMPAT_STRICT_CHDIR") == "1":
                die(f"failed to chdir to {chdir}: {exc}", 1)
            warn(f"keeping cwd because --chdir failed: {chdir}: {exc}")

    executable = command[0]
    argv_for_exec = command[:]
    if argv0:
        argv_for_exec[0] = argv0
    os.execvpe(executable, argv_for_exec, env)
    return 127


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

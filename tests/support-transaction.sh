#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-support-transaction.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - "$ROOT_DIR" "$TMP_DIR" <<'PYTHON'
from __future__ import annotations

import json
import os
import shutil
import sys
from pathlib import Path

from wrapper import source
from wrapper import support_transaction as transaction
from wrapper.errors import IntegrityError


source_root = Path(sys.argv[1])
temp = Path(sys.argv[2])


def paths(name: str):
    root = temp / name / "home/.local/lib/codex/termux"
    state = temp / name / "home/.local/share/codex/termux"
    prefix = temp / name / "prefix"
    return {
        "root": root,
        "manager": root / "manager",
        "verified_manager": root / "verified-manager",
        "source": root / "source-snapshot",
        "verified_source": root / "verified-source-snapshot",
        "state": state,
        "prefix": prefix,
        "launcher": prefix / "bin/codex",
        "config": state / "system-config",
        "lower": state / "support-activation.json",
        "recovery": state / transaction.RECOVERY_NAME,
    }


def seed(item: dict[str, Path], launcher_text: str, config_text: str) -> None:
    item["launcher"].parent.mkdir(parents=True, exist_ok=True)
    item["config"].mkdir(parents=True, exist_ok=True)
    item["launcher"].write_text(launcher_text, encoding="utf-8")
    item["launcher"].chmod(0o755)
    (item["config"] / "config.toml").write_text(config_text, encoding="utf-8")


def prepare(item: dict[str, Path], commit: str, timestamp: str):
    return source.prepare_support_install(
        source_root=source_root,
        wrapper_root=item["root"],
        manager_link=item["manager"],
        verified_manager_link=item["verified_manager"],
        state_dir=item["state"],
        prefix=item["prefix"],
        installed_at=timestamp,
        wrapper_commit=commit,
    )


# Normal commit and forced launcher failure rollback.
main = paths("main")
seed(main, "old launcher\n", "old config\n")
first = prepare(main, "firstcommit", "2026-07-17T00:00:00+09:00")
first_target = Path(first.target)
first_source = Path(first.source_target)
assert main["manager"].resolve() == first_target.resolve()
assert main["verified_manager"].resolve() == first_target.resolve()
assert main["source"].resolve() == first_source.resolve()
assert main["verified_source"].resolve() == first_source.resolve()
assert (main["manager"] / "managed.sh").is_file()
assert (main["manager"] / "src/wrapper/support_transaction.py").is_file()
assert (main["manager"] / "src/wrapper/notification/hooks.py").is_file()
assert (main["source"] / "native/codex-launcher.c").is_file()
manifest = json.loads((main["manager"] / "support-manifest.json").read_text(encoding="utf-8"))
assert manifest["support_id"] == first.support_id
assert manifest["source_id"] == first.source_id
(main["config"] / "config.toml").write_text("first config\n", encoding="utf-8")
source.commit_support_install(Path(first.transaction_file))
assert not main["lower"].exists()
assert not main["recovery"].exists()
assert b"codex termux managed launcher" in main["launcher"].read_bytes()
first_launcher = main["launcher"].read_bytes()

second = prepare(main, "secondcommit", "2026-07-17T00:01:00+09:00")
second_target = Path(second.target)
second_source = Path(second.source_target)
(main["config"] / "config.toml").write_text("candidate config\n", encoding="utf-8")
os.environ["CODEX_TERMUX_INSTALL_FAIL_LAUNCHER"] = "1"
try:
    source.commit_support_install(Path(second.transaction_file))
except IntegrityError as exc:
    assert "launcher" in str(exc)
else:
    raise AssertionError("forced launcher failure was accepted")
finally:
    os.environ.pop("CODEX_TERMUX_INSTALL_FAIL_LAUNCHER", None)
assert main["manager"].resolve() == first_target.resolve()
assert main["source"].resolve() == first_source.resolve()
assert main["launcher"].read_bytes() == first_launcher
assert (main["config"] / "config.toml").read_text(encoding="utf-8") == "first config\n"
assert not second_target.exists() and not second_source.exists()
assert not main["lower"].exists() and not main["recovery"].exists()

# A new prepare recovers a stale switched transaction before creating a candidate.
stale = prepare(main, "stalecommit", "2026-07-17T00:02:00+09:00")
stale_target = Path(stale.target)
stale_source = Path(stale.source_target)
(main["config"] / "config.toml").write_text("stale hook config\n", encoding="utf-8")
replacement = prepare(main, "replacement", "2026-07-17T00:03:00+09:00")
assert not stale_target.exists() and not stale_source.exists()
assert main["verified_manager"].resolve() == first_target.resolve()
assert main["verified_source"].resolve() == first_source.resolve()
assert (main["config"] / "config.toml").read_text(encoding="utf-8") == "first config\n"
source.rollback_support_install(Path(replacement.transaction_file))
assert main["manager"].resolve() == first_target.resolve()
assert main["source"].resolve() == first_source.resolve()

# Simulate process death after launcher replacement but before commit.  The next
# prepare must restore launcher/config/pointers before switching another candidate.
after_launcher = prepare(main, "launchercrash", "2026-07-17T00:04:00+09:00")
recovery_data = json.loads(main["recovery"].read_text(encoding="utf-8"))
transaction._install_launcher(recovery_data)
recovery_data["status"] = "launcher-installed"
transaction._write_json_durable(main["recovery"], recovery_data, mode=0o600)
assert main["launcher"].read_bytes() != first_launcher or b"managed launcher" in main["launcher"].read_bytes()
post_crash = prepare(main, "postcrash", "2026-07-17T00:05:00+09:00")
assert main["launcher"].read_bytes() == first_launcher
assert not Path(after_launcher.target).exists()
assert not Path(after_launcher.source_target).exists()
source.rollback_support_install(Path(post_crash.transaction_file))

# Rollback can be interrupted after file restoration and safely retried.
retry = prepare(main, "retryrollback", "2026-07-17T00:06:00+09:00")
(main["config"] / "config.toml").write_text("retry candidate\n", encoding="utf-8")
os.environ["CODEX_TERMUX_INSTALL_CRASH_POINT"] = "rollback-after-files"
try:
    source.rollback_support_install(Path(retry.transaction_file))
except IntegrityError as exc:
    assert "interruption" in str(exc)
else:
    raise AssertionError("rollback interruption was not injected")
finally:
    os.environ.pop("CODEX_TERMUX_INSTALL_CRASH_POINT", None)
assert main["recovery"].is_file()
assert main["lower"].is_file()
source.rollback_support_install(Path(retry.transaction_file))
assert main["manager"].resolve() == first_target.resolve()
assert main["source"].resolve() == first_source.resolve()
assert main["launcher"].read_bytes() == first_launcher
assert not main["recovery"].exists() and not main["lower"].exists()

# Legacy physical directories are restored exactly, not left as compatibility links.
legacy = paths("legacy")
legacy["manager"].mkdir(parents=True)
legacy["source"].mkdir(parents=True)
(legacy["manager"] / "marker").write_text("legacy manager\n", encoding="utf-8")
(legacy["source"] / "marker").write_text("legacy source\n", encoding="utf-8")
seed(legacy, "legacy launcher\n", "legacy config\n")
legacy_activation = prepare(legacy, "migrationcommit", "2026-07-17T00:07:00+09:00")
assert legacy["manager"].is_symlink() and legacy["source"].is_symlink()
source.rollback_support_install(Path(legacy_activation.transaction_file))
assert legacy["manager"].is_dir() and not legacy["manager"].is_symlink()
assert legacy["source"].is_dir() and not legacy["source"].is_symlink()
assert (legacy["manager"] / "marker").read_text(encoding="utf-8") == "legacy manager\n"
assert (legacy["source"] / "marker").read_text(encoding="utf-8") == "legacy source\n"

# Missing backups fail closed and preserve the journal for operator recovery.
missing = paths("missing-backup")
seed(missing, "backup launcher\n", "backup config\n")
missing_activation = prepare(missing, "missingbackup", "2026-07-17T00:08:00+09:00")
missing_data = json.loads(missing["recovery"].read_text(encoding="utf-8"))
shutil.rmtree(Path(missing_data["system_config_backup"]))
try:
    source.rollback_support_install(Path(missing_activation.transaction_file))
except IntegrityError as exc:
    assert "backup is missing" in str(exc)
else:
    raise AssertionError("missing rollback backup was accepted")
assert missing["recovery"].is_file()
assert missing["lower"].is_file()

# Corrupt recovery journals are rejected before any new source is activated.
corrupt = paths("corrupt")
seed(corrupt, "corrupt launcher\n", "corrupt config\n")
corrupt["state"].mkdir(parents=True, exist_ok=True)
corrupt["recovery"].write_text("{not-json\n", encoding="utf-8")
try:
    prepare(corrupt, "mustnotactivate", "2026-07-17T00:09:00+09:00")
except IntegrityError as exc:
    assert "journal is unreadable" in str(exc)
else:
    raise AssertionError("corrupt recovery journal was ignored")
assert not corrupt["manager"].exists()
assert corrupt["launcher"].read_text(encoding="utf-8") == "corrupt launcher\n"
PYTHON

printf 'support-transaction: ok\n'

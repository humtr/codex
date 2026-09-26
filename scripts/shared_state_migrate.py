#!/usr/bin/env python3
"""Bounded one-shot migration of legacy Codex profile conversations into shared state.

This operator tool intentionally does not merge or write SQLite. Legacy state_5.sqlite
files are read only to select at most five recent recoverable conversations per profile,
preserve explicit names in the manifest, and fail closed on hidden thread-scoped
dependencies. The authoritative rollout JSONL files are the only payload copied.

Steady-state conversation semantics remain owned by upstream Codex. After copy, the
finalize phase uses only official app-server APIs: thread/list repairs metadata from
rollout files, thread/resume materializes paginated history, and thread/name/set restores
explicit names. Once finalize starts, rollback is whole-backup restoration rather than
custom SQLite reversal.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import sqlite3
import stat
import subprocess
import sys
import time
from typing import Any

FORMAT = "codex-shared-state-migration-v1"
JOURNAL_FORMAT = "codex-shared-state-migration-journal-v1"
LIMIT_PER_PROFILE = 5
MAX_TARGET_ROLLOUTS = 8192
ACTIVITY_COLUMNS = (
    "recency_at_ms",
    "updated_at_ms",
    "recency_at",
    "updated_at",
    "created_at_ms",
    "created_at",
)
ALLOWED_ROLLOUT_ROOTS = {"sessions", "archived_sessions"}


class MigrationError(RuntimeError):
    pass


def _connect_ro(path: Path) -> sqlite3.Connection:
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    con.execute("PRAGMA query_only=ON")
    return con


def _table_names(con: sqlite3.Connection) -> list[str]:
    return [
        row[0]
        for row in con.execute(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )
    ]


def _columns(con: sqlite3.Connection, table: str) -> list[str]:
    escaped = table.replace('"', '""')
    return [row["name"] for row in con.execute(f'PRAGMA table_info("{escaped}")')]


def _quote_ident(value: str) -> str:
    return '"' + value.replace('"', '""') + '"'


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _read_session_meta(path: Path) -> dict[str, Any]:
    if path.is_symlink():
        raise MigrationError(f"rollout is a symlink: {path}")
    with path.open("rb") as handle:
        for raw in handle:
            if not raw.strip():
                continue
            if len(raw) > 4 * 1024 * 1024:
                raise MigrationError(f"session metadata line is too large: {path}")
            try:
                obj = json.loads(raw)
            except json.JSONDecodeError as exc:
                raise MigrationError(f"invalid rollout metadata JSON: {path}") from exc
            if not isinstance(obj, dict) or obj.get("type") != "session_meta":
                raise MigrationError(f"rollout does not begin with session_meta: {path}")
            payload = obj.get("payload")
            if not isinstance(payload, dict) or not isinstance(payload.get("id"), str):
                raise MigrationError(f"rollout session_meta has no thread id: {path}")
            return {
                "thread_id": payload["id"],
                "history_mode": payload.get("history_mode"),
            }
    raise MigrationError(f"empty rollout: {path}")


def _resolve_rollout(
    profile_root: Path, raw: str, *, alias_root: Path | None = None
) -> tuple[Path, Path]:
    profile_root = profile_root.resolve(strict=True)
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = profile_root / candidate
    candidate = Path(os.path.abspath(candidate))

    # The state row must still name a logical rollout path owned by this profile.
    try:
        logical_rel = candidate.relative_to(profile_root)
    except ValueError as exc:
        raise MigrationError(f"rollout path is outside profile root: {candidate}") from exc
    if (
        not logical_rel.parts
        or logical_rel.parts[0] not in ALLOWED_ROLLOUT_ROOTS
        or candidate.suffix != ".jsonl"
    ):
        raise MigrationError(
            f"rollout is outside canonical session roots: {candidate}"
        )

    # Broken legacy aliases are stale references, not migration-fatal corruption.
    if not candidate.exists():
        raise FileNotFoundError(candidate)

    resolved = candidate.resolve(strict=True)
    if resolved.suffix != ".jsonl" or not resolved.is_file():
        raise MigrationError(f"rollout is not a JSONL file: {candidate}")

    if alias_root is None:
        # Canonical target paths may never escape their own root through symlinks.
        try:
            resolved_rel = resolved.relative_to(profile_root)
        except ValueError as exc:
            raise MigrationError(f"rollout escapes profile root: {candidate}") from exc
        if (
            not resolved_rel.parts
            or resolved_rel.parts[0] not in ALLOWED_ROLLOUT_ROOTS
        ):
            raise MigrationError(
                f"rollout is outside canonical session roots: {candidate}"
            )
        if candidate.is_symlink():
            raise MigrationError(f"canonical rollout is a symlink: {candidate}")
    else:
        # Historical profile stores used cross-profile rollout aliases. Accept only
        # aliases whose resolved target remains under another canonical legacy
        # profile/session root. Nothing outside the declared legacy root is readable.
        allowed = alias_root.resolve(strict=True)
        try:
            resolved_rel = resolved.relative_to(allowed)
        except ValueError as exc:
            raise MigrationError(
                f"legacy rollout alias escapes legacy root: {candidate}"
            ) from exc
        if len(resolved_rel.parts) < 3:
            raise MigrationError(f"legacy rollout alias target is invalid: {candidate}")
        target_profile = allowed / resolved_rel.parts[0]
        if (
            not target_profile.is_dir()
            or target_profile.is_symlink()
            or resolved_rel.parts[1] not in ALLOWED_ROLLOUT_ROOTS
        ):
            raise MigrationError(
                f"legacy rollout alias target is outside profile sessions: {candidate}"
            )

    return resolved, logical_rel


def _resolve_target_rollout(
    target_root: Path, legacy_root: Path, raw: str
) -> tuple[Path, Path, str, str | None]:
    target_root = target_root.resolve(strict=True)
    legacy_root = legacy_root.resolve(strict=True)
    candidate = Path(raw)
    if not candidate.is_absolute():
        candidate = target_root / candidate
    candidate = Path(os.path.abspath(candidate))

    try:
        logical_rel = candidate.relative_to(target_root)
    except ValueError as exc:
        raise MigrationError(f"target rollout path is outside target root: {candidate}") from exc
    if (
        not logical_rel.parts
        or logical_rel.parts[0] not in ALLOWED_ROLLOUT_ROOTS
        or candidate.suffix != ".jsonl"
    ):
        raise MigrationError(f"target rollout path is outside canonical session roots: {candidate}")

    if not os.path.lexists(candidate):
        raise FileNotFoundError(candidate)

    if candidate.is_symlink():
        original_link = os.readlink(candidate)
        try:
            resolved = candidate.resolve(strict=True)
        except FileNotFoundError:
            raise
        try:
            rel = resolved.relative_to(legacy_root)
        except ValueError as exc:
            raise MigrationError(
                f"canonical rollout alias escapes legacy root: {candidate}"
            ) from exc
        if len(rel.parts) < 3:
            raise MigrationError(f"canonical rollout alias target is invalid: {candidate}")
        profile = legacy_root / rel.parts[0]
        if (
            not profile.is_dir()
            or profile.is_symlink()
            or rel.parts[1] not in ALLOWED_ROLLOUT_ROOTS
            or resolved.suffix != ".jsonl"
            or not resolved.is_file()
        ):
            raise MigrationError(
                f"canonical rollout alias target is outside legacy profile sessions: {candidate}"
            )
        return resolved, logical_rel, "legacy_alias", original_link

    resolved = candidate.resolve(strict=True)
    try:
        rel = resolved.relative_to(target_root)
    except ValueError as exc:
        raise MigrationError(f"canonical rollout escapes target root: {candidate}") from exc
    if not rel.parts or rel.parts[0] not in ALLOWED_ROLLOUT_ROOTS:
        raise MigrationError(f"canonical rollout is outside session roots: {candidate}")
    if resolved.suffix != ".jsonl" or not resolved.is_file():
        raise MigrationError(f"canonical rollout is not a JSONL file: {candidate}")
    return resolved, logical_rel, "regular", None


def _has_rollouts(profile_root: Path) -> bool:
    for name in ALLOWED_ROLLOUT_ROOTS:
        root = profile_root / name
        if root.is_dir() and next(root.rglob("*.jsonl"), None) is not None:
            return True
    return False


def _unsupported_dependencies(profile_root: Path, thread_ids: list[str]) -> None:
    if not thread_ids:
        return
    marks = ",".join("?" for _ in thread_ids)
    for db in sorted(profile_root.glob("*.sqlite")):
        # Upstream owns logs_2.sqlite as a dedicated diagnostic log store.
        # Its thread_id column is only a log filter and is not resume/history state.
        if db.name == "logs_2.sqlite":
            continue
        con = _connect_ro(db)
        try:
            for table in _table_names(con):
                if table.startswith("sqlite_"):
                    continue
                cols = _columns(con, table)
                if "thread_id" not in cols:
                    continue
                if db.name == "thread_history_1.sqlite":
                    # Upstream explicitly treats this projection as rebuildable from rollout.
                    continue
                if db.name == "state_5.sqlite" and table == "threads":
                    continue
                q = (
                    f"SELECT count(*) FROM {_quote_ident(table)} "
                    f"WHERE thread_id IN ({marks})"
                )
                count = con.execute(q, thread_ids).fetchone()[0]
                if count:
                    raise MigrationError(
                        f"selected conversations have unsupported dependency "
                        f"{db.name}:{table}"
                    )
        finally:
            con.close()


def _select_profile(profile_root: Path) -> dict[str, Any]:
    db = profile_root / "state_5.sqlite"
    if not db.is_file():
        if _has_rollouts(profile_root):
            raise MigrationError(
                f"profile {profile_root.name} has rollouts but no supported state_5.sqlite"
            )
        return {
            "profile": profile_root.name,
            "activity_column": None,
            "selected": [],
            "stale_skipped": 0,
        }

    con = _connect_ro(db)
    try:
        if "threads" not in _table_names(con):
            raise MigrationError(f"profile {profile_root.name} has no threads table")
        cols = _columns(con, "threads")
        required = {"id", "rollout_path"}
        if not required.issubset(cols):
            raise MigrationError(f"profile {profile_root.name} has unsupported threads schema")
        activity = next((col for col in ACTIVITY_COLUMNS if col in cols), None)
        if activity is None:
            raise MigrationError(
                f"profile {profile_root.name} has no safe activity ordering column"
            )
        select_cols = ["id", "rollout_path", activity]
        for optional in ("history_mode", "name", "daybreak_enabled", "project_id", "section"):
            if optional in cols:
                select_cols.append(optional)
        query = (
            "SELECT "
            + ",".join(_quote_ident(col) for col in select_cols)
            + f" FROM threads ORDER BY {_quote_ident(activity)} DESC, id ASC"
        )
        selected: list[dict[str, Any]] = []
        stale = 0
        seen: set[str] = set()
        for row in con.execute(query):
            thread_id = row["id"]
            if not isinstance(thread_id, str) or not thread_id or thread_id in seen:
                continue
            raw_path = row["rollout_path"]
            if not isinstance(raw_path, str) or not raw_path:
                stale += 1
                continue
            try:
                source_path, source_rel = _resolve_rollout(
                    profile_root, raw_path, alias_root=profile_root.parent
                )
            except FileNotFoundError:
                stale += 1
                continue
            meta = _read_session_meta(source_path)
            if meta["thread_id"] != thread_id:
                raise MigrationError(
                    f"profile {profile_root.name} rollout identity does not match state DB"
                )
            state_mode = row["history_mode"] if "history_mode" in row.keys() else None
            if (
                state_mode
                and meta["history_mode"]
                and state_mode != meta["history_mode"]
            ):
                raise MigrationError(
                    f"profile {profile_root.name} history mode does not match rollout"
                )
            if "daybreak_enabled" in row.keys() and row["daybreak_enabled"] not in (None, 0):
                raise MigrationError(
                    f"profile {profile_root.name} selected thread has unsupported "
                    "daybreak preference"
                )
            for unsupported in ("project_id", "section"):
                if unsupported in row.keys() and row[unsupported] not in (None, "", 0):
                    raise MigrationError(
                        f"profile {profile_root.name} selected thread has unsupported "
                        f"{unsupported} metadata"
                    )
            name = row["name"] if "name" in row.keys() else None
            if name == "":
                name = None
            if name is not None and not isinstance(name, str):
                raise MigrationError(f"profile {profile_root.name} has invalid thread name")
            selected.append(
                {
                    "thread_id": thread_id,
                    "source_rel": source_rel.as_posix(),
                    "sha256": _sha256(source_path),
                    "size": source_path.stat().st_size,
                    "history_mode": meta["history_mode"] or state_mode,
                    "activity": row[activity],
                    "name": name,
                }
            )
            seen.add(thread_id)
            if len(selected) == LIMIT_PER_PROFILE:
                break
    finally:
        con.close()

    _unsupported_dependencies(
        profile_root, [entry["thread_id"] for entry in selected]
    )
    return {
        "profile": profile_root.name,
        "activity_column": activity,
        "selected": selected,
        "stale_skipped": stale,
    }


def _target_state_rows(target_root: Path, thread_ids: set[str]) -> dict[str, dict[str, Any]]:
    db = target_root / "state_5.sqlite"
    if not db.is_file() or not thread_ids:
        return {}
    con = _connect_ro(db)
    try:
        if "threads" not in _table_names(con):
            return {}
        cols = _columns(con, "threads")
        if not {"id", "rollout_path"}.issubset(cols):
            raise MigrationError("target state DB has unsupported threads schema")
        wanted = ["id", "rollout_path"]
        if "name" in cols:
            wanted.append("name")
        marks = ",".join("?" for _ in thread_ids)
        q = (
            "SELECT "
            + ",".join(_quote_ident(c) for c in wanted)
            + f" FROM threads WHERE id IN ({marks})"
        )
        return {row["id"]: dict(row) for row in con.execute(q, sorted(thread_ids))}
    finally:
        con.close()


def _scan_target_selected(
    target_root: Path, selected_ids: set[str]
) -> dict[str, list[dict[str, Any]]]:
    found: dict[str, list[dict[str, Any]]] = {tid: [] for tid in selected_ids}
    count = 0
    for root_name in sorted(ALLOWED_ROLLOUT_ROOTS):
        root = target_root / root_name
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.jsonl")):
            count += 1
            if count > MAX_TARGET_ROLLOUTS:
                raise MigrationError("target rollout scan exceeds safety bound")
            # Canonical symlinks are handled only through authoritative state rows,
            # where legacy-alias normalization can be journaled and rolled back.
            if path.is_symlink():
                continue
            meta = _read_session_meta(path)
            tid = meta["thread_id"]
            if tid not in selected_ids:
                continue
            found[tid].append(
                {
                    "rel": path.resolve(strict=True)
                    .relative_to(target_root.resolve(strict=True))
                    .as_posix(),
                    "sha256": _sha256(path),
                }
            )
    return found


def _choose_source_name(contributors: list[dict[str, Any]]) -> str | None:
    named = [c for c in contributors if c["name"] is not None]
    if not named:
        return None
    activity_columns = {c["activity_column"] for c in named}
    if len(activity_columns) != 1:
        # Comparing values from different activity semantics is unsafe.
        names = {c["name"] for c in named}
        if len(names) == 1:
            return named[0]["name"]
        raise MigrationError(
            "duplicate conversation has conflicting names under incomparable activity schemas"
        )
    try:
        winner = max(named, key=lambda c: (c["activity"], c["profile"]))
    except TypeError as exc:
        raise MigrationError(
            "duplicate conversation names cannot be ordered deterministically"
        ) from exc
    return winner["name"]


def build_plan(legacy_root: Path, target_root: Path) -> dict[str, Any]:
    legacy_root = legacy_root.expanduser().resolve(strict=True)
    target_root = target_root.expanduser().resolve(strict=True)
    if legacy_root == target_root:
        raise MigrationError("legacy root and target root must differ")
    profiles = [
        _select_profile(path)
        for path in sorted(legacy_root.iterdir(), key=lambda p: p.name)
        if path.is_dir()
    ]

    by_id: dict[str, list[dict[str, Any]]] = {}
    for profile in profiles:
        for rank, entry in enumerate(profile["selected"], start=1):
            c = dict(entry)
            c["profile"] = profile["profile"]
            c["rank"] = rank
            c["activity_column"] = profile["activity_column"]
            by_id.setdefault(entry["thread_id"], []).append(c)

    selected_ids = set(by_id)
    target_rows = _target_state_rows(target_root, selected_ids)
    target_scan = _scan_target_selected(target_root, selected_ids)
    unique: list[dict[str, Any]] = []

    for thread_id in sorted(by_id):
        contributors = by_id[thread_id]
        hashes = {c["sha256"] for c in contributors}
        if len(hashes) != 1:
            raise MigrationError("duplicate conversation has divergent rollout content")
        digest = next(iter(hashes))
        history_modes = {c["history_mode"] for c in contributors}
        if len(history_modes) != 1:
            raise MigrationError("duplicate conversation has divergent history mode")
        representative = min(
            contributors, key=lambda c: (c["source_rel"], c["profile"])
        )
        chosen_name = _choose_source_name(contributors)

        target_status = "copy"
        target_rel = representative["source_rel"]
        target_name = None

        state_row = target_rows.get(thread_id)
        if state_row is not None:
            raw = state_row.get("rollout_path")
            if not isinstance(raw, str) or not raw:
                raise MigrationError("target thread has no rollout path")
            try:
                target_path, rel, target_kind, _ = _resolve_target_rollout(
                    target_root, legacy_root, raw
                )
            except FileNotFoundError as exc:
                raise MigrationError("target thread rollout is missing") from exc
            meta = _read_session_meta(target_path)
            if meta["thread_id"] != thread_id or _sha256(target_path) != digest:
                raise MigrationError("target already has divergent state for selected thread")
            target_status = (
                "replace_alias" if target_kind == "legacy_alias" else "identical"
            )
            target_rel = rel.as_posix()
            if "name" in state_row:
                target_name = state_row["name"] or None
            chosen_name = target_name
        else:
            matches = target_scan.get(thread_id, [])
            if matches:
                match_hashes = {m["sha256"] for m in matches}
                if len(match_hashes) != 1 or digest not in match_hashes:
                    raise MigrationError(
                        "target filesystem has divergent state for selected thread"
                    )
                target_status = "identical"
                target_rel = min(m["rel"] for m in matches)
                chosen_name = None
            else:
                dest = target_root / target_rel
                if os.path.lexists(dest):
                    if dest.is_symlink():
                        target_path, rel, target_kind, _ = _resolve_target_rollout(
                            target_root, legacy_root, target_rel
                        )
                        meta = _read_session_meta(target_path)
                        if (
                            target_kind != "legacy_alias"
                            or meta["thread_id"] != thread_id
                            or _sha256(target_path) != digest
                        ):
                            raise MigrationError("target rollout path collision")
                        target_status = "replace_alias"
                        target_rel = rel.as_posix()
                    else:
                        meta = _read_session_meta(dest)
                        if meta["thread_id"] != thread_id or _sha256(dest) != digest:
                            raise MigrationError("target rollout path collision")
                        target_status = "identical"

        unique.append(
            {
                "thread_id": thread_id,
                "sha256": digest,
                "size": representative["size"],
                "history_mode": representative["history_mode"],
                "representative_profile": representative["profile"],
                "source_rel": representative["source_rel"],
                "target_rel": target_rel,
                "target_status": target_status,
                "name": chosen_name,
                "contributors": [
                    {
                        "profile": c["profile"],
                        "rank": c["rank"],
                        "activity_column": c["activity_column"],
                        "activity": c["activity"],
                    }
                    for c in sorted(
                        contributors, key=lambda c: (c["profile"], c["rank"])
                    )
                ],
            }
        )

    target_paths: dict[str, str] = {}
    for entry in unique:
        prior = target_paths.get(entry["target_rel"])
        if prior is not None and prior != entry["thread_id"]:
            raise MigrationError(
                "distinct selected conversations collide on one canonical rollout path"
            )
        target_paths[entry["target_rel"]] = entry["thread_id"]

    plan: dict[str, Any] = {
        "format": FORMAT,
        "limit_per_profile": LIMIT_PER_PROFILE,
        "legacy_root": str(legacy_root),
        "target_root": str(target_root),
        "profiles": profiles,
        "unique": unique,
    }
    plan["plan_sha256"] = _plan_digest(plan)
    return plan


def _plan_digest(plan: dict[str, Any]) -> str:
    payload = dict(plan)
    payload.pop("plan_sha256", None)
    raw = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(raw).hexdigest()


def _atomic_json(path: Path, data: dict[str, Any]) -> None:
    path = path.expanduser()
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    tmp = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    fd = os.open(tmp, flags, 0o600)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(data, handle, sort_keys=True, separators=(",", ":"))
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, path)
        os.chmod(path, 0o600)
        dirfd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(dirfd)
        finally:
            os.close(dirfd)
    finally:
        if tmp.exists():
            tmp.unlink()


def write_plan(path: Path, plan: dict[str, Any]) -> None:
    if path.exists():
        raise MigrationError(f"refusing to replace existing manifest: {path}")
    _atomic_json(path, plan)


def load_plan(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        plan = json.load(handle)
    if plan.get("format") != FORMAT or plan.get("limit_per_profile") != LIMIT_PER_PROFILE:
        raise MigrationError("unsupported migration manifest")
    if plan.get("plan_sha256") != _plan_digest(plan):
        raise MigrationError("migration manifest digest mismatch")
    return plan


def _safe_target_path(target_root: Path, rel: str) -> Path:
    pure = Path(rel)
    if pure.is_absolute() or ".." in pure.parts or not pure.parts:
        raise MigrationError("unsafe target relative path")
    if pure.parts[0] not in ALLOWED_ROLLOUT_ROOTS or pure.suffix != ".jsonl":
        raise MigrationError("target path is outside canonical rollout roots")
    return target_root / pure


def _mkdir_private_chain(target_root: Path, parent: Path) -> None:
    target_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    current = target_root
    for part in parent.relative_to(target_root).parts:
        current = current / part
        if current.exists():
            if current.is_symlink() or not current.is_dir():
                raise MigrationError(f"unsafe target directory: {current}")
        else:
            current.mkdir(mode=0o700)
        os.chmod(current, 0o700)


def apply_plan(plan: dict[str, Any], journal_path: Path) -> dict[str, Any]:
    if journal_path.exists():
        raise MigrationError(f"journal already exists: {journal_path}")
    legacy_root = Path(plan["legacy_root"]).resolve(strict=True)
    target_root = Path(plan["target_root"]).resolve(strict=True)

    # Re-plan immediately before mutation to close selection/source/target races.
    fresh = build_plan(legacy_root, target_root)
    if fresh["plan_sha256"] != plan["plan_sha256"]:
        raise MigrationError("migration plan is stale; generate a new plan")

    journal: dict[str, Any] = {
        "format": JOURNAL_FORMAT,
        "plan_sha256": plan["plan_sha256"],
        "target_root": str(target_root),
        "status": "applying",
        "pending": None,
        "created": [],
    }
    _atomic_json(journal_path, journal)

    for entry in plan["unique"]:
        if entry["target_status"] == "identical":
            continue
        profile_root = legacy_root / entry["representative_profile"]
        source = profile_root / entry["source_rel"]
        source = source.resolve(strict=True)
        meta = _read_session_meta(source)
        if meta["thread_id"] != entry["thread_id"] or _sha256(source) != entry["sha256"]:
            raise MigrationError("source changed after planning")

        dest = _safe_target_path(target_root, entry["target_rel"])
        _mkdir_private_chain(target_root, dest.parent)

        kind = entry["target_status"]
        original_link = None
        if kind == "replace_alias":
            if not dest.is_symlink():
                raise MigrationError("target alias changed after planning")
            target_path, rel, target_kind, original_link = _resolve_target_rollout(
                target_root, legacy_root, entry["target_rel"]
            )
            meta = _read_session_meta(target_path)
            if (
                target_kind != "legacy_alias"
                or rel.as_posix() != entry["target_rel"]
                or meta["thread_id"] != entry["thread_id"]
                or _sha256(target_path) != entry["sha256"]
            ):
                raise MigrationError("target alias changed after planning")
        else:
            if os.path.lexists(dest):
                if dest.is_symlink():
                    raise MigrationError("target changed after planning")
                meta = _read_session_meta(dest)
                if meta["thread_id"] == entry["thread_id"] and _sha256(dest) == entry["sha256"]:
                    continue
                raise MigrationError("target changed after planning")

        pending = {
            "kind": kind,
            "target_rel": entry["target_rel"],
            "thread_id": entry["thread_id"],
            "sha256": entry["sha256"],
            "original_link": original_link,
            "dev": None,
            "ino": None,
        }
        journal["pending"] = pending
        _atomic_json(journal_path, journal)

        if kind == "replace_alias":
            dest.unlink()
            dirfd = os.open(dest.parent, os.O_RDONLY)
            try:
                os.fsync(dirfd)
            finally:
                os.close(dirfd)

        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
        try:
            fd = os.open(dest, flags, 0o600)
        except FileExistsError as exc:
            journal["pending"] = None
            _atomic_json(journal_path, journal)
            raise MigrationError("target changed after planning") from exc

        opened = os.fstat(fd)
        pending["dev"] = opened.st_dev
        pending["ino"] = opened.st_ino
        _atomic_json(journal_path, journal)

        with source.open("rb") as src, os.fdopen(fd, "wb") as out:
            for chunk in iter(lambda: src.read(1024 * 1024), b""):
                out.write(chunk)
            out.flush()
            os.fsync(out.fileno())

        if _sha256(dest) != entry["sha256"]:
            raise MigrationError("copied rollout hash mismatch")
        os.chmod(dest, 0o600)
        dirfd = os.open(dest.parent, os.O_RDONLY)
        try:
            os.fsync(dirfd)
        finally:
            os.close(dirfd)

        journal["created"].append(dict(pending))
        journal["pending"] = None
        _atomic_json(journal_path, journal)


    journal["status"] = "applied"
    _atomic_json(journal_path, journal)
    return journal


def _rollback_journal_entry(
    target_root: Path, entry: dict[str, Any], *, allow_partial: bool
) -> None:
    dest = _safe_target_path(target_root, entry["target_rel"])
    kind = entry.get("kind", "copy")
    original_link = entry.get("original_link")

    if os.path.lexists(dest):
        if dest.is_symlink():
            if kind == "replace_alias" and original_link is not None:
                if os.readlink(dest) == original_link:
                    return
            raise MigrationError("rollback target is an unexpected symlink")

        st = dest.stat()
        if not stat.S_ISREG(st.st_mode):
            raise MigrationError("rollback target is unsafe")
        dev = entry.get("dev")
        ino = entry.get("ino")
        if dev is None or ino is None:
            raise MigrationError("rollback journal lacks target inode identity")
        if st.st_dev != dev or st.st_ino != ino:
            raise MigrationError("rollback target inode changed after migration")
        if not allow_partial:
            meta = _read_session_meta(dest)
            if meta["thread_id"] != entry["thread_id"] or _sha256(dest) != entry["sha256"]:
                raise MigrationError("rollback target changed after migration")
        dest.unlink()
        dirfd = os.open(dest.parent, os.O_RDONLY)
        try:
            os.fsync(dirfd)
        finally:
            os.close(dirfd)

    if kind == "replace_alias":
        if not isinstance(original_link, str) or not original_link:
            raise MigrationError("rollback journal lacks original symlink target")
        os.symlink(original_link, dest)
        dirfd = os.open(dest.parent, os.O_RDONLY)
        try:
            os.fsync(dirfd)
        finally:
            os.close(dirfd)


def rollback(journal: dict[str, Any], journal_path: Path) -> dict[str, Any]:
    if journal.get("format") != JOURNAL_FORMAT:
        raise MigrationError("unsupported rollback journal")
    if journal.get("status") in {"activating", "activated"}:
        raise MigrationError(
            "upstream activation has started; restore the verified full target backup"
        )
    target_root = Path(journal["target_root"]).resolve(strict=True)

    pending = journal.get("pending")
    if pending is not None:
        _rollback_journal_entry(target_root, pending, allow_partial=True)
        journal["pending"] = None
        _atomic_json(journal_path, journal)

    for entry in reversed(journal.get("created", [])):
        _rollback_journal_entry(target_root, entry, allow_partial=False)

    journal["status"] = "rolled_back"
    _atomic_json(journal_path, journal)
    return journal


APP_SERVER_SOURCES = [
    "cli",
    "vscode",
    "exec",
    "appServer",
    "subAgent",
    "subAgentReview",
    "subAgentCompact",
    "subAgentThreadSpawn",
    "subAgentOther",
    "unknown",
]


class _AppServerClient:
    def __init__(
        self,
        executable: str,
        target_root: Path,
        *,
        runtime_home: Path | None,
    ) -> None:
        env = os.environ.copy()
        env["CODEX_HOME"] = str(target_root)
        env["CODEX_SQLITE_HOME"] = str(target_root)
        if runtime_home is not None:
            env["HOME"] = str(runtime_home)

        self.process = subprocess.Popen(
            [executable, "app-server", "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.sequence = 0

    def notify(self, method: str, params: dict[str, Any] | None = None) -> None:
        assert self.process.stdin is not None
        request: dict[str, Any] = {"method": method}
        if params is not None:
            request["params"] = params
        self.process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self.process.stdin.flush()

    def call(
        self, method: str, params: dict[str, Any], *, timeout: float = 45.0
    ) -> dict[str, Any]:
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.sequence += 1
        request_id = self.sequence
        request = {"id": request_id, "method": method, "params": params}
        self.process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self.process.stdin.flush()

        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            events = self.selector.select(max(0.0, deadline - time.monotonic()))
            if not events:
                break
            line = self.process.stdout.readline()
            if not line:
                break
            try:
                response = json.loads(line)
            except json.JSONDecodeError:
                continue
            if response.get("id") != request_id:
                continue
            if "error" in response:
                raise MigrationError(f"upstream app-server {method} failed")
            result = response.get("result")
            return result if isinstance(result, dict) else {}
        raise MigrationError(f"upstream app-server {method} timed out")

    def close(self) -> None:
        try:
            self.selector.close()
        except OSError:
            pass
        try:
            if self.process.stdin is not None:
                self.process.stdin.close()
        except OSError:
            pass
        try:
            self.process.terminate()
            self.process.wait(timeout=3)
        except (OSError, subprocess.TimeoutExpired):
            self.process.kill()
            self.process.wait(timeout=3)
        for stream in (self.process.stdout, self.process.stderr):
            if stream is not None:
                try:
                    stream.close()
                except OSError:
                    pass


def _verify_applied_payloads(plan: dict[str, Any]) -> Path:
    target_root = Path(plan["target_root"]).resolve(strict=True)
    for entry in plan["unique"]:
        dest = _safe_target_path(target_root, entry["target_rel"])
        if not dest.exists() or dest.is_symlink() or not dest.is_file():
            raise MigrationError("selected canonical rollout is not a regular file")
        meta = _read_session_meta(dest)
        if (
            meta["thread_id"] != entry["thread_id"]
            or _sha256(dest) != entry["sha256"]
        ):
            raise MigrationError("selected canonical rollout changed after apply")
    return target_root


def finalize_plan(
    plan: dict[str, Any],
    journal: dict[str, Any],
    journal_path: Path,
    *,
    executable: str,
    runtime_home: Path | None,
) -> dict[str, Any]:
    if journal.get("format") != JOURNAL_FORMAT:
        raise MigrationError("unsupported finalize journal")
    if journal.get("plan_sha256") != plan.get("plan_sha256"):
        raise MigrationError("finalize journal does not match migration manifest")
    if journal.get("status") != "applied":
        raise MigrationError("migration payload must be applied exactly once before finalize")

    target_root = _verify_applied_payloads(plan)

    # From this point forward upstream may write canonical SQLite projections.
    # Payload-only rollback is therefore no longer safe even if activation fails.
    journal["status"] = "activating"
    _atomic_json(journal_path, journal)

    client = _AppServerClient(
        executable,
        target_root,
        runtime_home=runtime_home.expanduser().resolve(strict=True)
        if runtime_home is not None
        else None,
    )
    try:
        client.call(
            "initialize",
            {
                "clientInfo": {"name": "codex-shared-state-migrate", "version": "1"},
                "capabilities": {"experimentalApi": True},
            },
        )
        client.notify("initialized")

        listed = client.call(
            "thread/list",
            {
                "limit": max(100, len(plan["unique"]) * 2),
                "useStateDbOnly": False,
                "sourceKinds": APP_SERVER_SOURCES,
            },
        )
        rows = listed.get("data")
        if not isinstance(rows, list):
            raise MigrationError("upstream thread/list returned no data")
        by_id = {
            str(row.get("id", "")): row
            for row in rows
            if isinstance(row, dict)
        }
        wanted = {entry["thread_id"] for entry in plan["unique"]}
        if wanted - set(by_id):
            raise MigrationError("upstream thread/list did not discover every selected thread")

        resumed = 0
        named = 0
        for entry in sorted(plan["unique"], key=lambda value: value["thread_id"]):
            thread_id = entry["thread_id"]
            client.call(
                "thread/resume",
                {"threadId": thread_id, "excludeTurns": True},
            )
            client.call(
                "thread/turns/list",
                {
                    "threadId": thread_id,
                    "limit": 1,
                    "sortDirection": "asc",
                    "itemsView": "notLoaded",
                },
            )
            resumed += 1
            name = entry.get("name")
            if name is not None:
                client.call("thread/name/set", {"threadId": thread_id, "name": name})
                named += 1

        final = client.call(
            "thread/list",
            {
                "limit": max(100, len(plan["unique"]) * 2),
                "useStateDbOnly": True,
                "sourceKinds": APP_SERVER_SOURCES,
            },
        )
        final_rows = final.get("data")
        if not isinstance(final_rows, list):
            raise MigrationError("upstream state-db-only thread/list returned no data")
        final_by_id = {
            str(row.get("id", "")): row
            for row in final_rows
            if isinstance(row, dict)
        }
        if wanted - set(final_by_id):
            raise MigrationError("upstream state DB lost selected threads")
        for entry in plan["unique"]:
            name = entry.get("name")
            if name is not None and final_by_id[entry["thread_id"]].get("name") != name:
                raise MigrationError("upstream thread name restoration mismatch")
    finally:
        client.close()

    journal["status"] = "activated"
    journal["activation"] = {
        "threads": len(plan["unique"]),
        "resumed": resumed,
        "named": named,
    }
    _atomic_json(journal_path, journal)
    return journal


def _summary(plan: dict[str, Any]) -> str:
    selected = sum(len(p["selected"]) for p in plan["profiles"])
    stale = sum(p["stale_skipped"] for p in plan["profiles"])
    unique = len(plan["unique"])
    copies = sum(1 for u in plan["unique"] if u["target_status"] == "copy")
    aliases = sum(1 for u in plan["unique"] if u["target_status"] == "replace_alias")
    existing = unique - copies - aliases
    named = sum(1 for u in plan["unique"] if u["name"] is not None)
    return (
        f"profiles={len(plan['profiles'])} selected={selected} unique={unique} "
        f"copy={copies} normalize_alias={aliases} existing={existing} "
        f"named={named} stale_skipped={stale}"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    p_plan = sub.add_parser("plan")
    p_plan.add_argument("--legacy-root", type=Path, required=True)
    p_plan.add_argument("--target-root", type=Path, required=True)
    p_plan.add_argument("--output", type=Path, required=True)

    p_apply = sub.add_parser("apply")
    p_apply.add_argument("--manifest", type=Path, required=True)
    p_apply.add_argument("--journal", type=Path, required=True)

    p_finalize = sub.add_parser("finalize")
    p_finalize.add_argument("--manifest", type=Path, required=True)
    p_finalize.add_argument("--journal", type=Path, required=True)
    p_finalize.add_argument("--codex", default="codex")
    p_finalize.add_argument("--runtime-home", type=Path)

    p_rollback = sub.add_parser("rollback")
    p_rollback.add_argument("--journal", type=Path, required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "plan":
            plan = build_plan(args.legacy_root, args.target_root)
            write_plan(args.output, plan)
            print(_summary(plan))
        elif args.command == "apply":
            plan = load_plan(args.manifest)
            journal = apply_plan(plan, args.journal)
            print(f"created={len(journal['created'])} status={journal['status']}")
        elif args.command == "finalize":
            plan = load_plan(args.manifest)
            with args.journal.open("r", encoding="utf-8") as handle:
                journal = json.load(handle)
            journal = finalize_plan(
                plan,
                journal,
                args.journal,
                executable=args.codex,
                runtime_home=args.runtime_home,
            )
            activation = journal["activation"]
            print(
                f"threads={activation['threads']} resumed={activation['resumed']} "
                f"named={activation['named']} status={journal['status']}"
            )
        else:
            with args.journal.open("r", encoding="utf-8") as handle:
                journal = json.load(handle)
            journal = rollback(journal, args.journal)
            print(f"removed={len(journal.get('created', []))} status={journal['status']}")
    except (MigrationError, OSError, sqlite3.Error, ValueError) as exc:
        print(f"shared-state migration: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Validate local Markdown document links AND GitHub-style heading fragments.

Only repository-tracked Markdown is authoritative; ignore generated files,
untracked private workspaces and external HTTP(S)/mailto destinations.
"""
from __future__ import annotations

import collections
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
HEADING = re.compile(r"^#{1,6}\s+(.+?)(?:\s+#+)?\s*$")
FENCE = re.compile(r"^\s*(`{3,}|~{3,})")
INLINE_LINK = re.compile(r"\[([^]]+)\]\([^)]+\)")
HTML_TAG = re.compile(r"<[^>]*>")


def heading_slug(text: str) -> str:
    text = INLINE_LINK.sub(r"\1", text)
    text = HTML_TAG.sub("", text)
    text = text.strip().lower()
    return re.sub(r"[^\w -]", "", text).replace(" ", "-")


def anchors_for(contents: str) -> set[str]:
    names: set[str] = set()
    duplicates: collections.Counter[str] = collections.Counter()
    fence: str | None = None
    for line in contents.splitlines():
        match_fence = FENCE.match(line)
        if match_fence:
            marker = match_fence.group(1)
            if fence is None:
                fence = marker[0]
            elif marker[0] == fence:
                fence = None
            continue
        if fence is not None:
            continue
        heading = HEADING.match(line)
        if heading is None:
            continue
        base = heading_slug(heading.group(1))
        suffix = duplicates[base]
        names.add(f"{base}-{suffix}" if suffix else base)
        duplicates[base] += 1
    return names


def validate(docs: dict[Path, str], root: Path) -> list[str]:
    heading_cache = {p: anchors_for(text) for p, text in docs.items()}
    errors: list[str] = []
    for source, contents in sorted(docs.items()):
        for lineno, line in enumerate(contents.splitlines(), 1):
            for match in LINK.finditer(line):
                target = match.group(1).split(" ", 1)[0].strip("<>")
                if target.startswith(("https://", "http://", "mailto:")):
                    continue
                file_part, _, fragment = target.partition("#")
                dest = (source.parent / file_part).resolve() if file_part else source
                if not dest.is_relative_to(root):
                    errors.append(f"{source.relative_to(root)}:{lineno}: out-of-tree link: {target}")
                elif not dest.exists():
                    errors.append(f"{source.relative_to(root)}:{lineno}: missing file: {target}")
                elif fragment and dest in heading_cache and fragment.lower() not in heading_cache[dest]:
                    errors.append(f"{source.relative_to(root)}:{lineno}: missing heading: {target}")
    return errors


def main() -> int:
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "--", "*.md"], cwd=ROOT
    ).split(b"\0")
    docs = {
        (ROOT / path.decode("utf-8")).resolve(): (ROOT / path.decode("utf-8")).read_text()
        for path in paths
        if path
    }
    errors = validate(docs, ROOT)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"Validated local Markdown files and heading anchors in {len(docs)} tracked documents.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

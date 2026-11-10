#!/usr/bin/env python3
"""Check current documentation ownership, split-module limits and relative links.

Legacy snapshots are immutable evidence and deliberately exempt from active
cross-link validation; their original source-relative links belong to the
archived commit, not a new current authority.
"""
from pathlib import Path
import re
import sys
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
AUTHORITY = {
    "AGENTS.md": 170,
    "SPEC.md": 150,
    "CURRENT.md": 160,
    "PRO_HANDOFF.md": 110,
    "README.md": 160,
    "providers/chatgpt/README.md": 180,
    "frontend/tui/README.md": 160,
    "docs/DOCUMENTATION.md": 160,
    "docs/TERMUX_CONSTRAINTS.md": 160,
    "docs/spec/BRIDGE.md": 180,
    "docs/spec/DELIVERY.md": 180,
    "docs/spec/USER_INTERFACES.md": 180,
}
NORMATIVE = ("docs/spec/product/", "docs/spec/bridge/", "docs/spec/delivery/", "docs/spec/interfaces/")
REQUIRED = ("docs/legacy/README.md", "docs/legacy/2026-11-10/CURRENT.md",
            "docs/legacy/2026-11-10/PRO_HANDOFF.md")
LINK = re.compile(r"!?\[[^\]\n]*\]\(([^)\n]+)\)")
HEADING = re.compile(r"^#{1,6}\s+(.+?)\s*#*\s*$", re.M)
CUSTOM_ANCHOR = re.compile(r'<a\s+id="([^"]+)"\s*>\s*</a>')
URL_PREFIX = ("https:", "http:", "mailto:", "tel:", "data:", "plugin:", "sandbox:")
errors = []


def issue(name, reason):
    errors.append(f"{name}: {reason}")


def git_slug(heading):
    heading = re.sub(r"<[^>]+>", "", heading).strip().lower()
    heading = re.sub(r"[^\w\- ]", "", heading, flags=re.UNICODE)
    return re.sub(r" +", "-", heading)


def valid_anchors(text):
    counts = {}
    anchors = set(CUSTOM_ANCHOR.findall(text))
    for header in HEADING.findall(text):
        stem = git_slug(header)
        repeat = counts.get(stem, 0)
        counts[stem] = repeat + 1
        anchors.add(stem + (f"-{repeat}" if repeat else ""))
    return anchors


def check_link(src, destination):
    # URL links and pathless fragments are intentional; fragment targets still
    # must exist for ordinary current content.
    if destination.startswith(URL_PREFIX):
        return
    destination = destination.strip().strip("<>")
    path_str, separator, fragment = destination.partition("#")
    if path_str.startswith("/") or path_str.startswith("$"):
        return
    if path_str:
        target = (src.parent / unquote(path_str)).resolve()
    else:
        target = src.resolve()
    try:
        target.relative_to(ROOT)
    except ValueError:
        issue(src.relative_to(ROOT), f"link leaves repository: {destination}")
        return
    if target.is_dir():
        target = target / "README.md"
    if not target.is_file():
        issue(src.relative_to(ROOT), f"missing link: {destination}")
        return
    if separator and fragment:
        # Legacy snapshots are evidence; their old section names are not
        # maintained against the current GitHub heading algorithm.
        if "/docs/legacy/" in str(target):
            return
        anchor = unquote(fragment)
        if anchor not in valid_anchors(target.read_text()):
            issue(src.relative_to(ROOT), f"unresolved anchor: {destination}")


def main():
    for relative, max_lines in AUTHORITY.items():
        p = ROOT / relative
        if not p.is_file():
            issue(relative, "missing authority")
            continue
        lines = p.read_text().splitlines()
        if len(lines) > max_lines:
            issue(relative, f"{len(lines)} > {max_lines} active-document lines")
    for relative in REQUIRED:
        if not (ROOT / relative).is_file():
            issue(relative, "missing legacy archive")
    for f in sorted((ROOT / "docs/spec").rglob("*.md")):
        relative = str(f.relative_to(ROOT))
        if not relative.startswith(NORMATIVE):
            continue
        length = len(f.read_text().splitlines())
        if length > 650:
            issue(relative, f"normative chapter too large ({length} > 650)")
    for p in sorted(ROOT.rglob("*.md")):
        relative = str(p.relative_to(ROOT))
        if relative.startswith("docs/legacy/") or ".git" in p.parts:
            continue
        content = p.read_text()
        for m in LINK.finditer(content):
            check_link(p, m.group(1))
    agents = (ROOT / "AGENTS.md").read_text()
    for must in ("docs/DOCUMENTATION.md", "docs/TERMUX_CONSTRAINTS.md",
                 "107 encoded-byte", "docs/legacy"):
        if must not in agents:
            issue("AGENTS.md", f"required rule/link missing: {must}")
    status = (ROOT / "CURRENT.md").read_text()
    if "native_thread_owner_unavailable" not in status or "5384d938" not in status:
        issue("CURRENT.md", "latest observed blocker or qualification source missing")
    if errors:
        print("Documentation check FAIL:", len(errors), "issues")
        print("\n".join(errors[:80]))
        return 1
    print("Documentation check OK: authority lengths, legacy preservation, module sizes and active relative links")
    return 0


if __name__ == "__main__":
    sys.exit(main())

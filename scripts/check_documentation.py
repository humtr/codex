#!/usr/bin/env python3
"""Validate active document contracts using one human/machine TOML dependency map."""
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = Path("docs/TEST_DEPENDENCIES.toml")
LINK = re.compile(r"!?\[[^\]\n]*\]\(([^)\n]+)\)")
HEADING = re.compile(r"^#{1,6}\s+(.+?)\s*#*\s*$", re.M)
CUSTOM_ANCHOR = re.compile(r'<a\s+id="([^"]+)"\s*>\s*</a>')
URL_PREFIX = ("https:", "http:", "mailto:", "tel:", "data:", "plugin:", "sandbox:")
errors = []


class DependencyError(ValueError):
    """A declared test input is missing, ambiguous, or unsupported."""


def issue(name, reason):
    errors.append(f"{name}: {reason}")


def dependencies(root=ROOT):
    src = root / MANIFEST
    if not src.is_file():
        raise DependencyError(f"required test manifest missing: {src}")
    data = tomllib.loads(src.read_text())
    if data.get("version") != 1:
        raise DependencyError("unsupported dependency manifest version")
    return data


def test_inputs(test_id, root=ROOT, data=None):
    """Resolve one test's declared inputs; required missing means explicit failure."""
    data = data or dependencies(root)
    if test_id not in data["tests"]:
        raise DependencyError(f"undeclared documentation test: {test_id}")
    entry = data["tests"][test_id]
    if entry.get("missing") != "fail" or not entry.get("purpose"):
        raise DependencyError(f"{test_id}: invalid fail-closed test contract")
    if entry.get("validation") != test_id:
        raise DependencyError(f"{test_id}: unbound validator")
    mapping = data["documents"]
    result = {}
    required = entry["required"]
    conditional = entry.get("conditional", [])
    if not required or len(set(required + conditional)) != len(required + conditional):
        raise DependencyError(f"{test_id}: empty/duplicate input declarations")
    for key in required + conditional:
        if key not in mapping:
            raise DependencyError(f"{test_id}: unknown document alias {key}")
        candidate = (root / mapping[key]).resolve()
        try:
            candidate.relative_to(root.resolve())
        except ValueError:
            raise DependencyError(f"{test_id}: path escapes repository: {key}")
        if not candidate.is_file():
            if key in required:
                raise DependencyError(f"{test_id}: required document missing: {key} ({mapping[key]})")
            print(f"optional document absent for {test_id}: {key} ({mapping[key]})")
        else:
            result[key] = candidate
    return result


def git_slug(heading):
    heading = re.sub(r"<[^>]+>", "", heading).strip().lower()
    heading = re.sub(r"[^\w\- ]", "", heading, flags=re.UNICODE)
    return re.sub(r" +", "-", heading)


def valid_anchors(text):
    counts = {}
    anchors = set(CUSTOM_ANCHOR.findall(text))
    for header in HEADING.findall(text):
        stem = git_slug(header)
        n = counts.get(stem, 0)
        counts[stem] = n + 1
        anchors.add(stem + (f"-{n}" if n else ""))
    return anchors


def check_link(src, destination, root=ROOT):
    if destination.startswith(URL_PREFIX):
        return
    destination = destination.strip().strip("<>")
    path_str, sep, fragment = destination.partition("#")
    if path_str.startswith(("/", "$")):
        return
    target = ((src.parent / unquote(path_str)) if path_str else src).resolve()
    try:
        target.relative_to(root.resolve())
    except ValueError:
        issue(src.relative_to(root), f"link leaves repository: {destination}")
        return
    if target.is_dir():
        target = target / "README.md"
    if not target.is_file():
        issue(src.relative_to(root), f"missing link: {destination}")
    elif sep and fragment and unquote(fragment) not in valid_anchors(target.read_text()):
        issue(src.relative_to(root), f"unresolved anchor: {destination}")


def product_contract(paths, root, data):
    text = paths["spec"].read_text()
    ids = re.findall(r"\*\*(P\d\d)\s+[—-]", text)
    if ids != [f"P{x:02d}" for x in range(1, 19)]:
        issue("product_contract", "18 ordered, unique high-level product obligations must remain")
    if "docs/" in text or ".md)" in text:
        issue("product_contract", "SPEC must not depend on document locations")
    for topic in ("Core", "Native", "Provider", "codex", "rollback", "approval", "recover", "upstream"):
        if topic.lower() not in text.lower():
            issue("product_contract", f"essential product dimension not represented: {topic}")


def agent_workflow(paths, root, data):
    agent = paths["agents"].read_text()
    policy = paths["policy"].read_text()
    for alias in ("policy", "index", "termux"):
        if data["documents"][alias] not in agent:
            issue("agent_workflow", f"agent router does not name declared {alias}")
    for topic in ("HEAD", "ownership", "Native writer", "107 encoded bytes", "compaction",
                  "diff --check", "source", "recovery"):
        if topic.lower() not in agent.lower():
            issue("agent_workflow", f"missing actionable gate: {topic}")
    if "optional" not in policy.lower() or "Git history" not in policy:
        issue("agent_workflow", "optional history/removal policy missing")


def index_routing(paths, root, data):
    index = paths["index"]
    linked = []
    for target in LINK.findall(index.read_text()):
        if target.startswith(URL_PREFIX):
            continue
        path = target.split("#", 1)[0]
        if path:
            p = (index.parent / unquote(path)).resolve()
            if p.suffix == ".md":
                linked.append(p)
    annexes = {(root / group / file.name).resolve()
               for group in data["discovery"]["annex_roots"]
               for file in (root / group).glob("*.md")}
    for p in annexes - set(linked):
        issue("index_routing", f"unrouted active annex: {p.relative_to(root)}")
    for p in linked:
        if p in annexes and linked.count(p) != 1:
            issue("index_routing", f"duplicate direct route: {p.relative_to(root)}")
    if not annexes:
        issue("index_routing", "no normative annexes discovered")
    if "non-normative" not in index.read_text():
        issue("index_routing", "router must be explicitly non-normative")


def current_status(paths, root, data):
    current, handoff = paths["current"].read_text(), paths["handoff"].read_text()
    if "SHA" not in current and "commit" not in current:
        issue("current_status", "CURRENT has no source/evidence binding")
    if "acceptance" not in handoff.lower() or "unfinished" not in handoff.lower():
        issue("current_status", "HANDOFF does not identify unfinished acceptance")


def termux_preflight(paths, root, data):
    text = paths["termux"].read_text().lower()
    for topic in ("107", "canonical", "tmpdir", "writer", "socket", "core"):
        if topic not in text:
            issue("termux_preflight", f"missing executable preflight: {topic}")


def relative_links(paths, root, data):
    excludes = [root / p for p in data["discovery"].get("excluded_roots", [])]
    for p in sorted(root.rglob("*.md")):
        if ".git" in p.parts or any(e in p.parents or p == e for e in excludes):
            continue
        for dest in LINK.findall(p.read_text()):
            check_link(p, dest, root)


CHECKS = {
    "product_contract": product_contract,
    "agent_workflow": agent_workflow,
    "index_routing": index_routing,
    "current_status": current_status,
    "termux_preflight": termux_preflight,
    "relative_links": relative_links,
}


def main(root=ROOT):
    errors.clear()
    try:
        data = dependencies(root)
        if set(data["tests"]) != set(CHECKS):
            raise DependencyError("declared tests and implemented validators differ")
        for test_id, validator in CHECKS.items():
            inputs = test_inputs(test_id, root, data)
            validator(inputs, root, data)
    except (KeyError, ValueError, OSError) as e:
        issue("dependencies", str(e))
    if errors:
        print(f"Documentation check FAIL: {len(errors)} issues")
        print("\n".join(str(e) for e in errors[:80]))
        return 1
    print("Documentation check OK: declared inputs, product contract, routing, preflight and links")
    return 0


if __name__ == "__main__":
    sys.exit(main())

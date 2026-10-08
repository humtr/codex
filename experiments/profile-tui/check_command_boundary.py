"""Refuse upstream slash collisions and preserve every upstream name and alias."""
import argparse
import json
from pathlib import Path
import re


def inventory(source):
    # Only the qualified Strum representation is understood. New syntax requires
    # qualification rather than silently overlooking a possible alias.
    source = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)
    match = re.search(
        r'#\[strum\(serialize_all = "kebab-case"\)\]\s*'
        r'pub enum SlashCommand\s*\{(.*?)^\}', source, re.S | re.M,
    )
    if match is None:
        raise ValueError("unqualified upstream slash naming representation")
    body = match[1].strip()
    result = {}
    while body:
        variant = re.match(r'((?:#\[[^\]]*\]\s*)*)([A-Z][A-Za-z0-9_]*),', body)
        if variant is None:
            raise ValueError("unqualified upstream slash variant")
        attributes, name = variant.groups()
        aliases = set()
        for attribute in re.findall(r'#\[([^\]]*)\]', attributes):
            if attribute.startswith("cfg(") and attribute.endswith(")"):
                continue  # Reserve names on every platform, even if cfg hides them.
            if not attribute.startswith("strum(") or not attribute.endswith(")"):
                raise ValueError("unqualified upstream slash attribute")
            for part in attribute[6:-1].split(","):
                alias = re.fullmatch(r'\s*(?:serialize|to_string)\s*=\s*"([a-z0-9-]+)"\s*', part)
                if alias is None:
                    raise ValueError("unqualified upstream slash alias")
                aliases.add(alias[1])
        if not aliases:
            kebab = re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", name)
            kebab = re.sub(r"([A-Z])([A-Z][a-z])", r"\1-\2", kebab)
            aliases.add(kebab.lower())
        if name in result:
            raise ValueError("duplicate upstream slash variant")
        result[name] = sorted(aliases)
        body = body[variant.end():].strip()
    if not result:
        raise ValueError("empty upstream slash inventory")
    return result


def qualify(pristine, patched=None):
    before = inventory(pristine)
    if "Switch" in before or any("switch" in names for names in before.values()):
        raise ValueError("upstream owns /switch; native extension refused")
    if patched is not None and inventory(patched) != {**before, "Switch": ["switch"]}:
        raise ValueError("native extension changed upstream command names or aliases")
    return {"upstream_variants": len(before), "extension": "switch",
            "patched_inventory_verified": patched is not None}


def nonzero_tests(log):
    log = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', log)
    summaries = re.findall(r'Summary\s*\[[^\]]+\]\s*(\d+) tests run: (\d+) passed', log)
    if len(summaries) != 1 or int(summaries[0][0]) == 0 or summaries[0][0] != summaries[0][1]:
        raise ValueError("native focused proof must run and pass a nonzero test set")
    return int(summaries[0][0])


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("--patched", type=Path)
    parser.add_argument("--test-log", type=Path)
    args = parser.parse_args()
    relative = "codex-rs/tui/src/slash_command.rs"
    proof = qualify((args.source / relative).read_text(),
                    (args.patched / relative).read_text() if args.patched else None)
    if args.test_log:
        proof["native_tests_passed"] = nonzero_tests(args.test_log.read_text())
    print(json.dumps(proof, sort_keys=True))

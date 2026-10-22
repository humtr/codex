#!/usr/bin/env python3
"""Validate and seal explicit F inputs without launching apps, native work or Chat.

This consumer implements the keywords used by qualification.schema.json and fails
closed on unsupported validation keywords. It needs no production dependency.
Execution drivers import prepare/verify; historical invocation remains separate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import time
import tomllib

SOURCE = Path(__file__).resolve().parents[1]
SCHEMA = SOURCE / "providers/chatgpt/qualification.schema.json"


class InvalidInputs(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise InvalidInputs(message)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def check_schema(schema):
    """Inspect every variant before matching, including unchosen branches."""
    supported = {"$ref", "$schema", "$id", "$defs", "title", "description", "type", "const", "enum",
                 "properties", "required", "additionalProperties", "items", "minItems", "maxItems", "uniqueItems",
                 "contains", "minLength", "maxLength", "pattern", "minimum", "maximum", "anyOf", "allOf", "if", "then"}
    require(not set(schema) - supported, "unsupported qualification schema keyword")
    for field in ("properties", "$defs"):
        for child in schema.get(field, {}).values():
            check_schema(child)
    for field in ("anyOf", "allOf"):
        for child in schema.get(field, []):
            check_schema(child)
    for field in ("items", "contains", "if", "then", "additionalProperties"):
        if isinstance(schema.get(field), dict):
            check_schema(schema[field])


def validate(value, schema, document=None, location="inputs"):
    """Bounded schema consumer; unknown constraints never become silent passes."""
    check_schema(schema)
    document = document or schema
    if "$ref" in schema:
        reference = schema["$ref"]
        require(reference.startswith("#/"), "external schema reference unsupported")
        resolved = document
        for component in reference[2:].split("/"):
            resolved = resolved[component.replace("~1", "/").replace("~0", "~")]
        validate(value, resolved, document, location)
    types = {"object": lambda v: isinstance(v, dict), "array": lambda v: isinstance(v, list),
             "string": lambda v: isinstance(v, str), "integer": lambda v: type(v) is int,
             "boolean": lambda v: type(v) is bool, "null": lambda v: v is None}
    if "type" in schema:
        choices = schema["type"] if isinstance(schema["type"], list) else [schema["type"]]
        require(all(t in types for t in choices), "unsupported qualification schema type")
        require(any(types[t](value) for t in choices), location + ": incorrect type")
    if "const" in schema:
        require(canonical(value) == canonical(schema["const"]), location + ": incorrect constant")
    if "enum" in schema:
        require(canonical(value) in [canonical(v) for v in schema["enum"]], location + ": unsupported choice")
    if isinstance(value, dict):
        require(set(schema.get("required", [])) <= set(value), location + ": missing field")
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            require(set(value) <= set(properties), location + ": unknown field")
        for key in set(value) & set(properties):
            validate(value[key], properties[key], document, location + "." + key)
    if isinstance(value, list):
        require(len(value) >= schema.get("minItems", 0) and len(value) <= schema.get("maxItems", len(value)), location + ": invalid item count")
        if schema.get("uniqueItems"):
            require(len(set(map(canonical, value))) == len(value), location + ": repeated item")
        for item in value:
            if "items" in schema:
                validate(item, schema["items"], document, location + "[]")
        if "contains" in schema:
            require(any(matches(item, schema["contains"], document) for item in value), location + ": required item absent")
    if isinstance(value, str):
        require(len(value) >= schema.get("minLength", 0) and len(value) <= schema.get("maxLength", len(value)), location + ": invalid string length")
        if "pattern" in schema:
            require(re.search(schema["pattern"], value) is not None, location + ": invalid format")
    if type(value) is int:
        require(value >= schema.get("minimum", value) and value <= schema.get("maximum", value), location + ": outside bounds")
    for condition in schema.get("allOf", []):
        validate(value, condition, document, location)
    if "anyOf" in schema:
        require(any(matches(value, condition, document) for condition in schema["anyOf"]), location + ": no supported variant")
    if "if" in schema and matches(value, schema["if"], document):
        validate(value, schema.get("then", {}), document, location)


def matches(value, schema, document):
    try:
        validate(value, schema, document)
        return True
    except InvalidInputs:
        return False


def path(value, directory=False, private=False):
    candidate = Path(value)
    require(candidate.is_absolute() and candidate.resolve(strict=True) == candidate, "canonical absolute input path required")
    require(all(not p.is_symlink() for p in [candidate, *candidate.parents]), "symlink input alias refused")
    metadata = candidate.stat()
    require(stat.S_ISDIR(metadata.st_mode) if directory else stat.S_ISREG(metadata.st_mode), "input path has incorrect kind")
    if private:
        require(metadata.st_uid == os.getuid() and stat.S_IMODE(metadata.st_mode) == (0o700 if directory else 0o600), "private owned input required")
    return candidate


def reference(value):
    selected = path(value["path"])
    require(digest(selected) == value["sha256"], "input hash changed: " + str(selected))
    return selected


def source_dependencies(files):
    if "scripts/qualify_provider_operator.py" in files:
        require({"scripts/qualify_permission_lifecycle.py", "scripts/qualify_tasks.py",
                 ".github/scripts/rald3_preflight.py", ".github/scripts/rald5_publication.py",
                 "scripts/fixtures/permission-model.json"} <= set(files),
                "source inventory omits transitive native fixture inputs")


def model_policy(config, defaults):
    selected=config.get("model", {}).get("default", "chatgpt-desktop/gpt-6")
    effort=config.get("model", {}).get("reasoning_effort", "none")
    models=config.get("catalog", {}).get("models", defaults["models"])
    matches=[model for model in models if model["id"]==selected]
    require(len(matches)==1, "selected model is not uniquely declared")
    model=matches[0]
    routes=[route for route in model["routes"] if route["effort"]==effort]
    require(len(routes)==1, "selected model effort has no unique route")
    host=routes[0]["host_model"].lower()
    require(not model.get("pro", False) and not re.search(r"gpt-[\w.]+-pro(?:$|-)", host),
            "automatic Pro qualification is excluded")
    require("5.6" not in selected.lower() and "5.6" not in host, "excluded model selected")
    return {"model":selected,"effort":effort,"host_model":host}


def source_files(source):
    # Embedded text/CSS/HTML, patch inputs and licenses are source inputs too.
    # An extension allowlist cannot establish a self-contained build snapshot.
    return {str(p.relative_to(source)) for directory in ("providers/chatgpt", "crates/runtime-binding")
            for p in (source / directory).rglob("*") if p.is_file()
            and not {"target", "__pycache__", ".git"}.intersection(p.relative_to(source).parts)}


def native_root(value):
    selected = Path(value)
    if selected.exists():
        return path(value, directory=True, private=True)
    require(selected.is_absolute() and selected.resolve() == selected, "canonical future native path required")
    parent = selected.parent
    while not parent.exists():
        parent = parent.parent
    path(str(parent), directory=True, private=True)
    return selected


def verify(inputs):
    schema = json.loads(SCHEMA.read_text())
    validate(inputs, schema)
    candidate, native, environment = inputs["candidate"], inputs["native"], inputs["environment"]
    source = path(candidate["source_root"], directory=True)
    for group in (candidate, native, inputs["fixture"]):
        for value in group.values():
            if isinstance(value, dict) and set(value) == {"path", "sha256"}:
                reference(value)
    manifest = json.loads(reference(candidate["source_manifest"]).read_text())
    require(manifest.get("base_commit") == candidate["base_commit"], "source commit mismatch")
    require(isinstance(manifest.get("files"), dict) and manifest["files"], "complete source file map required")
    require(len(manifest["files"]) <= 10000, "source inventory bound exceeded")
    source_dependencies(manifest["files"])
    discovered = source_files(source)
    require(discovered <= set(manifest["files"]), "source inventory omits current build/test inputs")
    for name, expected in manifest["files"].items():
        relative = Path(name)
        require(not relative.is_absolute() and ".." not in relative.parts, "source inventory escapes root")
        selected = path(str(source / relative))
        require(digest(selected) == expected, "source input changed: " + name)
    bridge = reference(candidate["app_bridge"]).read_bytes()
    require(digest(source / "providers/chatgpt/src/app_bridge.js") == candidate["app_bridge"]["sha256"], "embedded bridge/source mismatch")
    binary = reference(candidate["binary"]).read_bytes()
    require(binary[:4] == b"\x7fELF" and bridge in binary, "candidate ELF lacks declared embedded bridge")
    for name in ("tmp_root", "artifact_root"):
        path(environment[name], directory=True, private=True)
    roots = [native_root(native[name]) for name in ("home", "workspace")]
    temporary = Path(os.environ["TMPDIR"]).resolve(strict=True)
    for selected in roots:
        require(temporary in selected.parents, "native execution must use disposable TMPDIR roots")
    require(not (roots[0] == roots[1] or roots[0] in roots[1].parents or roots[1] in roots[0].parents), "native home/workspace overlap")
    require(source not in roots and all(source not in root.parents for root in roots), "native roots overlap source")
    config = tomllib.loads(reference(candidate["config"]).read_text())
    execution = config.get("execution", {})
    require(execution.get("home") == native["home"] and execution.get("workspace") == native["workspace"] and execution.get("core") == native["core"]["path"], "effective native execution input drift")
    model_policy(config, tomllib.loads((source / "providers/chatgpt/default-models.toml").read_text()))
    for name in ("host_session_ref", "account_ref", "connector_profile_ref"):
        if environment[name] is not None:
            path(environment[name], private=True)
    if environment["app_bindings_manifest"] is not None:
        reference(environment["app_bindings_manifest"])
    if inputs["limits"]["mode"] == "live":
        require(environment["app_version"] and environment["app_bindings_manifest"] and environment["account_ref"], "live account/app bindings required")
        path(inputs["return"]["destination_ref"], private=True)
    # Core's native rendezvous is under installation HOME, which can differ from
    # CODEX_HOME. The frozen fixture manifest supplies it explicitly at execution.
    return {"schema": "termux-codex-f-resolved-inputs-v1", "inputs": inputs,
            "inputs_sha256": hashlib.sha256(canonical(inputs).encode()).hexdigest(),
            "schema_sha256": digest(SCHEMA), "consumer_sha256": digest(__file__),
            "verified_at_unix_ns": time.time_ns(), "new_model_requests": 0,
            "native_calls": 0, "host_started": False, "acceptance": "not_run"}


def prepare(input_file):
    inputs = json.loads(path(str(input_file)).read_text())
    resolved = verify(inputs)
    destination = Path(inputs["environment"]["artifact_root"]) / (inputs["run_id"] + ".resolved.json")
    if destination.exists():
        previous = json.loads(path(str(destination), private=True).read_text())
        require(previous["inputs_sha256"] == resolved["inputs_sha256"] and previous["consumer_sha256"] == resolved["consumer_sha256"], "prepared run changed; preserve previous intent")
        return destination
    with destination.open("x") as stream:
        os.chmod(destination, 0o600)
        stream.write(json.dumps(resolved, ensure_ascii=False, indent=2) + "\n")
        stream.flush()
        os.fsync(stream.fileno())
    return destination


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    print(json.dumps({"resolved_inputs": str(prepare(args.inputs)), "inference_submitted": False, "acceptance": "not_run"}))

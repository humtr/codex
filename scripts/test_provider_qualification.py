import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from scripts import provider_qualification as q


class QualificationInputs(unittest.TestCase):
    def test_historical_driver_inputs_never_default_to_an_operating_device(self):
        for value in [None, "", " "]:
            environment = {} if value is None else {"CODEX_QUALIFICATION_PRIOR_ROOT": value}
            with self.subTest(value=value), self.assertRaisesRegex(q.InvalidInputs, "PRIOR_ROOT"):
                q.explicit_input("PRIOR_ROOT", environment=environment)
        with tempfile.TemporaryDirectory(dir=os.environ["TMPDIR"]) as name:
            environment = {"CODEX_QUALIFICATION_PRIOR_ROOT": name}
            self.assertEqual(q.explicit_input("PRIOR_ROOT", environment=environment), Path(name).resolve())
            environment["CODEX_QUALIFICATION_PRIOR_ROOT"] = str(Path(name) / "absent")
            with self.assertRaisesRegex(q.InvalidInputs, "path is unavailable"):
                q.explicit_input("PRIOR_ROOT", environment=environment)
        self.assertEqual(q.explicit_input("TUNNEL_ALIAS", path=False,
            environment={"CODEX_QUALIFICATION_TUNNEL_ALIAS": "explicit-owned-alias"}), "explicit-owned-alias")

    def test_source_inventory_includes_embedded_non_code_assets(self):
        with tempfile.TemporaryDirectory(prefix="fs.",dir=os.environ["TMPDIR"]) as temporary:
            root=Path(temporary);p=root / "providers/chatgpt";p.mkdir(parents=True)
            for name in ("native-summary-prefix.txt","mobile_controls.css","mobile_toolbar.html","termux_viewer.ts"):
                (p / name).write_text("owned source")
            (p / "target").mkdir();(p / "target/generated").write_text("excluded")
            self.assertEqual(q.source_files(root),{"providers/chatgpt/"+name for name in ("native-summary-prefix.txt","mobile_controls.css","mobile_toolbar.html","termux_viewer.ts")})
    def test_model_exclusions_follow_resolved_route_and_pro_flag(self):
        import tomllib
        defaults=tomllib.loads((q.SOURCE / "providers/chatgpt/default-models.toml").read_text())
        self.assertEqual(q.model_policy({}, defaults)["host_model"], "gpt-6")
        with self.assertRaises(q.InvalidInputs):
            q.model_policy({"model":{"default":"chatgpt-desktop/gpt-6-pro"}}, defaults)
        for host, pro in [("gpt-5.6", False), ("gpt-6-pro", False), ("custom", True)]:
            with self.subTest(host=host), self.assertRaises(q.InvalidInputs):
                q.model_policy({"model":{"default":"custom"},"catalog":{"models":[{"id":"custom","pro":pro,"routes":[{"effort":"none","host_model":host}]}]}},defaults)
    def test_native_fixture_inventory_must_include_transitive_data_and_helpers(self):
        with self.assertRaises(q.InvalidInputs):
            q.source_dependencies({"scripts/qualify_provider_operator.py": "hash"})
        q.source_dependencies({name: "hash" for name in (
            "scripts/qualify_provider_operator.py", "scripts/qualify_permission_lifecycle.py",
            "scripts/qualify_tasks.py", ".github/scripts/rald3_preflight.py",
            ".github/scripts/rald5_publication.py", "scripts/fixtures/permission-model.json")})
    def test_unselected_schema_variants_cannot_hide_unsupported_constraints(self):
        with self.assertRaises(q.InvalidInputs):
            q.validate("valid", {"anyOf": [{"type": "string"}, {"unknownConstraint": True}]})
    def setUp(self):
        self.schema = json.loads(q.SCHEMA.read_text())

    def test_schema_requires_routes_budgets_and_original_return_without_bool_integer_aliases(self):
        route = self.schema["$defs"]["route"]
        for entry, reasoning, transport in [("codex", "managed", "mcp"), ("codex", "managed", "local_function"),
                                             ("chatgpt_direct", "current", "mcp"), ("chatgpt_delegated", "managed", "mcp")]:
            q.validate({"entry": entry, "reasoning": reasoning, "return_path": transport}, route, self.schema)
        for entry, reasoning, transport in [("chatgpt_direct", "managed", "mcp"), ("chatgpt_direct", "current", "local_function"),
                                             ("codex", "current", "mcp")]:
            with self.assertRaises(q.InvalidInputs):
                q.validate({"entry": entry, "reasoning": reasoning, "return_path": transport}, route, self.schema)
        with self.assertRaises(q.InvalidInputs):
            q.validate(True, {"type": "integer"})
        with self.assertRaises(q.InvalidInputs):
            q.validate(1, {"type": "integer", "unimplemented_constraint": True})

    def test_live_limits_and_offline_limits_are_distinct(self):
        limits = {"mode": "offline", "max_app_starts": 0, "max_model_responses": 0, "max_native_calls": 0,
                  "max_elapsed_seconds": 30, "max_parallel": 1, "supervisor_turns": 0, "pro_automatic": False,
                  "excluded_models": ["gpt-5.6"], "authorization_ref": None}
        schema = self.schema["properties"]["limits"]
        q.validate(limits, schema, self.schema)
        changed = copy.deepcopy(limits)
        changed["max_app_starts"] = 1
        with self.assertRaises(q.InvalidInputs):
            q.validate(changed, schema, self.schema)
        changed.update(mode="live", max_model_responses=1, authorization_ref="existing F authorization")
        q.validate(changed, schema, self.schema)
        for key, value in [("supervisor_turns", 1), ("pro_automatic", True), ("excluded_models", []), ("authorization_ref", None)]:
            rejected = {**changed, key: value}
            with self.subTest(key=key), self.assertRaises(q.InvalidInputs):
                q.validate(rejected, schema, self.schema)

    def test_canonical_private_paths_and_hashes_reject_aliases_and_drift(self):
        with tempfile.TemporaryDirectory(prefix="fi.", dir=os.environ["TMPDIR"]) as temporary:
            root = Path(temporary).resolve()
            file = root / "owned"
            file.write_text("frozen")
            file.chmod(0o600)
            reference = {"path": str(file), "sha256": q.digest(file)}
            self.assertEqual(q.reference(reference), file)
            self.assertEqual(q.path(str(file), private=True), file)
            alias = root / "alias"
            alias.symlink_to(file)
            with self.assertRaises(q.InvalidInputs):
                q.path(str(alias))
            file.write_text("changed")
            with self.assertRaises(q.InvalidInputs):
                q.reference(reference)
            file.chmod(0o644)
            with self.assertRaises(q.InvalidInputs):
                q.path(str(file), private=True)


if __name__ == "__main__":
    unittest.main()

# 9. Doctor contract

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

## 9. Doctor contract

`codex doctor` is read-only. It runs the raw upstream doctor when supported and
adds a Termux Core/Manager diagnosis without recursively invoking the public
launcher. Human output begins with the bounded, sanitized upstream doctor
output itself, preserving the upstream doctor's own header, layout, and safe
ANSI SGR sequences; Core does not prepend an `[Upstream Codex doctor]` wrapper
heading or a duplicate synthetic status line. `--json` emits one redacted
envelope rather than concatenated documents:

```json
{
  "schema_version": 2,
  "upstream": {"status": "healthy", "output": "..."},
  "termux_core": {
    "status": "healthy",
    "generation_id": "...",
    "layout": "root-code-mode-host-v2",
    "runtime": {"status": "healthy"},
    "code_mode_host": {"status": "healthy"},
    "sandbox": {"status": "unsupported", "reason": "bwrap is not used"}
  },
  "manager": {},
  "summary": {}
}
```

When human output is connected to a TTY and `NO_COLOR` is absent, Core gives the
upstream doctor a bounded pseudo-terminal so its own headings, progress
cleanup, and ANSI SGR markup retain the upstream layout. `--color` is the
explicit exception for an interactive caller whose outer Termux/AI wrapper
injected `NO_COLOR`; Core removes that variable only from the upstream child
and enables the same bounded PTY path. Non-TTY output remains plain, and
explicit `NO_COLOR` remains plain unless `--color` was requested. Core
normalizes carriage-return/erase controls and preserves only safe SGR sequences
before composition. If the upstream doctor is unsupported or produces no
output, Core emits only a concise status diagnostic for that missing upstream
portion before the Termux section.
The Termux doctor section follows the legacy wrapper presentation: a `Codex
Termux Wrapper Doctor` status header,
`Runtime`, `Support`, `Wrapper`, `State`, and `Store` groups, colored health
rows when enabled, and a bounded summary. It reports the selected generation,
root-level code-mode companion or migration state, and the explicit reason that
Linux bwrap sandboxing is not used.

Unsupported upstream or Manager diagnostics are represented explicitly and do
not fabricate success. Diagnostic failure returns nonzero while preserving a
valid machine report when `--json` was requested. After valid doctor argument
parsing, an upstream probe/setup failure is represented as a redacted
`unhealthy` upstream status with empty output rather than an error string, and
the command still returns its nonzero health-failure status. Upstream doctor
output is capped at 64 KiB, has terminal control sequences and credential-like
values redacted before composition, and is emitted as a JSON string in the
envelope. Usage errors remain distinct from health failures and API
incompatibility.

A credential-free disposable release proof therefore does not require doctor
exit zero. It may accept only the documented success or health-failure exit
class when the bounded JSON envelope is valid, Termux Core/runtime/code-mode
health is `healthy` for the exact activated generation, and the upstream
section proves that the real upstream diagnostic ran by reporting
`healthy` or `unhealthy` rather than `unsupported`. The exit code must agree
with the composed summary. Missing user credentials or external provider
reachability may make the upstream diagnostic unhealthy, but must not be
reclassified as candidate runtime-integrity failure.

Doctor must not expose tokens, OAuth data, cookies, auth-derived private data,
notification content, or unredacted session content. A filesystem snapshot
before and after doctor must be unchanged except for operating-system access
metadata outside product control.

Generation descriptors written by the current release builder use
`codex-local-generation-v2` and the root-level companion layout above. Core
continues to read an already-installed `codex-local-generation-v1` generation
with `compat/codex-code-mode-host` only as a bounded migration input; it never
produces that layout, and it never treats an unlisted root-level symlink as the
companion. The next authenticated generation is the required permanent repair
for such an old layout.

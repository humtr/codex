# Codex Termux Wrapper

Purpose: install the official `@openai/codex` linux-arm64 package inside Termux, patch the runtime binary so resolver access goes through fd 33, install Termux-compatible `bwrap` and `rg` shims, and expose a managed `codex` launcher.

This repo intentionally contains only runtime/install code. Development-only tests, CI scripts, tracking notes, and agent instruction files are not part of this package.

## Use

```sh
bash install.sh
codex doctor
codex update
codex use
codex profile
```

Managed state lives under:

```text
~/.local/lib/codex/termux
~/.local/share/codex/termux
```

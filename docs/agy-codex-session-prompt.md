# AGY Prompt: `codex session` Surface

Implement the new wrapper command surface `codex session` for the Codex Termux wrapper.

## Context

This wrapper already supports:

- `codex` bare launch via the most recent wrapper profile
- `codex profile`
- `codex use`
- `codex version`
- `codex doctor`

Do not change those existing behaviors unless explicitly required below.

The wrapper is intentionally thin. Preserve the current structure:

- shell command dispatch and lightweight UI live in `lib/codex-termux.sh`
- structured helper logic lives in `tools/codex_termux/`
- upstream Codex behavior should remain intact wherever possible

## Goal

Add a new wrapper command:

```sh
codex session
```

This command must provide a wrapper-level picker for Codex sessions across all Codex homes:

- `~/.codex`
- `~/.codex-profiles/*`

The key requirement is:

> Any discovered Codex session can be resumed using the profile selected by the user in the wrapper UI, even if the session was originally created under a different profile.

This is a hard requirement. Treat it as supported behavior.

## Important behavioral clarification

Do **not** implement profile filtering as the main concept.

The user does **not** primarily want:

- “show sessions created by profile X”

The user **does** want:

- “choose the profile I want to work in now”
- “choose any discovered session”
- “resume that session under the profile I selected now”

So the core mental model is:

1. choose target profile
2. choose session
3. launch `codex resume` under the chosen profile home

## Evidence from an existing implementation

You may and should study `/data/data/com.termux/files/home/prj/ai`.

Relevant references:

- `/data/data/com.termux/files/home/prj/ai/lib/ai_session.py`
  - session discovery across `.codex` and `.codex-profiles/*`
  - resolution logic
- `/data/data/com.termux/files/home/prj/ai/verify/ai-tui-smoke.sh`
  - cross-profile resume planning behavior

Two specific proofs:

1. `ai` keeps the requested target profile even when the source session belongs to another profile:
   - see `verify/ai-tui-smoke.sh` around lines `258-295`
   - expected result there is effectively:
     - `codex resume s-team-alpha|`

2. `ai` can resume a selected session with a selected profile different from the session’s original profile:
   - see `verify/ai-tui-smoke.sh` around lines `449-475`
   - expected display result there is:
     - `codex -p main -d ~/work/main -s s-default`

That repo proves the product behavior is viable. You may reuse ideas and logic from it.

## Current wrapper behavior you must preserve

### Bare and unknown-command dispatch

Current wrapper behavior routes unknown commands through the recent wrapper profile:

- see `lib/codex-termux.sh` around `codex_main()`
- bare `codex resume` therefore currently opens under the recent profile

Do **not** change bare `codex resume` behavior in this task.

Add only the new explicit surface:

```sh
codex session
```

## Implementation boundary

Use the scaffold already added in this repo:

- `tools/codex_termux/session.py`

That module exists as the ownership boundary for this feature. Expand it rather than scattering unrelated logic into existing modules.

Expected ownership split:

- shell:
  - dispatch
  - minimal wrapper UI framing
  - launching the helper
- python helper:
  - profile discovery
  - session discovery/indexing/parsing
  - interactive list model / selection plan
  - final launch plan generation

## UX requirements

### Command

```sh
codex session
```

### First screen

Show selectable target profiles:

- `default`
- every valid directory under `~/.codex-profiles`

Single selection only for v1.

Do **not** implement multi-select in v1.

### Second screen

Show a unified session list aggregated from:

- `~/.codex/sessions`
- `~/.codex-profiles/*/sessions`

Each row should include enough metadata to disambiguate sessions, such as:

- source profile
- updated time
- workdir
- short title / prompt summary when available

### Launch behavior

When the user selects:

- target profile = `default`
- session = some session from any source profile

the wrapper should resume that session with default-profile semantics:

- no forced `CODEX_HOME`

When the user selects:

- target profile = non-default profile

the wrapper should resume that session with:

- `CODEX_HOME=~/.codex-profiles/<target>`

Do not mutate the original source session files.
Do not rewrite upstream session data as part of selection.

## Technical constraints

1. Do not patch or alter the upstream Codex binary.
2. Do not replace upstream `resume` semantics with a fake local session engine.
3. Prefer wrapper-side discovery and launch planning, then delegate actual resume to upstream Codex.
4. Preserve existing wrapper UI style:
   - `codex_ui_menu_header`
   - `codex_ui_menu_row`
   - `codex_prompt_interactive`
   - existing cancellation behavior
5. Keep ASCII-only edits unless an existing file already uses Unicode.
6. Avoid broad refactors outside the new surface.

## Recommended approach

### Shell

In `lib/codex-termux.sh`:

- add a `session` command branch in `codex_main()`
- add a small `codex_session()` wrapper function
- let that function invoke the python helper module

### Python helper

In `tools/codex_termux/session.py`:

- implement profile home discovery
- implement Codex session file discovery
- parse enough session metadata for a useful picker
- build a launch plan

You may port or adapt only the relevant parts of:

- `/data/data/com.termux/files/home/prj/ai/lib/ai_session.py`

Do not import `prj/ai` directly as a runtime dependency.
Copy/adapt only the minimum logic needed.

### Launch plan

The helper should return a structured plan to shell, for example:

- target profile
- target profile dir
- selected session ref
- source profile
- workdir
- argv to execute
- whether `CODEX_HOME` must be set

The shell should execute the final plan via the wrapper’s existing runtime execution path.

## Tests

Add focused tests only.

Minimum expected coverage:

1. profile discovery:
   - default + custom profiles

2. session discovery:
   - sessions under `.codex/sessions`
   - sessions under `.codex-profiles/*/sessions`

3. cross-profile launch plan:
   - source session from non-default profile
   - selected target profile = default
   - selected target profile = non-default

4. wrapper command contract:
   - `codex session` is recognized by wrapper dispatch

If you create a TUI-like picker in Python, keep tests around selection planning and rendering contracts focused and deterministic.

## Non-goals

- changing bare `codex resume`
- implementing multi-select profile filtering
- merging or migrating upstream session stores
- changing profile creation semantics
- touching upstream binary patch policy

## Deliverable standard

Finish with:

1. implemented `codex session`
2. tests passing
3. concise explanation of the launch model:
   - source session profile may differ
   - selected target profile controls the resume environment

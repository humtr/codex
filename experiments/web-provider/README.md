# Web-provider experiment

The separate `experiment/web-provider` worktree lives at `prj/web/codex-experiment`.
SPEC owns its contract, GOAL its accepted scope, WORKBOARD the current slice.
The original investigation files in `prj/web` remain untouched; do not run their
older HOME-inheriting probe. This experiment changes no Core/Manager command or
installed account/runtime. `/switch` continues to select execution homes.

`protocol.py` validates classic `tools` and Responses-Lite `additional_tools`,
exact namespaces/custom input, call IDs, arguments and outputs. Unknown or
ambiguous definitions and unselected call types refuse. Client tool-search
metadata is retained; its execution has not been qualified.

`probe.py` accepts a complete signed generation, public key and explicit Core /
runtime SHA-256 pins. It copies the exact inventory into temporary HOME,
CODEX_HOME, PREFIX and workspace, then runs the actual public Core with upstream
`-p web-probe exec --ephemeral`. The tools check their own HOME/CODEX_HOME/PREFIX
before reading a random nonce twice; the final reply derives from both correlated
returns. Non-login shell execution avoids caller startup configuration. No auth,
external model, browser session, TUI, compaction or resume proof is claimed.

The two known model names below seed only the copied client's protocol/tool
metadata. All inference responses are loopback fixtures; no real service/model
was used or verified.

```sh
PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s experiments/web-provider -p 'test_*.py'
python experiments/web-provider/probe.py \
  --capability-seed gpt-5.5 \
  --generation /PATH/TO/OWNED/SIGNED/GENERATION \
  --public-key /PATH/TO/PUBLIC/KEY.pem \
  --parent /data/data/com.termux/files/usr/tmp \
  --core-sha256 EXACT_CORE_DIGEST --runtime-sha256 EXACT_RUNTIME_DIGEST \
  --report /PATH/TO/OWNED/report.json
```

Repeat the probe with `--capability-seed gpt-5.6-sol` for actual namespaced custom
code-mode execution and Responses-Lite input. The first selected real-service
candidate is `miuuyy/codex-chatgpt-web` at immutable
`92a356fac2292e3af5a97ab7ba634edd8d38621e`. Its full-mode browser/connector lifecycle
is separate from this protocol baseline; desktop login and Codex setup have not
been run. Do not install its configuration over a real profile during development.

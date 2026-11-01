# Manager tmux integration — inactive development note

The user chose to remove the optional Manager tmux container/attach and
pane-target compatibility from the active product. The current SDD in
`SPEC.md` is authoritative: direct native and stock Android window return
remain supported, but the Manager neither launches tmux nor inspects its
sockets/panes. No alternate tool or background session manager replaces it.

This directory preserves **the pre-extraction source and tests**, for review
and possible separately authorized future work. It is deliberately outside
`crates/manager/src` and `crates/manager/tests`; Cargo must not compile or
run this code.

- `tmux_launch.rs`: old optional container launcher, selector and attach path.
- `terminal_with_tmux.rs`: pre-extraction foreground binding and pane scanning.
- `tmux_launch_tests.rs`, `tmux_public_path.py`,
  `tmux_origin_focus.py` and `window_paths_with_tmux.py`: previous fixtures.

The historical `--tmux=status` branch in `window_paths_with_tmux.py` failed
in integrated qualification while independent native/stock window cases
passed. This archive is not a claim that tmux status or attach works.
Do not reenable it merely to satisfy old tests. A future change would
require a new product decision and an updated active SDD.

Current functionality is implemented in `crates/manager/src/launch.rs`,
`terminal.rs`, `window.rs` and `origin.rs`. Historical `focus-v1` value `tmux\n` is no longer accepted.
This archive is reference material and provides no compatibility runtime.

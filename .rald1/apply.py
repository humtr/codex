from pathlib import Path

p = Path("crates/core/src/main.rs")
s = p.read_text()


def replace_once(old: str, new: str) -> None:
    global s
    count = s.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one match, found {count}: {old[:100]!r}")
    s = s.replace(old, new, 1)


replace_once(
    "#[cfg(unix)]\nmod rollback_guard;\n",
    "#[cfg(unix)]\nmod rollback_guard;\n\n#[cfg(unix)]\nmod local_derived;\n",
)

prepare = s.index("fn prepare_signed_local_release_with_hold_policy(")
current_decl = s.index(
    "    let (current_release, _) = verify_installed_local_release(", prepare
)
current_decl_end = current_decl + len(
    "    let (current_release, _) = verify_installed_local_release("
)
s = (
    s[:current_decl]
    + "    let (current_release, current_loaded) = verify_installed_local_release("
    + s[current_decl_end:]
)
sequence_start = s.index(
    "    if source_release.release_sequence < current_release.release_sequence {", prepare
)
hold_match = s.index("    match hold_policy {", sequence_start)
new_admission = '''    let already_current = local_derived::admit_signed_candidate(
        source_dir,
        roots,
        &before,
        &current_release,
        &current_loaded,
        &source_release,
        &source_loaded,
    )?;
    let validated_hold = rollback_guard::effective_update_hold(roots)?;
    if already_current {
        if hold_policy == UpdateHoldPolicy::ForceHeld {
            let hold = validated_hold
                .as_ref()
                .ok_or(LocalProductError::UpdateHold(
                    "forced update requires an active rollback hold",
                ))?;
            if source_release.release_sequence != hold.release_sequence {
                return Err(LocalProductError::UpdateHold(
                    "forced update candidate does not match the held release sequence",
                ));
            }
        }
        return Ok(PreparedSignedLocalRelease::AlreadyCurrent(before.current));
    }
'''
s = s[:sequence_start] + new_admission + s[hold_match:]

unified = s.index("fn activate_unified_update_with_hold_policy(")
fallback_start = s.index("        Err(LocalProductError::RemoteTransportFailed)", unified)
fallback_end = s.index("        Err(error) => Err(error),", fallback_start)
new_fallback = '''        Err(LocalProductError::RemoteTransportFailed)
            if hold_policy == UpdateHoldPolicy::Enforce =>
        {
            let generation_id = local_derived::activate(roots, process_env)?;
            Ok((
                SignedUpdateOutcome::Activated(generation_id),
                true,
                GithubPublicationOutcome::Skipped,
            ))
        }
'''
s = s[:fallback_start] + new_fallback + s[fallback_end:]

rollback = s.index("fn rollback_signed_local_release(")
rollback_lock = s.index(
    "    let lock = m2_generation_state::acquire_activation_lock(&state_paths)", rollback
)
local_marker = '''    let local_derived_current = local_derived::current_is_local_derived(
        &before,
        &current_release,
        &current_loaded,
    )?;
'''
s = s[:rollback_lock] + local_marker + s[rollback_lock:]

hold_start = s.index("    let hold = UpdateHoldRecord {", rollback)
guard_start = s.index("    let guard = if current_loaded.core_path.is_some() {", hold_start)
new_hold = '''    let hold = (!local_derived_current).then(|| UpdateHoldRecord {
        generation_id: before.current.clone(),
        release_sequence: current_release.release_sequence,
    });
'''
s = s[:hold_start] + new_hold + s[guard_start:]
guard_old = "    let guard = if current_loaded.core_path.is_some() {"
guard_pos = s.index(guard_old, rollback)
s = (
    s[:guard_pos]
    + "    let guard = if !local_derived_current && current_loaded.core_path.is_some() {"
    + s[guard_pos + len(guard_old):]
)
write_hold_start = s.index(
    "    if let Err(error) = write_update_hold_locked(roots, &hold) {", rollback
)
write_hold_end = s.index(
    "    if guard.is_some() && target_hold_aware {", write_hold_start
)
new_write_hold = '''    if let Some(hold) = hold.as_ref() {
        if let Err(error) = write_update_hold_locked(roots, hold) {
            return Err(LocalProductError::RollbackHoldAfterCommit(Box::new(error)));
        }
    }
'''
s = s[:write_hold_start] + new_write_hold + s[write_hold_end:]

replace_once(
    'const UPDATE_USAGE: &str =\n    "usage: codex update [--help] | --force | --local <DIRECTORY> | --remote <HTTPS_BASE_URL> | --rollback";',
    'const UPDATE_USAGE: &str =\n    "usage: codex update [--help] | --force | --build-local | --local <DIRECTORY> | --remote <HTTPS_BASE_URL> | --rollback";',
)

run_update = s.index("fn run_core_update(args: Vec<OsString>) -> i32 {")
local_parser = s.index(
    '    let local = args.len() == 2 && args[0] == OsStr::new("--local") && !args[1].is_empty();',
    run_update,
)
s = (
    s[:local_parser]
    + '    let build_local = args.len() == 1 && args[0] == OsStr::new("--build-local");\n'
    + s[local_parser:]
)
condition_old = "    if !local && !remote && !rollback {"
condition_pos = s.index(condition_old, run_update)
s = (
    s[:condition_pos]
    + "    if !build_local && !local && !remote && !rollback {"
    + s[condition_pos + len(condition_old):]
)
rollback_route = '''    if rollback {
        return run_core_rollback();
    }
    let process_env = capture_termux_process_env();
'''
rollback_route_pos = s.index(rollback_route, run_update)
new_route = '''    if rollback {
        return run_core_rollback();
    }
    let process_env = capture_termux_process_env();
    if build_local {
        return match local_derived::activate(&roots, &process_env) {
            Ok(generation_id) => {
                println!("activated local-derived generation {generation_id}");
                0
            }
            Err(err) => {
                eprintln!("codex update --build-local: {err}");
                1
            }
        };
    }
'''
s = (
    s[:rollback_route_pos]
    + new_route
    + s[rollback_route_pos + len(rollback_route):]
)
output_marker = 'println!("activated local-built generation {generation_id}");'
output_pos = s.index(output_marker, run_update)
s = (
    s[:output_pos]
    + 'println!("activated local-derived generation {generation_id}");'
    + s[output_pos + len(output_marker):]
)

for function_name in ["activate_local_built_update", "publish_local_update_to_github"]:
    marker = f"#[cfg(unix)]\nfn {function_name}("
    if marker not in s:
        raise SystemExit(f"function marker not found: {function_name}")
    s = s.replace(
        marker,
        f"#[cfg(unix)]\n#[allow(dead_code)]\nfn {function_name}(",
        1,
    )

p.write_text(s)

spec = Path("SPEC.md")
text = spec.read_text()
heading = "## RALD-1 local-derived update contract"
if heading in text:
    raise SystemExit("RALD-1 SPEC contract already exists on frozen parent")
text += '''

## RALD-1 local-derived update contract

`codex update --build-local` is the public local-derived selector. It accepts no path or
other selector and is never an alias for `--force`, `--local`, official signing, or
publication. The builder MUST resolve only official OpenAI stable metadata, fetch the
corresponding Android/AArch64 upstream archive, and require its exact SHA-256 to equal
the official metadata before adaptation.

A local-derived generation MUST be signed only with a fresh device-local ephemeral
Ed25519 key created inside a private `0700` staging root. Its private PEM MUST be a
`0600` regular file and MUST be deleted before activation; failure paths MUST clean it
best-effort through the same bounded staging lifetime. The private key MUST NOT enter
activation state, an installed generation, cache, publication tree, or log. Only the
ephemeral public verifier may become that generation's `current_key`.

`activation-state-v3.update_key` remains the official forward-update authority and MUST
remain byte-identical across local-derived activation. The local-derived signed
`generation.meta` MUST carry canonical `codex-local-derived-v1` provenance binding the
authenticated public baseline sequence, the official `update_key`, and the SHA-256 of
the authenticated baseline `release.manifest`. The local-derived release manifest uses
that baseline public sequence unchanged; it does not allocate or consume a synthetic
public sequence. Malformed provenance or provenance inconsistent with authenticated
state fails closed.

Signed stable admission while local-derived is current compares against that recorded
public baseline, not a synthetic local sequence. A signed candidate above the baseline
advances normally. The exact authenticated baseline manifest may restore official
current at the same sequence. Other same/lower-sequence candidates remain rollback
failures, and rollback-hold/`--force` comparison remains independently enforced.
Official candidate verification and any accepted official key rotation continue to use
`activation-state-v3.update_key`.

Local-derived activation uses the existing probe plus atomic current/previous generation
transaction while preserving `update_key`. Rolling back from a valid local-derived
current to its retained previous generation MUST NOT create a public update hold or the
legacy rollback-Core hold guard. Rolling back from an official current retains the
existing authenticated hold/guard semantics unchanged. `codex update --force` remains
limited to retrying the exact authenticated held public sequence and never targets a
local-derived generation.

If the ordinary signed-channel update encounters the accepted transport-unavailable
fallback condition, that fallback MUST invoke the same local-derived builder and MUST
report publication skipped. It MUST NOT inspect an official private signing-key path,
invoke `gh`, create a GitHub Release, update Pages/main/stable index, or change behavior
merely because maintainer credentials exist on the device. The legacy device-side
official producer helpers remain non-public implementation residue pending RALD-2 and
are not reachable from the local-derived selector or transport fallback.
'''
spec.write_text(text)

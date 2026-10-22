#!/usr/bin/env python3
"""Frozen external D3 contracts. Builds/tests a copy, never changes model work."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


API_TEST = r'''
#[cfg(test)] mod d3_external_contract {
    use super::*;
    #[test] fn entire_batch_rolls_back() {
        let mut state = ledger::Ledger::new();
        state.insert("a".into(), 10);
        state.insert("b".into(), 2);
        let original = state.clone();
        let commands = parse::parse("transfer a b 4\nadd a -99\n").unwrap();
        assert!(ledger::apply(&mut state, &commands).is_err());
        assert_eq!(state, original);
    }
    #[test] fn overflow_and_self_transfer_are_checked() {
        let mut state = ledger::Ledger::new();
        state.insert("a".into(), 10);
        state.insert("b".into(), i64::MAX);
        let original = state.clone();
        assert!(ledger::apply(&mut state, &parse::parse("transfer a b 1").unwrap()).is_err());
        assert_eq!(state, original);
        ledger::apply(&mut state, &parse::parse("transfer a a 10").unwrap()).unwrap();
        assert_eq!(state, original);
        for text in ["transfer a a 11", "transfer absent absent 1"] {
            assert!(ledger::apply(&mut state, &parse::parse(text).unwrap()).is_err());
            assert_eq!(state, original);
        }
        for amount in [0, -1] {
            let commands = [parse::Command::Transfer("a".into(), "b".into(), amount)];
            assert!(ledger::apply(&mut state, &commands).is_err());
            assert_eq!(state, original);
        }
    }
}
'''


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(project, stage, output):
    output.mkdir(parents=True, exist_ok=True)
    source_hashes = {str(p.relative_to(project)): sha(p) for p in project.rglob("*")
                     if p.is_file() and ".git" not in p.parts and "target" not in p.parts}
    trial = Path(tempfile.mkdtemp(prefix="d3check.", dir=os.environ["TMPDIR"]))
    shutil.copytree(project, trial / "w", ignore=shutil.ignore_patterns(".git", "target"))
    work = trial / "w"
    if stage:
        with (work / "src/main.rs").open("a") as stream:
            stream.write(API_TEST)
    target = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "codex/d3-contract-target" / trial.name
    env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_NET_OFFLINE": "true"}
    checks = []

    def command(arguments, name):
        result = subprocess.run(arguments, cwd=work, env=env, capture_output=True, timeout=120)
        (output / (name + ".log")).write_bytes(result.stdout + result.stderr)
        if result.returncode:
            raise AssertionError(name + " failed")

    command(["cargo", "test", "--offline", "--locked"], "cargo-test")
    command(["cargo", "build", "--offline", "--locked"], "cargo-build")
    binary = target / "debug/d3-ledger"

    def cli(text, args=(), expected=None, error=False):
        result = subprocess.run([str(binary), *map(str, args)], input=text.encode() if isinstance(text, str) else text,
                                capture_output=True, timeout=15)
        checks.append({"args": list(map(str, args)), "exit": result.returncode,
                       "stdout": result.stdout.decode(errors="replace"),
                       "stderr": result.stderr.decode(errors="replace")})
        if error:
            assert result.returncode == 2 and not result.stdout and result.stderr
            assert b"panicked" not in result.stderr
        else:
            assert result.returncode == 0 and not result.stderr
            assert result.stdout.decode() == expected

    try:
        cli("add z 4\nadd a 3\n", expected="a\t3\nz\t4\n")
        cli("", expected="")
        cli("add a -1\n", error=True)
        cli("bad\n", error=True)
        cli(b"\xff", error=True)
        cli("", ["--unknown"], error=True)
        if stage:
            cli("add a 10\nadd b 2\ntransfer a b 4\n", expected="a\t6\nb\t6\n")
            cli("add 서울 9\ntransfer 서울 東京 3\n", expected="東京\t3\n서울\t6\n")
            for text in ["add a 2\ntransfer a b 3\n", "transfer a b 1\n",
                         "add a 9223372036854775807\nadd a 1\n",
                         "add a 3\ntransfer a b 0\n"]:
                cli(text, error=True)
        if stage >= 2:
            state = trial / "state.tsv"
            state.write_bytes(b"a\t10\nb\t2\n")
            cli("transfer a b 4\n", ["--state", state], "a\t6\nb\t6\n")
            assert state.read_bytes() == b"a\t6\nb\t6\n"
            cli("add a 1\ntransfer a b 99\n", ["--state", state], error=True)
            assert state.read_bytes() == b"a\t6\nb\t6\n"
            cli("bad\n", ["--state", state], error=True)
            assert state.read_bytes() == b"a\t6\nb\t6\n"
            cli("add a 1\n", ["--state", trial / "absent/state"], error=True)
            for text in ["a\t1\na\t2\n", "a\t-1\n", "broken\n", "a\t9223372036854775808\n"]:
                state.write_text(text)
                cli("add a 1\n", ["--state", state], error=True)
                assert state.read_text() == text
            state.write_bytes(b"\xff")
            cli("add a 1\n", ["--state", state], error=True)
            assert state.read_bytes() == b"\xff"
            fresh = trial / "fresh.tsv"
            cli("add new 2\n", ["--state", fresh], "new\t2\n")
            assert fresh.read_bytes() == b"new\t2\n"
            fresh.unlink()  # This validator owns the temporary file, not model work.
            state.write_bytes(b"a\t6\nb\t6\n")
            assert sorted(p.name for p in trial.iterdir()) == ["state.tsv", "w"]
        if stage >= 3:
            state = trial / "state.tsv"
            cli("transfer a b 2\n", ["--state", state, "--dry-run"], "a\t4\nb\t8\n")
            assert state.read_bytes() == b"a\t6\nb\t6\n"
            cli("add a 1\n", ["--dry-run", "--state", trial / "new.tsv"], "a\t1\n")
            assert not (trial / "new.tsv").exists()
            cli("transfer a b 99\n", ["--state", state, "--dry-run"], error=True)
            assert state.read_bytes() == b"a\t6\nb\t6\n"
        assert source_hashes == {str(p.relative_to(project)): sha(p) for p in project.rglob("*")
                                 if p.is_file() and ".git" not in p.parts and "target" not in p.parts}
    finally:
        (output / "cli-results.json").write_text(json.dumps(checks, ensure_ascii=False, indent=2))
    result = {"accepted": True, "stage": stage, "cli_cases": len(checks),
              "external_Rust_tests": 2 if stage else 0, "live_project_mutated": False,
              "trial": str(trial), "compiler_target": str(target), "source_hashes": source_hashes}
    (output / "review.json").write_text(json.dumps(result, indent=2))
    return result


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    parser.add_argument("stage", type=int, choices=range(4))
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(json.dumps(check(args.project, args.stage, args.output)))

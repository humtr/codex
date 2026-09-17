#!/usr/bin/env python3
import argparse
import re
import sys

SHA_RE = re.compile(r"[0-9a-f]{40}\Z")


class ValidationError(Exception):
    pass


def fail(message: str) -> None:
    raise ValidationError(message)


def canonical_rc(value: str) -> int:
    if (
        not value
        or not value.isascii()
        or not value.isdigit()
        or (len(value) > 1 and value.startswith("0"))
    ):
        fail("ref update result is not canonical nonnegative decimal")
    parsed = int(value)
    if parsed > 255:
        fail("ref update result is out of range")
    return parsed


def classify(expected: str, new_commit: str, actual: str, update_rc: str) -> None:
    for value, label in [
        (expected, "expected ref"),
        (new_commit, "new commit"),
        (actual, "actual ref"),
    ]:
        if not SHA_RE.fullmatch(value):
            fail(f"{label} is not a canonical commit SHA")
    if expected == new_commit:
        fail("promotion commit must differ from expected parent")
    rc = canonical_rc(update_rc)
    if actual == new_commit:
        print("promotion_result=committed")
        print(f"ref_update_rc={rc}")
        return
    if actual == expected:
        fail(f"promotion CAS did not advance expected ref (rc={rc})")
    fail(f"promotion CAS race: expected {expected}, observed {actual} (rc={rc})")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--expected", required=True)
    parser.add_argument("--new-commit", required=True)
    parser.add_argument("--actual", required=True)
    parser.add_argument("--update-rc", required=True)
    args = parser.parse_args()
    try:
        classify(args.expected, args.new_commit, args.actual, args.update_rc)
    except ValidationError as exc:
        print(f"rald5 promotion: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

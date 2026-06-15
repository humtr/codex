# Wrapper Test Fixtures

This directory contains shared fixture builders used by the shell test suite.

Current builders:

- `activation-fixture.sh`: pointer/state/registry/runtime/raw fixture helpers for
  activation and rollback tests.
- `doctor-fixtures.sh`: wrapper doctor JSON fixtures for public render contract
  tests.

Rules:

- Put state/registry/runtime/raw builders here instead of duplicating JSON in
  individual tests.
- Prefer filesystem side-effect assertions over source-string assertions.
- Keep exact string checks only for public output contracts such as doctor
  headers, separators, and summary lines.
- Do not override internal shell functions when the same failure can be driven
  by fixture files, permissions, or the internal Python CLI.

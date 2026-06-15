#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

fail() {
    printf 'check-structure: %s\n' "$*" >&2
    exit 1
}

check_line_limit() {
    local path="$1"
    local max_lines="$2"
    local lines
    lines="$(wc -l < "$path")"
    if (( lines > max_lines )); then
        fail "$path has $lines lines; limit is $max_lines"
    fi
}

check_python_function_lengths() {
    python3 ci/check-python-imports.py --check-function-lengths "$@"
}

check_shell_function_lengths() {
    awk '
        BEGIN {
            max_lines = 80
            status = 0
        }
        FNR == 1 {
            depth = 0
            start_line = 0
            function_name = ""
        }
        depth == 0 &&
            match($0, /^[[:space:]]*(function[[:space:]]+)?[A-Za-z_][A-Za-z0-9_]*[[:space:]]*(\(\))?[[:space:]]*\{[[:space:]]*(#.*)?$/) {
            function_name = $0
            sub(/^[[:space:]]*function[[:space:]]+/, "", function_name)
            sub(/[[:space:]]*\(\).*/, "", function_name)
            sub(/[[:space:]]*\{.*/, "", function_name)
            sub(/^[[:space:]]*/, "", function_name)
            start_line = FNR
            depth = gsub(/\{/, "{") - gsub(/\}/, "}")
            if (depth <= 0) {
                depth = 0
            }
            next
        }
        depth > 0 {
            depth += gsub(/\{/, "{") - gsub(/\}/, "}")
            if (depth <= 0) {
                lines = FNR - start_line + 1
                if (lines > max_lines) {
                    printf "%s:%d: function %s has %d lines; limit is %d\n", \
                        FILENAME, start_line, function_name, lines, max_lines > "/dev/stderr"
                    status = 1
                }
                depth = 0
            }
        }
        END {
            exit status
        }
    ' "$@"
}

check_phase0_diff_patterns() {
    git diff --unified=0 -- \
        bin/install-runtime.sh \
        tests/installer-layout.sh \
        ci \
        tools/codex_native |
        awk '
            /^\+\+\+ b\// {
                file = substr($0, 7)
                next
            }
            /^\+\+\+/ {
                next
            }
            /^\+/ {
                line = substr($0, 2)
                if (line ~ /python3[[:space:]]+-[[:space:]]*<</) {
                    printf "%s: newly added embedded Python heredoc: %s\n", file, line > "/dev/stderr"
                    status = 1
                }
                if (line ~ /except Exception:[[:space:]]*(pass|return False|data[[:space:]]*=)/) {
                    printf "%s: newly added broad exception fallback: %s\n", file, line > "/dev/stderr"
                    status = 1
                }
                if (line ~ /\|\|[[:space:]]*true/) {
                    printf "%s: newly added blanket command fallback: %s\n", file, line > "/dev/stderr"
                    status = 1
                }
            }
            END {
                exit status
            }
        '
}

mapfile -t shell_files < <(
    find bin lib tools tests -type f -name '*.sh' -print
    printf '%s\n' install.sh
)
mapfile -t python_files < <(
    find tools ci -type f -name '*.py' -print
)
mapfile -t new_python_files < <(
    find tools/codex_native ci -type f -name '*.py' -print
)
mapfile -t c_files < <(
    find tools -type f -name '*.c' -print
)

for path in "${shell_files[@]}"; do
    bash -n "$path"
done

for path in "${python_files[@]}"; do
    python3 -m py_compile "$path"
done

if command -v clang >/dev/null 2>&1; then
    for path in "${c_files[@]}"; do
        clang -fsyntax-only -Wall -Wextra -Werror "$path"
    done
fi

check_line_limit lib/codex-termux-lib.sh 900
for path in tools/codex_native/*.py ci/check-python-imports.py; do
    check_line_limit "$path" 350
done

check_python_function_lengths "${new_python_files[@]}"
check_shell_function_lengths "${shell_files[@]}"

python3 ci/check-python-imports.py

if grep -R -n -E 'python3[[:space:]]+-[[:space:]]*<<' tools/codex_native ci; then
    fail "embedded Python block found in new package or CI"
fi
if grep -R -n -E 'except Exception:[[:space:]]*(pass|return False|data[[:space:]]*=)' tools/codex_native ci; then
    fail "forbidden broad exception fallback found in new package or CI"
fi
if grep -R -n -E '\|\|[[:space:]]*true' tools/codex_native ci; then
    fail "blanket command fallback found in new package or CI"
fi

check_phase0_diff_patterns

git diff --check

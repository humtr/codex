# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_termux_help() {
    codex_status_clear
    cat <<'USAGE'
Codex Termux wrapper commands

Usage:
  codex termux <command> [args...]

Commands:
  help                         Print this wrapper command help.
  install [VERSION]            Install support files and a fresh patched runtime.
  install support              Refresh support files and the public launcher only.
  install upstream [VERSION]   Install selected/latest upstream package with current support.
  install rebuild              Rebuild a patched runtime from the cached raw package.
  update [VERSION]             Refresh support files and runtime; same as install.
  repair                       Diagnose and repair the managed installation.
  doctor [--json]              Check launcher, runtime resources, resolver, CA, DNS patch, and state.
  version                      Print upstream, runtime, and wrapper version/date rows.
  use [--list|SELECTION]       List or promote cached/remote runtimes.
  profile [list|NAME]          List profiles or launch with an explicit CODEX_HOME profile.
  session [PROFILE] [--all]    Pick and resume discovered Codex sessions across profiles.
  notify [options]             Configure notification/toast hooks.
  remove                       Remove managed launcher/runtime and restore launcher backups.

Top-level codex arguments are reserved for upstream Codex. Use "codex termux help" for wrapper operations.
USAGE
}



codex_install_source_command() {
    if [ -n "${CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-}" ] && [ -x "$CODEX_TERMUX_INSTALL_RUNTIME_SOURCE" ]; then
        printf '%s\n' "$CODEX_TERMUX_INSTALL_RUNTIME_SOURCE"
        return 0
    fi
    return 1
}

codex_run_install_source_command() {
    local source="$1" command="$2" tmp source_root snapshot status=0
    shift 2
    source_root="$(cd "$(dirname "$source")/.." && pwd)" || return 1
    tmp="$(codex_mktemp_dir codex-install-source)" || return 1
    snapshot="$tmp/source"
    cp -R "$source_root" "$snapshot" || {
        rm -rf "$tmp"
        return 1
    }
    bash "$snapshot/bin/install-runtime.sh" "$command" "$@" || status=$?
    rm -rf "$tmp"
    return "$status"
}

codex_install_public() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_run_install_source_command "$source" install "$@"
}

codex_update_full_public() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_run_install_source_command "$source" update "$@"
}

codex_repair_surface_public() {
    local source
    case "${1:-}" in
        "")
            ;;
        -h|--help|help)
            codex_termux_help
            return 0
            ;;
        *)
            codex_fail "repair does not take arguments"
            return 2
            ;;
    esac
    source="$(codex_install_source_command)" || {
        codex_repair_public "$@"
        return $?
    }
    codex_run_install_source_command "$source" repair "$@"
}


codex_termux_version_public() {
    case "${1:-}" in
        "")
            codex_version
            ;;
        -h|--help|help)
            cat <<'USAGE'
Usage: codex termux version

Prints upstream Codex and managed wrapper/runtime version rows.
USAGE
            ;;
        *)
            codex_fail "termux version does not take arguments"
            return 2
            ;;
    esac
}

codex_termux_main() {
    case "${1:-}" in
        ""|help|-h|--help)
            codex_termux_help
            ;;
        install)
            shift
            codex_install_public "$@"
            ;;
        update)
            shift
            codex_update_full_public "$@"
            ;;
        repair)
            shift
            codex_repair_surface_public "$@"
            ;;
        doctor)
            shift
            codex_termux_doctor_public "$@"
            ;;
        version)
            shift
            codex_termux_version_public "$@"
            ;;
        use)
            shift
            codex_use "$@"
            ;;
        session)
            shift
            codex_session "$@"
            ;;
        profile)
            shift
            codex_profile_run "$@"
            ;;
        notify)
            shift
            codex_notify_public "$@"
            ;;
        remove)
            shift
            codex_remove
            ;;
        *)
            codex_fail "Unknown termux command: ${1:-}"
            return 2
            ;;
    esac
}

codex_main() {
    local status=0
    case "${1:-}" in
        termux)
            shift
            codex_termux_main "$@"
            ;;
        *)
            if ! codex_ensure_runtime_ready; then
                status=$?
            elif ! codex_auto_update_if_needed; then
                status=$?
            else
                codex_runtime_exec_with_context "$@"
            fi
            ;;
    esac
    status=$?
    codex_status_clear
    return "$status"
}

"""Install/update command planning for the Termux wrapper."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import shlex


ACTION_ERROR = "error"
ACTION_USAGE = "usage"
ACTION_INSTALL_FULL = "install_full"
ACTION_SUPPORT = "support"
ACTION_UPSTREAM = "upstream"
ACTION_REBUILD = "rebuild"
ACTION_REPAIR = "repair"


@dataclass(frozen=True)
class InstallPlan:
    action: str
    surface: str = ""
    version: str = ""
    exit_code: int = 0
    error: str = ""

    def to_dict(self) -> dict[str, object]:
        result = asdict(self)
        result["surface_message"] = surface_message(self.surface, self.version)
        result["success_message"] = success_message(self.surface)
        return result


def surface_message(surface: str, version: str = "") -> str:
    if surface == "install":
        return "Installing wrapper support and fresh upstream runtime"
    if surface == "update":
        return "Updating wrapper support and fresh upstream runtime"
    if surface == "support":
        return "Installing wrapper support and launcher"
    if surface == "upstream":
        if version:
            return f"Installing upstream Codex {version}"
        return "Installing latest upstream Codex"
    if surface == "rebuild":
        return "Rebuilding runtime from cached raw package"
    if surface == "repair":
        return "Repairing managed installation"
    return "Working"


def success_message(surface: str) -> str:
    if surface == "support":
        return "Support files and launcher are ready"
    return ""


def plan(command: str, args: list[str]) -> InstallPlan:
    args = _strip_remainder_separator(args)
    if command == "install":
        return _install(args)
    if command == "update":
        return _update(args)
    if command == "repair":
        return _repair(args)
    return InstallPlan(
        action=ACTION_ERROR,
        exit_code=2,
        error=f"unknown install command: {command}",
    )


def plan_shell_exports(command: str, args: list[str]) -> str:
    data = plan(command, args).to_dict()
    names = {
        "action": "CODEX_INSTALL_PLAN_ACTION",
        "surface": "CODEX_INSTALL_PLAN_SURFACE",
        "version": "CODEX_INSTALL_PLAN_VERSION",
        "exit_code": "CODEX_INSTALL_PLAN_EXIT_CODE",
        "error": "CODEX_INSTALL_PLAN_ERROR",
        "surface_message": "CODEX_INSTALL_PLAN_SURFACE_MESSAGE",
        "success_message": "CODEX_INSTALL_PLAN_SUCCESS_MESSAGE",
    }
    return "\n".join(
        f"{env_name}={shlex.quote(str(data[key]))}"
        for key, env_name in names.items()
    )


def _install(args: list[str]) -> InstallPlan:
    if not args:
        return InstallPlan(action=ACTION_INSTALL_FULL, surface="install")
    first, rest = args[0], args[1:]
    if _is_help(first):
        return InstallPlan(action=ACTION_USAGE)
    if first == "support":
        return _no_args_or_help(rest, surface="support", action=ACTION_SUPPORT)
    if first == "upstream":
        version = _optional_version("install upstream", rest)
        if isinstance(version, InstallPlan):
            return version
        return InstallPlan(action=ACTION_UPSTREAM, surface="upstream", version=version)
    if first == "rebuild":
        return _no_args_or_help(rest, surface="rebuild", action=ACTION_REBUILD)
    version = _optional_version("install", args)
    if isinstance(version, InstallPlan):
        return version
    return InstallPlan(action=ACTION_INSTALL_FULL, surface="install", version=version)


def _update(args: list[str]) -> InstallPlan:
    if args and _is_help(args[0]):
        return InstallPlan(action=ACTION_USAGE)
    version = _optional_version("update", args)
    if isinstance(version, InstallPlan):
        return version
    return InstallPlan(action=ACTION_INSTALL_FULL, surface="update", version=version)


def _repair(args: list[str]) -> InstallPlan:
    if not args:
        return InstallPlan(action=ACTION_REPAIR, surface="repair")
    if len(args) == 1 and _is_help(args[0]):
        return InstallPlan(action=ACTION_USAGE)
    return InstallPlan(
        action=ACTION_ERROR,
        exit_code=2,
        error="repair does not take arguments",
    )


def _no_args_or_help(args: list[str], *, surface: str, action: str) -> InstallPlan:
    if len(args) == 1 and _is_help(args[0]):
        return InstallPlan(action=ACTION_USAGE)
    if args:
        return InstallPlan(
            action=ACTION_ERROR,
            exit_code=2,
            error=f"install {surface} does not take arguments",
        )
    return InstallPlan(action=action, surface=surface)


def _optional_version(command_name: str, args: list[str]) -> str | InstallPlan:
    if not args:
        return ""
    if len(args) > 1:
        return InstallPlan(
            action=ACTION_ERROR,
            exit_code=64,
            error=f"{command_name} takes at most one version argument",
        )
    version = args[0]
    if _is_help(version):
        return InstallPlan(action=ACTION_USAGE)
    if version.startswith("-"):
        return InstallPlan(
            action=ACTION_ERROR,
            exit_code=64,
            error=f"{command_name} version must not start with '-': {version}",
        )
    return version


def _is_help(value: str) -> bool:
    return value in {"-h", "--help", "help"}


def _strip_remainder_separator(args: list[str]) -> list[str]:
    if args and args[0] == "--":
        return args[1:]
    return args

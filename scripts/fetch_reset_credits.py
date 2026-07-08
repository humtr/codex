#!/data/data/com.termux/files/usr/bin/python3
import argparse
import json
import os
from pathlib import Path
from urllib import error, request


DEFAULT_ENDPOINT = "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits"
KNOWN_CREDIT_KEYS = (
    "id",
    "reset_type",
    "status",
    "granted_at",
    "expires_at",
    "redeem_started_at",
    "redeemed_at",
    "profile_image_url",
    "profile_user_id",
    "title",
    "description",
)


def candidate_auth_paths() -> list[Path]:
    candidates: list[Path] = []
    env_path = os.environ.get("CODEX_AUTH_JSON")
    if env_path:
        candidates.append(Path(env_path).expanduser())

    codex_home = os.environ.get("CODEX_HOME")
    if codex_home:
        candidates.append(Path(codex_home).expanduser() / "auth.json")

    home = Path(os.environ.get("HOME") or str(Path.home()))
    candidates.extend([
        home / ".codex" / "auth.json",
        home / ".config" / "codex" / "auth.json",
    ])
    candidates.extend(sorted((home / ".codex-profiles").glob("*/auth.json")))
    return candidates


def load_auth() -> tuple[str, str]:
    for path in candidate_auth_paths():
        if not path.exists():
            continue
        data = json.loads(path.read_text())
        tokens = data.get("tokens", {})
        access_token = tokens.get("access_token")
        account_id = tokens.get("account_id")
        if access_token and account_id:
            return access_token, account_id
    raise SystemExit("Could not locate a usable Codex auth.json with access_token and account_id")


def fetch_reset_credits(endpoint: str) -> dict:
    access_token, account_id = load_auth()
    req = request.Request(
        endpoint,
        headers={
            "Authorization": f"Bearer {access_token}",
            "ChatGPT-Account-Id": account_id,
            "Accept": "application/json",
            "User-Agent": "credit-status-skill",
        },
    )
    try:
        with request.urlopen(req, timeout=20) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except error.HTTPError as exc:
        body = exc.read().decode("utf-8", "replace")
        raise SystemExit(f"HTTP {exc.code}: {body}") from exc
    except error.URLError as exc:
        raise SystemExit(f"Network error: {exc.reason}") from exc


def format_value(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def format_payload(payload: dict) -> str:
    credits = payload.get("credits")
    credit_list = credits if isinstance(credits, list) else []
    sorted_credits = sorted(
        [c for c in credit_list if isinstance(c, dict)],
        key=lambda c: str(c.get("granted_at") or ""),
    )
    lines: list[str] = ["조회 결과:"]

    available_count = payload.get("available_count", 0)
    total_earned_count = payload.get("total_earned_count", 0)
    lines.append(f"  - available_count: {format_value(available_count)}")
    lines.append(f"  - total_earned_count: {format_value(total_earned_count)}")

    for key, value in payload.items():
        if key in {"credits", "available_count", "total_earned_count"}:
            continue
        lines.append(f"  - {key}: {format_value(value)}")

    lines.append("")

    for index, raw_credit in enumerate(sorted_credits, start=1):
        credit_id = raw_credit.get("id") or f"credit-{index}"
        lines.append(f"  {index}. {format_value(credit_id)}")
        for key in KNOWN_CREDIT_KEYS:
            if key == "id":
                continue
            if key in raw_credit:
                lines.append(f"      - {key}: {format_value(raw_credit[key])}")
        for key, value in raw_credit.items():
            if key in KNOWN_CREDIT_KEYS:
                continue
            lines.append(f"      - {key}: {format_value(value)}")
        if index != len(sorted_credits):
            lines.append("")

    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Fetch OpenAI credit status for the current account")
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--json", action="store_true", help="Print raw JSON instead of formatted text")
    args = parser.parse_args()

    payload = fetch_reset_credits(args.endpoint)
    if args.json:
        print(json.dumps(payload, indent=2, ensure_ascii=False, sort_keys=False))
    else:
        print(format_payload(payload))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

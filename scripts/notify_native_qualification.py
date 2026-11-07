#!/usr/bin/env python3
"""One-shot qualification completion notice to the bound existing Codex thread."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import uuid


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value, exclusive=False):
    with path.open("x" if exclusive else "w") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.flush()
        os.fsync(stream.fileno())


def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def tail_rows(path):
    with path.open("rb") as stream:
        stream.seek(max(0, path.stat().st_size - 512 * 1024))
        if stream.tell():
            stream.readline()
        lines = stream.read().splitlines()
    return [json.loads(line) for line in lines if line]


def bind(root, label="D3"):
    assert label in ("D3", "F", "G"), "unsupported qualification label"
    assert not (root / "launch-intent.json").exists()
    thread = str(uuid.UUID(os.environ["CODEX_THREAD_ID"]))
    home = Path(os.environ["CODEX_HOME"])
    paths = list((home / "sessions").rglob("*" + thread + "*.jsonl"))
    assert len(paths) == 1, "exact supervisor rollout required"
    with paths[0].open() as stream:
        first = json.loads(stream.readline())
    assert first["type"] == "session_meta" and first["payload"]["id"] == thread
    turns = [r["payload"]["turn_id"] for r in tail_rows(paths[0])
             if r["payload"].get("turn_id") and (r["type"] in ("token_usage_record", "turn_context")
                 or r["type"] == "event_msg" and r["payload"].get("type") == "task_started")]
    assert turns, "current supervisor turn not observed"
    binary = Path(os.environ["CODEX_PROFILE_CORE"]).resolve(strict=True)
    keys = ("HOME", "CODEX_HOME", "CODEX_SQLITE_HOME", "CODEX_PROFILE_CORE", "PREFIX", "PATH", "TMPDIR", "LANG", "TERM")
    value = {"label": label, "thread_id": thread, "rollout": str(paths[0]), "source_turn": turns[-1], "rollout_offset": paths[0].stat().st_size,
             "core": str(binary), "core_sha256": sha(binary),
             "environment": {k: os.environ[k] for k in keys if k in os.environ}, "bound_UTC": stamp()}
    save(root / "notification-binding.json", value, exclusive=True)
    return {"bound_existing_thread": thread, "model_request_sent": False}


def send(root, probe=False):
    binding = json.loads((root / "notification-binding.json").read_text())
    label = binding.get("label", "D3")
    assert label in ("D3", "F", "G"), "unsupported qualification label"
    assert sha(Path(binding["core"])) == binding["core_sha256"], "supervisor Core changed"
    kind = "probe" if probe else "result"
    nonce = uuid.uuid4().hex
    if probe:
        message = (f"[{label}_NOTIFY_PROBE {nonce}]\n"
                   "사용자가 승인한 완료 통보·동일 세션 자동 재개 실증입니다. "
                   f"먼저 `python -B {root}/notify.py ack {root}`으로 정식 수신과 새 턴을 확인하세요. "
                   f"통과하면 사용자 승인대로 준비된 {label} 배치를 시작하세요: "
                   f"`python -B {root}/run.py start {root}`. "
                   "이미 launch-intent/run-started가 있으면 중복 시작하지 말고 상태를 대조하세요. "
                   "시작 확인 후 응답을 끝내고 감시하지 마세요. 최종 통보가 같은 세션으로 돌아옵니다.")
    else:
        result = json.loads((root / "result.json").read_text())
        state = "자동 검사 완료·최종 검수 대기" if result.get("automated_gates") else "실패/중단·보존 상태 검수 필요"
        message = (f"[{label}_NOTIFY_RESULT {nonce}] {state}\n"
                   f"결과: {root}/result.json\n이벤트: {root}/events.jsonl\n턴별 시간: {root}/turns.md\n"
                   f"사용자가 승인한 {label} 결과를 실제 diff·검사·원래 CLI·정식 기록·저장 Chat과 대조해 최종 보고하세요. "
                   "실패·불확실한 호출은 최신 실행 상태·원래 기록과 대조하고 재전송하지 마세요. "
                   "검수 후 사용자가 이미 승인한 전체 단계의 원인 보완과 다음 검증을 계속하세요. "
                   "새 검증은 확인된 종료/보존 상태에서 별도 입력·실행 의도로 구분하고, 이 통보를 전체 작업 중단 지시로 해석하지 마세요.")
    notice = {"kind": kind, "nonce": nonce, "thread_id": binding["thread_id"], "message": message,
              "notifier_sha256": sha(Path(__file__)), "intent_UTC": stamp()}
    save(root / f"notification-{kind}-intent.json", notice, exclusive=True)
    command = [binding["core"], "queue", "--thread", binding["thread_id"], "--message", message]
    try:
        result = subprocess.run(command, env=binding["environment"], capture_output=True, timeout=60)
    except subprocess.TimeoutExpired:
        save(root / f"notification-{kind}-unknown.json", {"state": "delivery_unknown_no_retry", "UTC": stamp()})
        raise
    (root / f"notification-{kind}-stdout.txt").write_bytes(result.stdout)
    (root / f"notification-{kind}-stderr.txt").write_bytes(result.stderr)
    match = re.search(rb"Queued message ([\w-]+) for thread ([\w-]+)\.", result.stdout)
    accepted = bool(result.returncode == 0 and match and match[2].decode() == binding["thread_id"])
    receipt = {"queue_accepted": accepted, "receiver_verified": False, "exit": result.returncode,
               "queue_id": match[1].decode() if match else None, "UTC": stamp()}
    save(root / f"notification-{kind}-receipt.json", receipt)
    assert accepted, "notification not positively accepted; preserve intent, never resend blindly"
    return receipt


def acknowledge(root):
    binding = json.loads((root / "notification-binding.json").read_text())
    assert os.environ["CODEX_THREAD_ID"] == binding["thread_id"], "different receiving session"
    intent = json.loads((root / "notification-probe-intent.json").read_text())
    receipt = json.loads((root / "notification-probe-receipt.json").read_text())
    assert receipt["queue_accepted"] and intent["notifier_sha256"] == sha(Path(__file__))
    with Path(binding["rollout"]).open("rb") as stream:
        stream.seek(binding["rollout_offset"])
        lines = stream.read().splitlines()
    rows = []
    for line in lines:
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            continue  # Bind offset may have intersected the old turn's append.
    current_turn = binding["source_turn"]
    receiving_turn = None
    for row in rows:
        payload = row.get("payload", {})
        if payload.get("turn_id"):
            current_turn = payload["turn_id"]
        delivered = (row["type"] == "event_msg" and payload.get("type") == "user_message" and payload.get("message") == intent["message"])
        delivered |= (row["type"] == "response_item" and payload.get("role") == "user"
                      and any(v.get("text") == intent["message"] for v in payload.get("content", []) if isinstance(v, dict)))
        if delivered and current_turn != binding["source_turn"]:
            receiving_turn = current_turn
    assert receiving_turn is not None, "canonical notice/new receiving turn absent; queue acceptance alone is insufficient"
    assert current_turn == receiving_turn, "ack must execute in the actual receiving turn"
    proof = {"queue_accepted": True, "receiver_verified": True, "same_thread": True,
             "thread_id": binding["thread_id"], "source_turn": binding["source_turn"],
             "receiving_turn": receiving_turn, "notifier_sha256": sha(Path(__file__)), "UTC": stamp()}
    save(root / "notification-probe-ack.json", proof, exclusive=True)
    return proof


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("bind", "probe", "send", "ack"))
    parser.add_argument("root", type=Path)
    parser.add_argument("--label", choices=("D3", "F", "G"), default="D3")
    args = parser.parse_args()
    functions = {"bind": lambda root: bind(root, args.label), "probe": lambda root: send(root, probe=True), "send": send, "ack": acknowledge}
    print(json.dumps(functions[args.action](args.root), ensure_ascii=False))

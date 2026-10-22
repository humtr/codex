import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import d3_notify as notify


class NotificationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="d3notice.", dir=os.environ["TMPDIR"])
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.binary = self.root / "core"
        self.binary.write_text("owned local fixture, never executed")
        self.thread = "01a15330-0000-7000-8000-000000000000"
        self.rollout = self.root / "rollout.jsonl"
        self.rollout.write_text("")
        notify.save(self.root / "notification-binding.json", {"thread_id": self.thread, "source_turn": "source",
            "rollout": str(self.rollout), "rollout_offset": 0, "core": str(self.binary),
            "core_sha256": notify.sha(self.binary), "environment": {"CODEX_HOME": "original-supervisor"}})

    def accepted(self, probe=True):
        response = subprocess.CompletedProcess([], 0, f"Queued message owned-id for thread {self.thread}.\n".encode(), b"")
        with patch.object(notify.subprocess, "run", return_value=response) as call:
            receipt = notify.send(self.root, probe=probe)
        self.assertTrue(receipt["queue_accepted"])
        args = call.call_args
        self.assertEqual(args.args[0][:4], [str(self.binary), "queue", "--thread", self.thread])
        self.assertEqual(args.kwargs["environment"] if "environment" in args.kwargs else args.kwargs["env"], {"CODEX_HOME": "original-supervisor"})
        self.assertNotIn("--model", args.args[0])
        return receipt

    def test_once_and_no_receiver_claim_from_enqueue(self):
        self.assertFalse(self.accepted()["receiver_verified"])
        with patch.object(notify.subprocess, "run") as call:
            with self.assertRaises(FileExistsError):
                notify.send(self.root, probe=True)
            call.assert_not_called()

    def test_failure_result_has_separate_notice_and_no_retry(self):
        notify.save(self.root / "result.json", {"automated_gates": False})
        self.accepted(probe=False)
        text = json.loads((self.root / "notification-result-intent.json").read_text())["message"]
        self.assertIn("실패/중단", text)
        self.assertIn("재전송", text)

    def test_f_notice_reuses_exact_receiver_and_does_not_claim_whole_phase_completion(self):
        binding=json.loads((self.root / "notification-binding.json").read_text())
        binding["label"]="F"
        notify.save(self.root / "notification-binding.json",binding)
        self.accepted()
        notice=json.loads((self.root / "notification-probe-intent.json").read_text())
        self.assertIn("[F_NOTIFY_PROBE",notice["message"])
        self.assertIn("준비된 F 배치",notice["message"])
        self.assertNotIn("실제 D3",notice["message"])

    def test_terminal_notice_preserves_whole_phase_authorization_without_replaying_unknown_calls(self):
        binding=json.loads((self.root / "notification-binding.json").read_text())
        binding["label"]="F"
        notify.save(self.root / "notification-binding.json",binding)
        notify.save(self.root / "result.json", {"automated_gates": False})
        self.accepted(probe=False)
        message=json.loads((self.root / "notification-result-intent.json").read_text())["message"]
        self.assertIn("재전송하지 마세요",message)
        self.assertIn("이미 승인한 전체 단계",message)
        self.assertIn("다음 검증을 계속",message)
        self.assertNotIn("시험을 새로 시작하지 마세요",message)

    def test_timeout_preserves_intent(self):
        with patch.object(notify.subprocess, "run", side_effect=subprocess.TimeoutExpired("owned", 60)):
            with self.assertRaises(subprocess.TimeoutExpired):
                notify.send(self.root, probe=True)
        self.assertTrue((self.root / "notification-probe-unknown.json").exists())
        self.assertTrue((self.root / "notification-probe-intent.json").exists())

    def test_real_receiving_turn_required(self):
        self.accepted()
        with patch.dict(os.environ, CODEX_THREAD_ID=self.thread):
            with self.assertRaises(AssertionError):
                notify.acknowledge(self.root)
            intent = json.loads((self.root / "notification-probe-intent.json").read_text())
            records = [{"type": "event_msg", "payload": {"type": "task_started", "turn_id": "received"}},
                       {"type": "response_item", "payload": {"role": "user", "content": [{"type": "input_text", "text": intent["message"]}]}},
                       {"type": "token_usage_record", "payload": {"turn_id": "received"}}]
            self.rollout.write_text("".join(json.dumps(r) + "\n" for r in records))
            proof = notify.acknowledge(self.root)
            self.assertTrue(proof["receiver_verified"])
            self.assertEqual(proof["receiving_turn"], "received")

    def test_other_session_cannot_ack(self):
        self.accepted()
        with patch.dict(os.environ, CODEX_THREAD_ID="other"):
            with self.assertRaises(AssertionError):
                notify.acknowledge(self.root)


if __name__ == "__main__":
    unittest.main()

"""Exercise the real entrypoint with local stand-ins; never contact Tailscale."""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time
import unittest

STUB = '''#!/usr/bin/env python3
import json, os, pathlib, signal, sys, time
root = pathlib.Path(os.environ["BOOKWORM_BOOT_TEST_DIR"])
name = pathlib.Path(sys.argv[0]).name
config = json.loads((root / "config.json").read_text())
def record(event):
    with (root / "events.jsonl").open("a") as f:
        f.write(json.dumps({"event": event, "pid": os.getpid()}) + "\\n")
def wait():
    while True: time.sleep(0.02)
if name == "sleep":
    time.sleep(0.02)
elif name == "tailscaled":
    record("daemon.started")
    wait()
elif name == "bookworm":
    record("app.started")
    if "app_exit" in config: sys.exit(config["app_exit"])
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(23))
    (root / "app.ready").touch()
    wait()
else:
    command = sys.argv[1]
    counter = root / (command + ".count")
    count = int(counter.read_text()) + 1 if counter.exists() else 1
    counter.write_text(str(count))
    record(command + ".attempt")
    if command == "up" and config.get("blocked_login"):
        wait()
    failures = config.get(command + "_failures", 0)
    if failures == -1 or count <= failures:
        record(command + ".failed")
        sys.exit(1)
    record(command + ".succeeded")
'''


class StartupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.process = None
        shutil.copy(Path(__file__).resolve().parents[1] / "start.sh", self.root / "start.sh")
        for name in ["bookworm", "tailscale", "tailscaled", "sleep"]:
            path = self.root / name
            path.write_text(STUB)
            path.chmod(0o755)

    def tearDown(self):
        if self.process:
            try:
                os.killpg(self.process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            self.process.wait(timeout=3)
        self.temp.cleanup()

    def start(self, **config):
        (self.root / "config.json").write_text(json.dumps(config))
        env = dict(os.environ, BOOKWORM_BOOT_TEST_DIR=str(self.root),
                   TS_AUTHKEY="test-key", PATH=f"{self.root}:{os.environ['PATH']}")
        self.process = subprocess.Popen(["sh", str(self.root / "start.sh")], env=env,
                                        start_new_session=True, stdout=subprocess.DEVNULL,
                                        stderr=subprocess.DEVNULL)

    def events(self):
        path = self.root / "events.jsonl"
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def await_condition(self, condition):
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if condition():
                return
            time.sleep(0.01)
        self.fail(f"Startup condition not met: exit={self.process.poll()}, events={self.events()}")

    def count(self, event):
        return sum(row["event"] == event for row in self.events())

    def test_blocked_login_does_not_block_application_or_sigterm(self):
        self.start(blocked_login=True)
        self.await_condition(lambda: (self.root / "app.ready").exists() and self.count("up.attempt"))
        self.assertEqual(self.count("serve.attempt"), 0)
        app = next(row for row in self.events() if row["event"] == "app.started")
        self.assertEqual(app["pid"], self.process.pid)
        self.process.terminate()
        self.assertEqual(self.process.wait(timeout=3), 23)

    def test_rejected_login_retries_while_application_stays_running(self):
        self.start(up_failures=-1)
        self.await_condition(lambda: self.count("up.failed") >= 3)
        self.assertEqual(self.count("app.started"), 1)
        self.assertEqual(self.count("serve.attempt"), 0)
        self.assertIsNone(self.process.poll())

    def test_login_and_serve_recover_without_restarting_application(self):
        self.start(up_failures=2, serve_failures=1)
        self.await_condition(lambda: self.count("serve.succeeded") == 1)
        self.assertEqual(self.count("app.started"), 1)
        events = [row["event"] for row in self.events()]
        self.assertLess(events.index("up.succeeded"), events.index("serve.attempt"))
        self.assertEqual(self.count("up.attempt"), 3)
        self.assertEqual(self.count("serve.attempt"), 2)
        self.assertIsNone(self.process.poll())

    def test_application_exit_status_is_preserved(self):
        self.start(app_exit=42, blocked_login=True)
        self.assertEqual(self.process.wait(timeout=3), 42)
        self.assertEqual(self.count("app.started"), 1)


if __name__ == "__main__":
    unittest.main()

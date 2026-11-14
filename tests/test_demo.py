"""Exercise every catalog example through the actual relay and mock HTTP server."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time
import unittest
from unittest import mock
from http.server import ThreadingHTTPServer
from urllib.error import HTTPError
from urllib.request import Request

import dev
from dev import LOCAL_HTTP, MockHandler
from tools.demo_catalog import load_cases, response_bytes

ROOT = Path(__file__).resolve().parents[1]
BRIDGE = ROOT / "target" / "debug" / ("jev-bridge.exe" if os.name == "nt" else "jev-bridge")


class DemoRelayTests(unittest.TestCase):
    @unittest.skipUnless(BRIDGE.is_file(), "Build jev-bridge before the HTTP integration check")
    def test_all_examples_preserve_http_bytes_and_inspection_captures(self):
        mock = ThreadingHTTPServer(("127.0.0.1", 0), MockHandler)
        mock.daemon_threads = True
        threading.Thread(target=mock.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as folder:
                connection_path = Path(folder) / "connection.json"
                with (Path(folder) / "bridge.log").open("w") as log:
                    process = subprocess.Popen([
                        str(BRIDGE), "--upstream", f"http://127.0.0.1:{mock.server_port}",
                        "--listen", "127.0.0.1:0", "--inspector", "127.0.0.1:0",
                        "--capture-bodies", "--connection-file", str(connection_path),
                    ], stdout=log, stderr=log, creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
                    try:
                        for _ in range(200):
                            if connection_path.is_file():
                                break
                            self.assertIsNone(process.poll(), "relay exited during startup")
                            time.sleep(0.025)
                        connection = json.loads(connection_path.read_text())

                        def inspect(path):
                            with LOCAL_HTTP.open(Request(connection["inspector_url"] + path,
                                    headers={"Authorization": "Bearer " + connection["token"]}), timeout=3) as response:
                                return json.load(response)

                        snapshot = inspect("/api/requests")
                        for case in load_cases(include_qa=True):
                            with self.subTest(example=case["id"]):
                                payload = None if case.get("request") is None else json.dumps(case["request"], ensure_ascii=False).encode()
                                request = Request(snapshot["relay_url"] + case["path"], data=payload, method=case["method"], headers={"Content-Type": "application/json"})
                                try:
                                    response = LOCAL_HTTP.open(request, timeout=5)
                                except HTTPError as error:
                                    response = error
                                with response:
                                    self.assertEqual(response.status, case["status"])
                                    self.assertEqual(response.read(), response_bytes(case))
                                for _ in range(100):
                                    snapshot = inspect("/api/requests")
                                    summary = next((r for r in snapshot["requests"] if r.get("request_id") == "demo-" + case["id"] and r["outcome"] == "complete"), None)
                                    if summary:
                                        break
                                    time.sleep(0.01)
                                self.assertIsNotNone(summary, "completed capture not published")
                                # Forwarding completes before the background JSON inspection worker.
                                for _ in range(200):
                                    detail = inspect(f"/api/requests/{summary['id']}")
                                    if all("Inspection is processing." not in detail[side]["notes"]
                                           for side in ("request", "response")):
                                        break
                                    time.sleep(0.01)
                                self.assertEqual(detail["request"]["json"], case.get("request"), detail["request"]["notes"])
                                self.assertEqual(detail["response"]["json"], case.get("response"))
                                self.assertEqual(detail["response"]["wire_bytes"], len(response_bytes(case)))
                                self.assertTrue(detail["response"]["complete"])
                                expected_usage = (case.get("response") or {}).get("usage")
                                self.assertEqual(detail["summary"].get("usage"), expected_usage)
                                current = inspect("/api/requests")
                                listed = next(r for r in current["requests"] if r["id"] == summary["id"])
                                self.assertEqual(listed.get("usage"), expected_usage,
                                                 "cost totals must work without fetching each detail")
                        self.assertEqual(snapshot["total_requests"], 14)
                    finally:
                        process.terminate()
                        process.wait(timeout=5)
        finally:
            mock.shutdown()
            mock.server_close()


class StartupCheckTests(unittest.TestCase):
    def test_old_python_stops_with_a_clear_message(self):
        with self.assertRaises(SystemExit) as stop:
            dev.require_python((3, 11, 3))
        self.assertIn("Python 3.11.4 or newer; this is 3.11.3", str(stop.exception))
        dev.require_python((3, 11, 4))

    def test_cargo_falls_back_to_rustup_home(self):
        with tempfile.TemporaryDirectory() as home, mock.patch("dev.shutil.which", return_value=None), \
                mock.patch("builtins.print"):
            cargo = Path(home) / ".cargo" / "bin" / ("cargo.exe" if os.name == "nt" else "cargo")
            with self.assertRaises(SystemExit) as stop:
                dev.find_cargo(home)
            self.assertIn("rustup", str(stop.exception))
            cargo.parent.mkdir(parents=True)
            cargo.touch()
            self.assertEqual(dev.find_cargo(home), str(cargo))

    def test_cargo_on_path_wins(self):
        with mock.patch("dev.shutil.which", return_value="/opt/cargo"):
            self.assertEqual(dev.find_cargo("/nonexistent"), "/opt/cargo")

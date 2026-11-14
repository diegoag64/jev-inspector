"""Check the terminal request sender: HTTP behavior, key handling, layout, and a real curses run."""
import curses
import json
import os
from pathlib import Path
import select
import sys
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from tools.request_sender import (CASES, CASES_BY_ID, MIN_H, MIN_W, SHOWCASE_COUNT, SenderApp, mask_key, meter,
                                  relay_listening, send_case, sparkline)

ROOT = Path(__file__).resolve().parents[1]


class EchoRelay(BaseHTTPRequestHandler):
    seen = []

    def log_message(self, *args):
        pass

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        EchoRelay.seen.append((self.path, json.loads(body), self.headers.get("Authorization")))
        self.send_response(429)
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"{}")


class GridApp(SenderApp):
    def put(self, y, x, text, style=""):
        for i, char in enumerate(text):
            if 0 <= y < self.height and 0 <= x + i < self.width:
                self.cells[y][x + i] = char
        return x + len(text)

    def render(self, width, height):
        self.cells = [[" "] * width for _ in range(height)]
        self.draw(width, height)
        return ["".join(row) for row in self.cells]


class RequestTests(unittest.TestCase):
    def setUp(self):
        self.relay = ThreadingHTTPServer(("127.0.0.1", 0), EchoRelay)
        threading.Thread(target=self.relay.serve_forever, daemon=True).start()

    def tearDown(self):
        self.relay.shutdown()
        self.relay.server_close()

    def test_sends_catalog_body_with_model_and_key(self):
        port = self.relay.server_port
        result = send_case(CASES_BY_ID["support-routing"], port)
        self.assertEqual((result["ok"], result["status"], result["bytes"]), (True, 429, 2))
        expected = {**CASES_BY_ID["support-routing"]["request"], "model": "jev-latest"}
        self.assertEqual(EchoRelay.seen[-1], ("/v1/systemone", expected, "Bearer synthetic-demo-key"))
        send_case(CASES_BY_ID["fraud-check"], port, model="custom", api_key="sk-test")
        self.assertEqual(EchoRelay.seen[-1][1]["model"], "custom")
        self.assertEqual(EchoRelay.seen[-1][2], "Bearer sk-test")

    def test_reports_missing_relay(self):
        port = self.relay.server_port
        self.tearDown()
        self.assertFalse(relay_listening(port))
        result = send_case(CASES_BY_ID["fraud-check"], port, timeout=1)
        self.assertEqual((result["ok"], result["error"]), (False, f"no relay on :{port}"))
        self.setUp()
        self.assertTrue(relay_listening(self.relay.server_port))


class AppTests(unittest.TestCase):
    def test_helpers(self):
        self.assertEqual(mask_key(""), "synthetic")
        self.assertEqual(mask_key("sk-demo-1234abcd"), "••••abcd")
        self.assertEqual(sparkline([0, 8], 10), "▁█")
        self.assertEqual(meter(50, 100, 4), "██  ")

    def test_prompts_validate_and_apply(self):
        app = SenderApp()
        for keys, check in [
            (["p", "\x15", "9", "x", "9", "9", "\n"], lambda: app.port == 999),
            (["p", "\x15", "7", "0", "0", "0", "0", "\n"], lambda: app.port == 999),
            (["m", "\x15", "\n"], lambda: app.model == "jev-latest"),
            (["k", *"sk-1", "\x7f", "2", "\n"], lambda: app.api_key == "sk-2"),
            (["k", "z", "\x1b"], lambda: app.api_key == "sk-2" and app.prompt is None),
        ]:
            for key in keys:
                self.assertTrue(app.handle_key(key))
            self.assertTrue(check(), keys)
        self.assertFalse(app.handle_key("q"))

    def test_navigation_wraps_and_quick_send_selects(self):
        app = SenderApp()
        app.send = lambda index: None
        app.handle_key(curses.KEY_UP)
        self.assertEqual(app.selected, len(CASES) - 1)
        app.handle_key("3")
        self.assertEqual(app.selected, 2)

    def test_layout_fits_minimum_and_scrolls_to_selection(self):
        app = GridApp()
        screen = app.render(MIN_W, MIN_H)
        self.assertIn("SHOWCASE", "\n".join(screen))
        app.selected = len(CASES) - 1
        screen = "\n".join(app.render(MIN_W, MIN_H))
        self.assertIn(CASES[-1]["title"][:12], screen)
        self.assertIn("Make the terminal", "\n".join(app.render(MIN_W - 1, MIN_H)))

    def test_results_update_counts_and_log(self):
        app = SenderApp()
        app.inflight = {"a": 1, "b": 1}
        app.results.put({"ok": True, "id": "a", "status": 503, "ms": 40, "title": "A", "at": "t"})
        app.results.put({"ok": False, "id": "b", "error": "no relay on :1", "ms": 1, "title": "B", "at": "t"})
        app.drain()
        self.assertEqual(app.counts, {"sent": 2, "ok": 0, "warn": 0, "err": 1, "fail": 1})
        self.assertEqual([e["id"] for e in app.log], ["b", "a"])
        self.assertFalse(app.relay_up)
        self.assertEqual(SHOWCASE_COUNT, sum(1 for c in CASES if not c.get("qa_only")))


@unittest.skipUnless(os.name == "posix", "Needs a pseudo-terminal")
class CursesSmokeTest(unittest.TestCase):
    def test_runs_in_a_terminal_sends_and_quits(self):
        import pty

        relay = ThreadingHTTPServer(("127.0.0.1", 0), EchoRelay)
        threading.Thread(target=relay.serve_forever, daemon=True).start()
        before = len(EchoRelay.seen)
        pid, fd = pty.fork()
        if pid == 0:
            os.environ.update(TERM="xterm-256color", LINES="30", COLUMNS="110")
            os.execv(sys.executable, [sys.executable, str(ROOT / "tools" / "request_sender.py"),
                                      "--port", str(relay.server_port)])
        output = b""

        def pump(seconds):
            nonlocal output
            end = time.monotonic() + seconds
            while time.monotonic() < end:
                if select.select([fd], [], [], 0.05)[0]:
                    try:
                        output += os.read(fd, 65536)
                    except OSError:
                        return

        try:
            pump(1.0)
            os.write(fd, b"1")
            pump(1.0)
            os.write(fd, b"q")
            pump(1.0)
            _, status = os.waitpid(pid, 0)
        finally:
            relay.shutdown()
            relay.server_close()
            os.close(fd)
        self.assertEqual(os.waitstatus_to_exitcode(status) if hasattr(os, "waitstatus_to_exitcode") else status >> 8, 0,
                         output[-2000:])
        self.assertIn(b"request sender", output)
        self.assertEqual(len(EchoRelay.seen), before + 1)
        self.assertNotIn(b"Traceback", output)


if __name__ == "__main__":
    unittest.main()

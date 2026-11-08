"""Run a local, synthetic Jev demo through the real Rust relay. No API key needed."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import HTTPError
from urllib.request import ProxyHandler, Request, build_opener
from tools.demo_catalog import load_cases, response_bytes

ROOT = Path(__file__).resolve().parent
LOCAL_HTTP = build_opener(ProxyHandler({}))
DEMO_CASES = load_cases()
DEMO_BY_ID = {case["id"]: case for case in load_cases(include_qa=True)}


class MockHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def handle(self):
        try:
            super().handle()
        except (BrokenPipeError, ConnectionResetError):
            pass  # The relay may close pooled HTTP connections on shutdown.

    def log_message(self, *args):
        pass

    def read_body(self):
        if self.headers.get("Transfer-Encoding", "").lower() == "chunked":
            chunks, size = [], 0
            while True:
                line = self.rfile.readline(1024)
                count = int(line.split(b";", 1)[0].strip(), 16)
                if count == 0:
                    while self.rfile.readline(8192) not in (b"\r\n", b"\n", b""):
                        pass
                    return b"".join(chunks)
                size += count
                if size > 1024 * 1024:
                    raise ValueError("Demo body too large")
                chunks.append(self.rfile.read(count))
                if self.rfile.read(2) != b"\r\n":
                    raise ValueError("Invalid chunk framing")
        count = int(self.headers.get("Content-Length", "0"))
        if count > 1024 * 1024:
            raise ValueError("Demo body too large")
        return self.rfile.read(count)

    def reply(self, status, body, request_id="demo-models"):
        data = (json.dumps(body, indent=2) + "\n").encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("x-typesafe-request-id", request_id)
        try:
            self.end_headers()
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_GET(self):
        if self.path != "/v1/models":
            self.reply(404, {"error": {"message": "Unknown demo endpoint"}})
            return
        self.reply_case(DEMO_BY_ID["models"])

    def reply_case(self, case):
        time.sleep(case["delay_ms"] / 1000)
        data = response_bytes(case)
        self.send_response(case["status"])
        if data:
            self.send_header("Content-Type", case.get("content_type", "application/json"))
        self.send_header("Content-Length", str(len(data)))
        self.send_header("x-typesafe-request-id", f"demo-{case['id']}")
        try:
            self.end_headers()
            self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_POST(self):
        try:
            payload = json.loads(self.read_body())
        except (ValueError, OSError):
            self.reply(400, {"error": {"message": "Invalid demo JSON"}})
            return
        if self.path != "/v1/systemone":
            self.reply(404, {"error": {"message": "Unknown demo endpoint"}})
            return
        state = payload.get("state", {}) if isinstance(payload, dict) else {}
        slug = state.get("demo_case", "support-routing") if isinstance(state, dict) else "support-routing"
        case = DEMO_BY_ID.get(slug)
        if case is None or case["method"] != "POST":
            self.reply(400, {"error": {"message": "Unknown synthetic example"}})
            return
        self.reply_case(case)


def run_demo(args):
    suffix = ".exe" if os.name == "nt" else ""
    binary_dir = ROOT / "target" / args.profile
    bridge = binary_dir / ("jev-bridge" + suffix)
    inspector = binary_dir / ("jev-inspector" + suffix)
    if not args.skip_build:
        from tools.prepare_rune import prepare
        prepare()
        command = ["cargo", "build", "--locked", "--workspace"]
        if args.profile != "debug":
            command += ["--profile", args.profile]
        subprocess.run(command, cwd=ROOT, check=True)
    if not bridge.exists() or (not args.no_ui and not inspector.exists()):
        raise SystemExit("Build the binaries first with cargo build --workspace --locked.")

    output = ROOT / "artifacts" / "demo"
    output.mkdir(parents=True, exist_ok=True)
    connection_file = ROOT / ".jev" / "demo-connection.json"
    mock = ThreadingHTTPServer(("127.0.0.1", 0), MockHandler)
    mock.daemon_threads = True
    threading.Thread(target=mock.serve_forever, daemon=True).start()
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    children = []
    stopped = threading.Event()
    connection = None
    try:
        with (output / "bridge.log").open("w", encoding="utf-8") as relay_log, (output / "inspector.log").open("w", encoding="utf-8") as ui_log:
            relay = subprocess.Popen([str(bridge), "--upstream", f"http://127.0.0.1:{mock.server_port}",
                "--listen", f"127.0.0.1:{args.port}", "--inspector", f"127.0.0.1:{args.inspector_port}",
                "--capture-bodies", "--connection-file", str(connection_file)],
                cwd=ROOT, stdout=relay_log, stderr=subprocess.STDOUT, creationflags=flags)
            children.append(relay)
            deadline = time.monotonic() + 10
            connection = None
            while time.monotonic() < deadline:
                if relay.poll() is not None:
                    raise RuntimeError(f"Relay stopped. See {output / 'bridge.log'}; ports may already be in use.")
                try:
                    connection = json.loads(connection_file.read_text())
                    request = Request(connection["inspector_url"] + "/api/requests", headers={"Authorization": "Bearer " + connection["token"]})
                    with LOCAL_HTTP.open(request, timeout=0.5) as response:
                        snapshot = json.load(response)
                    if snapshot["upstream"] == f"http://127.0.0.1:{mock.server_port}/":
                        break
                except (OSError, ValueError):
                    pass
                time.sleep(0.05)
            else:
                raise RuntimeError("Relay did not become ready within 10 seconds.")

            def traffic():
                index = 0
                while not stopped.is_set():
                    case = DEMO_CASES[index % len(DEMO_CASES)]
                    data = None if case.get("request") is None else json.dumps(case["request"], ensure_ascii=False).encode("utf-8")
                    request = Request(f"http://127.0.0.1:{args.port}{case['path']}", data=data, method=case["method"],
                        headers={"Content-Type": "application/json", "Authorization": "Bearer synthetic-demo-key"})
                    try:
                        with LOCAL_HTTP.open(request, timeout=5) as response:
                            response.read()
                    except HTTPError as error:
                        error.read()
                    except OSError:
                        stopped.set()
                    index += 1
                    if index == len(DEMO_CASES):
                        print(f"Seeded {index} varied examples. " + ("Continuing synthetic traffic." if args.continuous else "Traffic is idle; browse requests at your own pace."), flush=True)
                    if index >= len(DEMO_CASES) and not args.continuous:
                        return
                    stopped.wait(0.12 if index < len(DEMO_CASES) else 1.8)

            threading.Thread(target=traffic, daemon=True).start()
            ui = None
            if not args.no_ui:
                # A hidden Windows launcher can pass SW_HIDE to its children.
                # Keep helper consoles hidden, but explicitly show the GUI.
                ui_startup = None
                if os.name == "nt":
                    ui_startup = subprocess.STARTUPINFO()
                    ui_startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
                    ui_startup.wShowWindow = 1  # Windows SW_SHOWNORMAL
                ui_command = [str(inspector), "--connection-file", str(connection_file)]
                if args.diagram:
                    ui_command.append("--diagram")
                ui = subprocess.Popen(ui_command,
                    cwd=ROOT, stdout=ui_log, stderr=subprocess.STDOUT, creationflags=flags,
                    startupinfo=ui_startup)
                children.append(ui)
            print(f"Synthetic demo running through the Rust relay at http://127.0.0.1:{args.port}", flush=True)
            print("No TypeSafe calls or real API keys. Close the inspector or press Ctrl+C to stop the demo.", flush=True)
            started = time.monotonic()
            while not stopped.wait(0.2):
                if relay.poll() is not None:
                    raise RuntimeError("Relay stopped unexpectedly; see artifacts/demo/bridge.log.")
                if ui is not None and ui.poll() is not None:
                    if ui.returncode:
                        raise RuntimeError("Inspector stopped with an error; see artifacts/demo/inspector.log.")
                    break
                if args.duration and time.monotonic() - started >= args.duration:
                    break
    finally:
        stopped.set()
        for process in reversed(children):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        mock.shutdown()
        mock.server_close()
        # Leave the file only if another process replaced it.
        try:
            current = json.loads(connection_file.read_text())
            if connection and current.get("token") == connection.get("token"):
                connection_file.unlink()
        except (OSError, ValueError):
            pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["demo"])
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--profile", choices=["debug", "perf"], default="debug")
    parser.add_argument("--port", type=int, default=8787)
    parser.add_argument("--inspector-port", type=int, default=8788)
    parser.add_argument("--no-ui", action="store_true", help="Run the mock and relay without opening a window")
    parser.add_argument("--diagram", action="store_true", help="Open the inspector on the Diagram tab")
    parser.add_argument("--continuous", action="store_true", help="Keep sending examples after the initial varied batch")
    parser.add_argument("--duration", type=float, default=0, help="Stop the demo after this many seconds; 0 waits for the window to close")
    args = parser.parse_args()
    try:
        run_demo(args)
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()

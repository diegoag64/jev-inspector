"""Send synthetic demo requests to a Jev relay from a terminal UI (development only).

Start the demo relay first (``python3 dev.py demo``), then run::

    python3 tools/request_sender.py

Each example from ``examples/demo-requests.json`` is sent to ``127.0.0.1`` on the
relay port (8787 by default) with the request ``model`` set to ``jev-latest``.
Set ``JEV_API_KEY`` or press ``k`` to send a real API key; it stays in memory only.
"""
from __future__ import annotations

import argparse
import curses
import json
import locale
import os
from pathlib import Path
import queue
import random
import socket
import sys
import threading
import time
from urllib.error import HTTPError, URLError
from urllib.request import ProxyHandler, Request, build_opener

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tools.demo_catalog import load_cases  # noqa: E402

DEFAULT_PORT = 8787
DEFAULT_MODEL = "jev-latest"
SYNTHETIC_KEY = "synthetic-demo-key"
LOCAL_HTTP = build_opener(ProxyHandler({}))
_ALL = load_cases(include_qa=True)
CASES = [case for case in _ALL if not case.get("qa_only")] + [case for case in _ALL if case.get("qa_only")]
CASES_BY_ID = {case["id"]: case for case in CASES}
SHOWCASE_COUNT = sum(1 for case in CASES if not case.get("qa_only"))

SPINNER = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
SPARK = "▁▂▃▄▅▆▇█"
EIGHTHS = " ▏▎▍▌▋▊▉█"
GRADIENT = [205, 206, 170, 171, 135, 141, 99]  # Pink to violet, like the inspector's icon.
MIN_W, MIN_H = 76, 18


# ── Requests ──────────────────────────────────────────────────────────────────

def send_case(case: dict, port: int, model: str = DEFAULT_MODEL, api_key: str = "", timeout: float = 15.0) -> dict:
    """Send one catalog case to the relay on 127.0.0.1:port and summarize the outcome."""
    body = case.get("request")
    if isinstance(body, dict) and model:
        body = {**body, "model": model}
    data = None if body is None else json.dumps(body, ensure_ascii=False).encode("utf-8")
    request = Request(
        f"http://127.0.0.1:{port}{case['path']}", data=data, method=case["method"],
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {api_key or SYNTHETIC_KEY}"},
    )
    started = time.perf_counter()
    try:
        with LOCAL_HTTP.open(request, timeout=timeout) as response:
            size = len(response.read())
            status = response.status
    except HTTPError as error:
        size = len(error.read())
        status = error.code
    except (URLError, OSError) as error:
        reason = getattr(error, "reason", error)
        refused = isinstance(reason, ConnectionRefusedError) or "refused" in str(reason).lower()
        message = f"no relay on :{port}" if refused else f"relay :{port} failed: {reason}"
        return {"ok": False, "id": case["id"], "error": message,
                "ms": round((time.perf_counter() - started) * 1000)}
    return {"ok": True, "id": case["id"], "status": status, "bytes": size,
            "ms": round((time.perf_counter() - started) * 1000)}


def relay_listening(port: int, timeout: float = 0.3) -> bool:
    """Check for a listener with a bare TCP connect, so the probe never shows up as traffic."""
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=timeout):
            return True
    except OSError:
        return False


# ── Formatting helpers ────────────────────────────────────────────────────────

def mask_key(key: str) -> str:
    if not key:
        return "synthetic"
    return "•" * min(len(key), 8) if len(key) <= 8 else "••••" + key[-4:]


def sparkline(values: list[int], width: int) -> str:
    values = values[-width:]
    if not values:
        return ""
    top = max(values) or 1
    return "".join(SPARK[min(len(SPARK) - 1, round(v / top * (len(SPARK) - 1)))] for v in values)


def meter(value: float, top: float, width: int) -> str:
    """A horizontal bar with eighth-cell precision."""
    eighths = round(max(0.0, min(1.0, value / top if top else 0)) * width * 8)
    full, part = divmod(eighths, 8)
    return ("█" * full + (EIGHTHS[part] if part else "")).ljust(width)


def status_style(status: int | None) -> str:
    if status is None:
        return "err"
    return {2: "ok", 4: "warn"}.get(status // 100, "err")


def fit(text: str, width: int) -> str:
    if width <= 0:
        return ""
    return text if len(text) <= width else text[: max(0, width - 1)] + "…"


# ── App ───────────────────────────────────────────────────────────────────────

class SenderApp:
    """State and drawing. ``put`` is the only output primitive, so tests can render to a grid."""

    def __init__(self, port: int = DEFAULT_PORT, model: str = DEFAULT_MODEL, api_key: str = ""):
        self.port, self.model, self.api_key = port, model, api_key
        self.selected = 0
        self.scroll = 0
        self.log: list[dict] = []
        self.latencies: list[int] = []
        self.counts = {"sent": 0, "ok": 0, "warn": 0, "err": 0, "fail": 0}
        self.inflight: dict[str, int] = {}
        self.results: queue.Queue = queue.Queue()
        self.lock = threading.Lock()
        self.relay_up: bool | None = None
        self.prompt: dict | None = None
        self.notice = ("", "muted", 0.0)
        self.rows: dict[int, int] = {}
        self.frame = 0
        self.width = self.height = 0
        self.split = 0
        self.stopped = threading.Event()

    # Output primitive; replaced by the curses front end.
    def put(self, y: int, x: int, text: str, style: str = "") -> int:
        raise NotImplementedError

    # ── actions ──
    def send(self, index: int) -> None:
        case = CASES[index]
        with self.lock:
            self.inflight[case["id"]] = self.inflight.get(case["id"], 0) + 1
        port, model, key = self.port, self.model, self.api_key

        def work():
            result = send_case(case, port, model, key)
            result.update(title=case["title"], at=time.strftime("%H:%M:%S"))
            self.results.put(result)

        threading.Thread(target=work, daemon=True).start()

    def send_all(self) -> None:
        def work():
            for index in range(SHOWCASE_COUNT):
                if self.stopped.is_set():
                    return
                self.send(index)
                time.sleep(0.12)

        threading.Thread(target=work, daemon=True).start()
        self.say(f"Sending {SHOWCASE_COUNT} showcase requests", "accent")

    def say(self, text: str, style: str = "muted", seconds: float = 2.5) -> None:
        self.notice = (text, style, time.monotonic() + seconds)

    def drain(self) -> None:
        while True:
            try:
                result = self.results.get_nowait()
            except queue.Empty:
                return
            with self.lock:
                self.inflight[result["id"]] -= 1
            result["born"] = time.monotonic()
            self.log.insert(0, result)
            del self.log[200:]
            self.counts["sent"] += 1
            if result["ok"]:
                self.relay_up = True
                self.latencies.append(result["ms"])
                del self.latencies[:-60]
                self.counts[status_style(result["status"])] += 1
            else:
                self.relay_up = False
                self.counts["fail"] += 1

    def probe(self) -> None:
        while not self.stopped.is_set():
            self.relay_up = relay_listening(self.port)
            self.stopped.wait(1.5)

    def open_prompt(self, kind: str) -> None:
        label, value, secret = {
            "port": ("Relay port", str(self.port), False),
            "model": ("Model", self.model, False),
            "key": ("API key", self.api_key, True),
        }[kind]
        self.prompt = {"kind": kind, "label": label, "value": value, "secret": secret}

    def commit_prompt(self) -> None:
        prompt, self.prompt = self.prompt, None
        value = prompt["value"].strip()
        if prompt["kind"] == "port":
            if value.isdigit() and 1 <= int(value) <= 65535:
                self.port, self.relay_up = int(value), None
                self.say(f"Sending to 127.0.0.1:{self.port}", "ok")
            else:
                self.say("Port must be a number from 1 to 65535", "err")
        elif prompt["kind"] == "model":
            self.model = value or DEFAULT_MODEL
            self.say(f"Model set to {self.model}", "ok")
        else:
            self.api_key = value
            self.say("API key set for this session" if value else "Using the synthetic demo key", "ok")

    def handle_key(self, key) -> bool:
        """Apply one key press. Returns False to quit."""
        if self.prompt is not None:
            if key in ("\n", "\r", curses.KEY_ENTER):
                self.commit_prompt()
            elif key == "\x1b":
                self.prompt = None
            elif key in (curses.KEY_BACKSPACE, "\x7f", "\b"):
                self.prompt["value"] = self.prompt["value"][:-1]
            elif key == "\x15":  # Ctrl+U clears the field.
                self.prompt["value"] = ""
            elif isinstance(key, str) and key.isprintable() and len(self.prompt["value"]) < 200:
                if self.prompt["kind"] != "port" or key.isdigit():
                    self.prompt["value"] += key
            return True
        if key in ("q", "Q", "\x1b"):
            return False
        if key == curses.KEY_UP:
            self.selected = (self.selected - 1) % len(CASES)
        elif key == curses.KEY_DOWN:
            self.selected = (self.selected + 1) % len(CASES)
        elif key in ("\n", "\r", " ", curses.KEY_ENTER):
            self.send(self.selected)
        elif isinstance(key, str) and key in "123456789" and int(key) <= len(CASES):
            self.selected = int(key) - 1
            self.send(self.selected)
        elif key == "a":
            self.send_all()
        elif key == "r":
            self.selected = random.randrange(len(CASES))
            self.send(self.selected)
        elif key == "c":
            self.log.clear()
            self.latencies.clear()
            self.counts = dict.fromkeys(self.counts, 0)
        elif key in ("p", "m", "k"):
            self.open_prompt({"p": "port", "m": "model", "k": "key"}[key])
        return True

    def click(self, y: int, x: int) -> None:
        if y in self.rows and x < self.split:
            self.selected = self.rows[y]
            self.send(self.selected)

    # ── drawing ──
    def text(self, y: int, x: int, parts: list[tuple[str, str]]) -> int:
        for text, style in parts:
            x = self.put(y, x, text, style)
        return x

    def gradient(self, y: int, x: int, text: str, extra: str = "") -> int:
        span = max(1, len(text) - 1)
        for i, char in enumerate(text):
            self.put(y, x + i, char, f"g{round(i / span * (len(GRADIENT) - 1))} {extra}".strip())
        return x + len(text)

    def draw(self, width: int, height: int) -> None:
        self.width, self.height, self.rows = width, height, {}
        if width < MIN_W or height < MIN_H:
            message = f"Make the terminal at least {MIN_W}×{MIN_H}"
            self.put(height // 2, max(0, (width - len(message)) // 2), fit(message, width), "muted")
            return
        self.split = max(46, int(width * 0.45))
        self.draw_header(width)
        self.draw_cases(5, height - 3)
        for y in range(5, height - 3):
            self.put(y, self.split, "│", "faint")
        self.draw_activity(self.split + 2, 5, width - self.split - 4, height - 3)
        self.draw_footer(width, height)

    def draw_header(self, width: int) -> None:
        x = self.put(1, 2, "◆ ", "accent bold")
        x = self.gradient(1, x, "jev", "bold")
        self.put(1, x + 1, "request sender", "muted")

        if self.relay_up is None:
            dot, dot_style, state = "◌", "muted", "checking"
        elif self.relay_up:
            dot, dot_style, state = ("●" if self.frame // 8 % 2 else "◉"), "ok bold", "live"
        else:
            dot, dot_style, state = "○", "err bold", "offline"
        right = [(dot + " ", dot_style), ("relay ", "muted"), (f"127.0.0.1:{self.port} ", "bold"), (state, dot_style),
                 ("   model ", "muted"), (self.model, "bold"), ("   key ", "muted"), (mask_key(self.api_key), "bold")]
        self.text(1, width - 2 - sum(len(t) for t, _ in right), right)

        c = self.counts
        stats = [("sent ", "muted"), (str(c["sent"]), "bold"), ("   2xx ", "muted"), (str(c["ok"]), "ok bold"),
                 ("   4xx ", "muted"), (str(c["warn"]), "warn bold"), ("   5xx ", "muted"), (str(c["err"]), "err bold"),
                 ("   failed ", "muted"), (str(c["fail"]), "bold")]
        if self.latencies:
            stats += [("   avg ", "muted"), (f"{round(sum(self.latencies) / len(self.latencies))} ms", "bold")]
        x = self.text(2, 4, stats)
        spark = sparkline(self.latencies, max(0, min(32, width - x - 8)))
        if spark:
            self.put(2, width - 2 - len(spark) - 8, "latency ", "muted")
            self.gradient(2, width - 2 - len(spark), spark)

        rule = "━" * (width - 4)
        self.gradient(3, 2, rule)

    def draw_cases(self, top: int, bottom: int) -> None:
        lines: list[tuple[str, int | str]] = []
        for index, case in enumerate(CASES):
            if index == 0:
                lines.append(("head", "SHOWCASE"))
            elif index == SHOWCASE_COUNT:
                lines += [("gap", ""), ("head", "EDGE CASES")]
            lines.append(("case", index))
        visible = bottom - top
        selected_line = next(i for i, (kind, v) in enumerate(lines) if kind == "case" and v == self.selected)
        if selected_line < self.scroll:
            self.scroll = max(0, selected_line - 1)
        elif selected_line >= self.scroll + visible:
            self.scroll = selected_line - visible + 1
        self.scroll = min(self.scroll, max(0, len(lines) - visible))

        width = self.split - 3
        for offset, (kind, value) in enumerate(lines[self.scroll:self.scroll + visible]):
            y = top + offset
            if kind == "head":
                self.put(y, 4, value, "muted bold")
            elif kind == "case":
                self.draw_case(y, 2, width, value)

    def draw_case(self, y: int, x: int, width: int, index: int) -> None:
        case = CASES[index]
        self.rows[y] = index
        chosen = index == self.selected
        with self.lock:
            busy = self.inflight.get(case["id"], 0) > 0
        if chosen:
            self.put(y, x, "▌", "accent bold")
        if busy:
            self.put(y, x + 2, SPINNER[self.frame % len(SPINNER)], "accent bold")
        elif index < 9:
            self.put(y, x + 2, str(index + 1), "accent bold" if chosen else "muted")
        delay = f"~{case['delay_ms'] / 1000:.1f}s" if case["delay_ms"] >= 500 else ""
        tail = 5 + 6 + 7  # method, status pill, delay
        title_width = width - 5 - tail
        self.put(y, x + 5, fit(case["title"], title_width - 1).ljust(title_width), "bold" if chosen else "")
        cx = x + 5 + title_width
        self.put(y, cx, case["method"].ljust(5), "violet")
        self.put(y, cx + 5, f" {case['status']} ", f"{status_style(case['status'])} reverse bold")
        self.put(y, cx + 11, delay.rjust(6), "muted")

    def draw_activity(self, x: int, top: int, width: int, bottom: int) -> None:
        self.put(top, x, "ACTIVITY", "muted bold")
        if self.log:
            hint = "c clear"
            self.put(top, x + width - len(hint), hint, "faint")
        if not self.log:
            mid = top + (bottom - top) // 2
            art = ["  ·  ·  ◆  ·  ·  ", "", "Nothing sent yet", "Press 1–9, ⏎ or a to send"]
            for i, line in enumerate(art):
                style = "accent" if i == 0 else ("bold" if i == 2 else "muted")
                self.put(mid - 2 + i, x + max(0, (width - len(line)) // 2), line, style)
            return
        top_ms = max([entry["ms"] for entry in self.log if entry["ok"]] + [1])
        now = time.monotonic()
        for row, entry in enumerate(self.log[: bottom - top - 2]):
            y = top + 2 + row
            fresh = now - entry["born"] < 1.2
            self.put(y, x, entry["at"], "muted")
            pill_x = x + 10
            if entry["ok"]:
                self.put(y, pill_x, f" {entry['status']} ", f"{status_style(entry['status'])} reverse bold")
                bar = 8
                name_width = width - 16 - 8 - bar - 1
                self.put(y, pill_x + 6, fit(entry["title"], name_width), "bold" if fresh else "")
                self.put(y, x + width - bar - 9, f"{entry['ms']:>5} ms", "bold" if fresh else "muted")
                self.put(y, x + width - bar, meter(entry["ms"], top_ms, bar), status_style(entry["status"]))
            else:
                self.put(y, pill_x, " ERR ", "err reverse bold")
                rest = width - 16
                title = fit(entry["title"], rest // 2)
                self.put(y, pill_x + 6, title, "bold" if fresh else "")
                self.put(y, pill_x + 7 + len(title), fit(entry["error"], rest - len(title) - 1), "err")
            if fresh:
                self.put(y, x - 1, "›", "accent bold")

    def draw_footer(self, width: int, height: int) -> None:
        self.put(height - 2, 2, "─" * (width - 4), "faint")
        y = height - 1
        if self.prompt is not None:
            p = self.prompt
            shown = "•" * len(p["value"]) if p["secret"] else p["value"]
            x = self.text(y, 2, [("◆ ", "accent bold"), (p["label"], "bold"), ("  ›  ", "accent")])
            x = self.put(y, x, fit(shown, width - x - 30), "bold")
            self.put(y, x, "▏" if self.frame // 6 % 2 else " ", "accent bold")
            hint = [("⏎", "key"), (" save   ", "muted"), ("esc", "key"), (" cancel   ", "muted"), ("^U", "key"), (" clear", "muted")]
            self.text(y, width - 2 - sum(len(t) for t, _ in hint), hint)
            return
        keys = [("↑↓", "select"), ("⏎", "send"), ("1-9", "quick"), ("a", "all"), ("r", "random"),
                ("p", "port"), ("m", "model"), ("k", "key"), ("q", "quit")]
        x = 2
        for key, label in keys:
            x = self.text(y, x, [(f" {key} ", "key"), (f" {label}  ", "muted")])
        message, style, until = self.notice
        if message and time.monotonic() < until and x + len(message) + 2 < width:
            self.put(y, width - 2 - len(message), message, style)


# ── curses front end ──────────────────────────────────────────────────────────

class CursesSender(SenderApp):
    def __init__(self, screen, **kwargs):
        super().__init__(**kwargs)
        self.screen = screen
        self.styles = self.make_styles()

    def make_styles(self) -> dict[str, int]:
        styles = {"": curses.A_NORMAL, "bold": curses.A_BOLD, "reverse": curses.A_REVERSE, "dim": curses.A_DIM}
        if not curses.has_colors():
            styles.update(muted=curses.A_DIM, faint=curses.A_DIM, key=curses.A_REVERSE)
            return styles
        curses.start_color()
        try:
            curses.use_default_colors()
            background = -1
        except curses.error:
            background = curses.COLOR_BLACK
        rich = curses.COLORS >= 256
        pairs = {
            "accent": (205, curses.COLOR_MAGENTA), "violet": (141, curses.COLOR_BLUE),
            "muted": (245, curses.COLOR_WHITE), "faint": (240, curses.COLOR_WHITE),
            "ok": (78, curses.COLOR_GREEN), "warn": (214, curses.COLOR_YELLOW), "err": (203, curses.COLOR_RED),
        }
        for i, color in enumerate(GRADIENT):
            pairs[f"g{i}"] = (color, curses.COLOR_MAGENTA if i < 4 else curses.COLOR_BLUE)
        for number, (name, (fine, basic)) in enumerate(pairs.items(), start=1):
            curses.init_pair(number, fine if rich else basic, background)
            styles[name] = curses.color_pair(number)
        if not rich:
            styles["muted"] |= curses.A_DIM
            styles["faint"] |= curses.A_DIM
        styles["key"] = styles["violet"] | curses.A_REVERSE | curses.A_BOLD
        return styles

    def put(self, y: int, x: int, text: str, style: str = "") -> int:
        if not text or y < 0 or y >= self.height or x >= self.width:
            return x + len(text)
        attr = 0
        for name in style.split():
            attr |= self.styles.get(name, 0)
        clipped = text[: max(0, self.width - x - (1 if y == self.height - 1 else 0))]
        try:
            self.screen.addstr(y, x, clipped, attr)
        except curses.error:
            pass
        return x + len(text)

    def run(self) -> None:
        curses.curs_set(0)
        self.screen.keypad(True)
        self.screen.timeout(80)
        curses.mousemask(curses.BUTTON1_CLICKED | curses.BUTTON1_PRESSED)
        threading.Thread(target=self.probe, daemon=True).start()
        try:
            while True:
                self.drain()
                self.frame += 1
                self.screen.erase()
                height, width = self.screen.getmaxyx()
                self.width, self.height = width, height
                self.draw(width, height)
                self.screen.refresh()
                try:
                    key = self.screen.get_wch()
                except curses.error:
                    continue
                if key == curses.KEY_MOUSE:
                    try:
                        _, mx, my, _, _ = curses.getmouse()
                        self.click(my, mx)
                    except curses.error:
                        pass
                elif key != curses.KEY_RESIZE and not self.handle_key(key):
                    return
        finally:
            self.stopped.set()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="Relay port to send requests to (default 8787)")
    parser.add_argument("--model", default=DEFAULT_MODEL, help="Model name to put in request bodies (default jev-latest)")
    args = parser.parse_args()
    if not (sys.stdin.isatty() and sys.stdout.isatty()):
        raise SystemExit("request_sender.py is interactive; run it in a terminal.")
    os.environ.setdefault("ESCDELAY", "25")
    locale.setlocale(locale.LC_ALL, "")
    api_key = os.environ.get("JEV_API_KEY", "")
    try:
        curses.wrapper(lambda screen: CursesSender(screen, port=args.port, model=args.model, api_key=api_key).run())
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()

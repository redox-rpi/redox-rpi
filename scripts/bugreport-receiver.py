#!/usr/bin/env python3
"""Receive bug reports from `bugreport` in dev images (#759).

Usage:
    scripts/bugreport-receiver.py [--dir DIR] [--port 8090] [--bind 0.0.0.0]
    scripts/bugreport-receiver.py --list [--dir DIR]

Accepts `POST /bugreport` with a text body of at most 1 MiB, stores it as
`DIR/BR-NNNN.txt` with a short header (time, sender's address, size) and
answers with the ID, for example `BR-0427`. The tester passes on that ID;
a session reads the file and files the issue.

Lab network only, no authentication: dev images (experiment
bugreport-upload) are lab-only, release images never upload. Limits: 1 MiB
per report, MAX_REPORTS stored reports, one report at a time. Nothing else
is served.

The default DIR is ~/redox-rpi/logs/bugreports. The permanent service on
the build host is set up outside this repository; anyone can run this by
hand for a test.
"""
import argparse
import http.server
import os
import re
import sys
import threading
import time

MAX_BODY = 1024 * 1024
MAX_REPORTS = 500
ID_RE = re.compile(r"^BR-(\d{4,})\.txt$")
lock = threading.Lock()


def next_id(directory):
    nums = [int(m.group(1)) for f in os.listdir(directory) if (m := ID_RE.match(f))]
    return (max(nums) + 1 if nums else 1), len(nums)


class Handler(http.server.BaseHTTPRequestHandler):
    server_version = "bugreport-receiver/1"
    directory = "."

    def answer(self, code, text):
        body = (text + "\n").encode()
        self.send_response(code)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        self.answer(404, "only POST /bugreport")

    def do_POST(self):
        if self.path.rstrip("/") != "/bugreport":
            return self.answer(404, "only POST /bugreport")
        try:
            length = int(self.headers.get("Content-Length", ""))
        except ValueError:
            return self.answer(411, "Content-Length needed")
        if length <= 0 or length > MAX_BODY:
            return self.answer(413, f"a report has 1 to {MAX_BODY} bytes")
        self.connection.settimeout(30)
        body = self.rfile.read(length)
        if len(body) != length:
            return self.answer(400, "report cut short")
        if b"\0" in body:
            return self.answer(415, "text only")
        with lock:
            num, count = next_id(self.directory)
            if count >= MAX_REPORTS:
                return self.answer(507, f"{MAX_REPORTS} reports stored; clean up first")
            rid = f"BR-{num:04d}"
            path = os.path.join(self.directory, f"{rid}.txt")
            head = (f"# {rid}\n# received {time.strftime('%Y-%m-%d %H:%M:%S %Z')}"
                    f" from {self.client_address[0]}, {length} bytes\n\n")
            with open(path, "x", encoding="utf-8") as f:
                f.write(head + body.decode("utf-8", "replace"))
        self.log_message("stored %s (%d bytes)", rid, length)
        self.answer(200, rid)


def first_line(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        lines = f.read(4096).splitlines()
    try:
        return lines[lines.index("== What went wrong") + 1]
    except (ValueError, IndexError):
        return "?"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--dir", default=os.path.expanduser("~/redox-rpi/logs/bugreports"))
    ap.add_argument("--port", type=int, default=8090)
    ap.add_argument("--bind", default="0.0.0.0")
    ap.add_argument("--list", action="store_true", help="list the stored reports and exit")
    args = ap.parse_args()
    os.makedirs(args.dir, exist_ok=True)
    if args.list:
        for f in sorted(os.listdir(args.dir)):
            if ID_RE.match(f):
                print(f"{f[:-4]}  {first_line(os.path.join(args.dir, f))}")
        return
    Handler.directory = args.dir
    srv = http.server.HTTPServer((args.bind, args.port), Handler)  # one request at a time
    print(f"bugreport-receiver: {args.bind}:{args.port}, reports in {args.dir}", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())

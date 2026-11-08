#!/usr/bin/env python3
"""python3 -m http.server plus an ETag header, for the image server.

    scripts/http-etag-server.py [port] [--directory DIR]

Same arguments and behaviour as `python3 -m http.server`; every file
response that has Last-Modified also gets `ETag: "<mtime ns>-<size>"` (hex),
so SwiftFlash can tell a new file under an old name (docs/topics/ci.md,
"Image server"). A -latest symlink gets the ETag of its target. Nothing
else changes, so U-Boot's HTTP boot from /srv/tftp works as before.

Not in use: switching redox-http.service to it is the maintainer's call.
"""

import argparse
import contextlib
import functools
import http.server
import os
import socket


class Handler(http.server.SimpleHTTPRequestHandler):
    def send_header(self, keyword, value):
        super().send_header(keyword, value)
        if keyword == "Last-Modified":
            try:
                st = os.stat(self.translate_path(self.path))
                super().send_header("ETag", '"%x-%x"' % (st.st_mtime_ns, st.st_size))
            except OSError:
                pass


class DualStackServer(http.server.ThreadingHTTPServer):
    """As in `python3 -m http.server`: IPv4 and IPv6 on one socket."""

    def server_bind(self):
        with contextlib.suppress(Exception):
            self.socket.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0)
        return super().server_bind()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("port", nargs="?", type=int, default=8000)
    parser.add_argument("--bind", "-b", default=None)
    parser.add_argument("--directory", "-d", default=os.getcwd())
    args = parser.parse_args()
    http.server.test(HandlerClass=functools.partial(Handler, directory=args.directory),
                     ServerClass=DualStackServer, port=args.port, bind=args.bind)

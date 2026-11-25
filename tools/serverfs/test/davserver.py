#!/usr/bin/env python3
"""A throwaway, read-only WebDAV server for the serverfs tests (#713).

    davserver.py <dir> <port> [--auth user:password] [--tls cert.pem key.pem]

Serves <dir> on 127.0.0.1 only (QEMU's user network reaches it as
10.0.2.2), and answers just the requests serverfs uses: PROPFIND with
Depth 0 or 1, GET and HEAD. Anything else is 405. With --tls it speaks
HTTPS with the given certificate. Not for real use: no locking, no writes.
Each request is logged to stderr.
"""
import base64
import email.utils
import html
import os
import ssl
import sys
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.realpath(sys.argv[1])
PORT = int(sys.argv[2])
AUTH = None
TLS = None
args = sys.argv[3:]
while args:
    if args[0] == "--auth" and len(args) > 1:
        AUTH = "Basic " + base64.b64encode(args[1].encode()).decode()
        args = args[2:]
    elif args[0] == "--tls" and len(args) > 2:
        TLS = (args[1], args[2])
        args = args[3:]
    else:
        sys.exit(__doc__)


def local_path(url_path):
    rel = urllib.parse.unquote(urllib.parse.urlsplit(url_path).path).lstrip("/")
    full = os.path.realpath(os.path.join(ROOT, rel))
    if full != ROOT and not full.startswith(ROOT + os.sep):
        return None
    return full


def href(full):
    rel = os.path.relpath(full, ROOT)
    rel = "" if rel == "." else rel.replace(os.sep, "/")
    url = "/" + urllib.parse.quote(rel)
    if os.path.isdir(full) and not url.endswith("/"):
        url += "/"
    return url


def etag(st):
    return '"%x-%x"' % (st.st_mtime_ns, st.st_size)


def response_xml(full):
    st = os.stat(full)
    if os.path.isdir(full):
        props = "<D:resourcetype><D:collection/></D:resourcetype>"
    else:
        props = "<D:resourcetype/><D:getcontentlength>%d</D:getcontentlength>" % st.st_size
    props += "<D:getlastmodified>%s</D:getlastmodified>" % email.utils.formatdate(st.st_mtime, usegmt=True)
    props += "<D:getetag>%s</D:getetag>" % html.escape(etag(st))
    return (
        "<D:response><D:href>%s</D:href><D:propstat><D:prop>%s</D:prop>"
        "<D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>" % (html.escape(href(full)), props)
    )


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def authorized(self):
        if AUTH is None or self.headers.get("Authorization") == AUTH:
            return True
        self.send_response(401)
        self.send_header("WWW-Authenticate", 'Basic realm="serverfs-test"')
        self.send_header("Content-Length", "0")
        self.end_headers()
        return False

    def plain(self, code):
        self.send_response(code)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_PROPFIND(self):
        length = int(self.headers.get("Content-Length") or 0)
        if length:
            self.rfile.read(length)
        if not self.authorized():
            return
        full = local_path(self.path)
        if full is None or not os.path.exists(full):
            return self.plain(404)
        depth = self.headers.get("Depth", "1")
        if depth not in ("0", "1"):
            return self.plain(403)
        parts = [response_xml(full)]
        if depth == "1" and os.path.isdir(full):
            for name in sorted(os.listdir(full)):
                parts.append(response_xml(os.path.join(full, name)))
        body = ('<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:">%s</D:multistatus>'
                % "".join(parts)).encode()
        self.send_response(207)
        self.send_header("Content-Type", "application/xml; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self, head=False):
        if not self.authorized():
            return
        full = local_path(self.path)
        if full is None or not os.path.isfile(full):
            return self.plain(404)
        st = os.stat(full)
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(st.st_size))
        self.send_header("ETag", etag(st))
        self.end_headers()
        if not head:
            with open(full, "rb") as f:
                while chunk := f.read(1 << 16):
                    self.wfile.write(chunk)

    def do_HEAD(self):
        self.do_GET(head=True)

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("DAV", "1")
        self.send_header("Allow", "OPTIONS, GET, HEAD, PROPFIND")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def unsupported(self):
        self.plain(405)

    do_PUT = do_DELETE = do_MKCOL = do_MOVE = do_COPY = do_PROPPATCH = do_LOCK = do_POST = unsupported


server = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
if TLS:
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(*TLS)
    server.socket = context.wrap_socket(server.socket, server_side=True)
server.serve_forever()

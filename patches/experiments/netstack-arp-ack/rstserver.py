#!/usr/bin/env python3
"""rstserver.py PORT BODY: answer each connection with an HTTP header, then a
second later offer BODY bytes, and reset the connection (SO_LINGER 0) 2 s
after that. A client that reads slowly (netget --slow 5000) got only the
header in its first read; the body bytes it reports afterwards are the ones
that were in its receive buffer when the reset came."""
import socket, struct, sys, time
port = int(sys.argv[1]); body = int(sys.argv[2])
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("0.0.0.0", port)); s.listen(2)
while True:
    c, a = s.accept()
    c.recv(4096)
    c.sendall(b"HTTP/1.0 200 OK\r\nContent-Length: %d\r\n\r\n" % body)
    time.sleep(1)
    # More than the client's window: sendall stops when the window is full.
    c.settimeout(2)
    sent = 0
    try:
        while sent < body:
            sent += c.send(b"x" * min(65536, body - sent))
    except socket.timeout:
        pass
    c.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
    c.close()
    print(f"{time.strftime('%H:%M:%S')} {a[0]}: header, {sent} body bytes into the socket, then reset", flush=True)

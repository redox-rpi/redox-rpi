#!/usr/bin/env python3
"""Answer each connection on PORT with an HTTP header and BODY bytes, wait a
second and then reset the connection (SO_LINGER 0). A slow client still has
part of the body in its receive buffer when the reset comes."""
import socket, struct, sys, time
port = int(sys.argv[1]); body = int(sys.argv[2])
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("0.0.0.0", port)); s.listen(2)
while True:
    c, a = s.accept()
    c.recv(4096)
    # The body is larger than the client's window, so sendall stops; after
    # 2 s the client's buffer is full and the rest is thrown away by the reset.
    c.settimeout(2)
    try:
        c.sendall(b"HTTP/1.0 200 OK\r\nContent-Length: %d\r\n\r\n" % body + b"x" * body)
    except socket.timeout:
        pass
    c.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("ii", 1, 0))
    c.close()
    print(f"{time.strftime('%H:%M:%S')} offered {body} bytes to {a[0]}, then reset", flush=True)

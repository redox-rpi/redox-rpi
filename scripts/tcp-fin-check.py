#!/usr/bin/env python3
"""Check that Redox ends a TCP connection with FIN (redox-rpi #184).

Runs on the host. For each connection it waits for the Redox side to end
it and prints one line: FIN (the connection was closed properly), RST
(reset) or NONE (nothing came: the bug, the connection stays open).

  tcp-fin-check.py listen 0.0.0.0:12401 [--half 1,3]
      Accept connections; on Redox: nc <host>:12401
  tcp-fin-check.py connect <redox>:2222 [--half]
      Connect once to a listening Redox (on Redox first: nc -l 0.0.0.0:2222);
      retries for 60 s until something listens.

--half: the host closes its side 3 s after the connection opens (listen:
for the listed connection numbers). Redox nc then sees end of input and
exits, so Redox closes its side. Without --half, end the Redox nc by
hand (Ctrl-C): Redox closes first, as sshd does after its login grace time.
"""
import argparse, socket, threading, time

WAIT = 30
t0 = time.time()


def log(*a):
    print(f"[{time.time() - t0:7.1f}s]", *a, flush=True)


def watch(c, name, half):
    if half:
        time.sleep(3)
        c.shutdown(socket.SHUT_WR)
        log(name, "host closed its side")
    c.settimeout(WAIT)
    start = time.time()
    try:
        while True:
            d = c.recv(4096)
            if not d:
                log(name, f"FIN from Redox after {time.time() - start:.1f}s")
                break
    except socket.timeout:
        log(name, f"NONE from Redox within {WAIT}s")
    except ConnectionResetError:
        log(name, f"RST from Redox after {time.time() - start:.1f}s")
    c.close()


def addr(s):
    host, port = s.rsplit(":", 1)
    return host, int(port)


p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
p.add_argument("mode", choices=["listen", "connect"])
p.add_argument("address", type=addr)
p.add_argument("--half", nargs="?", const="1", default="")
p.add_argument("--time", type=float, default=600, help="listen: seconds to run")
a = p.parse_args()
half = {int(x) for x in a.half.split(",") if x}

if a.mode == "listen":
    s = socket.socket()
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(a.address)
    s.listen(5)
    s.settimeout(1)
    log("listening on", a.address)
    n, threads = 0, []
    while time.time() - t0 < a.time:
        try:
            c, peer = s.accept()
        except socket.timeout:
            continue
        n += 1
        log(f"conn{n}", "accepted from", peer)
        t = threading.Thread(target=watch, args=(c, f"conn{n}", n in half))
        t.start()
        threads.append(t)
    for t in threads:
        t.join()
else:
    while time.time() - t0 < 60:
        try:
            c = socket.create_connection(a.address, timeout=5)
        except OSError:
            time.sleep(2)
            continue
        c.settimeout(2)
        try:
            # QEMU's user network accepts at once and closes again when
            # nothing listens on Redox
            c.recv(1)
            c.close()
            time.sleep(2)
            continue
        except socket.timeout:
            pass
        except ConnectionResetError:
            c.close()
            time.sleep(2)
            continue
        log("conn1", "connected to", a.address)
        watch(c, "conn1", bool(half))
        break
    else:
        log("nothing listened within 60 s")

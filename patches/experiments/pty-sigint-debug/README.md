# pty-sigint-debug

Debug patch for issue Ctrl-C on PTYs: ptyd logs every change of a PTY's
foreground process group and what happens with each Ctrl-C (VINTR). Only for
QEMU checks; not for images people use.

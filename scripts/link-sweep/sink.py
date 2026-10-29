#!/usr/bin/env python3
"""TCP sink for socket-mux RX output: discard bytes, report goodput.

Listens on 127.0.0.1:31200 (RX's default --connect), accepts one connection
and reads to end of stream. Goodput covers the active window: from the first
byte until the first idle gap of at least 0.5 s, which excludes RX's timeout
wait for tail datagrams. Prints one key=value line.
"""
import socket
import time

IDLE_GAP = 0.5


def main() -> None:
    server = socket.socket()
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", 31200))
    server.listen(1)
    conn, _ = server.accept()
    total = 0
    first = 0.0
    last = 0.0
    active = None
    while True:
        chunk = conn.recv(1 << 20)
        now = time.time()
        if not chunk:
            break
        if total == 0:
            first = now
        elif active is None and now - last >= IDLE_GAP:
            active = (total, last - first)
        last = now
        total += len(chunk)
    if active is None:
        active = (total, last - first)
    active_bytes, active_secs = active
    rate = active_bytes * 8 / active_secs / 1e9 if active_secs > 0 else 0.0
    print(
        f"bytes={total} active_bytes={active_bytes} "
        f"active_secs={active_secs:.2f} Gbit/s={rate:.2f}",
        flush=True,
    )


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Deterministic HTTP/1.1 upstream used only by loopback gateway load contracts."""

import os
import socket
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Handler(BaseHTTPRequestHandler):
    """Serve a tiny fixed response without logging per-request noise."""

    protocol_version = "HTTP/1.1"

    def setup(self) -> None:
        """Keep the fixture from contributing delayed-ACK/Nagle latency to gateway evidence."""
        super().setup()
        self.connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

    def do_GET(self) -> None:
        """Return the configured fixed payload used to prove proxy correctness under concurrency."""
        payload = os.environ.get("UPSTREAM_PAYLOAD", "upstream-ok").encode("ascii")
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)
        self.wfile.flush()

    def log_message(self, format: str, *args: object) -> None:
        """Suppress request logs so timing evidence is not distorted by console I/O."""


if __name__ == "__main__":
    port = int(os.environ.get("UPSTREAM_PORT", "18081"))
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    server.serve_forever()

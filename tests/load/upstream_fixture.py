#!/usr/bin/env python3
"""Deterministic HTTP/1.1 upstream used only by the loopback gateway load contract."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Handler(BaseHTTPRequestHandler):
    """Serve a tiny fixed response without logging per-request noise."""

    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:
        """Return the fixed payload used to prove proxy correctness under concurrency."""
        payload = b"upstream-ok"
        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, format: str, *args: object) -> None:
        """Suppress request logs so timing evidence is not distorted by console I/O."""


if __name__ == "__main__":
    server = ThreadingHTTPServer(("127.0.0.1", 18081), Handler)
    server.daemon_threads = True
    server.serve_forever()

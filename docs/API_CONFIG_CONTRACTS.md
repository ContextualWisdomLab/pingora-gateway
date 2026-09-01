# API and Configuration Contracts

Schema version 1 requires `listener`, `limits` and one or more `routes`. Listener ports below 1024 are rejected. Limits are explicit positive values with hard safety ceilings. Each route has a bounded `id`, optional exact `host`, absolute `path_prefix`, origin-only `upstream`, and optional `allow_private_networks` defaulting false.

Route selection uses exact host routes before host-agnostic routes and longest prefix within that class. Duplicate `(host, path_prefix)` matchers are invalid. No path rewriting occurs. The upstream is resolved at startup and a validated socket is stored; request data cannot choose a host or port.

Local endpoints are `GET|HEAD /livez`, `/readyz` and `/metrics`. They are reserved and bypass upstream routing. `/metrics` uses Prometheus text format. Other methods on these endpoints return 405.

HTTPS upstreams verify certificates and hostnames. Incoming Forwarded/X-Forwarded/X-Real-IP values are removed. The upstream Host header is set from the configured origin authority.

# Technical Requirements Document

The binary is Rust and uses Cloudflare Pingora. The initial dependency is pinned to upstream commit `09696b51bc59315353d96686355861604d0bb48c`, which contains the 2026-08-24 security/dependency sync including `lru` 0.18.2; the latest released Pingora 0.8.1 predates that change.

Configuration is YAML schema version 1 with `serde(deny_unknown_fields)`. Route count, header count/bytes, body bytes and upstream timeouts are bounded. Upstream URLs cannot contain userinfo, paths, queries or fragments. DNS is resolved before listener bind; any private/link-local/loopback/multicast/unspecified resolution requires an explicit per-route private-network opt-in. The selected resolved socket is request-independent, preventing request-controlled upstream SSRF.

Route precedence is exact host before host-agnostic routes, then longest path prefix. Duplicate matchers fail validation. Pingora's default hop-by-hop request normalization is retained; inbound `Forwarded`, `X-Forwarded-*` and `X-Real-IP` are removed because no trusted-proxy model exists yet. HTTPS peers explicitly keep `verify_cert` and `verify_hostname` true.

The process exposes `/livez`, `/readyz` and `/metrics` on the same listener. Metrics have no request-controlled labels. Logs contain route ID and error type, not headers, cookies, tokens or raw paths.

# Technical Requirements

## Runtime

Rust edition 2021, minimum Rust `1.97.1`. The Pingora dependency is pinned to an exact upstream Git revision. The production composition root is `src/bin/cwl-pingora-gateway.rs`; it parses `--config`, validates the contract, constructs `GatewayProxy`, adds a TCP listener to `http_proxy_service`, and delegates lifecycle handling to `Server::run_forever()`.

## Contract

Configuration version 1 is YAML with `deny_unknown_fields`. Required top-level fields are `version`, `listener`, `max_request_body_bytes`, and `upstreams`. Exactly one upstream is accepted. Each upstream has `name`, `address`, `tls`, optional `sni`, and explicit positive timeout budgets.

TLS upstreams require SNI. The Pingora `HttpPeer` enables certificate verification and hostname verification. Cleartext upstreams must not carry an SNI value. No request may choose an upstream dynamically.

## Request policy

Pingora's standard upstream request policy strips hop-by-hop and connection-nominated headers and rejects malformed connection nominations. The gateway additionally strips client-provided `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`, and `X-Real-IP`, then emits `Forwarded: proto=http` for the v1 cleartext downstream listener. Client IP is deliberately not asserted.

Requests with a parseable `Content-Length` larger than `max_request_body_bytes` fail with 413 before upstream selection. Streamed chunks are counted and fail with 413 when the same bound is exceeded. Pingora's HTTP parser supplies its own finite header/protocol bounds; a configurable smaller header budget is not yet implemented.

## Health

`GET /livez` and `/readyz` currently produce 200 with an empty non-cacheable response. Readiness proves validated configuration plus an active production serving path, not upstream health.

## Packaging

`Dockerfile` uses a Rust builder and Debian runtime, installs only CA/OpenSSL runtime dependencies, and executes as uid/gid `65532`. The process has no intentional filesystem writes. A committed lockfile, image build test, SBOM/provenance, and immutable registry digest remain release gates.

# Technical Requirements

## Runtime

Rust edition 2021 with minimum Rust `1.98.0`. Pingora and `pingora-prometheus` are pinned to the exact upstream Git revision declared in `Cargo.toml`. The generic production composition root is `src/bin/cwl-pingora-gateway.rs`: it requires `--config`, validates `GatewayConfig`, constructs `GatewayProxy`, binds the explicit traffic and metrics listeners, and delegates lifecycle handling to Pingora `Server`. The separate `src/bin/cwl-pingora-pg-erd-migration.rs` composition root uses the same explicit command-line config path but admits only the bounded `PgErdMigrationConfig` profile and its prevalidated `MigrationGatewayProxy`; it is a migration candidate, not a protected release or consumer cutover.

## Generic edge contract

Configuration version 1 is strict YAML with unknown fields rejected. The generic aggregate requires an explicit non-zero traffic listener, a distinct non-zero metrics listener, positive `max_request_body_bytes`, `max_in_flight_requests`, and `upstream_keepalive_pool_size`, plus exactly one explicit upstream. Listener/metrics network authority rejects equal sockets, same-port wildcard aliases, and the platform-dependent IPv6-wildcard/IPv4 same-port overlap while preserving distinct concrete non-zero IP authorities.

Each upstream has a stable name, concrete non-zero socket address, `tls`, optional `sni`, optional absolute PEM trust-bundle path, and explicit positive connection/total-connection/read/write/idle timeout budgets. TLS upstreams require SNI; cleartext upstreams reject SNI and custom trust bundles. Peer materialization keeps certificate and hostname verification enabled and uses HTTP/1.1 upstream transport for the current contract. No downstream request may choose an upstream dynamically.

## Bounded pg-erd migration config

`PgErdMigrationConfig` is not a generic multi-route language. It exposes only deployment-variable listener/metrics sockets, positive body/in-flight/keepalive budgets, and concrete transport/TLS bindings for the fixed `backend` and `frontend` identities admitted by the characterized pg-erd migration plan. Route precedence, response-security policy, admitted upstream names, product authentication/authorization, service discovery, and business routing are not operator-configurable. Parsing validates authority without loading trust bytes; `build_proxy` materializes all peers and custom trust before listeners may obtain network authority.

## Request and forwarding policy

Pingora's standard upstream request policy strips standard hop-by-hop fields, strips connection-nominated extension fields, rejects malformed protected nominations, and preserves only normalized WebSocket upgrade behavior supplied by the pinned Pingora line. The generic gateway additionally removes request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http`; generic v1 deliberately does not assert client identity.

The pg-erd migration uses the separate `forwarding_policy` trust boundary. It discards the same request-controlled forwarding identity and reconstructs only the characterized compatibility fields from accepted downstream transport and request authority. Because the captured legacy entryPoint is clear-text `web`, the current migration scheme is explicitly `http`; HTTPS requires a separate downstream TLS contract.

Requests with a parseable `Content-Length` larger than `max_request_body_bytes` fail with HTTP 413 before upstream selection. Streamed chunks are counted against the same bound and fail with HTTP 413 when the cumulative body crosses it. Non-health application requests share the process in-flight admission budget and fail fast with HTTP 503 when it is exhausted; `/livez` and `/readyz` remain outside that application-capacity budget. A configurable smaller header budget, per-route budget, and origin-capacity budget are not yet implemented.

## Health, observability, retry, and drain

`/livez` and `/readyz` produce an empty non-cacheable HTTP 200 through the Pingora serving path. Readiness proves admitted configuration plus an active process serving path, not upstream health. Shared observability records only low-cardinality status/outcome/body-byte and backpressure facts; paths, query strings, headers, cookies, credentials, customer payloads, and product identifiers are outside the shared telemetry contract.

The version-1 process policy allows one total upstream attempt and therefore no automatic gateway retry. It overrides Pingora's keepalive-pool default from validated configuration and uses a 5-second grace period plus 10-second Pingora runtime shutdown timeout inside a 30-second external termination budget. Product idempotency/replay policy remains outside the gateway.

## Packaging and release evidence

The OCI runtime executes as uid/gid `65532`, is compatible with read-only-root and capability-free operation, and relies on digest-pinned builder/runtime bases. One Dockerfile owns a shared hardened `runtime-common` stage and two explicit final targets: default `gateway` packages only `cwl-pingora-gateway`, while explicit `pg-erd-migration` packages only `cwl-pingora-pg-erd-migration`. CI requires both exact-source images to expose their intended entrypoint, start under uid/gid 65532 with read-only root, all capabilities dropped and `no-new-privileges`, and serve `/livez` from their own bounded configuration. Supply-chain evidence builds and vulnerability-scans both image targets, records their local image IDs, and binds those results plus the shared dependency SBOM/policy files to the exact source SHA.

These remain unreleased candidate controls rather than deployable release authority. A committed lockfile, terminal exact-head test/coverage/rustdoc/load/security/supply-chain evidence, immutable registry digest, signature/attestation policy, SBOM/provenance/reproducibility, rollback rehearsal, and protected integration remain release gates. Source-level migration capability or a local OCI image is not parity, canary, cutover, rollback, or legacy-removal evidence.

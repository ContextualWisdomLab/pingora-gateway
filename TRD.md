# Technical Requirements

## Runtime

This branch uses Rust edition 2021 with `rust-version = "1.98.0"`. The Pingora crates are pinned to Cloudflare Pingora `0.8.0` at exact upstream Git revision `09696b51bc59315353d96686355861604d0bb48c`; mutable branch, tag, or contributor-PR resolution is not release authority.

There are two composition roots with deliberately different contracts:

- `src/bin/cwl-pingora-gateway.rs` activates generic version-1 `GatewayConfig` and the one-upstream `GatewayProxy`.
- `src/bin/cwl-pingora-pg-erd-migration.rs` activates only the bounded `PgErdMigrationConfig` profile and `MigrationGatewayProxy` for the characterized pg-erd edge surface.

Both parse an explicit `--config` path before creating listeners and delegate process lifecycle to Pingora's server lifecycle. The separate binaries prevent the generic v1 configuration language from being widened implicitly by one consumer migration.

## Generic edge contract

Generic configuration version 1 is strict YAML with unknown fields denied. It admits one explicit non-zero listener, a distinct non-zero metrics authority, exactly one non-zero concrete upstream socket, a positive request-body budget, positive process in-flight and upstream keepalive budgets, and explicit positive upstream I/O budgets. Listener/metrics validation rejects effective socket-authority overlap: equal sockets, same-family wildcard aliases, native/IPv4-mapped IPv4 aliases, and IPv6-wildcard/IPv4 same-port ambiguity. Listener bind wildcards remain admissible where deployment requires them; upstream destinations do not. Unspecified upstream addresses `0.0.0.0` and `[::]` fail closed because they do not identify a concrete remote TCP authority. Distinct concrete non-aliased addresses remain independent. HTTPS upstreams require SNI and certificate/hostname verification; cleartext upstreams must not carry SNI or trust-bundle data. No request may select arbitrary upstream authority dynamically.

Requests with a parseable `Content-Length` above the configured limit fail with 413 before upstream selection. Streamed body bytes are counted against the same bound. Saturated process admission fails with 503 and the lease is released when the request completes or aborts.

## Bounded pg-erd Admin Config

`PgErdMigrationConfig` is not a generic route language. Operators may provide only the traffic listener, metrics listener, positive body/in-flight/keepalive budgets, and concrete transport/TLS values for the already characterized `backend` and `frontend` identities. Route precedence, response-security fields and admitted upstream names remain compiled migration semantics. Product authentication/authorization, business routing, domain response semantics, Keyverse identity and Wardnet/EgressWeave verdicts remain outside this bounded context.

Configuration validation fails before listener activation on unsupported versions, zero ports or runtime budgets, overlapping traffic/metrics socket authority, zero keepalive capacity, missing/duplicate/extra/renamed upstream authority, unspecified upstream destination addresses, or invalid upstream TLS/transport data. The migration profile consumes the same shared effective socket-authority and concrete-upstream invariants as generic v1 while preserving its characterized zero-transport-authority error surface.

`PgErdMigrationConfig` derives public Serde `Deserialize`, so callers are not forced to enter through `PgErdMigrationConfig::from_yaml`. The public activation boundary therefore revalidates the complete deterministic configuration in `build_proxy()` before delivery peers or runtime limits are materialized. Only after that check may `RuntimeIsolationLimits::from_validated` reuse the proven positive budgets. Direct deserialization cannot bypass version, listener-authority, runtime, keepalive or transport-authority invariants.

Custom upstream trust-bundle bytes are not preloaded during YAML parsing. Peer/trust materialization happens once during `build_proxy()` before the composition root creates listeners, reducing validate-then-reload drift for operator-supplied trust material.

## Request and forwarding policy

Pingora's standard upstream-request policy handles hop-by-hop and connection-nominated headers. The generic gateway additionally removes request-controlled forwarding identity and emits only gateway-owned `Forwarded: proto=http` for its current cleartext downstream contract.

The pg-erd migration callback uses the separate Ingress Forwarding Policy. Request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP` and legacy `X-Forwarded-Server` values are discarded. `X-Forwarded-For` and `X-Real-IP` are rebuilt from the accepted client socket, `X-Forwarded-Host` preserves original Host authority, and `X-Forwarded-Port` comes from an explicit Host port or the admitted scheme default rather than the process listener bind. The currently characterized legacy entry point is cleartext `web`, so downstream scheme is explicitly `http`; HTTPS forwarding semantics require a separate downstream-TLS contract.

## Health, observability and graceful lifecycle

`/livez` and `/readyz` are gateway process endpoints served locally through the production Pingora path and do not become consumer routes. Pg-erd `/healthz` remains characterized product traffic to `backend`. Shared observability is low-cardinality and payload-free: request path/query, headers, cookies, credentials, customer payloads and product identifiers are outside the shared telemetry contract.

Both composition roots use the shared Pingora server policy and bounded graceful shutdown. Process tests terminate successful children through the graceful path so LLVM coverage profiles can flush; emergency cleanup remains a test-harness fallback rather than the normal lifecycle.

## Packaging and release boundary

The Docker builder is digest-pinned Rust 1.98.0 Bookworm and the final image is digest-pinned distroless Debian 13 `base-nossl` non-root. `CWL_GATEWAY_BIN` is a build-time-only fail-closed allowlist of exactly `cwl-pingora-gateway` and `cwl-pingora-pg-erd-migration`; the selected executable is normalized to one fixed runtime path, so the final image contains one admitted process identity and no runtime shell selector.

Exact-head OCI acceptance builds both profiles and starts each as uid/gid `65532` under read-only-root, all-capabilities-dropped and `no-new-privileges` restrictions with a read-only configuration mount. The supply-chain lane builds and vulnerability-scans both candidate images, binds both local image IDs and per-image scan outputs to the exact source SHA, and keeps failure diagnostics distinct from promotion-shaped success evidence. These are unreleased candidate receipts only.

A protected release remains blocked until exact-head CI, strict Clippy, warning-denied rustdoc, 100% owned production line/region coverage, load/runtime and supply-chain evidence are terminal GREEN; protected review/governance is satisfied without bypass; supplier/advisory policy is clean; and an immutable image digest, release-bound SBOM/provenance/reproducibility and rollback evidence exist. Source capability, predecessor GREEN or a mutable image/tag does not establish release, parity, shadow, canary, cutover or legacy-removal state.

## Protocol and migration limits

Generic v1 remains a cleartext downstream HTTP proxy with one explicit upstream per process. Downstream TLS termination, HTTP/2 admission, H2-to-H1 Cookie normalization, HTTP/3/QUIC, WebSocket/Extended CONNECT, dynamic reload, Kubernetes Gateway API, and consumer-specific multi-route behavior are versioned increments with separate realistic RED-to-GREEN evidence.

The pg-erd migration stack is a bounded consumer-characterization adapter and does not widen generic v1. Promotion still requires unchanged exact-head formatting, compile/test, strict Clippy, rustdoc, owned-production coverage, routed traffic/load/failure evidence, immutable release identity, consumer deployment pin, shadow/canary, rollback rehearsal, protected cutover, and verified legacy removal.

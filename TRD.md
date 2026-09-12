# Technical Requirements

## Runtime

This branch uses Rust edition 2021 with `rust-version = "1.98.0"`. The Pingora crates are pinned to Cloudflare Pingora `0.8.0` at exact upstream Git revision `09696b51bc59315353d96686355861604d0bb48c`; mutable branch or tag resolution is not release authority.

There are two composition roots with deliberately different contracts:

- `src/bin/cwl-pingora-gateway.rs` activates generic version-1 `GatewayConfig` and the one-upstream `GatewayProxy`.
- `src/bin/cwl-pingora-pg-erd-migration.rs` activates only the bounded `PgErdMigrationConfig` profile and `MigrationGatewayProxy` for the characterized pg-erd edge surface.

Both parse an explicit `--config` path before creating listeners and delegate process lifecycle to Pingora's server lifecycle. The separate binaries prevent the generic v1 configuration language from being widened implicitly by one consumer migration.

## Generic edge contract

Generic configuration version 1 is strict YAML with unknown fields denied. It admits one explicit listener, exactly one upstream, a positive request-body budget, positive process in-flight and upstream keepalive budgets, and explicit positive upstream I/O budgets. HTTPS upstreams require SNI and certificate/hostname verification; cleartext upstreams must not carry an SNI value. No request may select an arbitrary upstream authority dynamically.

Requests with a parseable `Content-Length` above the configured limit fail with 413 before upstream selection. Streamed body bytes are counted against the same bound. Saturated process admission fails with 503 and the lease is released when the request completes or aborts.

## Bounded pg-erd Admin Config

`PgErdMigrationConfig` is not a generic route language. Operators may provide only the traffic listener, metrics listener, positive body/in-flight/keepalive budgets, and concrete transport/TLS values for the already characterized `backend` and `frontend` identities. Route precedence, response-security fields and admitted upstream names remain compiled migration semantics. Product authentication/authorization, business routing, domain response semantics, Keyverse identity and Wardnet/EgressWeave verdicts remain outside this bounded context.

Configuration validation fails before listener activation on unsupported versions, zero ports or runtime budgets, overlapping traffic/metrics socket authority, zero keepalive capacity, missing/duplicate/extra/renamed upstream authority, or invalid upstream TLS/transport data. Listener collision validation includes exact equality, same-family wildcard aliases, IPv6 wildcard versus IPv4 dual-stack ambiguity, and IPv4-mapped IPv6 aliases.

`PgErdMigrationConfig` derives public Serde `Deserialize`, so callers are not forced to enter through `PgErdMigrationConfig::from_yaml`. The public activation boundary therefore revalidates the complete deterministic configuration in `build_proxy()` before delivery peers or runtime limits are materialized. Only after that check may `RuntimeIsolationLimits::from_validated` reuse the proven positive budgets. This keeps the earlier single-validation coverage simplification without allowing direct deserialization to bypass version, listener, runtime, keepalive or transport-authority invariants.

Custom upstream trust-bundle bytes are not preloaded during YAML parsing. Peer/trust materialization happens once during `build_proxy()` before the composition root creates listeners, reducing validate-then-reload drift for operator-supplied trust material.

## Request and forwarding policy

Pingora's standard upstream-request policy handles hop-by-hop and connection-nominated headers. The generic gateway additionally removes request-controlled forwarding identity and emits only gateway-owned generic forwarding information appropriate to its current cleartext downstream contract.

The pg-erd migration callback uses the separate Ingress Forwarding Policy. Request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP` and legacy `X-Forwarded-Server` values are discarded. `X-Forwarded-For` and `X-Real-IP` are then rebuilt from the accepted client socket, `X-Forwarded-Host` preserves the original Host authority, and `X-Forwarded-Port` is derived from an explicit Host port or the admitted scheme default. The process listener bind port is not external authority because container, Service, NAT, and port-publish layers can expose a different public port. The currently characterized legacy entry point is cleartext `web`, so downstream scheme is explicitly `http`; HTTPS forwarding semantics require a separate downstream-TLS contract.

## Health, observability and graceful lifecycle

`/livez` and `/readyz` are gateway process endpoints served locally through the production Pingora path and do not become consumer routes. Pg-erd `/healthz` remains characterized product traffic to `backend`. Shared observability is low-cardinality and payload-free: request path/query, headers, cookies, credentials, customer payloads and product identifiers are outside the shared telemetry contract.

Both composition roots use the shared Pingora server policy and bounded graceful shutdown. Process tests terminate successful children through the graceful path so LLVM coverage profiles can flush; emergency cleanup remains a test-harness fallback rather than the normal lifecycle.

## Packaging and release boundary

The current `Dockerfile` builds only `cwl-pingora-gateway`. It uses the digest-pinned Rust 1.98.0 Bookworm builder and a digest-pinned distroless Debian 13 `base-nossl` non-root runtime, copies only the required `libgcc_s` runtime library and gateway executable, and runs as uid/gid `65532`. CI exercises the image under a read-only root filesystem and least-privilege container settings.

The existence of `cwl-pingora-pg-erd-migration` in Cargo source is not deployable-artifact evidence. Dedicated one-binary-per-image packaging and invocation of the pg-erd binary under the same non-root/read-only-root/capability-free boundary is a descendant migration gate and must be proven on its own exact head before deployment credit.

A release remains blocked until exact-head CI, strict Clippy, warning-denied rustdoc, 100% owned production line/region coverage, load/runtime checks and supply-chain evidence are terminal GREEN; protected-branch review/governance is satisfied without bypass; the dependency policy is clean; and an immutable image digest, SBOM, provenance/reproducibility and rollback evidence exist. Source capability, predecessor GREEN or a mutable image/tag does not establish release, parity, shadow, canary, cutover or legacy-removal state.

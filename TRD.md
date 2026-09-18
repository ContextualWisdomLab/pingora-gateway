# Technical Requirements Document

## Runtime and composition roots

This branch uses Rust edition 2021 with manifest MSRV `1.98.0`. Cloudflare Pingora crates remain pinned to the exact upstream revision admitted by the parent stack; mutable branches, tags, contributor PRs, or local forks are not release authority.

There are two composition roots with intentionally different public contracts:

- `src/bin/cwl-pingora-gateway.rs` activates generic version-1 `GatewayConfig` and the single-upstream `GatewayProxy`.
- `src/bin/cwl-pingora-pg-erd-migration.rs` activates only bounded `PgErdMigrationConfig` and `MigrationGatewayProxy` for the characterized pg-erd edge surface.

Both parse and validate an explicit configuration before creating listeners and delegate serving/shutdown to Pingora. The pg-erd binary does not widen generic v1 into a product routing language. Product authentication/authorization, business logic, certificate issuance/ACME, Keyverse identity, Wardnet/EgressWeave policy, and consumer service discovery remain outside this runtime.

## Network and Admin Config authority

Generic v1 is strict YAML with unknown fields denied. It admits one explicit non-zero traffic listener, one distinct non-zero metrics listener, exactly one non-zero upstream authority, positive request-body/in-flight/keepalive budgets, and positive upstream I/O budgets. Listener and metrics validation rejects equal sockets, same-family wildcard/concrete aliases, native/IPv4-mapped IPv4 aliases, native or mapped wildcard aliases, and platform-dependent same-port IPv6-wildcard/IPv4 ambiguity while preserving distinct concrete non-aliased authority.

TLS upstreams require an admitted RFC 6066 DNS hostname for SNI/hostname verification and may optionally consume one absolute PEM trust-bundle path before listeners open. Clear-text upstreams may define neither SNI nor a trust bundle. The gateway does not issue, renew, rotate, or persist trust material.

`PgErdMigrationConfig` is a bounded Admin Config contract, not a generic route DSL. Operators may provide only the traffic/metrics sockets, positive runtime and keepalive budgets, and concrete transport/TLS values for the compiled `backend` and `frontend` identities. Route precedence, response-security policy, product auth, business routing, and arbitrary destinations are not operator-configurable. Missing, duplicate, extra, renamed, zero-port, recursive, multicast/broadcast, or otherwise invalid transport authority fails before listener activation.

The type is publicly deserializable, so `build_proxy()` revalidates the complete deterministic contract before creating delivery peers or infallible runtime limits. Direct deserialization cannot bypass version, listener, runtime, keepalive, or transport-authority invariants. Custom trust bytes are materialized exactly once during peer construction after deterministic validation and before listener registration.

## Version-2 response-body lifetime

Pg-erd version 1 preserves the existing unreleased characterization semantics and rejects `max_upstream_response_body_ms`. Version 2 requires an explicit positive `max_upstream_response_body_ms`; omission or zero is invalid. This version boundary prevents a timing policy from appearing through a hidden default.

`RuntimeIsolationLimits` carries the optional response-body lifetime only for the version-2 profile. `MigrationRequestContext` creates a `ResponseBodyLifetimeBudget` from those limits. `upstream_response_filter` starts the monotonic budget at the first non-informational upstream response header. Informational headers do not start it, and later final-header callbacks do not reset it. `upstream_response_body_filter` checks elapsed time only when a non-empty body chunk is observed; empty/end-of-stream bookkeeping cannot manufacture expiry.

A body-progress callback at or beyond the configured lifetime becomes an upstream-scoped fatal error. The lifetime is deliberately independent of Pingora peer `read_ms`: `read_ms` remains a per-read inactivity timer that resets after successful reads, while the version-2 lifetime bounds continuously progressing bodies at their first non-empty callback after the absolute lifetime is reached. With the pinned callback surface this is not an exact interrupt of an already-pending read, and slow delivery of an incomplete response header remains a separate gap.

Failure handling is response-phase-aware. Before a final downstream response has been written, existing `fail_to_proxy` behavior may still emit the policy-complete local error response for an upstream failure. After `Session::response_written()` reports a final response, the runtime returns error code 0 and writes no second status. A post-commit lifetime breach therefore terminates the incomplete response instead of rewriting it to 502 or routing to another pg-erd origin. The ordinary request context then releases its in-flight admission lease.

## Request, forwarding, and protocol policy

Every immutable Pingora peer uses `HttpUpstreamRequestPolicy::deny_upgrades()`. A separate transport-neutral request guard returns HTTP 501 for an HTTP/1 request carrying either `Upgrade` or a case-insensitive `upgrade` token in `Connection`, before request admission or upstream selection. This is deliberate protocol non-support, not WebSocket parity.

Generic v1 strips request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, and related proxy-identity fields before emitting only gateway-owned `Forwarded: proto=http`. It makes no client-identity or downstream proxy-provenance assertion.

The pg-erd migration adapter also removes request-controlled forwarding identity. It rebuilds only the characterized compatibility fields from accepted client transport plus validated request authority: `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host`, `X-Forwarded-Port`, and `X-Forwarded-Proto`. The currently characterized consumer entry point is clear-text `web`, so the admitted downstream scheme is `http`; HTTPS forwarding semantics require a separate downstream-TLS contract.

Non-health traffic acquires the process `max_in_flight_requests` lease before upstream selection. Saturation fails fast with HTTP 503 and increments bounded telemetry. A declared `Content-Length` above `max_request_body_bytes` fails with HTTP 413 before origin selection, and streamed body bytes are counted against the same limit. `/livez` and `/readyz` bypass application admission so process health remains observable under saturation.

## Routing, response policy, and delivery

`EdgeMigrationPlan` owns the characterized transport-neutral pg-erd route/policy composition. The route contract admits exact `/healthz -> backend`, raw `/api` prefix behavior including `/apiary -> backend`, and fallback `/ -> frontend` according to the captured consumer edge semantics. The response-policy contract owns only the characterized gateway response fields; it does not take product-domain response ownership.

`MigrationDeliveryPlan` binds each admitted upstream identity to exactly one prevalidated Pingora `HttpPeer`. Missing, duplicate, undeclared, recursive, or invalid transport bindings fail closed. Request data cannot select an arbitrary destination or create service-discovery authority.

The migration proxy applies response-security fields through replacement semantics. Upstream transport failures before response commitment use the bounded local error mapping; post-commit truncation, reset, or lifetime failure preserves the committed response rather than inventing a second status or silent failover.

## Health, observability, and graceful lifecycle

`GET /livez` and `/readyz` are process-local and return non-cacheable HTTP 200 through Pingora. They indicate validated process/configuration readiness, not consumer dependency health. Pg-erd consumer `/healthz` remains routed application traffic to `backend`.

Shared observability is low-cardinality and payload-free. The gateway records bounded request completion/error/body-byte/backpressure facts but excludes request paths, query strings, credentials, cookies, customer payloads, product identifiers, and unbounded labels. Pingora-family dependency diagnostics pass through the process-wide payload-safe logger so broad `RUST_LOG` settings cannot bypass this boundary.

The shared server policy makes one total upstream attempt, configures the admitted keepalive pool, applies an explicit five-second drain grace period, and uses the bounded runtime shutdown policy documented in `OPERABILITY.md`. Consumer retry/failover requires idempotency and product knowledge and is not inferred by the gateway.

## Packaging and supply chain

The Docker build admits only `cwl-pingora-gateway` or `cwl-pingora-pg-erd-migration` as build-time process identities and normalizes the selected executable into one distroless non-root runtime image. Exact-head OCI acceptance requires uid/gid `65532`, read-only root, dropped capabilities, `no-new-privileges`, and read-only configuration mounts. The pg-erd image must expose the traffic health endpoint and the separately published Prometheus metrics listener.

Supply-chain evidence remains exact-source-bound: committed lockfile, dependency/advisory policy, SBOM, both candidate image builds, and image vulnerability scans. These are candidate receipts, not immutable release identity. Protected promotion additionally requires immutable package/image digests, release-bound SBOM/provenance/attestation, reproducibility, rollback rehearsal, and current protected ancestry.

## Test and performance requirements

Every changed exact head must pass formatting, all-target compile/test, strict Clippy, warning-denied rustdoc, 100% owned-production line and region coverage without exclusions, realistic production-path traffic, OCI, supply-chain, and current-head review. Parent or historical GREEN never transfers after source, parent, or evidence changes.

The version-2 lifetime path requires both focused unit/config coverage and real slow-drip traffic. The real fixture must keep each upstream read inside `read_ms` while total body progress crosses `max_upstream_response_body_ms`, preserve committed 200/framing, terminate before the declared body completes, emit no second status or route failover, record exact error telemetry, retain `/readyz`, and prove an independent route can recover.

Applicable routed buyer paths use bounded Rust origins and k6/E2E with p95 below 20 ms on controlled loopback. Such loopback evidence is a regression gate only. Representative TLS, multi-hop, container/Kubernetes scheduling, origin-capacity, failure contention, and deployment traffic are required before production p95 credit; sample reduction, route omission, threshold weakening, or unrealistic warm-up are not repairs.

## Known limits and migration boundary

Generic v1 remains a clear-text downstream HTTP proxy with one explicit upstream per process. The pg-erd binary remains a bounded consumer-characterization adapter. Downstream TLS/H2, H2-to-H1 Cookie normalization, versioned WebSocket/Extended CONNECT, H3/QUIC, incomplete-response-header slow-drip, broader long-lived-stream semantics, dynamic reload, tracing, property/fuzz testing, and consumer-specific cutover behavior remain separate increments with their own RED-to-GREEN evidence.

Source capability is not parity or deployment state. Promotion still requires exact protected lineage, normal integration, immutable release identity, representative traffic/security/performance evidence, consumer shadow/canary, observed rollback, cutover, and verified legacy reverse-proxy removal.

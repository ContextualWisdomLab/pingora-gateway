# Technical Requirements

## Runtime

Rust edition 2021 with manifest MSRV `1.98.0` on this branch. Cloudflare Pingora and `pingora-prometheus` are pinned to exact public upstream revision `09696b51bc59315353d96686355861604d0bb48c`; mutable branch or contributor-PR dependencies are not release authority.

The generic production composition root is `src/bin/cwl-pingora-gateway.rs`. It parses one explicit `--config`, validates the transport-neutral contract before granting network authority, constructs `GatewayProxy`, exposes the dedicated metrics listener, adds the downstream TCP listener to Pingora `http_proxy_service`, and delegates serving and shutdown to Pingora's `Server` lifecycle.

The characterized `pg-erd-cloud` migration uses the separate `src/bin/cwl-pingora-pg-erd-migration.rs` composition root and `PgErdMigrationConfig`. Keeping a separate binary prevents the generic v1 contract from being widened into a product-routing configuration language. Product authentication/authorization, business routing, certificate issuance/ACME, Wardnet/EgressWeave policy, and Keyverse identity remain outside this process boundary.

## Contract

Generic configuration version 1 is strict YAML with `deny_unknown_fields`. Required top-level fields are `version`, `listener`, `metrics_listener`, `max_request_body_bytes`, `max_in_flight_requests`, `upstream_keepalive_pool_size`, and `upstreams`. Version 1 accepts exactly one upstream; it does not provide a generic product route table or request-controlled destination.

Traffic and metrics listeners must use non-zero, non-overlapping effective socket authority. Validation rejects same-family wildcard/concrete overlap, IPv6-wildcard/IPv4 dual-stack ambiguity, native IPv4 versus IPv4-mapped IPv6 aliases, and native/mapped or mapped-to-mapped IPv4 wildcard aliases while preserving distinct concrete non-aliased addresses. Request-body, in-flight, and keepalive-pool budgets must be positive.

Each upstream has a stable non-empty name, a non-zero concrete socket address, `tls`, optional `sni`, optional absolute `trust_bundle_file`, and explicit positive `connection_ms`, `total_connection_ms`, `read_ms`, `write_ms`, and `idle_ms` budgets. TLS upstreams require non-empty SNI. Pingora `HttpPeer` enables certificate and hostname verification. When `trust_bundle_file` is configured, the loaded PEM certificates become that peer's CA store rather than being silently merged with platform roots. Clear-text upstreams may define neither SNI nor a trust bundle. The gateway does not issue, renew, or rotate certificates.

`PgErdMigrationConfig` is a bounded Admin Config contract, not a second generic router. It admits operator-supplied listener/metrics sockets, non-zero runtime budgets, and concrete transport/TLS data only for the already characterized `backend` and `frontend` identities. Route selection and response-security policy remain compiled migration contracts. Missing, extra, renamed, zero-port, or overlapping transport authority fails before listener activation.

`PgErdMigrationConfig` also derives public Serde `Deserialize`; callers therefore are not forced through `PgErdMigrationConfig::from_yaml`. The public `build_proxy()` activation boundary revalidates the complete deterministic Admin Config contract before delivery peers or runtime limits are materialized. Only after that revalidation may `RuntimeIsolationLimits::from_validated` reuse the proven-positive budgets, so direct deserialization cannot bypass version, listener-authority, runtime, keepalive, or transport-authority invariants.

## Request policy

Every immutable Pingora upstream peer uses `HttpUpstreamRequestPolicy::deny_upgrades()`. This retains the pinned supplier's standard hop-by-hop and `Connection`-nomination sanitation but changes its HTTP/1 upgrade policy from the default `WebSocketOnly` behavior to `Deny`. The separate transport-neutral admission guard remains authoritative for returning HTTP 501 before application admission or origin selection. Keeping both boundaries aligned prevents callback/composition changes from implicitly enabling a supplier protocol capability that the versioned gateway contract does not admit.

The generic gateway additionally removes client-provided `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http` for the v1 clear-text downstream listener. Generic v1 deliberately makes no client-IP identity or downstream proxy-provenance claim.

The pg-erd migration adapter also discards request-controlled forwarding identity before rebuilding only the characterized compatibility fields from accepted transport/request authority. The current captured Traefik entry point is clear-text, so its forwarded scheme is explicitly `http`; HTTPS requires a separate TLS-derived contract rather than inference.

Non-health requests acquire the process `max_in_flight_requests` budget before upstream selection and fail closed with HTTP 503 at capacity. Requests with a parseable `Content-Length` above `max_request_body_bytes` fail with HTTP 413 before upstream selection; streamed body bytes are counted and fail with 413 if the same bound is exceeded. Pingora's parser retains its own finite protocol limits, but an operator-controlled smaller HTTP/1 header byte/count budget remains a separate edge-policy gap.

Generic v1 makes one prevalidated upstream peer available per request. The pg-erd migration adapter selects only peers already bound by `MigrationDeliveryPlan`; neither path performs request-controlled service discovery. Domain retries, failover, and idempotency policy are not invented by this runtime.

Failure handling is phase-aware. Before an upstream response header is committed downstream, transport failure may still be represented by the gateway's fail-closed error response under the one-attempt policy. After a valid response header has been committed, a later upstream framing/body failure cannot be rewritten into a second HTTP status or silently failed over: the incomplete downstream response terminates, low-cardinality error telemetry records the failed request, process readiness remains available, and independent routes must remain usable. This is an edge transport invariant, not product retry authority.

## Health and observability

`GET /livez` and `/readyz` return HTTP 200 with an empty, non-cacheable response through the process-local Pingora health boundary. Readiness proves validated configuration plus an active serving path, not product dependency health. In the pg-erd migration profile, consumer `/healthz` remains ordinary routed application traffic and is not confused with process liveness/readiness.

The shared process exposes bounded Prometheus counters for request completion, request errors, observed request-body bytes, and backpressure rejection. The canonical gateway log vocabulary records only low-cardinality transport completion facts; authorization headers, cookies, credentials, customer payloads, and unbounded product route labels are outside this shared observability contract.

## Packaging

The Docker builder is digest-pinned `rust:1.98.0-bookworm`; the final image is digest-pinned `gcr.io/distroless/base-nossl-debian13:nonroot`. The pinned Pingora OpenSSL path is vendored, so the final image does not carry Debian `libssl`. The Dockerfile exposes only one build-time selector, `CWL_GATEWAY_BIN`, and fail-closes unless its value is exactly `cwl-pingora-gateway` or `cwl-pingora-pg-erd-migration`. The selected executable is normalized to one fixed runtime path before the distroless stage, so a final image contains one admitted process identity rather than both binaries or a runtime shell selector.

Both image profiles run as uid/gid `65532`, have no intentional application writes, and are required to remain compatible with a read-only root filesystem, all capabilities dropped, and `no-new-privileges`. The OCI gate builds the default generic image and an explicit pg-erd image, then independently starts each exact candidate under those restrictions with a read-only configuration mount and requires `/livez` to become reachable. `examples/pg-erd-migration.yaml` is bounded smoke configuration for this process-level OCI proof; it does not substitute for routed origin/load/failure acceptance. Until the exact current head reaches terminal success, the new workflow is source-defined acceptance rather than hosted GREEN evidence.

The candidate supply-chain lane builds and vulnerability-scans both admitted images and binds both local image IDs plus per-image scan outputs to the exact source SHA. Its dependency SBOM describes the shared committed Rust dependency graph. A protected release still requires registry-bound immutable image digests, release-bound SBOM/provenance/reproducibility evidence, and rollback rehearsal; no Draft PR head or local image ID is a deployable release identity.

## Protocol and migration limits

Generic v1 is a clear-text downstream HTTP proxy with one explicit upstream per process. HTTP/1 Upgrade is explicitly denied both before request admission and at immutable peer construction; that is non-support evidence, not WebSocket parity. Downstream TLS termination, HTTP/2 admission, H2→H1 Cookie normalization, HTTP/3/QUIC, versioned WebSocket/Extended CONNECT, dynamic reload, Kubernetes Gateway API, and consumer-specific multi-route behavior are separate increments with realistic RED→GREEN evidence.

The concrete pg-erd migration stack is a bounded consumer-characterization adapter and does not widen generic v1. Source presence is not parity. Promotion still requires unchanged exact-head formatting, compile/test, strict Clippy, rustdoc, owned-production coverage, routed traffic/load/failure evidence, terminal dedicated OCI/supply-chain execution, immutable release identity, consumer deployment pin, shadow/canary, rollback rehearsal, protected cutover, and verified legacy removal.

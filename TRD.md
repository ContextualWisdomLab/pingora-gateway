# Technical Requirements

## Runtime

Rust edition 2021 with manifest MSRV `1.98.0` on this branch. The Pingora dependency is pinned to exact public upstream revision `09696b51bc59315353d96686355861604d0bb48c`; mutable branch or contributor-PR dependencies are not release authority. Compiler-repair Draft #56 separately moves release-producing paths to Rust 1.98.1 and must be adopted only after its own exact-head GREEN evidence.

The production composition root is `src/bin/cwl-pingora-gateway.rs`. It parses `--config`, validates the transport-neutral contract before granting network authority, constructs `GatewayProxy`, exposes the dedicated metrics listener, adds the downstream TCP listener to Pingora `http_proxy_service`, and delegates serving/shutdown to Pingora's `Server` lifecycle. Product-specific routing, authentication/authorization, certificate issuance/ACME, Wardnet/EgressWeave policy and Keyverse identity remain outside this process boundary.

## Contract

Configuration version 1 is strict YAML with `deny_unknown_fields`. Required top-level fields are `version`, `listener`, `metrics_listener`, `max_request_body_bytes`, `max_in_flight_requests`, `upstream_keepalive_pool_size`, and `upstreams`. Version 1 accepts exactly one upstream; it does not provide a generic product route table or request-controlled destination.

Traffic and metrics listeners must be distinct. Request-body, in-flight and keepalive-pool budgets must be positive. Each upstream has a stable non-empty name, a concrete socket address, `tls`, optional `sni`, optional absolute `trust_bundle_file`, and explicit positive `connection_ms`, `total_connection_ms`, `read_ms`, `write_ms`, and `idle_ms` budgets.

TLS upstreams require non-empty SNI. Pingora `HttpPeer` enables certificate and hostname verification. An optional trust bundle augments operator-supplied trust authority and is loaded before listener activation; a clear-text upstream may define neither SNI nor a trust bundle. The gateway does not issue, renew or rotate certificates.

## Request policy

Pingora's standard upstream request policy supplies the pinned supplier's hop-by-hop and Connection-nomination sanitation. The gateway additionally removes client-provided `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http` for the v1 clear-text downstream listener. Generic v1 deliberately makes no client-IP identity claim.

Non-health requests acquire the process `max_in_flight_requests` budget before upstream selection and fail closed with HTTP 503 at capacity. Requests with a parseable `Content-Length` above `max_request_body_bytes` fail with HTTP 413 before upstream selection; streamed body bytes are counted and fail with 413 if the same bound is exceeded. Pingora's parser retains its own finite protocol limits, but an operator-controlled smaller HTTP/1 header byte/count budget remains a separate supplier/edge-policy gap.

The generic adapter makes one prevalidated upstream peer available per request. Domain retries, failover and idempotency policy are not invented by this runtime.

## Health and observability

`GET /livez` and `/readyz` return HTTP 200 with an empty, non-cacheable response through the production Pingora path. Readiness proves validated configuration plus an active serving path, not product dependency health.

The shared process exposes bounded Prometheus counters for request completion, request errors, observed request-body bytes and backpressure rejection. The canonical gateway log vocabulary records only low-cardinality transport completion facts; authorization headers, cookies, credentials, customer payloads and unbounded product route labels are outside this shared observability contract.

## Packaging

The Docker builder is digest-pinned `rust:1.98.0-bookworm` on this branch. The final image is digest-pinned `gcr.io/distroless/base-nossl-debian13:nonroot`; the pinned Pingora OpenSSL path is vendored, so the final image does not carry Debian `libssl`. The runtime copies only the built gateway binary plus the required `libgcc_s.so.1`, runs as uid/gid `65532`, has no intentional application writes, and is required to remain compatible with a read-only root filesystem, dropped capabilities and `no-new-privileges` in executable OCI acceptance.

A committed lockfile, exact-head image build/runtime test, dependency/advisory/license checks, SBOM, provenance, reproducibility evidence, immutable image digest and rollback rehearsal remain release gates. No Draft PR head or mutable upstream branch is a deployable release identity.

## Protocol and migration limits

Generic v1 is a clear-text downstream HTTP proxy with one explicit upstream per process. Downstream TLS termination, HTTP/2 admission, H2→H1 Cookie normalization, HTTP/3/QUIC, WebSocket/Extended CONNECT, dynamic reload, Kubernetes Gateway API and consumer-specific multi-route behavior are versioned increments with separate realistic RED→GREEN evidence.

The concrete `pg-erd-cloud` migration stack is a separate bounded composition and does not widen this generic v1 contract. Any consumer cutover still requires executable legacy characterization, Pingora parity, immutable release identity, consumer deployment pin, shadow/canary, rollback rehearsal, protected cutover and legacy removal evidence.
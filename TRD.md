# Technical Requirements

## Runtime and dependency authority

The current branch manifest is Rust edition 2021 with MSRV `1.98.0`. `pingora = 0.9.0` and `pingora-prometheus = 0.9.0` are exact registry dependencies recorded by the committed Cargo lock; mutable supplier branches or contributor PRs are not release authority. The canonical promotion graph separately requires the Rust 1.98.1 release-compiler foundation before protected release because that compiler repairs the tracked 1.98.0 vtable-generation defect.

The generic production root is `src/bin/cwl-pingora-gateway.rs`. It parses one explicit `--config`, validates the transport-neutral contract, composes the runtime and one prevalidated upstream peer, materializes optional downstream TLS immediately before listener construction, exposes the dedicated one-worker metrics listener, and delegates serving/shutdown to Pingora.

The characterized pg-erd migration uses `src/bin/cwl-pingora-pg-erd-migration.rs` and `PgErdMigrationConfig`. The separate binary prevents generic edge configuration from becoming a product route language. Product authentication/authorization, business routing, certificate issuance/ACME/private-key custody, Wardnet/EgressWeave policy and Keyverse identity remain outside this process boundary.

## Admin Config and activation

Generic version 1 remains the cleartext one-upstream contract. Generic version 2 retains its network/runtime invariants and requires `downstream_tls`. Pg-erd version 1 remains the historical cleartext profile; version 2 additionally requires a positive `max_upstream_response_body_ms`; version 3 retains that response-lifetime contract and requires `downstream_tls`. Earlier versions reject fields introduced by later versions so timing or transport authority cannot change silently.

Both configuration aggregates reject unknown fields, zero or overlapping listener authority, invalid runtime budgets, missing/extra upstream authority and invalid TLS declarations before network activation. `service_threads` is bounded to 1–256 and becomes Pingora's global proxy worker count; Prometheus has a service-level one-worker override. Host CPU count is never inferred into configuration.

`DownstreamTlsConfig` is transport-neutral and owns only absolute certificate-chain/private-key references plus explicit ALPN policy. `tls_delivery` converts those references to Pingora `TlsSettings` once, checks certificate/private-key consistency and returns materialization failure before listener registration. Certificate lifecycle remains outside the gateway.

## Downstream TLS and HTTP protocol policy

The admitted downstream ALPN policy is `h2_http1`. It maps to Pingora 0.9.0 `TlsSettings::enable_h2()`, which prefers HTTP/2 and allows HTTP/1.1 fallback. TLS-enabled roots use `add_tls_with_settings`; cleartext versions continue to use `add_tcp`. h2c is not enabled and must not be inferred from HTTP/2 support.

Real-wire acceptance must verify the configured server identity against a local CA, assert negotiated ALPN and pass an actual request through the production composition path. HTTP/1.1 fallback must remain deliberate and separately exercised. Invalid, unreadable or mismatched certificate/key material must stop startup. Test child processes must be bounded and reaped on failure, timeout and panic paths.

Basic H2 wire traffic is not complete H2 operational parity. Issue #51 remains authoritative for parallel streams, stream reset/cancellation, GOAWAY/drain, header/body admission, flow control/backpressure, origin failure/recovery, forwarding trust, readiness, timing and cutover observability. H2-downstream to H1-upstream Cookie coalescing and zero-length body-framing remain release-gated by supplier `cloudflare/pingora#901` and `#936`; the gateway must not vendor their mutable contributor implementations.

HTTP/3 remains unsupported. A future H3 increment requires release-qualified server capability and a separate UDP/QUIC/QPACK/security/operability contract; TLS/H2 source presence must never be reported as H3 readiness.

## Request, forwarding and upstream policy

Each upstream has a stable non-empty identity, a concrete non-zero address, explicit TLS/SNI/trust data where applicable, and positive connection/total-connection/read/write/idle budgets. `read_ms` is a per-read inactivity budget rather than a whole-response deadline. Generic v1/v2 expose one prevalidated upstream per process. The pg-erd adapter selects only peers bound by its compiled migration plan. Neither path performs request-controlled service discovery.

Every immutable Pingora upstream peer uses `HttpUpstreamRequestPolicy::deny_upgrades()`. The transport-neutral admission boundary also rejects HTTP/1 Upgrade before application admission/origin selection. This is explicit non-support for WebSocket/HTTP Upgrade rather than partial parity.

Hostile `Forwarded`, `X-Forwarded-*` and `X-Real-IP` values are not trusted. The generic path emits only gateway-owned transport facts; the pg-erd path rebuilds only the characterized compatibility fields from accepted transport/request authority. TLS-enabled configurations derive `https` from the listener transport instead of request input.

Non-health requests acquire `max_in_flight_requests` before origin selection and fail with HTTP 503 at capacity. Body size is bounded by `max_request_body_bytes`. `/livez` and `/readyz` remain process-local and available during application saturation.

## Supplier-owned protocol gaps

HTTP/1 parser byte/count admission and whole-request-header lifetime occur before `ProxyHttp` callbacks and remain supplier capability roots. A callback-only semantic rejection is not parser/pre-allocation admission. HTTP/2 decoded header-list accounting is a different model and must not be presented as equivalent HTTP/1 wire-byte admission.

Shutdown correctness and configured worker topology are tracked separately from request protocol admission. Pingora 0.9.0 has already passed the exact consumer shutdown correctness fixture in the current stack; representative many-core/NUMA performance remains a distinct evidence requirement. Lower-pressure hosted evidence cannot be promoted into NUMA contention closure.

Pg-erd version 2/3 response-body lifetime begins at the first non-informational upstream response header and is evaluated on non-empty body progress. It complements per-read inactivity and is not an exact timer interrupt for a pending read. Incomplete upstream response-header slow delivery remains a separate transport root.

## Security and observability

Shared telemetry is low-cardinality and payload-free: request/error/body-byte/backpressure facts only. Authorization, cookies, tokens, arbitrary paths, customer payloads, trust-bundle contents, certificate private-key material and configuration credentials must not enter logs or metric labels. Pingora-family dependency logs remain subject to the repository logging policy.

Container candidates run as uid/gid 65532 with a read-only root filesystem, all capabilities dropped and `no-new-privileges`. The build-time selector admits only the two known binaries. Supply-chain evidence must remain source-bound, use the committed lock, scan both candidate images and produce SBOM/provenance appropriate to the promotion stage.

## Performance evidence

Exact-head load lanes use checksum-pinned k6 against release-built gateway candidates. Generic and bounded pg-erd loopback measurements are regression evidence, not production SLOs. Representative production evidence must retain real routing/TLS/network I/O, declared concurrency and sample floors. For TLS paths, new-connection/handshake latency and reused-connection latency must be distinguishable. Applicable buyer paths remain subject to the declared p95 `<20 ms` threshold without sample reduction or artificial warm-only measurement.

Issue #46 separately requires the representative configured-worker/NUMA shutdown profile: actual proxy worker count, registered service overrides, CPU/socket/NUMA topology, many parked/reused H1 keep-alives, repeated shutdown rounds, zero survivors, tail latency and complete scheduler/off-CPU evidence where available.

## Packaging and promotion

A PR head or local image ID is not a deployable release identity. Promotion requires unchanged exact-head formatting, locked compile/test, strict Clippy, warnings-denied rustdoc, complete owned-production coverage, protocol/traffic/failure evidence, rootless OCI execution and current supply-chain checks. Protected release then requires version/CHANGELOG, tag/package, immutable artifact digest, SBOM/provenance/reproducibility evidence and rollback.

Consumer migration requires a real immutable gateway identity, deployment pin, parity validation on the concrete edge, shadow/canary, observed rollback, cutover and verified legacy Nginx/OpenResty removal. The presence of downstream TLS/H2 source does not skip supplier #901/#936 or the remaining issue #51 protocol matrix.

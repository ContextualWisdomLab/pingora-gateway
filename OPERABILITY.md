# Operability and Recovery

## Start and configuration activation

Run the generic process as `cwl-pingora-gateway --config /path/to/gateway.yaml` and the bounded migration process as `cwl-pingora-pg-erd-migration --config /path/to/pg-erd-migration.yaml`. Both parse and validate configuration before granting listener authority. Invalid/missing configuration, zero/overlapping listener authority, invalid runtime budgets or invalid upstream transport data exits non-zero.

Generic version 1 remains cleartext. Generic version 2 requires downstream TLS. Pg-erd version 1 remains the historical cleartext profile; version 2 requires `max_upstream_response_body_ms`; version 3 retains that lifetime and additionally requires downstream TLS. Earlier versions reject fields introduced by later versions so transport/timing semantics cannot change silently.

`service_threads` is a bounded data-plane worker input (`1..=256`) passed to Pingora's global server configuration. The Prometheus service remains explicitly one worker. Do not infer worker count from host CPU count or report the scalar as the total process thread count.

## Downstream TLS operation

TLS-enabled deployments mount the certificate chain and private key read-only from the canonical certificate/secret owner and reference them with absolute paths. The gateway does not issue, renew, revoke, back up or rotate these materials itself. Do not bake private keys into an image layer or writable configuration volume.

During startup the transport-neutral TLS declaration is validated first. Immediately before listener construction, `tls_delivery` asks Pingora/OpenSSL to load the material and explicitly verifies certificate/private-key pairing. Any invalid, unreadable or mismatched material prevents the traffic listener from opening. Rotation is therefore deployment revision/restart work until a separately versioned dynamic-reload contract exists.

The admitted ALPN policy is `h2_http1`: HTTP/2 is preferred when offered and HTTP/1.1 remains allowed. h2c is not enabled. HTTP/3/QUIC remains unsupported and must not be advertised merely because TLS/H2 is available.

For TLS incidents, distinguish configuration-reference failure, file permission/mount failure, certificate/key mismatch, service-identity/certificate validation, ALPN negotiation and application/origin failure. Do not move private-key contents into logs to debug these classes. Exact material paths may be deployment-sensitive and should not become request labels or routine access-log fields.

## Health and backpressure

Both process identities reserve `/livez` and `/readyz` and return process-local health through Pingora. Readiness proves admitted configuration and serving-path availability, not product dependency health. In pg-erd, consumer `/healthz` remains routed application traffic to `backend` and is distinct from the gateway-local probes.

`max_in_flight_requests` limits concurrently admitted non-health requests. At capacity the gateway fails new application traffic with HTTP 503 and keeps health observable. `max_request_body_bytes` bounds declared and streamed request bodies. `upstream_keepalive_pool_size` bounds retained reusable upstream connections; it is not load-balancer or product retry policy.

Pg-erd version 2/3 `max_upstream_response_body_ms` limits monotonic body-delivery progress after the first non-informational upstream response header. It complements per-read `read_ms` and does not interrupt a pending read at an exact wall-clock instant. After a response header is committed, a later body failure terminates that incomplete response rather than fabricating another status or silently failing over.

## Forwarding and protocol boundary

Hostile `Forwarded`, `X-Forwarded-*`, `X-Real-IP` and legacy proxy identity are removed before the gateway rebuilds only accepted transport facts. Cleartext configurations derive scheme `http`; TLS-enabled configurations derive `https` from listener transport, not from request-controlled headers. These fields do not create user/tenant/auth identity.

HTTP/1 Upgrade remains denied before origin selection and at Pingora peer construction. WebSocket, HTTP/2 Extended CONNECT, h2c and HTTP/3 are not enabled by the TLS increment. Any future protocol transition requires an explicit versioned contract and traffic/drain/backpressure evidence.

The current downstream TLS/H2 stack has real-wire coverage for concurrent streams, reset/cancellation, GOAWAY/graceful drain, decoded header and request-body admission, stream- and connection-window flow control, bounded upstream read-ahead under exhausted connection credit, partial-request cleanup, origin failure before and after downstream response commitment, forwarding-scheme trust, and finite cutover transport telemetry. These source/runtime capabilities do not imply protected integration or production cutover. Generic client-IP/trusted-hop provenance is deliberately not synthesized without a separately admitted trust contract. H2→H1 Cookie/body-framing behavior remains supplier-gated until the applicable Pingora fixes have maintainer-integrated, release-qualified identities.

## Header/parser and shutdown residuals

HTTP/1 parser-phase byte/count admission and whole-request-header lifetime occur before gateway callbacks and remain supplier capability roots. A callback-only 431 or tiny per-read timeout is not equivalent resource admission and must not be presented as closure.

Pingora 0.9.0 has passed the current consumer parked-read shutdown correctness fixture, but issue #46 remains open for representative configured-worker/NUMA contention. Sibling #74 owns the evidence harness. Production closure requires actual process-allowed CPUs/socket/NUMA topology, configured proxy workers/service overrides, many parked/reused H1 keep-alives, repeated SIGTERM rounds, zero survivors, shutdown-tail distribution and complete scheduler/off-CPU evidence where available. Low-core hosted CI is characterization only.

SIGTERM continues through Pingora graceful termination. The gateway's request-drain/runtime timeout and the external supervisor budget are distinct controls. Do not shorten grace, disable keepalive or reduce worker/connection pressure to make shutdown tests pass.

## Logging and data minimization

Both production roots install the shared payload-safe logger before activation. `RUST_LOG` may change levels/targets, but Pingora-family message bodies pass through the repository redaction policy. Routine logs and Prometheus metrics are low-cardinality transport facts only. Authorization, cookies, tokens, request/response bodies, arbitrary product routes, customer payloads, trust-bundle contents and TLS private-key material are excluded.

The cutover counter `cwl_pingora_gateway_requests_by_transport_total` partitions completed requests only by the finite labels `outcome={ok,error}`, `protocol={h1,h2}`, and `transport={cleartext,tls}`. Protocol comes from the accepted Pingora session; TLS presence comes from its transport digest. These eight possible series are suitable for parity/shadow/canary comparisons without adding routes, hosts, client addresses, certificate paths, customer/product identifiers, deployment revisions, or any other unbounded label. The existing aggregate counters remain available for compatibility.

## Container operation

The Docker build selector admits only `cwl-pingora-gateway` and `cwl-pingora-pg-erd-migration`. Final candidates run as uid/gid `65532`, with read-only root filesystem, capabilities dropped and `no-new-privileges`. Mount config, upstream CA bundles and downstream certificate/key material read-only. The application has no intentional writable state.

OCI source acceptance starts both admitted profiles under those restrictions and checks process health. Supply Chain independently builds/scans both images and binds SBOM/image receipts to the exact source SHA. PR-local image IDs are not deployment identities.

## Performance operation

Controlled local k6 gates are regression bounds. The dedicated downstream TLS/H2 performance lane runs release-built gateway and bounded Rust origin candidates in two isolated process runs while preserving the generic load floor of 4 VUs and 400 iterations. Only health/readiness endpoints are touched before measurement; the application route is not warmed.

Fresh mode disables downstream connection reuse, requires every measured response to negotiate HTTP/2 and pay a non-zero TLS handshake, and records `connecting + tls_handshaking + duration` separately from TLS-handshake time. Reuse mode accepts only observations whose connection and TLS setup timings are both zero, so warm H2 requests cannot hide handshake cost. Both applicable p95 metrics retain the `<20 ms` gate, certificate verification stays enabled through the local CA, and exact-SHA JSON receipts are preserved. A hosted GREEN is a controlled loopback regression bound, not representative CPU/socket/NUMA, WAN, production shadow/canary or session-resumption evidence.

Worker performance reporting must include configured proxy worker count, registered service overrides and CPU/socket/NUMA topology. `service_threads` is not total OS process threads. Representative NUMA shutdown/contention profiling remains issue #46 work even after ordinary load and downstream TLS/H2 performance gates pass.

## Cutover and rollback

Keep the last known-good protected deployment manifest/image digest and executable legacy characterization until the migrated edge is verified. Promotion order is protected integration → immutable release identity → concrete parity → shadow/canary → observed rollback → cutover → verified legacy removal.

During parity, shadow and canary stages, compare the bounded transport counter by protocol/transport/outcome against the intended traffic mix. A nonzero H2/TLS sample proves that the new edge path is actually carrying completed requests; an error-rate shift can then be separated from unrelated H1/cleartext traffic. This metric does not by itself prove latency, handshake cost, deployment identity, rollback success or product correctness, so those evidence streams remain independently required.

Rollback restores the exact prior protected deployment revision; do not edit a live container. Certificate lifecycle, identity, product authorization/business policy and Wardnet/EgressWeave security-verdict ownership remain with their canonical owners during both cutover and rollback.

No consumer may rely on a PR head as its gateway dependency. A deployable release requires version/CHANGELOG/tag/package, immutable artifact digest, SBOM/provenance/reproducibility and rehearsed rollback. Downstream TLS/H2 source capability and hosted performance evidence do not waive release-qualified supplier framing, representative NUMA evidence, governance, immutable release, shadow/canary or consumer cutover requirements.
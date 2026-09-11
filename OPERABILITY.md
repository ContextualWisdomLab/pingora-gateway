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

Basic H2 traffic is not complete H2 operability. Before a production TLS/H2 cutover, issue #51 still requires concurrent-stream behavior, reset/cancellation, GOAWAY/graceful drain, header/body admission, flow control/backpressure, origin failure/recovery, forwarding trust and cutover telemetry. H2→H1 Cookie and zero-length body-framing paths remain supplier-gated by `cloudflare/pingora#901` and `#936` until release-qualified.

## Header/parser and shutdown residuals

HTTP/1 parser-phase byte/count admission and whole-request-header lifetime occur before gateway callbacks and remain supplier capability roots. A callback-only 431 or tiny per-read timeout is not equivalent resource admission and must not be presented as closure.

Pingora 0.9.0 has passed the current consumer parked-read shutdown correctness fixture, but issue #46 remains open for representative configured-worker/NUMA contention. Sibling #74 owns the evidence harness. Production closure requires actual process-allowed CPUs/socket/NUMA topology, configured proxy workers/service overrides, many parked/reused H1 keep-alives, repeated SIGTERM rounds, zero survivors, shutdown-tail distribution and complete scheduler/off-CPU evidence where available. Low-core hosted CI is characterization only.

SIGTERM continues through Pingora graceful termination. The gateway's request-drain/runtime timeout and the external supervisor budget are distinct controls. Do not shorten grace, disable keepalive or reduce worker/connection pressure to make shutdown tests pass.

## Logging and data minimization

Both production roots install the shared payload-safe logger before activation. `RUST_LOG` may change levels/targets, but Pingora-family message bodies pass through the repository redaction policy. Routine logs and Prometheus metrics are low-cardinality transport facts only. Authorization, cookies, tokens, request/response bodies, arbitrary product routes, customer payloads, trust-bundle contents and TLS private-key material are excluded.

## Container operation

The Docker build selector admits only `cwl-pingora-gateway` and `cwl-pingora-pg-erd-migration`. Final candidates run as uid/gid `65532`, with read-only root filesystem, capabilities dropped and `no-new-privileges`. Mount config, upstream CA bundles and downstream certificate/key material read-only. The application has no intentional writable state.

OCI source acceptance starts both admitted profiles under those restrictions and checks process health. Supply Chain independently builds/scans both images and binds SBOM/image receipts to the exact source SHA. PR-local image IDs are not deployment identities.

## Performance operation

Controlled local k6 gates are regression bounds. For TLS/H2, record new-connection/handshake latency separately from reused-connection traffic and keep actual routing/TLS I/O in the measured path. Do not hide handshake cost by reporting only warm reuse. Applicable buyer paths retain the declared p95 `<20 ms` threshold without reducing samples or concurrency.

Worker performance reporting must include configured proxy worker count, registered service overrides and CPU/socket/NUMA topology. `service_threads` is not total OS process threads. Representative NUMA shutdown/contention profiling remains issue #46 work even after ordinary load gates pass.

## Cutover and rollback

Keep the last known-good protected deployment manifest/image digest and executable legacy characterization until the migrated edge is verified. Promotion order is protected integration → immutable release identity → concrete parity → shadow/canary → observed rollback → cutover → verified legacy removal.

Rollback restores the exact prior protected deployment revision; do not edit a live container. Certificate lifecycle, identity, product authorization/business policy and Wardnet/EgressWeave security-verdict ownership remain with their canonical owners during both cutover and rollback.

No consumer may rely on a PR head as its gateway dependency. A deployable release requires version/CHANGELOG/tag/package, immutable artifact digest, SBOM/provenance/reproducibility and rehearsed rollback. Downstream TLS/H2 source capability does not waive supplier #901/#936 or the remaining issue #51 acceptance matrix.

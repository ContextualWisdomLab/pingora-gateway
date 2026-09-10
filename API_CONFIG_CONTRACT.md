# Configuration Contracts

## Shared invariants

Both production binaries parse strict YAML and reject unknown fields. Traffic and metrics listeners must be non-zero and must not overlap one effective socket authority, including wildcard/concrete and IPv4-mapped IPv6 aliases. Request-body, in-flight, data-plane worker and upstream keepalive budgets must be positive; `service_threads` is admitted only in `1..=256`. The gateway never derives worker count from host CPU count. The proxy service follows Pingora's global `ServerConf::threads`; the Prometheus service is explicitly overridden to one worker.

Every upstream has a stable non-empty name, a non-zero socket address, an explicit `tls` flag and positive connection/total-connection/read/write/idle budgets. TLS upstreams require non-empty SNI. Optional `trust_bundle_file` must be an absolute path and is read once during peer materialization before listener activation. Cleartext upstreams may specify neither SNI nor trust bundle. Request-controlled destinations are never admitted.

Pingora `read_ms` is a per-read inactivity budget, not a whole-response deadline. The gateway does not reinterpret it as an application SLO or total-response timer. `max_in_flight_requests` is a process-local admission budget for non-health requests; saturation fails fast with HTTP 503. `/livez` and `/readyz` remain process-local health paths and do not consume that budget.

## Generic `cwl-pingora-gateway`

### Version 1 — cleartext downstream

```yaml
version: 1
listener: 0.0.0.0:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
service_threads: 1
upstream_keepalive_pool_size: 32
upstreams:
  - name: application
    address: 10.0.0.20:8443
    tls: true
    sni: application.internal.example
    trust_bundle_file: /etc/cwl/application-ca.pem
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
```

Version 1 remains the original one-upstream cleartext downstream contract. It must not contain `downstream_tls`; attempting to add the field fails closed rather than silently changing transport semantics. The generic runtime has no route table, request-controlled destination, credentials, ACME, retry count, static-root, WebSocket switch or load-balancer policy.

For cleartext downstream traffic the forwarding policy removes request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP`, then emits only gateway-owned transport facts. It does not assert user identity.

### Version 2 — downstream TLS with H2/H1 ALPN

```yaml
version: 2
listener: 0.0.0.0:6443
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
service_threads: 4
upstream_keepalive_pool_size: 32
downstream_tls:
  certificate_chain_file: /etc/cwl/tls/server.crt
  private_key_file: /etc/cwl/tls/server.key
  alpn: h2_http1
upstreams:
  - name: application
    address: 10.0.0.20:8443
    tls: true
    sni: application.internal.example
    trust_bundle_file: /etc/cwl/application-ca.pem
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
```

Version 2 retains all version-1 generic network/runtime invariants and requires `downstream_tls`. Both certificate/key references must be non-empty absolute paths. Admin parsing validates only deterministic references; `tls_delivery` materializes them once immediately before listener construction and fails activation if Pingora/OpenSSL cannot load them or if the private key does not match the certificate. Certificate issuance, renewal, revocation, backup and private-key custody remain outside this repository.

The only admitted ALPN policy is `h2_http1`. The delivery adapter uses Pingora/OpenSSL's public ALPN callback surface to prefer `h2`, fall back to `http/1.1` only when the client actually offers it, and fail the TLS handshake for an ALPN-bearing client with no admitted protocol overlap or a malformed ALPN vector. This deliberately strengthens Pingora 0.9.0's `enable_h2()` convenience behavior to satisfy RFC 7301's fatal `no_application_protocol` requirement on no overlap. h2c is not enabled. HTTP/3/QUIC is not implied by this field and remains unsupported by this contract.

A version-2 source capability is not a complete mixed-protocol parity claim. Supplier `cloudflare/pingora#901` and `#936` continue to gate H2-downstream to H1-upstream Cookie/body-framing correctness until maintainer-integrated, release-qualified identities exist or an alternate deployment contract makes those downgrade paths unreachable.

## Bounded `cwl-pingora-pg-erd-migration`

This binary consumes a migration-specific Admin Config. It is not a generic multi-route language. Operator input can bind only listener/metrics authority, bounded runtime values, and concrete transport data for the already characterized `backend` and `frontend` upstream identities. Route rules and edge-owned response policy remain compiled migration contracts.

### Version 1 — historical cleartext profile

Version 1 preserves the original cleartext profile. It rejects `max_upstream_response_body_ms` and `downstream_tls`.

### Version 2 — cleartext plus response-body progress lifetime

```yaml
version: 2
listener: 0.0.0.0:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
max_upstream_response_body_ms: 30000
service_threads: 1
upstream_keepalive_pool_size: 32
upstreams:
  - name: backend
    address: 10.0.0.20:8000
    tls: false
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
  - name: frontend
    address: 10.0.0.21:3000
    tls: false
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
```

Version 2 requires a positive explicit `max_upstream_response_body_ms` and remains cleartext downstream. The value starts at the first non-informational upstream response header and is checked on non-empty body-progress callbacks; it is not an exact timer interrupt for a pending read. Version 2 rejects `downstream_tls`.

### Version 3 — response lifetime plus downstream TLS/H2

```yaml
version: 3
listener: 0.0.0.0:6443
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
max_upstream_response_body_ms: 30000
service_threads: 4
upstream_keepalive_pool_size: 32
downstream_tls:
  certificate_chain_file: /etc/cwl/tls/server.crt
  private_key_file: /etc/cwl/tls/server.key
  alpn: h2_http1
upstreams:
  - name: backend
    address: 10.0.0.20:8000
    tls: false
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
  - name: frontend
    address: 10.0.0.21:3000
    tls: false
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
```

Version 3 retains all version-2 response-lifetime semantics and additionally requires `downstream_tls` with the same read-only materialization and strict `h2_http1` policy as generic version 2. Earlier versions reject this field. Missing TLS material or missing response lifetime in version 3 fails before listener activation.

The characterized routing profile remains fixed: `/healthz` is routed to `backend`, the raw `/api` prefix contract routes to `backend`, and fallback routes to `frontend`; `/livez` and `/readyz` remain process-local. Product authentication/business rules, Keyverse identity, Wardnet/EgressWeave verdicts, arbitrary service discovery and request-controlled routing are not configurable here.

## Protocol admission and security boundaries

Every immutable Pingora upstream peer uses `HttpUpstreamRequestPolicy::deny_upgrades()`. The transport-neutral request guard also rejects HTTP/1 Upgrade attempts before application admission/origin selection. This is explicit non-support for WebSocket/HTTP Upgrade, not partial WebSocket parity.

Downstream HTTP/1 request-header byte/count admission and whole-header lifetime remain supplier-owned pre-callback constraints tracked separately. Callback-only validation is not credited as parser/pre-allocation admission. HTTP/2 decoded-header-list accounting is a different protocol model and must not be presented as equivalent to HTTP/1 wire/parser byte admission.

Downstream TLS/H2 source admission does not close the full HTTP/2 operational matrix. Real migration acceptance still requires the issue #51 matrix for concurrent streams, reset/cancellation, GOAWAY/drain, header/body limits, flow control/backpressure, origin failure/recovery, forwarding trust, timing and rollback/cutover observability. HTTP/3 remains a separate QUIC/UDP contract.

## Observability and data minimization

The shared process exposes bounded request/error/body-byte/backpressure metrics and low-cardinality transport completion logs. It does not log authorization headers, cookies, tokens, customer payloads, arbitrary request paths, trust-bundle contents, certificate private-key material or configuration credentials. TLS listener activation must not turn certificate paths or cryptographic material into metric labels or access-log content.

## Performance and release evidence

`service_threads` is Pingora's global data-plane worker input for services without a service-level override. The proxy follows it; the Prometheus service remains one worker. Values above 256 fail closed. The ceiling is a safety bound, not a deployment recommendation. Representative capacity evidence must record configured proxy workers, registered services/overrides and CPU/socket/NUMA topology rather than reporting `service_threads` as total OS threads.

Controlled loopback k6 evidence is a regression bound, not a production SLO. TLS/H2 buyer-path evidence must distinguish new-connection/handshake cost from reused-connection traffic and keep applicable routing/TLS I/O in the measurement. The repository's `<20 ms` gate applies only where the specific buyer path declares it and cannot be met by reducing samples/concurrency or measuring unrealistic warm-only traffic.

A PR head is not a deployable identity. Protected release promotion requires exact-head formatting/tests/Clippy/rustdoc/owned-production coverage, realistic traffic and failure evidence, rootless read-only OCI execution, current supply-chain scan, immutable artifact digest, SBOM/provenance/reproducibility evidence, rollback rehearsal, consumer deployment pin, shadow/canary, observed rollback, cutover and verified legacy removal.

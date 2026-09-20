# Versioned Configuration Contracts

## Generic `cwl-pingora-gateway` version 1

```yaml
version: 1
listener: 0.0.0.0:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
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

Unknown fields are rejected. `version` must be `1`. `listener`, `metrics_listener`, and `address` are socket addresses with non-zero ports. Port zero is rejected because production authority must remain operator-declared rather than OS-selected or unusable. Traffic and metrics listeners must not overlap one effective socket authority: equal sockets, same-port same-family wildcard/concrete aliases, exact native/IPv4-mapped aliases, native or mapped IPv4 wildcard aliases, and the platform-dependent same-port IPv6-wildcard/IPv4 combination fail closed. Distinct concrete non-aliased addresses may share a non-zero port. `max_request_body_bytes`, `max_in_flight_requests`, and `upstream_keepalive_pool_size` must all be positive. Generic v1 requires exactly one upstream and a non-empty stable upstream name. Every timeout must be positive.

The timeout fields map directly to the pinned Pingora peer options rather than defining a second gateway timer model. In particular, `read_ms` is a **per-read inactivity budget**: Pingora waits at most that long for each individual upstream `read()` and resets the timer after a successful read. It is not a total-response deadline. A connected upstream that sends no response bytes is therefore bounded by `read_ms`, while a slow-drip response can remain alive across multiple successful reads. Generic v1 has no whole-response lifetime and must not infer one from `read_ms`.

`max_in_flight_requests` is a process-local backpressure boundary for non-health downstream requests. When the budget is exhausted, the runtime fails fast with HTTP 503 instead of admitting unbounded work. `/livez` and `/readyz` bypass this application admission budget so saturation does not hide process health. The admission lease is released when the request context ends, including failed requests. `upstream_keepalive_pool_size` is wired directly into Pingora's `ServerConf`; the runtime does not inherit Pingora's framework default of 128 reusable upstream connections.

`tls: true` requires a non-empty RFC 6066 SNI `HostName`: an ASCII DNS hostname without a trailing root dot or literal IPv4/IPv6 address. Each DNS label must be 1–63 ASCII letters, digits, or hyphens, with an alphanumeric first and last byte; the complete textual name is limited to 253 bytes. Internationalized names must therefore arrive as ASCII IDNA A-labels rather than raw Unicode U-labels. Invalid SNI identity fails during transport-neutral configuration validation before Pingora peer construction or listener activation. Pingora then uses the admitted name for SNI and hostname verification together with certificate verification. `trust_bundle_file` is optional and, when present, must be an absolute path to a non-empty PEM certificate bundle readable during peer activation before listeners open. The bundle supplies trust anchors for that upstream instead of changing certificate-authority ownership: issuance and rotation remain external responsibilities. If `trust_bundle_file` is omitted, Pingora uses platform trust roots. `tls: false` forbids both `sni` and `trust_bundle_file`.

The generic contract does not include route tables, user-selected destinations, credentials, downstream certificates, ACME, retry counts, static roots, WebSocket switches, or load-balancer policy. Adding one of those fields changes public semantics and requires a versioned contract/ADR plus behavior tests.

Generic v1 downstream transport is cleartext TCP. Before proxying, the generic adapter removes request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http` to the upstream; it does not assert client identity. Upstream HTTP remains HTTP/1.1-only in this release line.

## Bounded `cwl-pingora-pg-erd-migration` candidate

The dedicated pg-erd migration binary consumes a different, migration-specific Admin Config profile. Version 1 preserves the existing unreleased characterization semantics and rejects `max_upstream_response_body_ms`. Version 2 is the explicit response-lifetime increment and requires a positive `max_upstream_response_body_ms`; the runtime never injects a hidden default.

```yaml
version: 2
listener: 0.0.0.0:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
max_upstream_response_body_ms: 30000
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

The numeric response-lifetime value above is illustrative configuration, not a production SLO. A deployment owner must choose the version-2 value from its observed long-response contract before canary or cutover.

`max_upstream_response_body_ms` starts at the first non-informational upstream response header. Runtime Isolation compares elapsed monotonic time only when a non-empty upstream body chunk is observed. Progress at or beyond the configured lifetime raises an upstream-scoped fatal error; empty/end-of-stream bookkeeping callbacks do not manufacture a timeout. If the final response has already been written, `fail_to_proxy` observes Pingora's `Session::response_written()` commitment state and emits no second status. Pre-commit upstream failures retain the existing policy-complete local error response behavior. No route failover is introduced by this contract.

This callback guard is not an exact timer interrupt. `read_ms` remains a per-read inactivity budget that resets after a successful read. A continuously progressing body is stopped at the first non-empty body callback at or beyond `max_upstream_response_body_ms`; a response that becomes quiescent is bounded by `read_ms`. The current callback surface does not wake a pending read exactly at the body-lifetime instant, and slow delivery of an incomplete response header remains a separate transport gap.

This is not a generic multi-route configuration language. Operator input can bind only concrete transport/TLS values for the compiled `backend` and `frontend` identities. Missing, extra, duplicate, renamed, port-zero, or otherwise invalid listener/metrics/upstream transport authorities fail closed before listener activation. Listener and metrics sockets consume the same effective-authority invariant as generic v1, while the migration profile keeps its specific zero-transport-authority error contract. Routes and edge-owned response fields are not configurable: the characterized profile fixes exact `/healthz -> backend`, raw `PathPrefix(`/api`) -> backend` semantics including `/apiary`, fallback `/ -> frontend`, and the four captured response fields `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and `Permissions-Policy: geolocation=(), microphone=(), camera=()`.

Admin parsing validates only deterministic configuration and authority invariants. It does not read custom trust-bundle bytes. If an admitted TLS upstream supplies `trust_bundle_file`, the canonical Pingora peer adapter reads and parses that material exactly once during `build_proxy`, still before listeners are registered. An unreadable or invalid bundle therefore blocks activation without a validate-then-reload trust-file window.

The migration adapter reserves `/livez` and `/readyz` as process-local Pingora health endpoints and does not route them to either consumer origin. The legacy consumer `/healthz` remains distinct routed application traffic to `backend`. Hostile request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, and `X-Forwarded-Server` identity is not trusted. `X-Forwarded-For` and `X-Real-IP` are rebuilt from the accepted client socket; `X-Forwarded-Host` preserves the original Host authority; `X-Forwarded-Port` uses the explicit Host port when present or the admitted scheme default otherwise. The process listener bind port is deliberately not external authority because container, Service, NAT, and port-publish layers may expose a different public port. The current captured Traefik entryPoint is cleartext `web`, so this candidate emits downstream scheme `http`. HTTPS/TLS listener behavior requires a separate executable contract.

The migration profile cannot configure product authentication/business rules, Keyverse identity, Wardnet/EgressWeave verdicts, certificate issuance/rotation, service discovery, arbitrary destinations, or Context Graph/EA state. Source-level listener capability is not release, deployment, parity, canary, cutover, or legacy-removal evidence.

## Shared observability boundary

Both binaries reserve `/livez` and `/readyz` for process health and use the dedicated metrics listener for low-cardinality Prometheus telemetry. Operators should normally bind metrics to loopback, a pod-only address, or another access-controlled observability network rather than the public traffic address. Shared application telemetry is limited to request count, request-error count, observed request-body bytes, and backpressure rejection count. Access logs record only response status, coarse success/error outcome, and observed request-body byte count; request URI, host, client identity, authorization, cookies, tokens, and configured credentials are intentionally absent.

# Version 1 Configuration Contracts

## Generic `cwl-pingora-gateway`

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

Unknown fields are rejected. `version` must be `1`. `listener`, `metrics_listener`, and `address` are socket addresses. Traffic and metrics listeners must be distinct. `max_request_body_bytes`, `max_in_flight_requests`, and `upstream_keepalive_pool_size` must all be positive. Generic v1 requires exactly one upstream and a non-empty stable upstream name. Every timeout must be positive.

`max_in_flight_requests` is a process-local backpressure boundary for non-health downstream requests. When the budget is exhausted, the runtime fails fast with HTTP 503 instead of admitting unbounded work. `/livez` and `/readyz` bypass this application admission budget so saturation does not hide process health. The admission lease is released when the request context ends, including failed requests. `upstream_keepalive_pool_size` is wired directly into Pingora's `ServerConf`; the runtime does not inherit Pingora's framework default of 128 reusable upstream connections.

`tls: true` requires non-empty `sni`, which Pingora uses for SNI and hostname verification together with certificate verification. `trust_bundle_file` is optional and, when present, must be an absolute path to a non-empty PEM certificate bundle readable during peer activation before listeners open. The bundle supplies trust anchors for that upstream instead of changing certificate-authority ownership: issuance and rotation remain external responsibilities. If `trust_bundle_file` is omitted, Pingora uses platform trust roots. `tls: false` forbids both `sni` and `trust_bundle_file`.

The generic contract does not include route tables, user-selected destinations, credentials, downstream certificates, ACME, retry counts, static roots, WebSocket switches, or load-balancer policy. Adding one of those fields changes public semantics and requires a versioned contract/ADR plus behavior tests.

Generic v1 downstream transport is cleartext TCP; the generic adapter strips inbound forwarding-identity headers and emits `Forwarded: proto=http` to the upstream. Upstream HTTP remains HTTP/1.1-only in this release line.

## Bounded `cwl-pingora-pg-erd-migration` candidate

The dedicated pg-erd migration binary consumes a different, migration-specific Admin Config profile. It deliberately reuses the same top-level deployment value names while admitting exactly two fixed transport authorities:

```yaml
version: 1
listener: 0.0.0.0:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
max_in_flight_requests: 128
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

This is not a generic multi-route configuration language. Operator input can bind only concrete transport/TLS values for the compiled `backend` and `frontend` identities. Missing, extra, duplicate, renamed, port-zero, or otherwise invalid listener/metrics/upstream transport authorities fail closed before listener activation. Port zero is rejected because this deployment contract requires stable operator-declared socket authority rather than an OS-selected ephemeral listener or an unusable upstream destination. Routes and edge-owned response fields are not configurable: the characterized profile fixes exact `/healthz -> backend`, raw `PathPrefix(`/api`) -> backend` semantics including `/apiary`, fallback `/ -> frontend`, and the four captured response fields `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and `Permissions-Policy: geolocation=(), microphone=(), camera=()`.

Admin parsing validates only deterministic configuration and authority invariants. It does not read custom trust-bundle bytes. If an admitted TLS upstream supplies `trust_bundle_file`, the canonical Pingora peer adapter reads and parses that material exactly once during `build_proxy`, still before listeners are registered. An unreadable or invalid bundle therefore blocks activation without a validate-then-reload trust-file window.

The migration adapter reserves `/livez` and `/readyz` as process-local Pingora health endpoints and does not route them to either consumer origin. The legacy consumer `/healthz` remains distinct routed application traffic to `backend`. Hostile request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, and `X-Forwarded-Server` identity is not trusted; characterized compatibility fields are rebuilt from accepted downstream transport/request authority. The current captured Traefik entryPoint is cleartext `web`, so this candidate emits downstream scheme `http`. HTTPS/TLS listener behavior requires a separate executable contract.

The migration profile cannot configure product authentication/business rules, Keyverse identity, Wardnet/EgressWeave verdicts, certificate issuance/rotation, service discovery, arbitrary destinations, or Context Graph/EA state. Source-level listener capability is not release, deployment, parity, canary, cutover, or legacy-removal evidence.

## Shared observability boundary

Both binaries reserve `/livez` and `/readyz` for process health and use the dedicated metrics listener for low-cardinality Prometheus telemetry. Operators should normally bind metrics to loopback, a pod-only address, or another access-controlled observability network rather than the public traffic address. Shared application telemetry is limited to request count, request-error count, observed request-body bytes, and backpressure rejection count. Access logs record only response status, coarse success/error outcome, and observed request-body byte count; request URI, host, client identity, authorization, cookies, tokens, and configured credentials are intentionally absent.
# Configuration Contracts

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

Unknown fields are rejected. `version` must be `1`. `listener`, `metrics_listener`, and `address` are socket addresses with non-zero ports. Port zero is rejected because this deployment contract requires stable operator-declared listener authority and a concrete connectable upstream rather than OS-selected ephemeral listener ports or unusable upstream destinations. Traffic and metrics listeners must not overlap one effective socket authority, and no upstream may overlap either gateway-owned listener. Equal addresses fail, same-port IPv4/IPv6 wildcard aliases fail, and an IPv6 wildcard plus an IPv4 authority on the same port fails closed because dual-stack bind behavior is platform-dependent. Distinct concrete IP addresses may use the same port. Rejecting listener/upstream overlap prevents a configured origin from recursively targeting the gateway's own traffic socket and prevents ordinary application routing from targeting the internal metrics surface. This does not prohibit two product-owned upstream identities from sharing an endpoint in a future multi-upstream contract; generic v1 still admits exactly one upstream. `max_request_body_bytes`, `max_in_flight_requests`, and `upstream_keepalive_pool_size` must all be positive. Generic v1 requires exactly one upstream and a non-empty stable upstream name. Every timeout must be positive.

The timeout fields map directly to the pinned Pingora peer options rather than defining a second gateway timer model. In particular, `read_ms` is a **per-read inactivity budget**: Pingora waits at most that long for each individual upstream `read()` and resets the timer after a successful read. It is not a total-response deadline. A connected upstream that sends no response bytes is therefore bounded by `read_ms`, while a slow-drip response can remain alive across multiple successful reads. Generic v1 still has no whole-response lifetime and must not infer one from `read_ms`.

`max_in_flight_requests` is a process-local backpressure boundary for non-health downstream requests. When the budget is exhausted, the runtime fails fast with HTTP 503 instead of admitting unbounded work. `/livez` and `/readyz` bypass this application admission budget so saturation does not hide process health. The admission lease is released when the request context ends, including failed requests. `upstream_keepalive_pool_size` is wired directly into Pingora's `ServerConf`; the runtime does not inherit Pingora's framework default of 128 reusable upstream connections.

Generic v1 has no operator-facing HTTP/1 request-header byte/count field. At the pinned Pingora revision, finite HTTP/1 parser ceilings are supplier constants applied before the `ProxyHttp` request callback; a later `request_filter()` size check can express application semantics but cannot be credited as parser/pre-allocation admission. Issue #43 and supplier `cloudflare/pingora#993` track a supported parser-phase control. Until an immutable supplier capability exists, adding a speculative field here would create a configuration promise the runtime cannot enforce at the required phase. HTTP/2 decoded-header-list limits are a different protocol accounting model and must not be reused as HTTP/1 wire/parser bytes.

`tls: true` requires non-empty `sni`, which Pingora uses for SNI and hostname verification together with certificate verification. `trust_bundle_file` is optional and, when present, must be an absolute path to a non-empty PEM certificate bundle readable during peer activation before listeners open. The bundle supplies trust anchors for that upstream instead of changing certificate-authority ownership: issuance and rotation remain external responsibilities. If `trust_bundle_file` is omitted, Pingora uses platform trust roots. `tls: false` forbids both `sni` and `trust_bundle_file`.

The generic contract does not include route tables, user-selected destinations, credentials, downstream certificates, ACME, retry counts, static roots, WebSocket switches, or load-balancer policy. Adding one of those fields changes public semantics and requires a versioned contract/ADR plus behavior tests.

Generic v1 downstream transport is cleartext TCP. Before proxying, the generic adapter removes request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http` to the upstream; it does not assert client identity. Upstream HTTP remains HTTP/1.1-only in this release line.

## Bounded `cwl-pingora-pg-erd-migration` candidate

The dedicated pg-erd migration binary consumes a different, migration-specific Admin Config profile. Version 1 remains readable only to preserve the existing unreleased characterization stack. Version 2 is the opt-in response-lifetime increment and requires an explicit `max_upstream_response_body_ms`; version 1 rejects that field so the old contract cannot silently acquire new timing semantics.

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

The numeric value above is an illustrative configuration example, not a pg-erd production SLO. A deployment owner must choose the value from its observed long-response contract before canary or cutover. Version 2 rejects zero or a missing response-body lifetime rather than substituting a hidden default.

`max_upstream_response_body_ms` starts when Pingora invokes the upstream-response-header filter for the first non-informational response, before its body-progress callbacks are processed. At each upstream response-body progress callback, Runtime Isolation compares elapsed monotonic time with the configured lifetime. Once the lifetime is reached, the callback raises an upstream-scoped fatal error. If the response status/header was already committed, the gateway terminates that downstream response instead of inventing a second status or silently routing to the other pg-erd origin. The ordinary request context then drops its in-flight admission lease.

This callback guard is deliberately not described as an exact timer interrupt. At the pinned Pingora revision, `read_ms` still applies independently to each upstream read and resets after a successful read. A continuously progressing body is therefore stopped at the first body callback at or beyond `max_upstream_response_body_ms`; a response that becomes quiescent is bounded by `read_ms`. The current callback surface does not wake a pending read at the absolute body-lifetime instant, and slow-drip of an incomplete **response header** remains a separate transport gap. Neither limitation may be hidden in parity or production-SLO claims.

The request-header parser-admission gap above also applies to pg-erd versions 1 and 2. Neither version may silently acquire a header-budget field before the supplier exposes an enforceable parser-phase hook and a deliberate later Admin Config version adopts it with real-listener RED→GREEN evidence. The fixed current Pingora HTTP/1 ceiling is not a migration-specific configurable budget, and callback-only rejection is not equivalent resource evidence.

This is not a generic multi-route configuration language. Operator input can bind only concrete transport/TLS values for the compiled `backend` and `frontend` identities. Missing, extra, duplicate, renamed, port-zero, or otherwise invalid transport authority fails closed before listener activation. The migration profile reuses the same effective network-authority invariant as the generic contract: traffic and metrics listeners cannot overlap, and neither characterized upstream may overlap the traffic listener or metrics listener through an exact address, a same-port wildcard alias, or the conservative IPv6-wildcard/IPv4 dual-stack case. Distinct concrete IP authorities on the same port remain valid. Port zero is rejected because this deployment contract requires stable operator-declared socket authority rather than an OS-selected ephemeral listener or an unusable upstream destination. Routes and edge-owned response fields are not configurable: the characterized profile fixes exact `/healthz -> backend`, raw `PathPrefix(`/api`) -> backend` semantics including `/apiary`, fallback `/ -> frontend`, and the four captured response fields `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and `Permissions-Policy: geolocation=(), microphone=(), camera=()`.

Admin parsing validates only deterministic configuration and authority invariants. It does not read custom trust-bundle bytes. If an admitted TLS upstream supplies `trust_bundle_file`, the canonical Pingora peer adapter reads and parses that material exactly once during `build_proxy`, still before listeners are registered. An unreadable or invalid bundle therefore blocks activation without a validate-then-reload trust-file window.

The migration adapter reserves `/livez` and `/readyz` as process-local Pingora health endpoints and does not route them to either consumer origin. The legacy consumer `/healthz` remains distinct routed application traffic to `backend`. Hostile request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, and `X-Forwarded-Server` identity is not trusted; characterized compatibility fields are rebuilt from accepted downstream transport/request authority. The current captured Traefik entryPoint is cleartext `web`, so this candidate emits downstream scheme `http`. HTTPS/TLS listener behavior requires a separate executable contract.

The migration profile cannot configure product authentication/business rules, Keyverse identity, Wardnet/EgressWeave verdicts, certificate issuance/rotation, service discovery, arbitrary destinations, or Context Graph/EA state. Source-level listener capability is not release, deployment, parity, canary, cutover, or legacy-removal evidence.

## Shared observability boundary

Both binaries reserve `/livez` and `/readyz` for process health and use the dedicated metrics listener for low-cardinality Prometheus telemetry. Operators should normally bind metrics to loopback, a pod-only address, or another access-controlled observability network rather than the public traffic address. The Admin Config invariant also prevents either application upstream from resolving back to that metrics socket, so product routing cannot accidentally proxy ordinary traffic into the gateway-owned metrics service. Shared application telemetry is limited to request count, request-error count, observed request-body bytes, and backpressure rejection count. Access logs record only response status, coarse success/error outcome, and observed request-body byte count; request URI, host, client identity, authorization, cookies, tokens, and configured credentials are intentionally absent.

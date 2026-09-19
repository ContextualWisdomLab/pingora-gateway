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

Unknown fields are rejected. `version` must be `1`. `listener`, `metrics_listener`, and `address` are socket addresses with non-zero ports. Port zero is rejected because production authority must remain operator-declared rather than OS-selected or unusable. Listener addresses may intentionally use bind wildcards, but an upstream `address` must identify a concrete remote authority: unspecified `0.0.0.0` and `[::]` upstream addresses are rejected rather than passed to Pingora as destinations. Traffic and metrics listeners must not overlap one effective socket authority: equal sockets, same-port same-family wildcard/concrete aliases, exact native/IPv4-mapped aliases, native or mapped IPv4 wildcard aliases, and the platform-dependent same-port IPv6-wildcard/IPv4 combination fail closed. An upstream authority must also remain disjoint from both process-owned listeners after the same alias canonicalization; a configuration that points application traffic back at the gateway listener or its metrics socket is rejected before activation to prevent recursive self-proxying or accidental metrics exposure. Distinct concrete non-aliased addresses may share a non-zero port. `max_request_body_bytes`, `max_in_flight_requests`, and `upstream_keepalive_pool_size` must all be positive. Generic v1 requires exactly one upstream and a non-empty stable upstream name. Every timeout must be positive.

The timeout fields map directly to the pinned Pingora peer options rather than defining a second gateway timer model. In particular, `read_ms` is a **per-read inactivity budget**: Pingora waits at most that long for each individual upstream `read()` and resets the timer after a successful read. It is not a total-response deadline. A connected upstream that sends no response bytes is therefore bounded by `read_ms`, while a slow-drip response can remain alive across multiple successful reads. Whole-response lifetime remains an explicit open runtime-isolation requirement and must not be inferred from `read_ms`.

`max_in_flight_requests` is a process-local backpressure boundary for non-health downstream requests. When the budget is exhausted, the runtime fails fast with HTTP 503 instead of admitting unbounded work. `/livez` and `/readyz` bypass this application admission budget so saturation does not hide process health. Every other request remains inside the budget even when the gateway answers locally rather than selecting an upstream, including malformed or exhausted TRACE/OPTIONS `Max-Forwards` requests. The admission lease is released when the request context ends, including failed requests. `upstream_keepalive_pool_size` is wired directly into Pingora's `ServerConf`; the runtime does not inherit Pingora's framework default of 128 reusable upstream connections.

`tls: true` requires non-empty `sni`, which Pingora uses for SNI and hostname verification together with certificate verification. `trust_bundle_file` is optional and, when present, must be an absolute path to a non-empty PEM certificate bundle readable during peer activation before listeners open. The bundle supplies trust anchors for that upstream instead of changing certificate-authority ownership: issuance and rotation remain external responsibilities. If `trust_bundle_file` is omitted, Pingora uses platform trust roots. `tls: false` forbids both `sni` and `trust_bundle_file`.

The generic contract does not include route tables, user-selected destinations, credentials, downstream certificates, ACME, retry counts, static roots, WebSocket switches, or load-balancer policy. Adding one of those fields changes public semantics and requires a versioned contract/ADR plus behavior tests.

Generic v1 downstream transport is cleartext TCP. Before proxying, the generic adapter removes request-controlled `Forwarded`, every header in the `X-Forwarded-*` namespace, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http` to the upstream. This includes compatibility fields such as `X-Forwarded-Prefix` and identity-bearing fields such as `X-Forwarded-Client-Cert`; generic v1 asserts neither client IP, externally visible path prefix, nor client-certificate identity. Headers outside that namespace remain application metadata. RFC 9110 `Via` is protocol trace rather than trusted identity. On forwarded requests, the received chain is preserved and the gateway appends a pseudonymous `cwl-pingora-gateway` hop using the actual downstream HTTP protocol version. On forwarded upstream responses, the received response chain is likewise preserved and the gateway appends its hop using the upstream response protocol version. Locally generated health responses are not forwarded and do not receive this trace. `Via` must not be consumed as authentication, authorization, or trusted-proxy evidence. Upstream HTTP remains HTTP/1.1-only in this release line.

For TRACE and OPTIONS, an inbound `Max-Forwards` field is intermediary control rather than application metadata. When present it must be exactly one decimal value. Positive values are decremented before forwarding and capped at the generic gateway's supported maximum of 255. Zero is never forwarded: generic v1 becomes the final recipient and returns an empty HTTP 501 because local TRACE echo and resource-specific OPTIONS semantics are not implemented. Malformed or duplicate values fail closed with HTTP 400. Those local 400/501 outcomes remain ordinary application traffic for `max_in_flight_requests` and declared-body admission; only process health bypasses the application budget. `Max-Forwards` on other methods is left unchanged under RFC 9110's permission to ignore it there.

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

This is not a generic multi-route configuration language. Operator input can bind only concrete transport/TLS values for the compiled `backend` and `frontend` identities. Missing, extra, duplicate, renamed, port-zero, unspecified-address, listener-overlapping, or otherwise invalid listener/metrics/upstream transport authorities fail closed before listener activation. Listener, metrics, and upstream sockets consume the same effective-authority invariant as generic v1, including native/IPv4-mapped aliases and wildcard overlap, while the migration profile keeps its specific zero-transport-authority error contract. Routes and edge-owned response fields are not configurable: the characterized profile fixes exact `/healthz -> backend`, raw `PathPrefix(`/api`) -> backend` semantics including `/apiary`, fallback `/ -> frontend`, and the four captured response fields `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and `Permissions-Policy: geolocation=(), microphone=(), camera=()`.

Admin parsing validates only deterministic configuration and authority invariants. It does not read custom trust-bundle bytes. If an admitted TLS upstream supplies `trust_bundle_file`, the canonical Pingora peer adapter reads and parses that material exactly once during `build_proxy`, still before listeners are registered. An unreadable or invalid bundle therefore blocks activation without a validate-then-reload trust-file window.

The migration adapter reserves `/livez` and `/readyz` as process-local Pingora health endpoints and does not route them to either consumer origin. The legacy consumer `/healthz` remains distinct routed application traffic to `backend`. Hostile request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP`, and `X-Forwarded-Server` identity is not trusted. `X-Forwarded-For` and `X-Real-IP` are rebuilt from the accepted client socket; `X-Forwarded-Host` preserves the original Host authority; `X-Forwarded-Port` uses the explicit Host port when present or the admitted scheme default otherwise. Before Host is promoted into trusted compatibility metadata, it must satisfy RFC 9110 `Host = uri-host [ ":" port ]` using RFC 3986 host syntax; userinfo, path/query delimiters, malformed percent escapes, non-IP bracket literals, empty/zero/invalid explicit ports, and unbracketed IPv6 therefore fail closed with HTTP 400. Valid bracketed IPv6/IPvFuture literals and valid `reg-name` syntax remain admissible. The process listener bind port is deliberately not external authority because container, Service, NAT, and port-publish layers may expose a different public port. Before any admitted pg-erd request is forwarded, the adapter preserves the received RFC 9110 `Via` chain and appends one pseudonymous `cwl-pingora-gateway` hop using the actual downstream session protocol. This request-side trace is required HTTP-to-HTTP intermediary metadata and is never trusted as identity, authentication, authorization, or proxy provenance. The pg-erd response path does not add `Via` in this increment; response-side Via is optional for an HTTP gateway. The current captured Traefik entryPoint is cleartext `web`, so this candidate emits downstream scheme `http`. HTTPS/TLS listener behavior requires a separate executable contract.

Pg-erd consumes the same internal RFC 9110 `Max-Forwards` parser/decrement policy as generic v1. TRACE/OPTIONS with one positive decimal value are decremented immediately before upstream forwarding and capped at 255. Zero is never forwarded: after the ordinary in-flight lease and declared-body admission succeed, the migration gateway answers locally with empty HTTP 501 and still applies the characterized response-security fields. Malformed or duplicate TRACE/OPTIONS values fail closed with HTTP 400 only after the same application admission and declared-body boundary, so an already oversized declared body retains HTTP 413 precedence. `Max-Forwards` on other methods is not interpreted as intermediary control and is forwarded unchanged. This adds no route, product OPTIONS/TRACE behavior, authentication, authorization, or upstream authority.

The migration profile cannot configure product authentication/business rules, Keyverse identity, Wardnet/EgressWeave verdicts, certificate issuance/rotation, service discovery, arbitrary destinations, or Context Graph/EA state. Source-level listener capability is not release, deployment, parity, canary, cutover, or legacy-removal evidence.

## Shared observability boundary

Both binaries reserve `/livez` and `/readyz` for process health and use the dedicated metrics listener for low-cardinality Prometheus telemetry. Operators should normally bind metrics to loopback, a pod-only address, or another access-controlled observability network rather than the public traffic address. Shared application telemetry is limited to request count, request-error count, observed request-body bytes, and backpressure rejection count. Access logs record only response status, coarse success/error outcome, and observed request-body byte count; request URI, host, client identity, authorization, cookies, tokens, and configured credentials are intentionally absent.

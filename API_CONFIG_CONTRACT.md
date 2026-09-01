# Version 1 Configuration Contract

```yaml
version: 1
listener: 0.0.0.0:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
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

Unknown fields are rejected. `version` must be `1`. `listener`, `metrics_listener`, and `address` are socket addresses. Traffic and metrics listeners must be distinct. `max_request_body_bytes` must be positive. V1 requires exactly one upstream and a non-empty stable upstream name. Every timeout must be positive.

`tls: true` requires non-empty `sni`, which Pingora uses for SNI and hostname verification together with certificate verification. `trust_bundle_file` is optional and, when present, must be an absolute path to a non-empty PEM certificate bundle that is readable before listeners open. The bundle supplies trust anchors for that upstream instead of changing certificate-authority ownership: issuance and rotation remain external responsibilities. If `trust_bundle_file` is omitted, Pingora uses the platform trust roots. `tls: false` forbids both `sni` and `trust_bundle_file`.

The contract does not include route tables, user-selected destinations, credentials, downstream certificates, ACME, retry counts, static roots, WebSocket switches, or load-balancer policy. Adding one of those fields changes product semantics and requires a versioned ADR plus behavior tests.

Operational endpoints `/livez` and `/readyz` are reserved by the traffic runtime and are not proxied. The dedicated metrics listener serves Pingora's Prometheus application; operators should normally bind it to loopback, a pod-only address, or another access-controlled observability network rather than the public traffic address. Metrics are deliberately label-free in the initial vertical: request count, request-error count, and observed request-body bytes. Access logs record only response status, coarse success/error outcome, and observed request-body byte count; request URI, host, client identity, authorization, cookies, tokens, and configured credentials are not logged by the gateway application.

V1 downstream transport is cleartext TCP; the gateway strips inbound forwarding-identity headers and emits `Forwarded: proto=http` to the upstream. Upstream HTTP remains HTTP/1.1-only in this release line.

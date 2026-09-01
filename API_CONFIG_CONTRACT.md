# Version 1 Configuration Contract

```yaml
version: 1
listener: 0.0.0.0:6188
max_request_body_bytes: 1048576
upstreams:
  - name: application
    address: 10.0.0.20:8443
    tls: true
    sni: application.internal.example
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
```

Unknown fields are rejected. `version` must be `1`. `listener` and `address` are socket addresses. `max_request_body_bytes` must be positive. V1 requires exactly one upstream and a non-empty stable upstream name. Every timeout must be positive.

`tls: true` requires non-empty `sni`, which Pingora uses for SNI and hostname verification together with certificate verification. `tls: false` forbids `sni`.

The contract does not include route tables, user-selected destinations, credentials, downstream certificates, ACME, retry counts, static roots, WebSocket switches, or load-balancer policy. Adding one of those fields changes product semantics and requires a versioned ADR plus behavior tests.

Operational endpoints `/livez` and `/readyz` are reserved by the runtime and are not proxied. V1 downstream transport is cleartext TCP; the gateway strips inbound forwarding-identity headers and emits `Forwarded: proto=http` to the upstream.

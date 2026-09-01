# Architecture

The product responsibility is a reusable edge runtime, not a central home for product behavior.

```text
versioned YAML -> Runtime Configuration -> Edge Routing aggregate
                                      \-> Pingora Delivery ACL -> explicit upstream
                                      \-> Telemetry
```

**Core subdomain — Edge Routing.** `RouteTable` owns route uniqueness and deterministic exact-host/longest-prefix selection. `Route`, `RouteId` and `UpstreamTarget` are domain values/entities. It cannot import Pingora.

**Supporting subdomain — Runtime Configuration.** Parses the v1 contract, resolves upstreams before bind, enforces route/resource ceilings and explicit private-network opt-in, then constructs domain values.

**Generic subdomain — Telemetry.** Process counters and structured logs use bounded fields only.

**Delivery adapter / Anti-Corruption Layer.** `src/delivery/pingora_proxy.rs` maps HTTP/Pingora types to domain inputs and converts selected domain routes into `HttpPeer`. It owns transport timeouts, TLS verification and sanitization of untrusted forwarded headers.

Certificate issuance/rotation is external. TLS termination, static serving, WebSocket, dynamic reload, advanced load balancing and Kubernetes Gateway API are future bounded increments only when a consumer proves the need.

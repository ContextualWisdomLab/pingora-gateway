# Architecture

## Domain shape

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It does not own any consumer Core Domain.

The **Edge Contract** bounded context owns admission of network authority. `GatewayConfig` is the aggregate root: its invariants decide whether a process may listen and which upstream it may contact. `UpstreamConfig` is a configuration value with a stable name inside that aggregate; it has no independent lifecycle in v1. Listener address, TLS identity, request-body budget, and timeout budgets are value objects conceptually, represented by Rust primitives/structs where a richer type would not yet add an invariant.

The **Pingora Delivery** adapter maps admitted values to `HttpPeer`, proxy callbacks, header policy, health responses, and request-body enforcement. Pingora types never cross into the edge contract. `GatewayCommand` is the application startup service that reads and validates configuration before the composition root grants network authority.

There is no domain event in immutable startup-only v1 because nothing in the domain changes after activation. Dynamic reload, if introduced, would require explicit configuration-accepted/rejected lifecycle events rather than retrofitting hidden mutation.

## Dependency direction

```text
operator YAML
    |
    v
startup application service --> Edge Contract bounded context
                                      |
                                      v
                               Pingora Delivery adapter
                                      |
                                      v
                               Cloudflare Pingora
```

Consumer repositories depend on documented image/config/deployment contracts, not Rust internals. Product-specific behavior stays upstream or in a separately justified adapter owned by that consumer.

## Anti-corruption boundary

`edge_contract` is the anti-corruption boundary against Pingora-specific transport semantics. `pingora_delivery` may translate to `HttpPeer`; `gateway_proxy` may implement `ProxyHttp`. Reversing this dependency is a DDD defect.

# Architecture

## Domain shape

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It does not own any consumer Core Domain.

The **Edge Contract** bounded context owns admission of network authority. `GatewayConfig` is the aggregate root: its invariants decide whether a process may listen and which upstream it may contact. `UpstreamConfig` is a configuration value with a stable name inside that aggregate; it has no independent lifecycle in v1. Listener address, TLS identity, request-body budget, and timeout budgets are value objects conceptually, represented by Rust primitives/structs where a richer type would not yet add an invariant.

The **Edge Routing** bounded context characterizes deterministic request-path selection among explicitly named upstream identities. It is transport-neutral and currently not connected to the v1 one-upstream runtime. Consumer-derived path contracts may be represented here only when they express shared edge responsibility; product authorization and business routing remain in the consumer bounded context.

The **HTTP Policy** bounded context characterizes explicit edge-owned HTTP response mutations independently from route selection and Pingora delivery. The current candidate records response-security fields already emitted by a consumer edge. It does not own application response semantics, authentication/authorization, Wardnet/EgressWeave verdicts, or Keyverse identity, and it is not yet activated in Pingora callbacks.

The **Pingora Delivery** adapter maps admitted values to `HttpPeer`, proxy callbacks, header policy, health responses, and request-body enforcement. Pingora types never cross into the transport-neutral bounded contexts. `GatewayCommand` is the application startup service that reads and validates configuration before the composition root grants network authority.

There is no domain event in immutable startup-only v1 because nothing in the active domain changes after activation. Dynamic reload, if introduced, would require explicit configuration-accepted/rejected lifecycle events rather than retrofitting hidden mutation.

## Dependency direction

```text
consumer legacy-edge evidence
       |             |
       v             v
 Edge Routing     HTTP Policy       (characterization; not active v1)
       \             /
        \           /
operator YAML --> Edge Contract --> startup application service
                                      |
                                      v
                               Pingora Delivery adapter
                                      |
                                      v
                               Cloudflare Pingora
```

Consumer repositories depend on documented image/config/deployment contracts, not Rust internals. Product-specific behavior stays upstream or in a separately justified adapter owned by that consumer. Characterization modules may encode only observed shared-edge semantics and do not by themselves grant runtime authority.

## Anti-corruption boundary

`edge_contract`, `edge_routing`, and `http_policy` are transport-neutral anti-corruption boundaries against Pingora-specific delivery semantics. `pingora_delivery` may translate admitted network values to `HttpPeer`; `gateway_proxy` may implement `ProxyHttp`. Reversing these dependencies, importing product-domain authorization/business code, or letting request-controlled destinations become upstream authority is a DDD defect.

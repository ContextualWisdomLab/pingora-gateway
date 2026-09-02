# Architecture

## Domain shape

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It does not own any consumer Core Domain.

The **Edge Contract** bounded context owns admission of active process network authority. `GatewayConfig` is the aggregate root: its invariants decide whether a process may listen and which upstream it may contact. `UpstreamConfig` is a configuration value with a stable name inside that aggregate; it has no independent lifecycle in v1. Listener address, TLS identity, request-body budget, and timeout budgets are value objects conceptually, represented by Rust primitives/structs where a richer type would not yet add an invariant. Active `GatewayConfig` v1 intentionally remains one-upstream-per-process.

The **Edge Routing** bounded context characterizes deterministic request-path selection among explicitly named upstream identities. It is transport-neutral. The current `pg-erd-cloud` migration callback consumes its exact/prefix rules, but the production startup/config path still activates only the v1 single-upstream adapter. Product authorization and business routing remain in the consumer bounded context.

The **HTTP Policy** bounded context characterizes explicit edge-owned HTTP response mutations independently from route selection and Pingora delivery. The current migration callback applies the four characterized `pg-erd-cloud` response-security fields with replacement semantics. It does not own application response semantics, authentication/authorization, Wardnet/EgressWeave verdicts, or Keyverse identity.

The **Migration Plan** bounded context composes characterized Edge Routing and HTTP Policy with an explicit upstream-authority set before runtime wiring. It proves that every characterized route points only to an admitted stable upstream identity; it does not grant network authority itself.

The **Migration Delivery** adapter binds every upstream identity in an `EdgeMigrationPlan` to exactly one explicit, validated `UpstreamConfig` and prebuilds its Pingora `HttpPeer`. It rejects missing, duplicate, or undeclared transport authority and performs no service discovery. Request-path selection remains in the transport-neutral migration plan.

The **Runtime Isolation** bounded context owns transport-neutral request-body and concurrent-request admission budgets. Both the active single-upstream adapter and the multi-route migration callback consume the same non-zero limits so migration code cannot silently bypass resource controls.

The **Observability** bounded context owns low-cardinality transport completion facts and shared gateway counters/access-log shape. `RequestObservation` contains only downstream status, `ok`/`error`, and observed request-body bytes. Paths, query strings, headers, cookies, credentials, customer payloads, and product identifiers are deliberately outside the shared telemetry contract. Both Pingora adapters delegate request completion and backpressure telemetry to this context rather than duplicating metrics.

The **Pingora Delivery** adapters map admitted values to `HttpPeer`, `ProxyHttp` callbacks, response/request header policy, health responses, runtime isolation, and transport observability. `GatewayProxy` is the active v1 one-upstream adapter. `MigrationGatewayProxy` is a pre-listener multi-route callback adapter over an already validated `MigrationDeliveryPlan`; it selects only prevalidated peers, applies the characterized HTTP policy, rejects unmatched routes, enforces body/in-flight limits, sanitizes forwarding identity, and records the same payload-free observability. Pingora types never cross into the transport-neutral bounded contexts.

`GatewayCommand` is the application startup service that reads and validates configuration before the composition root grants network authority. It still activates `GatewayProxy`; a later bounded Admin Config / startup transition is required before `MigrationGatewayProxy` can receive production traffic.

There is no domain event in immutable startup-only v1 because nothing in the active domain changes after activation. Dynamic reload, if introduced, requires explicit configuration-accepted/rejected lifecycle events rather than hidden mutation.

## Dependency direction

```text
consumer legacy-edge evidence
       |             |
       v             v
 Edge Routing     HTTP Policy       (transport-neutral characterization)
       \             /
        \           /
         Migration Plan             (explicit stable upstream authority)
                |
                v
        Migration Delivery          (explicit UpstreamConfig -> HttpPeer binding)
                |
                v
        MigrationGatewayProxy ------> Runtime Isolation
                |                   \-> Observability
                v
          Cloudflare Pingora

operator YAML --> Edge Contract --> Startup/Activation --> GatewayProxy
                                                   |        |       |
                                                   |        |       +-> Observability
                                                   |        +----------> Runtime Isolation
                                                   v
                                             Cloudflare Pingora
```

Consumer repositories depend on documented image/config/deployment contracts, not Rust internals. Product-specific behavior stays upstream or in a separately justified adapter owned by that consumer. Characterization and migration-binding modules may encode only observed shared-edge semantics and explicit operator-supplied transport authority; they do not by themselves grant production traffic state.

## Anti-corruption boundary

`edge_contract`, `edge_routing`, `http_policy`, `migration_plan`, `runtime_isolation`, and the transport-only observation vocabulary are anti-corruption boundaries around Pingora delivery semantics. `migration_delivery` and `pingora_delivery` translate already admitted network values to `HttpPeer`; `gateway_proxy` and `migration_proxy` compose those contracts through `ProxyHttp`. Reversing these dependencies, importing product-domain authorization/business code, performing implicit service discovery, recording request/customer payloads in shared telemetry, or letting request-controlled destinations become upstream authority is a DDD defect.

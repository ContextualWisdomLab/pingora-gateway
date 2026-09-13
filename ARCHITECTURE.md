# Architecture

## Domain shape

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It does not own any consumer Core Domain.

The **Edge Contract** bounded context owns admission of active generic-process network authority. `GatewayConfig` is the aggregate root for the version-1 single-upstream runtime: its invariants decide whether that process may listen and which upstream it may contact. `UpstreamConfig` is a configuration value with a stable name; it has no independent lifecycle in v1. Listener address, TLS identity, request-body budget, and timeout budgets are value objects conceptually, represented by Rust primitives/structs where a richer type would not yet add an invariant. Active generic `GatewayConfig` v1 intentionally remains one-upstream-per-process.

The **Edge Routing** bounded context characterizes deterministic request-path selection among explicitly named upstream identities. It is transport-neutral. The current `pg-erd-cloud` migration callback consumes its exact/prefix rules. Consumer-derived route contracts belong here only when they express shared edge responsibility; product authorization and business routing remain in the consumer bounded context.

The **HTTP Policy** bounded context characterizes explicit edge-owned HTTP response mutations independently from route selection and Pingora delivery. The current migration callback applies the four characterized `pg-erd-cloud` response-security fields with replacement semantics. Field values are admitted through the RFC 9110 field-content boundary before transport use. HTTP Policy does not own application response semantics, authentication/authorization, Wardnet/EgressWeave verdicts, or Keyverse identity.

The **Ingress Forwarding Policy** bounded context owns only the trust transition from accepted downstream transport/request authority to compatibility forwarding fields. `ForwardingContext` contains the transport-observed client IP, original request Host authority, the port derived from that authority or the admitted scheme default, and the characterized downstream scheme. The listener bind socket is deliberately not authority because container, Service, NAT, or port-publish layers can expose a different external port. The policy removes request-controlled `Forwarded`, `X-Forwarded-*`, and `X-Real-IP` values before rebuilding only the characterized `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host`, `X-Forwarded-Port`, and `X-Forwarded-Proto` fields. It does not infer user identity, authorization, tenancy, or product truth. `X-Forwarded-Server` is not fabricated because the current migration has no verified proxy-host identity contract that requires it.

`EdgeMigrationPlan` is a transport-neutral **application composition**, not another bounded context. It composes already validated Edge Routing and HTTP Policy objects with an explicit stable upstream-identity authority set before runtime wiring. It proves every characterized route names admitted authority without moving route ownership, HTTP-policy ownership, network activation, service discovery, product logic, or security-owner decisions into a new domain boundary.

The **Migration Delivery** adapter binds every upstream identity in an `EdgeMigrationPlan` to exactly one explicit, validated `UpstreamConfig` and prebuilds its Pingora `HttpPeer`. It rejects missing, duplicate, or undeclared transport authority and performs no service discovery. Request-path selection remains in the application composition. Peer construction alone is not traffic activation.

The **Runtime Isolation** bounded context owns transport-neutral request-body and concurrent-request admission budgets. Both the active single-upstream adapter and the characterized multi-route migration callback consume the same non-zero limits so migration code cannot silently bypass resource controls.

The **Observability** bounded context owns low-cardinality transport completion facts and shared gateway counters/access-log shape. `RequestObservation` contains only downstream status, `ok`/`error`, and observed request-body bytes. Paths, query strings, headers, cookies, credentials, customer payloads, and product identifiers are outside the shared telemetry contract. Both Pingora adapters delegate request completion and backpressure telemetry to this context rather than duplicating metrics.

The **Admin Config** bounded context for the characterized `pg-erd-cloud` migration is `PgErdMigrationConfig`. It is deliberately narrower than a generic multi-route configuration language. Operators may choose listener and metrics addresses, non-zero body/in-flight/keepalive budgets, and concrete transport/TLS data for the already admitted `backend` and `frontend` upstream identities. Route rules, response-header policy, and admitted upstream names are compiled into the migration profile. Unknown fields, future versions, listener collisions, zero budgets, missing/extra/renamed transport authorities, and invalid upstream transport/TLS data fail before listener activation. Configuration cannot become a second product-routing or service-discovery authority.

The **Pingora Delivery** adapters map admitted values to `HttpPeer`, `ProxyHttp` callbacks, response/request header policy, health responses, runtime isolation, forwarding metadata, and transport observability. `GatewayProxy` remains the active generic v1 one-upstream adapter. `MigrationGatewayProxy` is the characterized multi-route adapter over an already validated `MigrationDeliveryPlan`; it selects only prevalidated peers, applies characterized HTTP policy, rejects unmatched routes, enforces body/in-flight limits, derives pg-erd compatibility forwarding identity from accepted Pingora session data rather than request-controlled proxy headers, serves process-local `/livez` and `/readyz`, and records the same payload-free observability. Pingora types do not cross into transport-neutral bounded contexts.

The current `pg-erd-cloud` characterization exposes only Traefik's clear-text `web` entryPoint, so its migration adapter explicitly emits downstream scheme `http`. A TLS listener is a separate contract: HTTPS must not be inferred or claimed until listener/TLS activation and parity traffic are characterized and executable.

`GatewayCommand` remains the shared command-line application service requiring exactly one explicit `--config` path. `cwl-pingora-gateway` loads generic `GatewayConfig` and activates `GatewayProxy`. The separate `cwl-pingora-pg-erd-migration` composition root reads the same explicit path into `PgErdMigrationConfig` and can activate only the compiled pg-erd migration profile. Keeping separate binaries prevents the generic v1 config contract from being silently widened and makes deployment intent observable at process identity level. Source-level listener capability is not deployment, canary, cutover, or release evidence.

There is no domain event in immutable startup-only v1 because nothing in the active domain changes after activation. Dynamic reload, if introduced, requires explicit configuration-accepted/rejected lifecycle events rather than hidden mutation.

## Dependency direction

```text
consumer legacy-edge evidence
       |             |
       v             v
 Edge Routing     HTTP Policy       (transport-neutral characterization)
       \             /
        \           /
         EdgeMigrationPlan          (application composition;
                ^                    explicit stable identity admission)
                |
     explicit migration upstream identities
                |
                v
        Migration Delivery          (explicit UpstreamConfig -> HttpPeer binding)
                |
                v
          Admin Config              (fixed migration profile + operator transport data)
                |
                v
        MigrationGatewayProxy ------> Runtime Isolation
                |       |           \-> Observability
                |       +--------------> Ingress Forwarding Policy
                v
          Cloudflare Pingora

operator YAML --> Edge Contract --> Startup/Activation --> GatewayProxy
                                                   |        |       |
                                                   |        |       +-> Observability
                                                   |        +----------> Runtime Isolation
                                                   v
                                             Cloudflare Pingora
```

Consumer repositories depend on documented image/config/deployment contracts, not Rust internals. Product-specific behavior stays upstream or in a separately justified adapter owned by that consumer. Characterization modules encode only observed shared-edge semantics; `migration_plan` composes those contracts with explicit stable identities, `migration_delivery` binds them to explicit operator-supplied transport authority, and `migration_proxy` exercises callback composition without moving consumer business authority into the gateway.

## Anti-corruption boundary

`edge_contract`, `edge_routing`, `http_policy`, `migration_admin`, `forwarding_policy`, `runtime_isolation`, and the transport-only observation vocabulary are bounded anti-corruption surfaces around Pingora-specific delivery semantics. `migration_plan` is application composition over admitted domain concepts rather than a shared domain model. `migration_delivery` and `pingora_delivery` translate already admitted network values to `HttpPeer`; `gateway_proxy` and `migration_proxy` compose those contracts through `ProxyHttp`. Reversing these dependencies, importing product-domain authorization/business code, trusting request-controlled proxy identity, performing implicit service discovery, recording request/customer payloads in shared telemetry, or letting request-controlled destinations become upstream authority is a DDD defect.

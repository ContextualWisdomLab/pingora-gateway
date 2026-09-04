# Architecture

## Domain shape

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It does not own any consumer Core Domain.

The **Edge Contract** bounded context owns admission of active generic-process network authority. `GatewayConfig` is the aggregate root for the version-1 single-upstream runtime: its invariants decide whether that process may listen and which upstream it may contact. `UpstreamConfig` is a configuration value with a stable name; it has no independent lifecycle in v1. Listener address, TLS identity, request-body budget, and timeout budgets are value objects conceptually, represented by Rust primitives/structs where a richer type would not yet add an invariant. Active generic `GatewayConfig` v1 intentionally remains one-upstream-per-process.

The **Edge Routing** bounded context characterizes deterministic request-path selection among explicitly named upstream identities. It is transport-neutral. The current `pg-erd-cloud` migration callback consumes its exact/prefix rules. Product authorization and business routing remain in the consumer bounded context.

The **HTTP Policy** bounded context characterizes explicit edge-owned HTTP response mutations independently from route selection and Pingora delivery. The current migration callback applies the four characterized `pg-erd-cloud` response-security fields with replacement semantics. It does not own application response semantics, authentication/authorization, Wardnet/EgressWeave verdicts, or Keyverse identity.

The **Ingress Forwarding Policy** bounded context owns only the trust transition from an accepted downstream transport to compatibility forwarding fields. `ForwardingContext` contains transport-observed client IP, original request authority, listener port and characterized downstream scheme. It removes request-controlled `Forwarded`, `X-Forwarded-*` and `X-Real-IP` values before rebuilding the characterized `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host`, `X-Forwarded-Port` and `X-Forwarded-Proto` fields. It does not infer user identity, authorization, tenancy or product truth. `X-Forwarded-Server` is not fabricated because the current Pingora migration has no verified proxy-host identity contract that the consumer requires.

The **Migration Plan** bounded context composes characterized Edge Routing and HTTP Policy with an explicit upstream-authority set before runtime wiring. It proves that every characterized route points only to an admitted stable upstream identity; it does not grant network authority itself.

The **Migration Delivery** adapter binds every upstream identity in an `EdgeMigrationPlan` to exactly one explicit, validated `UpstreamConfig` and prebuilds its Pingora `HttpPeer`. It rejects missing, duplicate, or undeclared transport authority and performs no service discovery. Request-path selection remains in the transport-neutral migration plan.

The **Runtime Isolation** bounded context owns transport-neutral request-body and concurrent-request admission budgets shared by both Pingora adapters. For the version-2 bounded pg-erd profile it also owns the opt-in monotonic upstream response-body progress lifetime: the budget starts at the first non-informational upstream response header and is checked on body-progress callbacks without resetting on progress. This is deliberately not an exact interrupt for a pending supplier read. The generic v1 runtime has no whole-response lifetime. A smaller operator-controlled downstream request-header parser budget is not part of Runtime Isolation yet because pinned Pingora exposes no supported HTTP/1 parser-phase admission hook; `pingora-gateway#43` / `cloudflare/pingora#993` track that supplier dependency rather than duplicating parser source here.

The **Observability** bounded context owns low-cardinality transport completion facts and shared gateway counters/access-log shape. `RequestObservation` contains only downstream status, `ok`/`error`, and observed request-body bytes. Paths, query strings, headers, cookies, credentials, customer payloads, and product identifiers are deliberately outside the shared telemetry contract. Both Pingora adapters delegate request completion and backpressure telemetry to this context rather than duplicating metrics.

The **Admin Config** bounded context for the characterized `pg-erd-cloud` migration is `PgErdMigrationConfig`. It is deliberately narrower than a generic multi-route configuration language. Operators may choose listener and metrics addresses, non-zero body/in-flight/keepalive budgets, and the concrete transport/TLS data for the already admitted `backend` and `frontend` upstream identities. Version 2 additionally requires the positive `max_upstream_response_body_ms` Runtime Isolation budget; version 1 rejects that field so timing semantics cannot change silently. Route rules, response-header policy and admitted upstream names are compiled into the migration profile. Unknown fields, future versions, listener collisions, zero budgets, missing/extra/renamed transport authorities, and invalid upstream transport/TLS data fail before listener activation. This prevents an operator config file from becoming a second product-routing or service-discovery authority.

The **Pingora Delivery** adapters map admitted values to `HttpPeer`, `ProxyHttp` callbacks, response/request header policy, health responses, runtime isolation, forwarding metadata and transport observability. `GatewayProxy` remains the active generic v1 one-upstream adapter. `MigrationGatewayProxy` is the characterized multi-route adapter over an already validated `MigrationDeliveryPlan`; it selects only prevalidated peers, applies the characterized HTTP policy, rejects unmatched routes, enforces body/in-flight limits plus the versioned response-body progress lifetime when configured, derives pg-erd compatibility forwarding identity from the accepted Pingora session rather than request-controlled proxy headers, and records the same payload-free observability. `pingora_delivery` also pins the supplier request-forwarding policy for every materialized peer to `HttpUpstreamRequestPolicy::deny_upgrades()`. That adapter-level invariant mirrors the transport-neutral HTTP/1 protocol-transition admission rule, preserving standard hop-by-hop/connection-nomination sanitization while ensuring Pingora's default WebSocket forwarding capability cannot become an implicit CWL capability. Pingora types never cross into the transport-neutral bounded contexts.

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
         Migration Plan             (explicit stable upstream authority)
                |
                v
        Migration Delivery          (explicit UpstreamConfig -> HttpPeer binding)
                |
                v
         Admin Config               (fixed migration profile + operator transport data)
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

Consumer repositories depend on documented image/config/deployment contracts, not Rust internals. Product-specific behavior stays upstream or in a separately justified adapter owned by that consumer. Characterization and migration-binding modules may encode only observed shared-edge semantics and explicit operator-supplied transport authority; they do not by themselves grant production traffic state.

## Anti-corruption boundary

`edge_contract`, `edge_routing`, `http_policy`, `migration_plan`, `migration_admin`, `forwarding_policy`, `runtime_isolation`, and the transport-only observation vocabulary are anti-corruption boundaries around Pingora delivery semantics. `migration_delivery` and `pingora_delivery` translate already admitted network values to `HttpPeer`; `gateway_proxy` and `migration_proxy` compose those contracts through `ProxyHttp`. Reversing these dependencies, importing product-domain authorization/business code, trusting request-controlled proxy identity, performing implicit service discovery, recording request/customer payloads in shared telemetry, or letting request-controlled destinations become upstream authority is a DDD defect.

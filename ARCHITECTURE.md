# Architecture

## Domain shape

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It does not own any consumer Core Domain.

The **Edge Contract** bounded context owns admission of active generic-process network authority. `GatewayConfig` is the aggregate root. Generic version 1 is the cleartext single-upstream contract. Version 2 retains the same one-upstream/runtime invariants and adds a required downstream TLS declaration. `UpstreamConfig` is a configuration value with a stable name; it has no independent lifecycle in these versions. Listener address, request-body budget, worker count and timeout budgets remain explicit value contracts. The generic runtime intentionally remains one-upstream-per-process; TLS activation must not become a route language or product-policy store.

The **Downstream TLS** bounded context is `src/downstream_tls.rs`. `DownstreamTlsConfig` is a transport-neutral value object containing only absolute certificate-chain/private-key references and the admitted ALPN policy. It validates deterministic reference invariants without reading secret material. Certificate issuance, renewal, ACME, revocation workflow, private-key backup/custody and identity lifecycle remain with canonical external owners. `src/tls_delivery.rs` is the Pingora anti-corruption adapter: it materializes the configured references once immediately before listener construction, verifies the certificate/private-key pairing and maps `h2_http1` to Pingora's H2-preferred/H1-allowed ALPN behavior. Pingora types do not enter the transport-neutral contract.

The **Edge Routing** bounded context characterizes deterministic request-path selection among explicitly named upstream identities. It is transport-neutral. The current `pg-erd-cloud` migration callback consumes its exact/prefix rules. Product authorization and business routing remain in the consumer bounded context.

The **HTTP Policy** bounded context characterizes explicit edge-owned HTTP response mutations independently from route selection and Pingora delivery. The current migration callback applies the four characterized `pg-erd-cloud` response-security fields with replacement semantics. It does not own application response semantics, authentication/authorization, Wardnet/EgressWeave verdicts, or Keyverse identity.

The **Ingress Forwarding Policy** bounded context owns only the trust transition from an accepted downstream transport to compatibility forwarding fields. `ForwardingContext` contains transport-observed client IP, original request authority, listener port and characterized downstream scheme. It removes request-controlled `Forwarded`, `X-Forwarded-*` and `X-Real-IP` values before rebuilding the characterized compatibility fields. It does not infer user identity, authorization, tenancy or product truth. Cleartext versions use `http`; TLS-enabled versions derive `https` from the admitted listener transport rather than from hostile request headers.

The **Migration Plan** bounded context composes characterized Edge Routing and HTTP Policy with an explicit upstream-authority set before runtime wiring. It proves that every characterized route points only to an admitted stable upstream identity; it does not grant network authority itself.

The **Migration Delivery** adapter binds every upstream identity in an `EdgeMigrationPlan` to exactly one explicit, validated `UpstreamConfig` and prebuilds its Pingora `HttpPeer`. It rejects missing, duplicate, or undeclared transport authority and performs no service discovery. Request-path selection remains in the transport-neutral migration plan.

The **Runtime Isolation** bounded context owns transport-neutral request-body and concurrent-request admission budgets shared by both Pingora adapters. For pg-erd version 2 and later it also owns the opt-in monotonic upstream response-body progress lifetime: the budget starts at the first non-informational upstream response header and is checked on body-progress callbacks without resetting on progress. This is deliberately not an exact interrupt for a pending supplier read. Supplier-owned protocol/runtime limits that cannot be enforced before Pingora callbacks remain explicit external dependencies rather than copied into this bounded context: HTTP/1 parser byte/count admission, downstream whole-request-header lifetime and other supplier roots tracked in issues. Open contributor PRs are not Shared Kernels or released contracts.

The **Observability** bounded context owns low-cardinality transport completion facts and shared gateway counters/access-log shape. `RequestObservation` contains only downstream status, `ok`/`error`, and observed request-body bytes. Paths, query strings, headers, cookies, credentials, customer payloads, private-key material and product identifiers are outside the shared telemetry contract.

The **Admin Config** bounded context has two aggregates. Generic `GatewayConfig` version 1 remains cleartext; version 2 requires `downstream_tls`. `PgErdMigrationConfig` remains deliberately narrower than a generic multi-route language: version 1 is the historical cleartext profile, version 2 adds the required positive response-body lifetime, and version 3 retains that lifetime and additionally requires downstream TLS/H2. Earlier versions reject later-version fields so transport or timing semantics cannot change silently. Unknown fields, future versions, listener collisions, zero budgets, missing/extra/renamed transport authorities, and invalid upstream or downstream transport data fail before listener activation.

The **Pingora Delivery** adapters map admitted values to `HttpPeer`, `ProxyHttp` callbacks, response/request header policy, health responses, runtime isolation, forwarding metadata, observability and listener construction. `GatewayProxy` remains the generic one-upstream adapter. `MigrationGatewayProxy` remains the characterized multi-route adapter over an already validated `MigrationDeliveryPlan`. Both production composition roots build downstream `TlsSettings` only for the TLS-enabled version and call `add_tls_with_settings`; cleartext versions retain `add_tcp`. Every materialized upstream peer pins `HttpUpstreamRequestPolicy::deny_upgrades()` so Pingora's supplier-default WebSocket forwarding cannot become an implicit gateway capability.

The current TLS increment admits HTTP/2 only through TLS ALPN and explicitly allows HTTP/1.1 fallback. It does not admit h2c. Basic real-wire H2 traffic and H1 fallback are executable on the branch, but complete HTTP/2 operational parity is not implied by listener capability: concurrent streams, reset/cancellation, GOAWAY/drain, header/body limits, flow control/backpressure, failure recovery and cutover observability remain issue #51 acceptance. H2-downstream to H1-upstream Cookie and zero-length body-framing correctness remain gated by release-qualified supplier disposition of `cloudflare/pingora#901` and `#936`.

HTTP/3/QUIC remains outside the current runtime. TLS support does not imply HTTP/3: a future increment requires release-qualified server support plus UDP/Kubernetes exposure, QUIC/TLS/ALPN, QPACK/header limits, stream and connection flow control, path migration, loss/congestion behavior, amplification defenses, 0-RTT replay policy, drain and rollback evidence.

`GatewayCommand` remains the shared command-line application service requiring exactly one explicit `--config` path. `cwl-pingora-gateway` loads `GatewayConfig` and activates `GatewayProxy`. The separate `cwl-pingora-pg-erd-migration` root reads the same explicit path into `PgErdMigrationConfig` and can activate only the compiled pg-erd migration profile. Separate binaries keep the generic contract from becoming a product-routing language and make deployment intent observable at process identity level. Source-level listener capability is not deployment, canary, cutover or release evidence.

There is no domain event in the immutable startup-only configuration model because nothing in the admitted domain changes after activation. Dynamic reload, if introduced, requires explicit configuration-accepted/rejected lifecycle events rather than hidden mutation.

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
         Admin Config ----------------------> Downstream TLS
                |                                  |
                v                                  v
        MigrationGatewayProxy                TLS Delivery
                |       |                         |
                |       +-> Runtime Isolation     |
                |       +-> Observability         |
                |       +-> Ingress Forwarding    |
                +-----------------------------+---+
                                              v
                                      Cloudflare Pingora

operator YAML --> Edge Contract --> Startup/Activation --> GatewayProxy
                     |                                  |       |
                     +-> Downstream TLS -> TLS Delivery-+       +-> Observability
                                                        +----------> Runtime Isolation
                                                        v
                                                  Cloudflare Pingora
```

Consumer repositories depend on documented image/config/deployment contracts, not Rust internals. Product-specific behavior stays upstream or in a separately justified adapter owned by that consumer. Characterization and migration-binding modules may encode only observed shared-edge semantics and explicit operator-supplied transport authority; they do not by themselves grant production traffic state.

## Anti-corruption boundary

`edge_contract`, `downstream_tls`, `edge_routing`, `http_policy`, `migration_plan`, `migration_admin`, `forwarding_policy`, `runtime_isolation`, and the transport-only observation vocabulary are transport-neutral anti-corruption boundaries around Pingora delivery semantics. `tls_delivery`, `migration_delivery` and `pingora_delivery` translate already admitted values to Pingora listener/peer structures; `gateway_proxy` and `migration_proxy` compose those contracts through `ProxyHttp`.

Reversing these dependencies, importing product-domain authorization/business code, trusting request-controlled proxy identity, performing implicit service discovery, recording request/customer payloads or private-key material in shared telemetry, copying mutable supplier lifecycle/parser internals into CWL ownership, letting request-controlled destinations become upstream authority, or turning certificate consumption into certificate lifecycle ownership is a DDD defect.

## Decision records

ADR 0012 owns explicit bounded service-worker topology. Sibling PR #74 already uses ADR 0013 for representative NUMA shutdown profiling. The downstream TLS/H2 decision therefore uses ADR 0014 rather than reusing a stale historical ADR number; ADR 0014 remains Proposed until the relevant protected integration and governance gates are complete.

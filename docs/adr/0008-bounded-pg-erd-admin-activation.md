# ADR 0008: Activate the pg-erd migration through a bounded admin profile

- Status: Candidate
- Date: 2026-09-02
- Bounded contexts: Admin Config, Edge Routing, HTTP Policy, Migration Delivery, Runtime Isolation, Pingora Delivery

## Context

PR #11 makes the characterized `pg-erd-cloud` route, HTTP-policy, forwarding-trust, runtime-isolation and observability contracts executable as `MigrationGatewayProxy`, but the only production composition root still consumes `GatewayConfig` v1. That public v1 contract intentionally admits one upstream per process, while the captured pg-erd edge requires two stable transport authorities: `backend` and `frontend`.

The next migration step needs a real listener-capable composition root without turning the generic gateway configuration into a programmable routing language. In particular, operator input must not become authority to invent routes, service discovery, product authentication/business policy, Keyverse identity, Wardnet/EgressWeave verdicts, or new upstream names.

A second constraint is trust-material handling. `pingora_delivery::build_peer` loads an operator-selected custom PEM trust bundle. Parsing configuration must not preload that file and then load it again later, because a validate-then-reload sequence creates an avoidable time-of-check/time-of-use window around mutable trust material.

## Alternatives

1. **Widen `GatewayConfig` v1 to arbitrary multi-route configuration.** Rejected for this slice. It would combine a public contract version change with migration-specific routing semantics and create a second authority for product routing.
2. **Encode the pg-erd plan directly in the binary without an Admin Config boundary.** Rejected. Listener addresses, runtime budgets and concrete upstream transport/TLS data are operational inputs that require strict validation and a reusable application boundary.
3. **Use a bounded migration profile with fixed shared-edge semantics and operator-controlled transport values.** Selected. It exposes only values that actually vary by deployment while compiling the characterized migration contract into code.

## Decision

Introduce `PgErdMigrationConfig` version 1 and a separate `cwl-pingora-pg-erd-migration` composition root.

The profile accepts:

- downstream and metrics socket addresses;
- non-zero request-body, in-flight and upstream keepalive budgets;
- exactly one concrete transport binding for each characterized stable upstream identity, `backend` and `frontend`;
- each binding's address, TLS/SNI/trust-bundle and timeout values through the existing `UpstreamConfig` contract.

The profile does not accept route rules, response headers, product policy, identity/authentication configuration, service-discovery inputs, or arbitrary migration upstream names. Its fixed plan preserves the observed pg-erd Traefik behavior: exact `/healthz -> backend`, raw `PathPrefix(`/api`) -> backend` including `/apiary`, fallback `/ -> frontend`, and the four characterized response-security fields.

Configuration parsing performs pure contract/authority validation. It checks the exact transport-authority bijection and calls `UpstreamConfig::validate`, but it does not load custom trust-bundle bytes. `build_proxy` materializes `MigrationDeliveryPlan` once immediately before the composition root creates listeners, so custom trust material is read once through the canonical Pingora delivery adapter. Any read/PEM failure therefore remains fail-closed before listener activation without a duplicate preload.

The generic `cwl-pingora-gateway` binary and `GatewayConfig` v1 stay unchanged. Process identity therefore makes the intended deployment contract explicit rather than silently changing the semantics of an existing executable.

## RED -> GREEN evidence

RED commit `251330f5f47cebde186bb2c26f1bd01284f37090` introduced the executable admin-config contract before `migration_admin` existed. Initial GREEN commits `38c0ee803f826bd3e1f61dee1cdf5c4c59553218`, `242a7a6b09c6c45f3da05ffc427ef46054b6f086`, and `52acafeecef807ce8e362ebc15d1e049ae29c613` added the bounded profile, public module and separate composition root.

A fresh source review then found that `from_yaml` called `build_delivery`, causing custom trust material to be loaded during parse and again during `build_proxy`. Commit `4af750e272eb7a2c48378f9f7e76c3b346c5356f` removes that duplicate materialization and performs pure exact-authority validation instead. Commit `ce8d7f28b6499b4373dde3ffedca3f721faf90d1` makes missing, extra, duplicate and renamed transport authority explicit executable cases.

Hosted exact-head evidence must be reacquired after every later source/documentation movement; predecessor runs do not transfer.

## Risks and consequences

This slice can start a clear-text multi-route listener in source, so mistakes now have a larger blast radius than characterization-only code. It therefore remains Draft and pre-traffic until compiled-binary tests prove upstream-observed route/header/forwarding semantics, request-body/backpressure behavior, failure handling and concurrency against the unchanged exact head.

The fixed migration profile is intentionally consumer-specific. It must not become a pattern of accumulating unrelated product semantics in the shared gateway. A future reusable multi-route contract requires separately proven common semantics and a versioned public design, not copy/paste growth of this profile.

The current pg-erd source entryPoint is HTTP. HTTPS/TLS listener activation, HTTP/2 or HTTP/3, WebSocket/streaming policy, load balancing, retries beyond the generic one-attempt invariant, shadow/canary, cutover and legacy removal remain separate evidence-backed decisions.

## Context Fabric and authority boundary

This source candidate is producer evidence only. `context-graph-contracts` and `enterprise-architecture-core` remain read-only to this writer. It does not become authoritative EA `validated execution` until an immutable released Context Assertion/CloudEvent contract, an immutable protected Pingora release and real parity -> shadow/canary -> cutover/rollback evidence exist and are admitted through the Context Fabric owner path.

No request bodies, forwarding headers, cookies, credentials, customer data, runtime logs or product-domain facts are copied into Context Graph or EA stores, and no cross-service SQL is introduced.

## Next acceptance

Add compiled-binary listener tests around `cwl-pingora-pg-erd-migration` with real loopback backend/frontend origins. They must prove exact route and response-header parity, hostile forwarding-header replacement from accepted transport metadata, body and concurrent-request rejection/recovery, unmatched-route fail-closed behavior, timeout/reset/streaming failure behavior, and payload-free observability. Only after representative routed load measurements may the 20 ms p95 objective be treated as an applicable deployment target.

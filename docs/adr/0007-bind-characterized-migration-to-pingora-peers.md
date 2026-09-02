# ADR 0007: Bind characterized migrations to explicit Pingora peers

- Status: Candidate
- Date: 2026-09-02
- Bounded contexts: Edge Routing, Admin Config, Pingora Delivery

## Context

ADR 0006 introduced `EdgeMigrationPlan` so the captured `pg-erd-cloud` route and HTTP-policy contracts cannot name network authorities outside the explicit `backend` / `frontend` migration set. That contract intentionally stops before Pingora transport activation.

The existing active `GatewayConfig` v1 remains deliberately narrower: one gateway process admits exactly one upstream. Widening that public configuration contract and changing `GatewayProxy` request delivery at the same time would collapse characterization, transport binding, runtime routing, and traffic activation into one change. It would also make it harder to prove that a multi-route migration never obtains implicit service-discovery authority.

## Decision

Introduce `MigrationDeliveryPlan` as a delivery adapter between a validated `EdgeMigrationPlan` and concrete Pingora `HttpPeer` values.

Every upstream admitted by the migration plan must have exactly one explicit `UpstreamConfig`. A concrete configuration whose normalized stable name is duplicated or absent from the migration plan fails closed. Concrete peers are created only through the existing `pingora_delivery::build_peer` path, retaining its TLS identity, trust-bundle, protocol, and timeout validation.

Request-path selection remains owned by `EdgeMigrationPlan`; this adapter only resolves the selected stable identity to a prevalidated peer clone. A path with no characterized route receives no invented fallback destination. A selected identity without an activated peer fails closed.

For the current `pg-erd-cloud` characterization, transport authority is therefore limited to explicit `backend` and `frontend` configurations. No request header, URI authority, DNS response, service registry, product datum, or runtime payload may introduce another destination.

## RED -> GREEN evidence

RED commit `0435cd837cebe29f71204009f5af0a925d947ff1` adds an executable consumer contract before `migration_delivery` exists. It requires complete explicit `backend` / `frontend` binding, preserves `/healthz`, raw `/api` prefix including `/apiary`, and `/` fallback selection, and rejects missing, duplicate, and undeclared transport authority.

GREEN begins at `e52cdf30bc69ff59eb74dd05f456daaee8dae9ce`, which implements the fail-closed binding and reuses the existing Pingora peer-construction adapter. Commit `1f29dbcccc5871c0270e88859f38f665bec39623` exposes the delivery boundary through the public library surface.

## Responsibility boundary

This decision does not widen `GatewayConfig` v1 and does not alter `GatewayProxy`. It therefore does not yet make the compiled serving path multi-route, mutate response headers in Pingora callbacks, terminate downstream TLS, change client-IP trust, implement retries/load balancing, or claim WebSocket/streaming parity.

It does not own DNS/service discovery, certificate issuance/rotation, product authentication/authorization or business routing, Keyverse identity, Wardnet/EgressWeave security verdicts, or product data. It introduces no cross-service SQL and copies no requests, headers, cookies, credentials, logs, or customer data into Context Graph or Enterprise Architecture authority.

## Context Fabric handoff

This transport binding is producer-side migration evidence only. It is not `validated execution` for an Enterprise Architecture projection because no immutable Pingora release, released Context Assertion/CloudEvent contract, parity traffic, shadow/canary, cutover, or rollback evidence exists yet. Exact dependency and migration state is handed to `ContextualWisdomLab/.github#1608`; this migration loop does not edit `context-graph-contracts` or `enterprise-architecture-core` source/PR state.

## Next acceptance

A later runtime slice may make the compiled proxy select these prevalidated peers only after preserving this explicit-authority invariant and adding compiled-binary traffic tests for route selection plus response-policy application. That slice must remain separable from shadow/canary and production cutover, and it must reacquire 100% owned production statement/branch coverage, rustdoc, load/failure/security, and then-live governance evidence on its exact head.

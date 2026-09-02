# ADR 0006: Bind pg-erd-cloud route parity to explicit upstream authority

- Status: Candidate
- Date: 2026-09-02
- Bounded contexts: Edge Routing, HTTP Policy, Admin Config

## Context

`ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535` declares its production-style Traefik edge contract in `deploy/traefik/dynamic.yaml`: `/healthz` and `PathPrefix(`/api`)` route to the `backend` service, fallback `PathPrefix(`/`)` routes to `frontend`, and all three routes attach the same four response-security headers. PRs #5 and #6 characterize those route and response-header behaviors independently.

Independent route and header objects are necessary but not sufficient for a migration candidate. A syntactically valid route could still name an upstream that the migration never admitted. Deferring that mismatch to later Pingora wiring would let service-discovery or network authority appear implicitly at the delivery boundary.

## Decision

Introduce a transport-neutral `EdgeMigrationPlan` that composes three already bounded concerns before any network listener is changed:

1. an explicit normalized set of admitted upstream identities;
2. a validated `RouteTable`;
3. a validated `ResponseHeaderPolicy`.

Every route target must match an admitted upstream identity exactly. Empty upstream sets, empty identities, normalized duplicates, unknown route targets, invalid route tables, and invalid HTTP policies fail closed.

For the characterized pg-erd-cloud contract, the admitted identities are exactly `backend` and `frontend`. The plan preserves the observed raw Traefik prefix behavior, including `/apiary -> backend`; changing that behavior belongs to a separately reviewed product/edge contract rather than this migration characterization.

## RED -> GREEN evidence

RED commit `eeafaca3a55056be80c92961ff88e1516572c623` adds an executable consumer contract requiring the migration plan and fail-closed upstream-authority validation before the implementation exists.

GREEN begins at `69acd6703ee8918853ce0ff16420a2c21a462b25`, which implements the transport-neutral composition boundary. Commit `92e064d48d40c5ef5e4d3f8b4ab5b521cae0003f` exposes the bounded context through the public library surface.

The executable contract proves the exact pg-erd-cloud route/header profile, including `/healthz`, `/api/*`, the raw `/apiary` prefix case, SPA fallback, case-insensitive response-header lookup, missing route/header behavior, and all migration-plan validation branches.

## Responsibility boundary

This decision does not widen `GatewayConfig` v1, which still permits one explicit upstream per process, and it does not wire multi-upstream routing or response mutations into Pingora callbacks. It therefore does not claim parity traffic, shadowing, canary, production cutover, or legacy removal.

The plan does not own DNS/service discovery, certificate issuance, product authentication/authorization, application business routing, Keyverse identity, Wardnet/EgressWeave security verdicts, or product data. It contains no cross-service SQL and does not project requests, headers, cookies, credentials, logs, or customer data into Context Graph or Enterprise Architecture stores.

## Context Fabric handoff

A completed edge migration may become authoritative EA evidence only after the Context Fabric owner supplies an immutable released `context-graph-contracts` bundle with the required Context Assertion/CloudEvent schema, conformance/admission, provenance, and temporal semantics. Until then, this plan is producer-side candidate evidence only. The Pingora migration loop must update the central Context Fabric handoff lane rather than editing `context-graph-contracts` or `enterprise-architecture-core` source or PR state.

## Next acceptance

After the parent stack is releasable and exact-head checks are terminal, the next runtime slice may bind the validated plan to multiple prevalidated Pingora peers. That slice must retain explicit upstream authority and prove route/header parity through compiled-binary traffic tests before any shadow/canary work. TLS/client-IP/header/cookie/body-limit/timeout/backpressure/streaming/WebSocket/HTTP-version semantics that are not present in the characterized source contract remain separate evidence requirements rather than inferred parity.

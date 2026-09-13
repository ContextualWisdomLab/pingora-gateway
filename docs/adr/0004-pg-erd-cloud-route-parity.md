# ADR 0004: Characterize pg-erd-cloud path precedence before Pingora activation

- Status: Accepted for characterization; runtime activation remains blocked
- Date: 2026-09-02

## Context

The live `ContextualWisdomLab/pg-erd-cloud` default branch currently carries a production-style Traefik edge contract in `deploy/traefik/dynamic.yaml` at repository commit `8dc746920c12988f082e914879d95e13c9693535` (file blob `656d18fdfedb19b2556312db4102740044531719`). Its routing behavior is explicit:

| Priority | Match | Upstream |
| ---: | --- | --- |
| 110 | exact `/healthz` | `backend` |
| 100 | `PathPrefix(`/api`)` | `backend` |
| 1 | `PathPrefix(`/`)` | `frontend` |

The same Traefik configuration applies response security headers, but response-header parity belongs to the HTTP Policy bounded context and is intentionally not folded into this routing slice.

Pingora Gateway v1 currently activates exactly one upstream and therefore cannot claim parity with this consumer. Before adding multi-upstream transport authority, the legacy route-selection behavior must exist as an executable transport-neutral contract.

## Decision

Introduce an Edge Routing bounded context containing only deterministic request-path-to-upstream selection.

- Exact and raw path-prefix matchers are explicit contract values.
- Numeric priority is explicit and evaluated from highest to lowest.
- Equal priorities fail closed rather than inheriting undocumented proxy-specific secondary precedence.
- Empty route names, empty upstream identities, relative/empty paths, duplicate route names, and an empty route table fail before activation.
- Traefik `PathPrefix(`/api`)` parity is preserved literally, so `/apiary` selects `backend`. Tightening that behavior is a product-owned contract change and must not be smuggled into an edge migration.

The characterization API is transport-neutral and does not yet alter `GatewayConfig` or `GatewayProxy`. Multi-upstream Pingora activation is a later slice after exact-head gates and parent security/release dependencies are coherent.

## Responsibility boundary

This decision does not move authentication, authorization, business routing, database access, Keyverse identity, Wardnet/EgressWeave policy, certificate issuance, or application health semantics into the gateway. The gateway owns only edge route selection between explicitly configured upstream identities.

`/livez` and `/readyz` remain gateway-local runtime probes. The consumer's `/healthz` path is part of its externally observed route contract and is therefore characterized separately rather than conflated with gateway readiness.

## Verification

RED commit `13dcabe44f3bacf79e72b6379f6dd12cb728b7f0` added the consumer contract before the routing module existed. GREEN implementation begins at `a066a4ffe07d45ef1e847c13acee4f32a1241d10`; public exposure follows at `13ba16eccbd8babdf160a48e8ff32539f4c073a7`.

The executable contract covers `/healthz`, `/api`, `/api/erd`, `/apiary`, `/`, and a normal SPA path, plus exact-route non-prefix behavior, ambiguous-priority rejection, and malformed authority rejection. Every head movement must reacquire the repository's 100% owned production coverage, public rustdoc, CI, supply-chain, SAST and security evidence.

## Consequences

This slice reduces migration ambiguity but does not authorize canary, cutover, or legacy removal. Next runtime work must bind validated route targets to prevalidated Pingora peers without introducing arbitrary per-request destinations, then separately characterize and implement the Traefik response-header policy. The existing parent release/security blockers and Context Graph/EA release-admission dependencies remain fail-closed.

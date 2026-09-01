# Product / Technical Gap Baseline

This baseline is code-current for the Pingora migration stack. Exact source heads, base tips, reviews, workflow/security runs, rulesets, and sibling Context Fabric heads are always re-read live; predecessor evidence never transfers across source, documentation, dependency, base, or governance movement. Moving `context-graph-contracts` / `enterprise-architecture-core` PR state is intentionally not mirrored here; the current cross-loop evidence ledger is `ContextualWisdomLab/.github#1608`.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on ancestor branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; every changed head must reacquire hosted evidence |
| DDD ownership | Implemented | Edge admission invariants live in `edge_contract`; consumer-derived transport-neutral route characterization lives in `edge_routing`; edge response-header characterization lives separately in `http_policy`; Pingora types and trust-bundle loading stay in delivery/application modules. Product auth/business policy, certificate issuance/rotation, Wardnet/EgressWeave decisions, and Keyverse identity remain outside this boundary |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/trust-path/timeout validation; active v1 deliberately admits exactly one upstream and cannot yet replace a multi-route edge |
| Edge Routing | Characterized, not activated | Draft parent PR #5 captures `pg-erd-cloud` Traefik precedence as executable exact/prefix route selection with fail-closed ambiguity. It does not yet bind multiple validated upstreams into the production Pingora request path |
| HTTP Policy | Characterized, not activated | This child captures `pg-erd-cloud` Traefik response-security fields as a transport-neutral validated policy with ASCII case-insensitive field identity, duplicate rejection, exact values and CR/LF rejection. It is not wired into Pingora response callbacks yet |
| Upstream TLS | Implemented candidate | Compiled-binary local-CA/hostname verification proves configured custom trust and SNI mismatch behavior. The delivery adapter also has an explicit no-custom-bundle regression proving platform trust roots remain selected instead of being accidentally replaced; every changed head must reacquire exact-current-head evidence before release |
| HTTP protocol scope | Partial | Initial upstream adapter explicitly uses HTTP/1.1. No HTTP/2 or HTTP/3 parity claim exists without executable downstream/upstream contract evidence |
| Hop-by-hop / forwarding trust | Implemented on ancestor | Pingora standard request policy plus explicit removal/reconstruction of forwarding identity; trusted client-IP chain configuration remains a future bounded contract |
| Retry policy | Implemented, intentionally minimal | `max_retries=1` means one total upstream attempt and zero generic automatic retries; domain idempotency/replay policy stays with the product owner |
| Request limits | Partial | Declared and streamed/chunked body size are bounded; configurable header and connection budgets remain gaps. Process-wide in-flight backpressure is implemented |
| Failure recovery | Partial executable evidence | A compiled-binary loopback contract requires a refused origin connection to return HTTP 502 within the configured connection-budget envelope and proves `/readyz` remains healthy afterward. Timeout, reset, partial-response, streaming and saturation cases remain gaps |
| Health | Implemented on ancestor | `/livez` and `/readyz` are served through the production Pingora path; readiness does not invent product-specific dependency probes. Consumer `/healthz` behavior remains consumer routing evidence, not gateway readiness |
| Graceful drain | Implemented candidate behavior | SIGTERM uses a bounded 5 s grace plus 10 s runtime shutdown timeout inside a 30 s external termination budget; exact-release evidence must be reacquired |
| Logs / metrics / traces | Partial | Low-cardinality counters and credential/cookie-safe coarse access logs exist; tracing and richer bounded operability evidence remain gaps |
| OCI isolation | Implemented candidate hardening | Runtime is uid/gid 65532, read-only-root compatible, capability-free and `no-new-privileges`; both builder and runtime base images are digest-pinned, and exact-head OCI/Scorecard evidence must reacquire |
| Dependency policy | Release-blocked | `.github#1605` owns the exact-release Pingora vs patched-`lru` decision and disposition of unmaintained `derivative 2.2.0` / `RUSTSEC-2024-0388`; `.github#810` independently owns the public non-fork Dependency Review compare-API HTTP 403 incident. Known-unsound downgrade, blanket advisory waiver, fail-open 403 handling, or substitute-scanner promotion is prohibited |
| Coverage / public API docs | Gates implemented | Owned production line/region coverage is required at 100%; `#![deny(missing_docs)]` and warning-denied rustdoc cover public APIs. New `edge_routing` and `http_policy` production statements/branches are subject to the same exact-head gate |
| Load / 20 ms p95 | Executable ancestor candidate | Checksum-pinned k6 2.2.0 exercises 400 release-mode loopback requests across four VUs and gates the minimal HTTP/1.1 path at p95 <20 ms with zero failures. This is only a local regression bound; representative routed/TLS/network deployment evidence is still required before a 20 ms production SLO is claimed |
| Rollback | Documented, not rehearsed | Rehearsal requires an immutable protected release artifact/digest |

## Organization edge inventory

Fresh organization code evidence finds no OpenResty usage. Responsibility class, not process name alone, determines migration scope.

| Repository / evidence | Classification | Migration consequence |
| --- | --- | --- |
| `linux-cluster-ops/docs/architecture/nginx-routing-inventory.md` plus Nginx/Certbot recovery evidence | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | True shared-edge candidate, but current multi-vhost routing, static/PHP-FPM and certificate-adjacent operations exceed Pingora v1. Split authority and freeze executable traffic/TLS contracts first |
| `pg-erd-cloud/deploy/traefik/dynamic.yaml` at source commit `8dc746920c12988f082e914879d95e13c9693535` | ACTIVE_DEPLOYMENT / PLAUSIBLE_CONSUMER | Ordered exact `/healthz -> backend`, raw-prefix `/api -> backend`, fallback `/ -> frontend` plus four response-security fields. Route precedence is characterized in PR #5 and HTTP response policy in this child; neither multi-upstream routing nor response mutation is active in the Pingora runtime yet |
| `naruon` NGINX ingress/live-E2E plus Traefik evaluation | ACTIVE_DEPLOYMENT / TEST_RUNTIME | Its Nginx proxy contract includes HTTP/1.1, long read/send timeouts, WebSocket Upgrade/Connection and forwarded identity semantics. Keycloak/authentication stays outside Pingora; only transport/edge policy can migrate after explicit parity evidence |
| `scopeweave`, `LineageWeave`, `inkspan` Nginx static-serving images/config | ACTIVE_STATIC_RUNTIME | Static hosting is not automatically a shared-edge migration; prove gateway responsibility before queueing |
| `life-os` ClusterIP-only base manifests with separately managed edge namespace | DELEGATED EDGE | Repository base manifests do not prove an embedded legacy edge to migrate |

No consumer is marked migrated, shadowed, canaried, cut over, or legacy-removed. Required sequence remains executable legacy characterization -> Pingora parity -> shadow/canary -> protected production cutover -> rollback evidence -> legacy removal.

## Context Graph and Enterprise Architecture dependencies — read only

`ContextualWisdomLab/context-graph-contracts` and `ContextualWisdomLab/enterprise-architecture-core` are not writable from this loop while the Context Fabric writer is active. This repository therefore does not duplicate their moving PR heads, base ancestry, checks, reviews, or branch-transition state. Every sweep re-reads all open PR/issues and records exact current evidence in `ContextualWisdomLab/.github#1608` for the sole owner path.

GREEN for an edge migration requires an immutable protected Context Graph release carrying canonical object/authority refs, truth status/origin, valid/system time, provenance, Context Assertion + CloudEvent schema/profile/AsyncAPI semantics, exact package identity, and conformance/admission evidence. Runtime request/log/customer data must not be copied into Context Graph authority.

The EA owner path must consume that released contract and project each approved edge migration as `current technology/interface -> migration initiative/scenario -> target technology/interface -> validated execution`, linking affected application/service/API, current and target provider/version, lifecycle, security/operability risk, accountable owner, dependency, canary/cutover/rollback state, and immutable Pingora artifact identity. It must preserve foreign authority and provenance, reject provisional PR heads, and prohibit cross-service application-table SQL.

## Dependency-ordered blockers

1. Reacquire exact-current-head CI, 100% owned production line/region coverage, rustdoc, k6, OCI, SAST and supply-chain evidence after every source or documentation movement; repair only evidence-backed repository defects.
2. Keep `.github#1605` and `.github#810` fail-closed until their respective policy and GitHub dependency-review availability owner paths are resolved; do not suppress `RUSTSEC-2024-0388` generically.
3. Keep PR #5 and this HTTP Policy child characterization-only until their exact-head quality/security evidence is terminal. No child evidence repairs a parent release blocker.
4. After those contracts are stable, introduce an explicit versioned runtime/config transition that binds only validated route identities to prevalidated Pingora peers and attaches only validated HTTP response policies. Do not admit arbitrary per-request destinations or product authorization/business rules.
5. Add explicit broader timeout/reset/streaming/network-failure recovery evidence and representative routed concurrency/origin-capacity measurements before adopting a production 20 ms p95 objective.
6. Add a protected release path that publishes an immutable image digest with provenance and rehearse rollback against that exact digest.
7. Satisfy then-live protected-branch review/governance without self-approval, bot-as-human claims, stale evidence transfer, or routine administrator bypass.
8. Wait for an immutable released Context Graph bundle and a coherent compatible GREEN EA admission path before asserting authoritative architecture execution state.
9. Only then freeze and migrate the highest-impact consumer whose actual responsibility belongs to the shared edge bounded context through parity -> shadow/canary -> cutover -> rollback -> legacy removal.

# Product / Technical Gap Baseline

This baseline is code-current for the bootstrap PR. Exact source heads, base tips, reviews, workflow/security runs, rulesets, and sibling Context Fabric heads are always re-read live; predecessor evidence never transfers across source, documentation, dependency, base, or governance movement.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; every changed head must reacquire hosted evidence |
| DDD ownership | Implemented | Edge invariants live in `edge_contract`; Pingora types and trust-bundle loading stay in delivery/application modules; product auth/business policy, certificate issuance/rotation, Wardnet/EgressWeave decisions, and Keyverse identity remain outside this boundary |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/trust-path/timeout validation; v1 deliberately admits exactly one upstream and cannot replace a multi-route edge |
| Upstream TLS | Repair candidate | Compiled-binary local-CA/hostname verification now has a causal trust-bundle repair and rustfmt repair; every later source identity, including load/OCI changes, must reacquire exact-current-head GREEN before this becomes release evidence |
| HTTP protocol scope | Partial | Initial upstream adapter explicitly uses HTTP/1.1. No HTTP/2 or HTTP/3 parity claim exists without executable downstream/upstream contract evidence |
| Hop-by-hop / forwarding trust | Implemented on branch | Pingora standard request policy plus explicit removal/reconstruction of forwarding identity; trusted client-IP chain configuration remains a future bounded contract |
| Retry policy | Implemented, intentionally minimal | `max_retries=1` means one total upstream attempt and zero generic automatic retries; domain idempotency/replay policy stays with the product owner |
| Request limits | Partial | Declared and streamed/chunked body size are bounded; configurable header, connection, concurrency and backpressure budgets remain gaps |
| Health | Implemented on branch | `/livez` and `/readyz` are served through the production Pingora path; readiness does not invent product-specific dependency probes |
| Graceful drain | Implemented candidate behavior | SIGTERM uses a bounded 5 s grace plus 10 s runtime shutdown timeout inside a 30 s external termination budget; exact-release evidence must be reacquired |
| Logs / metrics / traces | Partial | Low-cardinality counters and credential/cookie-safe coarse access logs exist; tracing and richer bounded operability evidence remain gaps |
| OCI isolation | Implemented candidate hardening | Runtime is uid/gid 65532, read-only-root compatible, capability-free and `no-new-privileges`; both builder and runtime base images are digest-pinned after the Scorecard review finding, and exact-head OCI/Scorecard evidence must reacquire |
| Dependency policy | Release-blocked | `.github#1605` owns the exact-release Pingora vs patched-`lru` decision; current upstream also carries unmaintained `derivative 2.2.0` (`RUSTSEC-2024-0388`, no fixed release). `.github#810` independently owns the public non-fork Dependency Review compare-API HTTP 403 availability incident. Known-unsound downgrade, blanket advisory waiver, fail-open 403 handling, or substitute-scanner promotion is prohibited |
| Coverage / public API docs | Gates implemented | Owned production line/region coverage is required at 100%; `#![deny(missing_docs)]` and warning-denied rustdoc cover public APIs. Every changed head must satisfy the same gates |
| Load / 20 ms p95 | Executable candidate | Checksum-pinned k6 2.2.0 now exercises 400 release-mode loopback requests across four VUs and gates the minimal HTTP/1.1 path at p95 <20 ms with zero failures. This is only a local regression bound; representative consumer/TLS/network deployment evidence is still required before a 20 ms production SLO is claimed |
| Rollback | Documented, not rehearsed | Rehearsal requires an immutable protected release artifact/digest |

## Organization edge inventory

Fresh organization code evidence still finds no actionable OpenResty deployment. Responsibility class, not process name alone, determines migration scope.

| Repository / evidence | Classification | Migration consequence |
| --- | --- | --- |
| `linux-cluster-ops/docs/architecture/nginx-routing-inventory.md` plus Nginx/Certbot recovery evidence | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | True shared-edge candidate, but current multi-vhost routing, static/PHP-FPM and certificate-adjacent operations exceed Pingora v1. Split authority and freeze executable traffic/TLS contracts first |
| `pg-erd-cloud/deploy/traefik/dynamic.yaml` and production compose/docs | ACTIVE_DEPLOYMENT / PLAUSIBLE_CONSUMER | Ordered `/healthz` -> backend, `/api*` -> backend, `/` -> SPA plus response-security headers. One-upstream Pingora v1 is not parity-equivalent |
| `naruon` NGINX ingress/live-E2E plus Traefik evaluation | ACTIVE_DEPLOYMENT / TEST_RUNTIME | More-specific writer owns mutation. Keycloak/authentication stays outside Pingora; only transport/edge policy can migrate after owner handoff and parity evidence |
| `scopeweave`, `LineageWeave`, `inkspan` Nginx static-serving images/config | ACTIVE_STATIC_RUNTIME | Static hosting is not automatically a shared-edge migration; prove gateway responsibility before queueing |
| `life-os` ClusterIP-only base manifests with separately managed edge namespace | DELEGATED EDGE | Repository base manifests do not prove an embedded legacy edge to migrate |

No consumer is marked migrated, shadowed, canaried, cut over, or legacy-removed. Required sequence remains executable legacy characterization -> Pingora parity -> shadow/canary -> protected production cutover -> rollback evidence -> legacy removal.

## Context Graph dependency — read only

`ContextualWisdomLab/context-graph-contracts` is not writable from this loop. Repository metadata still follows the central protected-main transition owner path rather than being hard-coded here. The current Context Assertion event-semantic tail is Draft #21, live head `de376b0608a60ad195e06f5522887be2e63d7b60`, based on #20 `0044d7193a8e9f477e42e961d49b71dc1a956c47`. Repository-owned exact-head `ci`, `reproducibility`, `receipt-package-smoke`, and `supply-chain` runs are terminal success at the latest read; no submitted review or inline review thread exists. Draft/open-head success is not an immutable released Shared Kernel.

GREEN for an edge migration requires a protected immutable Context Graph release carrying canonical object/authority refs, truth status/origin, valid/system time, provenance, Context Assertion + CloudEvent schema/profile/AsyncAPI semantics, exact package identity, and conformance/admission evidence. Runtime request/log/customer data must not be copied into Context Graph authority.

## Enterprise Architecture dependency — read only

`ContextualWisdomLab/enterprise-architecture-core` is also not writable from this loop. Current Context Fabric projection tail is Draft #40, live head `82d099d5a728efc8bf0bc846e5207b3ee6a1673b`, based on #39 `b44635b686c66e78ebd7f1218343a933a510cd89`. At the latest read, exact-head `runtime-readiness` and `supply-chain` are terminal success while `ci` is terminal failure; no submitted review or inline review thread exists. The failing PostgreSQL invariant says a verified target state bypasses freshness monitoring in the canonical planner: `project_technology_target_state_plan(...)` must deterministically return `transformation_state_code='verified'`, `decision_readiness_code='target_state_verified'`, and `recommended_action_code='monitor_target_state'` before Pingora may assert `validated execution` through this owner path.

The owner path is correct when it binds one released `contracts/context-graph-dependency.json`, requires exact `ContextualWisdomLab/<repository>` ownership, `direction_code=inbound_projection`, `exchange_kind=context_assertion_cloudevent`, `ea_core_owns=false`, canonical/source refs, truth status, effective/system time and provenance, and rejects provisional PR heads as release authority.

For each eventual edge migration, EA admission must version `current technology/interface -> migration initiative/scenario -> target technology/interface -> validated execution`, linking affected application/service/API, current and target provider/version, lifecycle, security/operability risk, accountable owner, dependency, canary/cutover/rollback state, and immutable Pingora artifact identity. Cross-service application-table SQL remains prohibited.

## Dependency-ordered blockers

1. Make the local-CA trust/hostname repair plus loopback k6 load contract terminal GREEN on one exact head with 100% owned production line/region coverage, rustdoc, CI, SAST, security, dependency and OCI evidence; repair only evidence-backed failures.
2. Keep `.github#1605` and `.github#810` fail-closed until their respective policy and GitHub dependency-review availability owner paths are resolved; do not suppress `RUSTSEC-2024-0388` generically.
3. Add explicit concurrency/backpressure budgets and broader upstream/network failure recovery evidence, then benchmark representative consumer traffic before adopting a production 20 ms p95 objective.
4. Add a protected release path that publishes an immutable image digest with provenance and rehearse rollback against that exact digest.
5. Satisfy then-live protected-branch review/governance without self-approval, bot-as-human claims, stale evidence transfer, or routine administrator bypass.
6. Wait for an immutable released Context Graph bundle and compatible GREEN EA admission path before asserting authoritative architecture execution state.
7. Only then characterize and migrate the highest-impact consumer whose actual responsibility belongs to the shared edge bounded context.

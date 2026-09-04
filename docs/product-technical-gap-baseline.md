# Product / Technical Gap Baseline

This baseline is code-current for the foundation supply-chain RED child. Exact source heads, base tips, reviews, workflow/security runs, rulesets, and sibling Context Fabric heads are always re-read live; predecessor evidence never transfers across source, documentation, dependency, base, or governance movement.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; every changed head must reacquire hosted evidence |
| DDD ownership | Implemented | Edge invariants live in `edge_contract`; Pingora types and trust-bundle loading stay in delivery/application modules; product auth/business policy, certificate issuance/rotation, Wardnet/EgressWeave decisions, and Keyverse identity remain outside this boundary |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/trust-path/timeout validation; v1 deliberately admits exactly one upstream and cannot replace a multi-route edge |
| Upstream TLS | Implemented candidate | Compiled-binary local-CA/hostname verification proves configured custom trust and SNI mismatch behavior. The delivery adapter also has an explicit no-custom-bundle regression proving platform trust roots remain selected instead of being accidentally replaced; every changed head must reacquire exact-current-head evidence before release |
| HTTP protocol scope | Partial | Initial upstream adapter explicitly uses HTTP/1.1. No HTTP/2 or HTTP/3 parity claim exists without executable downstream/upstream contract evidence |
| Hop-by-hop / forwarding trust | Implemented on branch | Pingora standard request policy plus explicit removal/reconstruction of forwarding identity; trusted client-IP chain configuration remains a future bounded contract |
| Retry policy | Implemented, intentionally minimal | `max_retries=1` means one total upstream attempt and zero generic automatic retries; domain idempotency/replay policy stays with the product owner |
| Request limits | Partial | Declared and streamed/chunked body size are bounded; configurable header, connection, concurrency and backpressure budgets remain gaps |
| Failure recovery | Partial executable evidence | A compiled-binary loopback contract requires a refused origin connection to return HTTP 502 within the configured connection-budget envelope and proves `/readyz` remains healthy afterward. Timeout, reset, partial-response, streaming and saturation cases remain gaps |
| Health | Implemented on branch | `/livez` and `/readyz` are served through the production Pingora path; readiness does not invent product-specific dependency probes |
| Graceful drain | Implemented candidate behavior | SIGTERM uses a bounded 5 s grace plus 10 s runtime shutdown timeout inside a 30 s external termination budget; exact-release evidence must be reacquired |
| Logs / metrics / traces | Partial | Low-cardinality counters and credential/cookie-safe coarse access logs exist; tracing and richer bounded operability evidence remain gaps |
| OCI isolation | Implemented candidate hardening | Runtime is uid/gid 65532, read-only-root compatible, capability-free and `no-new-privileges`; both builder and runtime base images are digest-pinned after the Scorecard review finding, and exact-head OCI/Scorecard evidence must reacquire |
| Dependency policy | Release-blocked with executable RED | `.github#1605` owns the exact-release Pingora vs patched-`lru` decision. `pingora-gateway#13` owns the separate unmaintained `derivative 2.2.0` / `RUSTSEC-2024-0388` supplier-intake blocker. `tests/supply_chain_policy.rs` now makes that defect an explicit committed-lock RED: the exact package record must disappear rather than be hidden by an advisory ignore. The current graph is expected to fail this contract until a reviewed immutable supplier/backport/replacement is consumed. OSV/RustSec remains authoritative and this test is not a scanner substitute. `.github#810` independently owns the public non-fork Dependency Review compare-API HTTP 403 availability incident. Known-unsound downgrade, blanket advisory waiver, fail-open 403 handling, or substitute-scanner promotion is prohibited |
| Coverage / public API docs | Gates implemented | Owned production line/region coverage is required at 100%; `#![deny(missing_docs)]` and warning-denied rustdoc cover public APIs. The platform-root peer branch that previously left two uncovered regions now has a focused executable regression; every changed head must satisfy the same gates |
| Load / 20 ms p95 | Executable candidate | Checksum-pinned k6 2.2.0 exercises 400 release-mode loopback requests across four VUs and gates the minimal HTTP/1.1 path at p95 <20 ms with zero failures. This is only a local regression bound; representative consumer/TLS/network deployment evidence is still required before a 20 ms production SLO is claimed |
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

`ContextualWisdomLab/context-graph-contracts` is not writable from this loop. Repository metadata still follows the central protected-main transition owner path rather than being hard-coded here. The current Context Assertion event-semantic tail is Draft #21, live head `de376b0608a60ad195e06f5522887be2e63d7b60`, based on #20 `0044d7193a8e9f477e42e961d49b71dc1a956c47`. Repository-owned exact-head `ci`, `reproducibility`, `receipt-package-smoke`, and `supply-chain` runs are terminal success at the latest read; no submitted review or inline review thread exists. Draft/open-head success is not an immutable released Shared Kernel, and the repository currently has no GitHub Release.

GREEN for an edge migration requires a protected immutable Context Graph release carrying canonical object/authority refs, truth status/origin, valid/system time, provenance, Context Assertion + CloudEvent schema/profile/AsyncAPI semantics, exact package identity, and conformance/admission evidence. Runtime request/log/customer data must not be copied into Context Graph authority.

## Enterprise Architecture dependency — read only

`ContextualWisdomLab/enterprise-architecture-core` is also not writable from this loop. The DDD parent for the Context Fabric projection moved during this run: Draft #39 is exact `731b3b60264aa9a4d11db3fa5a68f86df944dd0c` on #36 `fff51536c64ba751a37d4ccfd8d2865296b115b9`. This parent now owns the explicit hosted-runner acquisition repair. Exact-head `supply-chain` (`33538539326`) and `runtime-readiness` (`33538539392`) are terminal success, but `ci` (`33538539197`) is terminal FAILURE. Runner acquisition and most acceptance lanes are healthy: Python 3.11–3.14 validation, package, and compose-runtime are GREEN; `postgres-migration` job `99958740497` fails specifically at `Exercise database invariants` after successful foundation migration, idempotent-upgrade/checksum-drift rehearsal, previous-boundary upgrade, atomic rollback, and schema/ledger verification. This is a repository/runtime invariant defect and must be causally repaired by the EA owner rather than treated as infrastructure or weakened.

The Context Fabric projection child #40 remains exact `b3ec93a42528ab0defc0116ac4695d669298240f`, but its recorded base is the superseded #39 head `b44635b686c66e78ebd7f1218343a933a510cd89`. Fresh comparison against current #39 reports `diverged`, merge base `b44635b...`, `ahead_by=67`, `behind_by=4`. Therefore #40's former terminal GREEN `ci` (`33536723144`), `runtime-readiness` (`33536722617`), and `supply-chain` (`33536723303`) are historical only and cannot satisfy the current parent/child integration boundary. The earlier terminal-planner fixture defect was causally repaired on that historical child head, but that repair must be preserved and re-proven after a non-destructive restack. The repository still has no GitHub Release.

The Context Fabric owner path must first causally repair #39's PostgreSQL invariant failure and make the resulting exact parent terminal-clean under live policy. It must then non-destructively restack #40 onto that exact repaired parent while preserving only child-owned Context Fabric/EA projection delta, and reacquire every applicable exact-head repository/security/coverage/package/SBOM/provenance/review artifact. Pingora does not perform that source or PR-state mutation.

The owner path is correct when it binds one released `contracts/context-graph-dependency.json`, requires exact `ContextualWisdomLab/<repository>` ownership, `direction_code=inbound_projection`, `exchange_kind=context_assertion_cloudevent`, `ea_core_owns=false`, canonical/source refs, truth status, effective/system time and provenance, and rejects provisional PR heads as release authority.

For each eventual edge migration, EA admission must version `current technology/interface -> migration initiative/scenario -> target technology/interface -> validated execution`, linking affected application/service/API, current and target provider/version, lifecycle, security/operability risk, accountable owner, dependency, canary/cutover/rollback state, and immutable Pingora artifact identity. Cross-service application-table SQL remains prohibited.

## Dependency-ordered blockers

1. Reacquire exact-current-head CI, 100% owned production line/region coverage, rustdoc, k6, OCI, SAST and supply-chain evidence after every source or documentation movement; repair only evidence-backed repository defects.
2. Keep `.github#1605` and `.github#810` fail-closed until their respective policy and GitHub dependency-review availability owner paths are resolved; keep `tests/supply_chain_policy.rs` RED while `derivative` remains in the committed graph and do not suppress `RUSTSEC-2024-0388` to make it pass.
3. Require the Context Fabric owner to repair EA #39's exact-current-head PostgreSQL invariant failure without weakening semantics, prove the repaired parent terminal GREEN, then restack #40 non-destructively on that exact parent and reacquire all child admission/provenance evidence; do not transfer historical #40 GREEN runs.
4. Add explicit concurrency/backpressure budgets and broader timeout/reset/streaming/network-failure recovery evidence, then benchmark representative consumer traffic before adopting a production 20 ms p95 objective.
5. Add a protected release path that publishes an immutable image digest with provenance and rehearse rollback against that exact digest.
6. Satisfy then-live protected-branch review/governance without self-approval, bot-as-human claims, stale evidence transfer, or routine administrator bypass.
7. Wait for an immutable released Context Graph bundle and a coherent compatible GREEN EA admission path before asserting authoritative architecture execution state.
8. Only then characterize and migrate the highest-impact consumer whose actual responsibility belongs to the shared edge bounded context.

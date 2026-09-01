# Product / Technical Gap Baseline

State refreshed on 2026-09-01 while PR #1 remains Draft against protected `main@f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`. Exact contributor head, reviews, workflow runs, rulesets, and external dependency heads are intentionally read live rather than treated as durable release facts in this document. Checks from predecessor source heads never transfer after code, dependency, base, documentation, or governance movement.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; every changed head must reacquire hosted evidence |
| DDD ownership | Implemented | Edge invariants live in `edge_contract`; Pingora types stay in delivery/application modules; product authentication, tenant/business policy, certificate authority, Wardnet/EgressWeave decisions, and Keyverse identity remain outside this boundary |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/timeout validation; v1 deliberately admits exactly one upstream and therefore cannot yet replace a multi-route edge |
| HTTP/HTTPS upstream | Implemented in adapter | Certificate and hostname verification plus explicit SNI are enabled; local-CA verified-TLS integration and hostname-failure evidence remain missing |
| HTTP protocol scope | Partial | Initial upstream adapter explicitly uses HTTP/1.1. No HTTP/2 or HTTP/3 parity claim is made until executable downstream/upstream contract evidence exists |
| Hop-by-hop / forwarding trust | Implemented on branch | Pingora standard request policy plus explicit removal/reconstruction of forwarding identity; trusted client-IP chain configuration remains a future bounded contract |
| Retry policy | Implemented, intentionally minimal | `max_retries=1` means one total upstream attempt and zero generic automatic retries; replay/failover semantics that need domain idempotency knowledge stay with the product owner |
| Request limits | Partial | Declared and streamed/chunked body size are bounded; configurable header, connection, concurrency and backpressure budgets remain gaps |
| Health | Implemented on branch | `/livez` and `/readyz` are served through the production Pingora path; readiness intentionally does not invent product-specific dependency probes |
| Graceful drain | Implemented candidate behavior | SIGTERM uses a bounded 5 s grace plus 10 s runtime shutdown timeout inside a 30 s external termination budget; compiled in-flight shutdown evidence exists but must be reacquired on every release candidate |
| Logs / metrics / traces | Partial | Low-cardinality counters and credential/cookie-safe coarse access logs exist; tracing and richer bounded operability evidence remain gaps |
| OCI isolation | Implemented candidate hardening | Hosted CI has exercised uid/gid 65532, read-only root, all Linux capabilities dropped and `no-new-privileges`; any later source head must reacquire this evidence |
| Reproducibility | Implemented candidate control | `Cargo.lock` is committed; tests, clippy and OCI builds use locked dependency resolution and CI rejects lock mutation |
| Dependency policy | Repaired but release-blocked | Pingora packages are immutable-revision plus exact-version pinned. `cargo-deny` rejects unknown registries/git and requires git `rev`. The current patched post-release Pingora commit avoids `RUSTSEC-2026-0253`, but `.github#1605` must resolve its conflict with the organization exact-release rule before release/cutover |
| SBOM / image security | Candidate gate | Supply-chain workflow binds checkout to the exact candidate SHA, runs pinned `cargo-deny`, builds the exact image, emits SPDX JSON, scans it with pinned Trivy tooling and hashes evidence. Candidate hashes/local image IDs are not an immutable registry digest or protected release provenance |
| Public rustdoc | Gate implemented | `#![deny(missing_docs)]` makes missing public API documentation a compile-time defect and CI builds docs with `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked` |
| Production coverage | Gate implemented; causal repair proven | Exact predecessor `dd0f37bfe3f08fb461867e7f3f74d26e1cc06d3e` passed the hosted `--fail-under-lines 100 --fail-under-regions 100` gate after the startup parser was made monomorphic and impossible literal health-header construction errors were treated as programmer invariants. The gate still forbids filename/function/branch exclusions; later heads must reacquire exact evidence |
| Benchmark | RED | No representative latency/throughput/CPU/RSS/connection-reuse/TLS benchmark yet supports a 20 ms p95 claim |
| Rollback | Documented, not rehearsed | Consumer rollback can only be rehearsed after an immutable protected release artifact exists |

The coverage repair is deliberately causal rather than cosmetic. Pre-repair exact head `bf3493e621d093459b814bad5e3f69b849cdfe69` exported diagnostics showing 244/245 production lines (99.59%) and 327/332 production regions (98.49%). The remaining startup deficit came from compiler-generated generic parser instantiations despite complete logical-path tests; the remaining proxy regions came from `?` edges on constant-valid HTTP response/header literals. Exact `dd0f37...` then passed compile/test, clippy, public rustdoc, the full owned-production coverage gate and rootless/read-only OCI runtime. Documentation changes after that commit intentionally make its workflow evidence predecessor-only for release readiness.

## Organization edge inventory

Fresh organization search found no actionable literal OpenResty deployment. Responsibility class matters: static serving, PHP-FPM, certificate authority, ingress test fixtures and product-domain routing are not automatically responsibilities of the shared Pingora runtime.

| Repository / evidence | Classification | Migration consequence |
| --- | --- | --- |
| `linux-cluster-ops/docs/architecture/nginx-routing-inventory.md` plus Nginx/Certbot backup-recovery evidence | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | Host-native Nginx combines multi-vhost routing with static/PHP-FPM and certificate-adjacent operations. Split those authorities and capture executable traffic/TLS contracts before any shared-edge cutover |
| `pg-erd-cloud/deploy/traefik/dynamic.yaml` and `compose.prod.yaml` | ACTIVE_DEPLOYMENT / PLAUSIBLE_CONSUMER | Current Traefik has ordered routes `/healthz` -> backend, `/api*` -> backend and `/` -> frontend plus response-security headers. Current one-upstream Pingora v1 is not behaviorally equivalent; parity remains RED |
| `scopeweave/infra/nginx/default.conf` and static image | ACTIVE_STATIC_RUNTIME | Static SPA serving is not sufficient evidence that shared reverse-proxy ownership is appropriate |
| `inkspan` Nginx runtime | ACTIVE_STATIC_RUNTIME | Built demo bundle serving must be characterized as static hosting, not silently migrated as an edge gateway |
| `LineageWeave/frontend/nginx.conf` | ACTIVE_STATIC_RUNTIME | A more-specific repository writer owns mutation; this loop is read-only there |
| `naruon` NGINX ingress/live-E2E configuration | ACTIVE_DEPLOYMENT / TEST_RUNTIME | A more-specific repository writer owns mutation; authentication/Keycloak authority must remain outside Pingora and current `proxy_pass` evidence is not migrated by this writer |

No consumer is marked migrated, canaried, cut over, or legacy-removed. A migration requires executable traffic parity first, then shadow/canary, protected deployment evidence, rehearsed rollback and only then legacy removal.

## Context Graph dependency — read-only

`ContextualWisdomLab/context-graph-contracts` is not writable from this loop. Fresh live inventory on 2026-09-01 shows repository default `develop`; `develop@99cb5468ba3c15c5e79688f53dee74724fae2d13` and `main@99cb5468ba3c15c5e79688f53dee74724fae2d13` currently point to the same integration content while the intended protected-main transition remains centrally owned. Organization ruleset `18156473` is active. The open stack remains unreleased and includes `#4 -> #6 -> #7 -> #8 -> #12 -> #13 -> #14 -> #16 -> #17 -> #18 -> #19 -> #20 -> #21`, with issue #15 still open.

Live PR metadata, not body prose, is authoritative for tail Draft #21: exact head `3f0e04bb7a824f4ebac2b845a99b82e5801f4be8`, base `#20@0044d7193a8e9f477e42e961d49b71dc1a956c47`, mergeable, no submitted review and no inline review thread. Exact-head `ci 33517627876` and `receipt-package-smoke 33517627712` were pending while `reproducibility 33517627596` and `supply-chain 33517627628` were queued at the fresh read. #21 supplies the candidate structured Context Assertion CloudEvent envelope and `context-assertion-event-semantics:v1`, but it remains a provisional PR head.

Therefore no edge migration may treat #21 as a released Shared Kernel. GREEN requires an immutable protected Context Graph release carrying canonical object/authority references, truth status/origin, valid/system time, provenance, Context Assertion + CloudEvent schema/profile/AsyncAPI semantics, exact package identity and conformance/admission evidence. Runtime request/log/customer data must not be copied into Context Graph authority.

## Enterprise Architecture dependency — read-only

`ContextualWisdomLab/enterprise-architecture-core` is also not writable from this loop. Fresh live inventory on 2026-09-01 shows repository default `develop`, `develop@1c0fa8b15ceb9e72186274aeb255d6777eb84ef4`, intended `main@ca6889497728e1a3f09d68790a9096576e13a3ff`, active organization ruleset `18156473`, no release, and open issues #20 and #25 alongside the current stacked PR queue.

Live PR metadata, not stale body prose, is authoritative for Context Fabric consumer-mapping tail Draft #40: exact head `284f81eaf4be92e04fba273cd9e967a8e24c055e`, base `#39@b44635b686c66e78ebd7f1218343a933a510cd89`, mergeable, with no submitted review or inline review thread. Exact-head `runtime-readiness 33509741145` was pending while `ci 33509741319` and `supply-chain 33509741433` were queued at observation time.

Its fail-closed boundary is the correct owner path: bind one `contracts/context-graph-dependency.json`, require exact `ContextualWisdomLab/<repository>` ownership, `direction_code=inbound_projection`, `exchange_kind=context_assertion_cloudevent`, `ea_core_owns=false`, the released Context Assertion event semantic profile, canonical/source refs, truth status, effective/system time and provenance, and reject `provisional-pr-head` as a release. The Pingora writer only supplies exact evidence to that path through `.github#1608`; it does not edit EA source or PR state.

For each eventual migration, EA GREEN requires a versioned projection of `current technology/interface -> migration initiative/scenario -> target technology/interface -> validated execution`, including affected application/service/API, current and target provider/version, lifecycle, security/operability risk, accountable owner, dependency, canary/cutover/rollback state and immutable target Pingora artifact identity. Cross-service application-table SQL is prohibited.

## Dependency-ordered release and migration blockers

1. Reacquire exact-current-head CI, supply-chain and applicable central/security GREEN after the final source/documentation mutation; repair only evidence-backed failures.
2. Keep `.github#1605` fail-closed until the policy owner selects and encodes a bounded Pingora dependency path; never downgrade to the known-unsound release line or add a blanket advisory waiver.
3. Add verified local-CA TLS integration including hostname-failure behavior, then realistic concurrency/backpressure/load and upstream/network failure recovery evidence.
4. Add a protected release path that publishes an immutable image digest with provenance, then rehearse rollback against that exact digest.
5. Benchmark representative gateway traffic before deciding whether 20 ms p95 is realistic; record measured bottlenecks rather than manufacturing an SLO.
6. Obtain then-required review evidence and integrate only an unchanged policy-clean candidate through normal protected-main governance.
7. Wait for a released Context Graph contract and matching EA admission before asserting architecture execution state.
8. Only then characterize, RED-contract and migrate the highest-impact consumer whose actual edge responsibility belongs in this bounded context. `pg-erd-cloud` is a plausible target, but it is not parity-ready against one-upstream v1.

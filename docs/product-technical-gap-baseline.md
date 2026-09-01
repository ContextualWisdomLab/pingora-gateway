# Product / Technical Gap Baseline

State refreshed on 2026-09-01 while PR #1 remains Draft against protected `main@f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`. Exact contributor head, reviews, workflow runs, rulesets, and external dependency heads are intentionally read live rather than treated as durable facts in this document. Checks from predecessor source heads never transfer after code, dependency, base, or governance movement.

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
| OCI isolation | Implemented candidate hardening | Exact-head CI exercises uid/gid 65532, read-only root, all Linux capabilities dropped and `no-new-privileges` |
| Reproducibility | Implemented candidate control | `Cargo.lock` is committed; tests, clippy and OCI builds use locked dependency resolution and CI rejects lock mutation |
| Dependency policy | Repaired | Pingora packages are immutable-revision plus exact-version pinned. `cargo-deny` rejects unknown registries/git and requires git `rev`; vulnerabilities/unsound advisories stay fail-closed while unmaintained transitive framework dependencies are reported without pretending they are exploitable defects. `CC0-1.0` is explicitly admitted for the observed `tiny-keccak` transitive dependency |
| SBOM / image security | Candidate gate | Supply-chain workflow binds checkout to the exact candidate SHA, runs pinned `cargo-deny`, builds the exact image, emits SPDX JSON, scans it with pinned Trivy tooling and hashes evidence. Candidate hashes/local image IDs are not an immutable registry digest or protected release provenance |
| Public rustdoc | Gate added | `#![deny(missing_docs)]` makes missing public API documentation a compile-time defect and CI builds docs with `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked`; exact-current-head GREEN is still required |
| Production coverage | RED | No executable gate yet proves 100% owned production statement and branch coverage. Reachable production paths must not be excluded merely to satisfy the number |
| Benchmark | RED | No representative latency/throughput/CPU/RSS/connection-reuse/TLS benchmark yet supports a 20 ms p95 claim |
| Rollback | Documented, not rehearsed | Consumer rollback can only be rehearsed after an immutable protected release artifact exists |

A previous exact head proved the repaired dependency audit passes before image construction, and the immediately following exact head proved format/test/clippy/lock/least-privilege OCI acceptance after the formatting repair. Those results are causal evidence for the fixes, not transferable release evidence for later heads.

## Organization edge inventory

Fresh organization search found no actionable literal OpenResty deployment. Responsibility class matters: static serving, PHP-FPM, certificate authority, ingress test fixtures and product-domain routing are not automatically responsibilities of the shared Pingora runtime.

| Repository / evidence | Classification | Migration consequence |
| --- | --- | --- |
| `linux-cluster-ops/docs/architecture/nginx-routing-inventory.md` plus Nginx/Certbot backup-recovery evidence | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | Host-native Nginx combines routing with static/PHP-FPM and certificate-adjacent operations. Split those authorities before any shared-edge cutover |
| `pg-erd-cloud/deploy/traefik/dynamic.yaml` and `compose.prod.yaml` | ACTIVE_DEPLOYMENT / PLAUSIBLE_CONSUMER | Current Traefik v3.5.4 has ordered routes `/healthz` -> backend, `/api*` -> backend and `/` -> frontend plus response-security headers. Current one-upstream Pingora v1 is not behaviorally equivalent; parity remains RED |
| `scopeweave/infra/nginx/default.conf` and static image | ACTIVE_STATIC_RUNTIME | Static SPA serving is not sufficient evidence that shared reverse-proxy ownership is appropriate |
| `inkspan` Nginx runtime | ACTIVE_STATIC_RUNTIME | Built demo bundle serving must be characterized as static hosting, not silently migrated as an edge gateway |
| `LineageWeave/frontend/nginx.conf` | ACTIVE_STATIC_RUNTIME | A more-specific repository writer owns mutation; this loop is read-only there |
| `naruon` NGINX ingress/live-E2E configuration | ACTIVE_DEPLOYMENT / TEST_RUNTIME | A more-specific repository writer owns mutation; current `proxy_pass` evidence is test/runtime-specific and is not migrated by this writer |

No consumer is marked migrated, canaried, cut over, or legacy-removed. A migration requires executable traffic parity first, then shadow/canary, protected deployment evidence and rehearsed rollback.

## Context Graph dependency — read-only

`ContextualWisdomLab/context-graph-contracts` is not writable from this loop. Fresh live inventory on 2026-09-01 shows default `develop`, protected `develop@99cb5468ba3c15c5e79688f53dee74724fae2d13`, unprotected `main@99cb5468ba3c15c5e79688f53dee74724fae2d13`, active organization ruleset `18156473`, no release, and open stack `#4 -> #6 -> #7 -> #8 -> #12 -> #13 -> #14 -> #16 -> #17 -> #18 -> #19 -> #20 -> #21` plus issue #15.

Live PR metadata, not its stale body prose, is authoritative for tail Draft #21: head `61a37575ef881dcdc1055b514b57f1cabe4e514c`, base SHA `0044d7193a8e9f477e42e961d49b71dc1a956c47`, mergeable, no submitted review and no inline review thread. Its exact-head `ci 33502630138`, `reproducibility 33502630145`, `supply-chain 33502630127`, and `receipt-package-smoke 33502630111` were queued at observation time. #21 supplies the structured Context Assertion CloudEvent envelope and `context-assertion-event-semantics:v1`, but it remains a provisional PR head.

Therefore no edge migration may treat that head as a released Shared Kernel. GREEN requires an immutable protected Context Graph release carrying the complete schema/profile/AsyncAPI bundle, exact package/provenance identity and admission/conformance evidence. Runtime request/log/customer data must not be copied into Context Graph authority.

## Enterprise Architecture dependency — read-only

`ContextualWisdomLab/enterprise-architecture-core` is also not writable from this loop. Fresh live inventory on 2026-09-01 shows default `develop`, protected `develop@1c0fa8b15ceb9e72186274aeb255d6777eb84ef4`, intended integration `main@ca6889497728e1a3f09d68790a9096576e13a3ff`, no release, 24 open PRs (`#11, #12, #15, #16, #17, #18, #19, #21, #22, #23, #24, #26, #27, #29, #30, #31, #32, #33, #34, #35, #36, #37, #39, #40`) and open issues #20 and #25.

Live PR metadata, not stale body prose, is authoritative for Context Fabric consumer-mapping tail Draft #40: head `bbf07a0530c78bcb1638b369ee7f36fa07b2aa00`, base SHA `b44635b686c66e78ebd7f1218343a933a510cd89`, mergeable, with no submitted review or inline review thread. At observation time exact-head `runtime-readiness 33502843786` was pending while `ci 33502843798` and `supply-chain 33502843940` were queued.

Its fail-closed boundary is the correct owner path: bind one Context Graph dependency manifest, require the Context Assertion event semantic profile, preserve canonical/source refs, truth status, effective/system time and provenance, and reject `provisional-pr-head` as a release. The Pingora writer only supplies exact evidence to that path; it does not edit EA source or PR state.

For each eventual migration, EA GREEN requires a versioned projection of `current technology/interface -> migration initiative/scenario -> target technology/interface -> validated execution`, including affected application/service/API, current and target provider/version, lifecycle, security/operability risk, accountable owner, canary/cutover/rollback state and immutable target Pingora artifact identity. Cross-service application-table SQL is prohibited.

## Dependency-ordered release and migration blockers

1. Reacquire exact-current-head CI, supply-chain and applicable central/security GREEN after every source mutation; repair only evidence-backed failures.
2. Prove 100% owned production statement/branch coverage with an executable exact-head gate while preserving realistic integration paths.
3. Prove the newly added missing-public-rustdoc gate on the exact current head.
4. Add verified local-CA TLS integration including hostname-failure behavior, then realistic concurrency/backpressure/load and upstream/network failure recovery evidence.
5. Add a protected release path that publishes an immutable image digest with provenance, then rehearse rollback against that exact digest.
6. Benchmark representative gateway traffic before deciding whether 20 ms p95 is realistic; record measured bottlenecks rather than manufacturing an SLO.
7. Obtain then-required review evidence and integrate only an unchanged policy-clean candidate through normal protected-main governance.
8. Wait for a released Context Graph contract and matching EA admission before asserting architecture execution state.
9. Only then characterize, RED-contract and migrate the highest-impact consumer whose actual edge responsibility belongs in this bounded context. `pg-erd-cloud` is a plausible target, but it is not parity-ready against one-upstream v1.

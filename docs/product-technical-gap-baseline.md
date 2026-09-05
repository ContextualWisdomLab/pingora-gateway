# Product / Technical Gap Baseline

This document is the code-current migration baseline for `ContextualWisdomLab/pingora-gateway`. It records the reusable edge/runtime contract and the currently known dependency roots without treating mutable PR heads as release authority. Exact heads, bases, reviews, workflow/security runs, supplier state, releases, and consumer deployment evidence must be re-read before every promotion step; predecessor evidence never transfers across source, documentation, dependency, ancestry, or governance movement.

Snapshot date: 2026-09-05 KST. Any later live repository evidence supersedes the exact identities recorded below.

## Product and DDD boundary

Pingora Gateway is a Generic/Supporting edge runtime. It may own reusable transport concerns only: Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. Product authentication/authorization, tenancy and business routing remain in the product domain; Keyverse remains the identity backend; Wardnet/EgressWeave remain security-policy authorities; certificate issuance/ACME/key custody stays with its actual certificate/secret owner.

The bounded contexts must remain explicit in code, APIs and tests. Transport identities and limits are Value Objects; admitted route/config state is validated before listener activation; runtime policy is composed through narrow domain services/adapters; product-specific semantics cross the boundary only through characterized contracts or ACLs. No source copy, cross-service application-table SQL, mutable dependency, or hidden Shared Kernel is accepted as migration closure.

## Shared runtime state

| Area | Current state | Evidence / remaining gap |
| --- | --- | --- |
| Executable Pingora path | Implemented candidate | Production composition exists on the foundation stack; every moved head must reacquire exact-head fmt/compile/test/Clippy/rustdoc/coverage/load/OCI/security/supply-chain evidence. |
| Admin Config | Implemented candidate | Strict versioned configuration and fail-closed transport authority exist. Generic v1 intentionally remains narrow; product-domain configuration is not moved into the shared gateway. |
| Upstream TLS | Implemented candidate | Explicit trust/SNI behavior has compiled-listener acceptance in the migration stack. Downstream TLS/HTTP/2 remains a separate supplier- and traffic-evidence lane. Certificate lifecycle is not owned here. |
| HTTP policy / forwarding trust | Partial | Hop-by-hop sanitation and forwarding-identity distrust/reconstruction are characterized. Header admission/lifetime, H2→H1 Cookie normalization, body framing and later HTTP/3/QUIC semantics remain independently gated. |
| Runtime isolation | Partial | Body/in-flight/connection/timeout/drain contracts exist across the stack; broader header budgets, parked-read shutdown, long-lived streaming and representative failure traffic remain dependency-ordered gaps. |
| Observability | Partial | Shared low-cardinality, payload-free transport telemetry exists. Product logs/identity/business telemetry remain outside this owner; richer tracing and production deployment evidence remain open. |
| OCI | Implemented candidate | Rootless/read-only-root/capability-free/no-new-privileges contracts exist, but immutable release/SBOM/provenance/reproducibility and deployment rollback still gate release readiness. |
| Performance | Local executable contract only | k6 loopback contracts gate applicable routes at p95 `<20 ms` without application-route warm-up. This is not a production SLO; representative TLS/network/origin/deployment traffic is still required. |
| Release / rollback | Not released | Protected `pingora-gateway/main` is still `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases are empty. No immutable gateway artifact exists for a real consumer cutover. |

## Current dependency root

### Foundation workflow admission repair — #58 / #59 / #60

Issue #58 owns a wrong-base repair: merged #57 placed valid repository workflow-coalescing policy inside the H2→H1 Cookie protocol lineage. The valid delta must be promoted to the earliest workflow-owning foundation ancestor and then adopted down descendants without force-push, destructive rebase, or discarding the protocol fixture.

Draft #59 is the RED/oracle child of foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`. Current exact head is `8eaccce7251b4eb8666212b57207da31c0146b9d`; its effective range is two Rust test contracts only. Scheduler evidence remains RED because the foundation admits both feature-branch push and PR runs. Predecessor `badf84789a9d868a99a0479a0f640ea5029ecbcc` eventually acquired hosted runners: Supply Chain, OCI and load passed, while the test job failed deterministically at rustfmt before semantic tests. The exact formatter output was adopted forward without changing policy assertions. Current CI `33927302700` and Supply Chain `33927302706` supersede that predecessor; at the 2026-09-05 snapshot all current CI jobs remain pre-checkout queued with `runner_id=0` and no steps, so hosted semantic RED is not yet credited.

Draft #60 is the minimal GREEN child. Current exact head is `7da487d8cd42c9e3004b2700609e13b569b94f20` on exact #59 `8eaccce...`; its effective child delta is exactly `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml`. Both workflows restrict duplicate push evidence to protected `main`, scope cancellation identity to workflow/repository/PR with `github.run_id` fallback, and cancel only pull-request runs. Current scheduler evidence is GREEN: only PR-triggered CI `33927380877` and Supply Chain `33927380417` materialized for the child head. Their CI jobs are still pre-checkout queued, so hosted semantic GREEN is not yet credited.

Promotion order is fixed: `#59 current exact review + hosted semantic RED → #60 current exact review + unchanged-contract hosted GREEN → ordinary foundation adoption → descendant ancestry repair`. Static/bot review, scheduler admission and hosted semantic execution are separate evidence classes.

### Rust 1.98.1 compiler prerequisite — #56 / #54

Draft #56 current exact head is `68fffc8a66a11b4657c259739ae1c3984f5818d7` on foundation #1, fresh compare **ahead 17 / behind 0** with the foundation as exact merge base. It moves release-producing paths to Rust 1.98.1, the 2026-09-03 repair release for the Rust 1.98.0 vtable-generation miscompilation. Predecessor `955b3e98e1f1bd945a05b8a7cdd5f16e75c99c77` proved exact checkout, Rust 1.98.1 selection, OCI and load execution; its remaining test failure was rustfmt and was repaired forward.

A later current-line review found a distinct compiler-authority fail-open: the contract rejected only the literal `cargo +`, so valid shell whitespace such as `cargo  +1.98.0 build` could bypass the alternate-toolchain guard after the verified Rust 1.98.1 setup. Commit `68fffc8...` replaces that literal check with line-continuation normalization and token-based `cargo` followed by non-empty `+<toolchain>` detection, with regressions for single/multiple spaces, tabs, continuation, and an explicit path ending in `/cargo`. The review thread re-verified the stated repair and is resolved. Current exact CI `33933024607` and Supply Chain `33933024567` supersede all predecessor execution and must still produce terminal exact-head GREEN before compiler promotion.

Draft #54 current head `d082704637e4d6a77112e6449dcf0d141b166117` is a non-force child of current #56. Fresh compare against #56 is **ahead 20 / behind 0** with `68fffc8...` as exact merge base and an effective child delta of exactly four derivative-advisory evidence files. The parent repair was adopted in forward commit `6569eede3f0cdfb0096e7f257655bce6ef21ea53`, then ordinary two-parent merge `d082704...` recorded current #56 without rewriting history. Current CI `33933107776` and Supply Chain `33933107770` supersede earlier child evidence.

#54 intentionally requires committed `Cargo.lock` to contain no `derivative`. The supplier advisory `RUSTSEC-2024-0388` is an unmaintained-dependency finding with no patched release; CWL policy does not convert that into a blanket audit exception. The intended RED is valid only after the parent compiler/format path is independently GREEN.

### Supplier and protocol prerequisites

Draft #53 remains the real-wire RFC 9113 H2→H1 Cookie oracle, but current `bf5436a7b482fffd2c22fb847672076d1063a26a` is temporarily a five-file effective range because #57 added workflow policy on the wrong base. It must not receive protocol RED/GREEN or merge credit until #58 restores protocol-test-only ancestry. The unchanged semantic fixture must prove a real TLS/H2 client sends two Cookie fields and the raw H1 origin receives exactly one `Cookie: session_id=abc123; preferred_language=en` field.

Protected public `cloudflare/pingora/main` is `09696b51bc59315353d96686355861604d0bb48c` at this snapshot. Public Cookie candidate #901 remains open/unmerged and non-mergeable at `b856ddfc6be15f1727601d2d76cb10d2d72f95f0`; it targets an older `PeerOptions.h2_to_h1_concat_cookies` shape and is not an immutable consumable dependency. A repair must be adapted to current `HttpUpstreamRequestPolicy` with default-on Cookie normalization and a narrow opt-out that preserves unrelated hop-by-hop/Connection protections.

Body-framing candidate #936 remains open/unmerged but mergeable at `e40ed4cceb0c0ed8c05cc39eb01a8c73dea5497a`, directly on current protected Pingora main. It addresses zero-length chunked writes emitting a premature H1 terminator in both async and cancel-safe write paths. It remains a separate prerequisite from Cookie normalization.

Cloudflare issue #889 remains open with no public repair PR found at this snapshot. Current public source uses `derivative 2.2.0` in `PeerOptions` debug derivation and load-balancing `Backend` identity traits. An acceptable supplier repair must preserve omitted callback/debug fields and keep `Backend::Extensions` excluded from equality/hash/order semantics, remove the dependency declarations, pass upstream tests/Clippy/audit, and become maintainer-integrated immutable supplier truth before the gateway bumps it.

## Actions owner-plane

Organization Actions queue/admission ownership remains in `ContextualWisdomLab/.github`, not in leaf source churn. Protected `.github/main` is `b5efbc2762e472e4a380b0503b1f050f76fbb008` at this snapshot. Merged #1878 on predecessor `1b65dbc35e7183722ad77894e2d80b39993be90d` removed the organization queue sweep, ignored legacy sweep dispatches, coalesced Noema before job admission, and coalesced current-head runs by PR. The subsequent protected merge #1877 repairs owner test drift around Strix changed-scope parity and stale Noema cancellation fixtures without changing Pingora leaf source. These are valid owner-side queue/review-control advances, but they are not direct Pingora GREEN evidence: the current #56/#54/#61 waves remain queued or pending on fresh reads.

Pingora predecessor heads have already shown that `materialized/no runner → delayed assignment/exact checkout → terminal semantic result` can occur without leaf no-op churn. Current #59/#60 and the newer #56/#54/#61 heads remain incomplete until their exact jobs acquire runners and reach terminal conclusions. The correct classification is intermittent/delayed organization runner assignment, not proof of a leaf source defect. Do not change runner selectors, add no-op commits, weaken gates, or promote predecessor success merely to retrigger execution.

The canonical read-only queue-health PR `.github#1150@bbacf9e81ae954eb8365fbfe1856d8698a768a4a` is stale relative to protected main: fresh compare against `main@b5efbc2762e472e4a380b0503b1f050f76fbb008` is **67 ahead / 163 behind** with merge base `8c085835fbf77de2321b72fa6b8dd946227e523e`, while its body still describes that old merge base as current and says the branch is no longer behind. Its dedicated owner must read/adopt the intervening deltas and reconcile non-destructively; Pingora does not modify that repository's source/ref/PR state.

## Organization edge inventory and true migration scope

Responsibility class, not the presence of an Nginx/OpenResty/Traefik process name, determines whether a repository belongs in the shared Pingora migration.

| Repository / evidence | Classification | Migration consequence |
| --- | --- | --- |
| `linux-cluster-ops` current Nginx/Certbot/OJS recovery path | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | True shared-edge candidate only after certificate lifecycle, edge runtime and OJS/PHP/FastCGI application authority are decomposed. |
| `pg-erd-cloud` Traefik route/header deployment | ACTIVE_DEPLOYMENT / PLAUSIBLE_CONSUMER | Characterized multi-route consumer; gateway parity must preserve exact route/header/forwarding behavior before any shadow/canary. |
| `naruon` ingress/Traefik evaluation | ACTIVE_DEPLOYMENT / TEST_RUNTIME | Dedicated writer owns consumer mutation. Keyverse/product authentication remains outside Pingora. |
| `scopeweave`, `LineageWeave`, `inkspan` static Nginx images/config | ACTIVE_STATIC_RUNTIME | Static hosting is not automatically a shared-edge responsibility; migrate only after proving a reusable gateway boundary. |
| `life-os` ClusterIP-only base manifests with separately managed edge namespace | DELEGATED EDGE | Base manifests do not prove an embedded legacy edge to replace. |

`linux-cluster-ops#267` is the correct operations owner path and remains `status: blocked`. It requires certificate issuance/renewal/key custody to be separated from public edge termination and from OJS/PHP/FastCGI application/runtime ownership. It also requires a reviewed immutable `pingora-gateway` release before the operations repository may replace Nginx.

`linux-cluster-ops#251@3e5598c4d89f233dd2862d6d6bc20c797037232b` remains an unresolved prerequisite. Its `tar -U` change cannot by itself prove extraction-path/symlink confinement. Preserve its valid ReDoS and option-termination deltas, but the restore boundary still needs a realistic RED and a causal safe-extraction design such as a new root-owned empty staging directory, finite member path/type/link-target/ownership validation, and promotion only after validation, or an equivalently strong cryptographic trust contract for generated archives. This repository has a dedicated writer, so Pingora records the dependency rather than mutating its source or PR state.

## Shared-kernel / EA boundary

Context Graph and Enterprise Architecture repositories are read-only dependencies to this writer. Mutable Draft PR heads, copied schemas, or cross-service database access are not accepted as a Shared Kernel. When those owners publish a canonical versioned contract, Pingora may consume only the immutable released contract through an ACL and must preserve provenance, valid/system time and ownership semantics without copying runtime request, credential, cookie, log or customer payloads into architecture authority.

An eventual EA projection may describe `current edge technology/interface → migration initiative → target Pingora technology/interface → validated execution`, but `validated execution` requires the actual immutable Pingora artifact plus parity/shadow/canary/cutover/rollback evidence. A source branch or documentation claim is not deployment truth.

## Release and cutover acceptance

A release-ready exact protected head must prove version/CHANGELOG consistency, immutable tag/package/image identity, SBOM, provenance, reproducibility, rollback and the then-live security/review rules. It must also preserve applicable TLS, HTTP/1.1, HTTP/2 and later HTTP/3/QUIC, WebSocket/streaming, timeout/retry/backpressure, header/cookie/client-IP/body-limit, health/drain, rootless-container and failure-traffic contracts.

Consumer migration credit follows only this order: executable legacy characterization → Pingora parity → immutable gateway release → consumer deployment bump to that immutable identity → shadow/canary under realistic traffic → rollback rehearsal → protected cutover → legacy runtime/config removal. Local loopback p95 `<20 ms` remains a regression gate, not a production claim; production/buyer-path performance must be measured with representative origin/TLS/network/deployment conditions and profiled causally if it exceeds the bound.

No current consumer is marked migrated, shadowed, canaried, cut over, or legacy-removed.

## Dependency-ordered next actions

1. Obtain current exact review and hosted semantic RED for #59, then current exact review and unchanged-contract hosted GREEN for #60; adopt the policy into the foundation normally and repair descendant ancestry without force.
2. Obtain exact-head GREEN for Rust 1.98.1 #56 after the compiler-selector review repair; only then execute #54's intentional `derivative` RED and drive #889 to a maintainer-integrated immutable supplier repair before a downstream lockfile GREEN.
3. Restore #53 to protocol-test-only scope, execute the real TLS/H2→H1 Cookie RED, adapt #901 to current supplier policy ownership, and require immutable supplier integration plus unchanged wire GREEN. Keep #936 as the distinct H1 body-framing prerequisite.
4. Keep organization queue/admission RCA on the `.github` owner path. Pingora leaf heads wait/refetch without no-op churn when the only evidence is pre-checkout runner starvation.
5. On the operations owner path, preserve #251's valid security delta but repair safe extraction, then complete #267's structural inventory and certificate/application boundary before any Nginx cutover work.
6. Publish an immutable protected `pingora-gateway` release with SBOM/provenance/reproducibility/rollback evidence before any consumer deployment pin or canary.
7. Perform parity → shadow/canary → rollback → cutover → legacy removal per consumer, and project validated execution to EA only after those facts exist.

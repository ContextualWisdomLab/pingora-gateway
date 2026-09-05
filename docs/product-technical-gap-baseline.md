# Product / Technical Gap Baseline

This document is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway`. Mutable PR heads are evidence candidates, not release authority. Promotion requires a fresh read of exact head/base ancestry, review threads, hosted checks, security/supply-chain evidence, protected branch, supplier identity, release artifacts, and consumer deployment state.

Snapshot: 2026-09-05 KST. Later live evidence supersedes the exact identities below.

## Product and DDD boundary

`pingora-gateway` owns transport/runtime concerns only: Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, low-cardinality transport telemetry, and immutable edge packaging.

It must not duplicate product authentication/authorization, tenancy or business routing, Keyverse identity authority, Wardnet/EgressWeave policy authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR heads, and hidden Shared Kernels are rejected.

No material admin UI is in the current dependency root, so Figma/Storybook/i18n UI acceptance is not applicable to the work below.

## Current reusable runtime state

| Area | Current state | Release/cutover gap |
| --- | --- | --- |
| Pingora executable path | Implemented candidate | Every moved exact head must reacquire fmt/compile/test/Clippy/rustdoc/coverage/load/OCI/security/supply-chain evidence. |
| Ingress / routing | Implemented candidate | Representative production host/SNI/routing parity and shadow traffic remain required. |
| TLS | Partial | Certificate lifecycle stays outside this owner. Downstream TLS/H2 and later HTTP/3/QUIC need dependency-ordered traffic evidence. |
| HTTP policy | Partial | H2→H1 Cookie normalization and zero-length chunk framing remain supplier-gated. |
| WebSocket / streaming | Partial | Long-lived-stream shutdown/backpressure/failure traffic still need release-candidate E2E evidence. |
| Runtime isolation | Partial | Body/in-flight/timeout/drain controls exist; broader header budgets and failure traffic remain open. |
| Observability | Partial | Payload-free low-cardinality edge telemetry exists; product/identity/business telemetry remains outside this owner. |
| OCI | Implemented candidate | Immutable release, SBOM, provenance, reproducibility and rollback remain open. |
| Performance | Local contract only | Applicable buyer paths require realistic async/k6 evidence with p95 ≤20 ms; this is not yet a production SLO. |
| Release | Not released | Protected `main` is `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; no immutable gateway release exists. |

## Dependency root: workflow admission repair (#58 / #59 / #60)

Merged #57 placed a valid repository workflow-coalescing delta inside #53's H2→H1 Cookie protocol lineage. Both deltas must survive. Workflow policy is promoted to the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

### RED oracle — #59

Draft #59 is based directly on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`. Current exact head is `3e452c850a6585ffca94468d841a8b8b3d817040`; effective range is exactly two Rust test contracts: `tests/workflow_concurrency_contract.rs` and `tests/workflow_tag_filter_contract.rs`.

Predecessor `8eaccce7251b4eb8666212b57207da31c0146b9d` acquired hosted runners. Supply Chain, OCI and load-contract were GREEN; CI reached `cargo test --all-targets --locked` and failed the then-defined foundation invariants for workflow/repository/PR cancellation identity and protected-main-only duplicate-push scope. That is causal hosted semantic RED evidence, but it does not transfer to a moved head.

The current oracle is stronger. It covers rerun isolation, `converted_to_draft` / `ready_for_review`, direct-job Draft admission, fail-closed event syntax, branch/tag scope, and semantic tag-filter parsing. Fresh independent review found one remaining fail-open: `direct_job_count` ignored a valid direct job line such as `  build: # comment`, so the expected number of Draft guards could be under-counted. `3e452c850a6585ffca94468d841a8b8b3d817040` repairs this by parsing only the top-level `jobs:` block, accepting canonical `job_id:` keys followed by whitespace/inline comments, failing closed on non-canonical direct children, and adding a regression that also proves the next top-level mapping ends the block. The finding thread was answered with this exact commit and resolved after the source repair existed.

Current exact runs are CI `33943265637` and Supply Chain `33943265714`. CI jobs `101244606205`, `101244606294`, and `101244606384` are materialized on `ubuntu-24.04` but remain queued with `steps=[]` and `runner_id=0` at the latest read. Current exact terminal RED and fresh exact-range review are still required.

### GREEN implementation — #60

#60 is Ready on exact #59 `3e452c850a6585ffca94468d841a8b8b3d817040`. Current exact head is `0e70ea9916146bcf82a305a349778ea4ec33e10a`. Fresh compare is ahead 33 / behind 0 with exact merge base and exactly two effective files: `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` (`+17/-9`). No Rust test delta remains in the GREEN range.

The implementation restricts duplicate push evidence to protected `main`; coalesces first-attempt PR work by workflow/repository/PR; isolates reruns with `github.run_id`; cancels only PR runs; admits `opened`, `synchronize`, `reopened`, `converted_to_draft`, and `ready_for_review`; and keeps every direct PR job out of the runner queue while Draft. Exact-SHA checkout, test/load/OCI and supply-chain gates remain intact.

The branch was non-force restacked after #59 moved. An ordinary two-parent commit adopted current #59 ancestry using the RED-parent tree, then the two intended GREEN workflow blobs were reapplied as forward commits. The final compare proves no inherited test or documentation delta remains.

Current exact runs are CI `33943319652` and Supply Chain `33943319632`, still pending/queued at the latest read. Current all-gates hosted GREEN and fresh exact-range review remain required.

### Documentation projection — #61

#61 is the writer-safe documentation child. It was non-force restacked onto current #60 and then adopted the exact current #59 test blob so the effective range returned to documentation-only scope. Current compare against #60 `0e70ea9916146bcf82a305a349778ea4ec33e10a` is ahead 31 / behind 0 with exact merge base and exactly six documentation files: `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline.

Stable Ready→Draft scheduler evidence captured on the earlier #61 source identity remains behavioral evidence for #60: converting a Ready PR to Draft emitted skipped Draft runs and cancelled the older Ready runs; marking the unchanged source Ready again re-admitted checks. That evidence is not a substitute for current #59 terminal Cargo RED, current #60 all-gates GREEN, or current #61 documentation checks/review.

Promotion order remains `#59 exact terminal RED + current review → #60 exact all-gates GREEN + current review → ordinary foundation adoption → descendant ancestry repair`.

## Compiler and supply-chain prerequisites (#56 / #54)

Draft #56 is exact `3b70ba734aae5f43a620b32bda5f0b59fe2b602b`, based on foundation #1 with 11 effective files. It moves release-producing compiler authority to Rust 1.98.1 and rejects alternate compiler authority through rustup selectors, direct shell `RUSTC` / `CARGO_BUILD_RUSTC`, and workflow/job/step YAML `env` mappings. Exact hosted GREEN and current-range review are still required.

Draft #54 is exact `02a739c0a3fa2772a649ef54bbd3563ac533ec5b`, an exact child of #56 with four derivative-advisory evidence files. It intentionally requires committed `Cargo.lock` to contain no `derivative`. The intended RUSTSEC-2024-0388 RED is valid only after #56 independently proves its compiler/bootstrap path GREEN; audit ignores or mutable supplier pins are not closure.

## Protocol and public supplier prerequisites

Draft #53 remains blocked from protocol credit while wrong-base workflow changes remain in its lineage. Its H2→H1 Cookie real-wire fixture must return to protocol-test-only scope before supplier RED/GREEN, review-complete, merge, or release credit.

Mutable supplier work is evidence only. The required path is `foundation repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → downstream dependency bump → unchanged wire GREEN`. Zero-length chunk framing and `derivative` removal remain separate supplier prerequisites and must not be collapsed into the Cookie repair.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`, not in the Pingora leaf. Protected `.github/main` is `8272e4f95c253ab067592460cc9288581bf3a422` at this snapshot. The Pingora lane reads owner evidence and reports reproducible specimens but does not mutate `.github` source/refs/PR state while its dedicated writer is active. Delayed runner assignment is not treated as a source race and does not justify no-op commits or runner-selector churn.

## Legacy Nginx/OpenResty migration

`linux-cluster-ops` is dedicated-writer territory. The edge migration remains release-first: safe backup/extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or an unbounded product-specific FastCGI subsystem merely because the legacy proxy co-located them.

## Release and cutover gate

Commercial release credit is forbidden until the exact protected release candidate has version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility evidence, rollback artifact/runbook, and all then-live governance checks. Consumer migration additionally requires parity → shadow/canary → observed rollback path → cutover → verified legacy removal. A Draft, queued/skipped check, static review, local p95 result, mutable supplier PR, documentation-only PR, or available administrative bypass is not equivalent to a shipped edge.

At this snapshot protected `pingora-gateway/main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; immutable release, canary, cutover and legacy-removal counts remain zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline intentionally keeps only code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.
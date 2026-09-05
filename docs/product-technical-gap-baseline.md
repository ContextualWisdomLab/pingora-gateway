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
| Release | Not released | Protected `main` is `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases remains empty. |

## Dependency root: workflow admission repair (#58 / #59 / #60)

Merged #57 placed a valid repository workflow-coalescing delta inside #53's H2→H1 Cookie protocol lineage. Both deltas must survive. Workflow policy is promoted to the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

### RED oracle — #59

Draft #59 is based directly on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`. Current exact head is `8893ab433a06bf25b63385689118d60861fe6a78`; fresh compare is ahead 22 / behind 0 with the foundation as exact merge base. The effective range is exactly three Rust test contracts: `tests/workflow_concurrency_contract.rs`, `tests/workflow_tag_filter_contract.rs`, and `tests/workflow_job_admission_contract.rs`.

Predecessor `8eaccce7251b4eb8666212b57207da31c0146b9d` acquired hosted runners. Supply Chain, OCI and load-contract were GREEN; CI reached `cargo test --all-targets --locked` and failed the then-defined foundation invariants for workflow/repository/PR cancellation identity and protected-main-only duplicate-push scope. That is causal hosted semantic RED evidence, but it does not transfer to a moved or stronger head.

The oracle has since been strengthened for rerun isolation, `converted_to_draft` / `ready_for_review`, direct-job Draft admission, fail-closed event syntax, branch/tag scope, and semantic YAML parsing. Review found `  build: # comment` could evade lexical direct-job counting; `3e452c850a6585ffca94468d841a8b8b3d817040` repaired the jobs-block parser. A following self-review found `jobs: # comment` could still make the lexical guard-count equality vacuous; `2d84080562d5c1d39067d89eb542147e40e217cc` added a semantic `serde_yaml` companion contract.

Fresh review of that companion exposed another fail-open: GitHub Actions accepts scalar and sequence trigger forms such as `on: pull_request` and `on: [push, pull_request]`, while the previous helper only recognized mapping-shaped `on`. An unguarded PR job in either form could therefore be skipped by the semantic Draft-admission oracle. Current `8893ab433a06bf25b63385689118d60861fe6a78` parses all supported semantic trigger shapes (string, sequence, mapping), fails closed on unsupported shapes/non-string sequence entries, and adds scalar/sequence regressions that expose an unguarded direct job. Foundation CI itself still has unguarded PR jobs, so the current contract remains an intentional RED rather than parser-only dead code.

Current exact runs are CI `33946975892` and Supply Chain `33946975863`. They supersede predecessor runs for exact-head credit. Current exact terminal RED and fresh independent exact-range review are still required.

### GREEN implementation — #60

#60 is Ready on exact #59 `8893ab433a06bf25b63385689118d60861fe6a78`. Current exact head is `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a`. Fresh compare is ahead 35 / behind 0 with exact merge base and exactly two effective files: `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml`. No Rust test delta remains in the GREEN range.

The implementation restricts duplicate push evidence to protected `main`; coalesces first-attempt PR work by workflow/repository/PR identity; isolates reruns with `github.run_id`; cancels only PR runs; admits `opened`, `synchronize`, `reopened`, `converted_to_draft`, and `ready_for_review`; and keeps every direct PR job out of the runner queue while Draft. Exact-SHA checkout, test/load/OCI and supply-chain gates remain intact.

After #59 gained semantic trigger-shape coverage, #60 was repaired non-destructively. Current `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a` has old #60 `275aa356b59968337c60a964c6759fb3eb4378ca` as first parent and current #59 as second parent; its tree starts from current RED parent state and reapplies only the two intended workflow blobs. The branch ref advanced with `force=false`.

Current exact runs are CI `33947016327` and Supply Chain `33947016368`. Current all-gates hosted GREEN and fresh independent exact-range review remain required.

### Documentation projection — #61

#61 is the writer-safe documentation child of current #60. It has been non-force restacked using the current #60 tree plus the existing six documentation blobs. Its effective range remains documentation-only: `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. No workflow, production Rust, or test delta belongs in the #61 effective range.

Stable Ready→Draft scheduler evidence captured on an earlier #61 source identity remains behavioral evidence for #60: converting a Ready PR to Draft emitted skipped Draft runs and cancelled the older Ready runs; marking the unchanged source Ready again re-admitted checks. That evidence is not a substitute for current #59 terminal Cargo RED, current #60 all-gates GREEN, or current #61 documentation checks/review.

Promotion order remains `#59 exact terminal RED + current review → #60 exact all-gates GREEN + current review → ordinary foundation adoption → descendant ancestry repair`.

## Compiler and supply-chain prerequisites (#56 / #54)

Draft #56 remains exact `3b70ba734aae5f43a620b32bda5f0b59fe2b602b`, based on foundation #1 with 11 effective files. It moves release-producing compiler authority to Rust 1.98.1 and rejects alternate compiler authority through rustup selectors, direct shell `RUSTC` / `CARGO_BUILD_RUSTC`, and semantic workflow/job/step YAML `env` mappings. Exact hosted GREEN and current-range review are still required. The Rust Official Image catalog still lacks a `1.98.1` image at this snapshot, so the digest-pinned 1.98.0 bootstrap plus rustup 1.98.1 bridge remains a temporary prerequisite rather than removable churn.

Draft #54 remains exact `02a739c0a3fa2772a649ef54bbd3563ac533ec5b`, an exact child of #56 with four derivative-advisory evidence files. It intentionally requires committed `Cargo.lock` to contain no `derivative`. The intended RUSTSEC-2024-0388 RED is valid only after #56 independently proves its compiler/bootstrap path GREEN; audit ignores or mutable supplier pins are not closure.

## Protocol and public supplier prerequisites

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a` and blocked from protocol credit while wrong-base workflow changes remain in its lineage. Its H2→H1 Cookie real-wire fixture is one file, but the effective PR range is still five files because merged #57 added four workflow-policy files. The foundation repair must reach #52/#53 ancestry and restore protocol-test-only scope before supplier RED/GREEN, review-complete, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c`. Cookie candidate #901 remains mutable supplier work until maintainer-integrated current-main truth exists; body-framing #936 and supplier issue #889 remain separate prerequisites. Mutable supplier work is evidence only. The required path is `foundation repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → downstream dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`, not in the Pingora leaf. Protected `.github/main` is now `1e84a69631a1bba959170e1734951f7d3574bdcc` after merged #1895 restored user-directed model-timeout policy by reverting the 900-second OpenCode and Strix limits. That owner movement does not justify leaf no-op commits or runner-selector churn.

Canonical queue-health PR `.github#1150@bbacf9e81ae954eb8365fbfe1856d8698a768a4a` remains stale: against current protected `.github/main` it is diverged 67 ahead / 180 behind with merge base `8c085835fbf77de2321b72fa6b8dd946227e523e`, while its body still describes that old merge base as current protected main. The Pingora lane reports exact specimens to owner issue #712 but does not mutate `.github` source, refs, or PR state while its dedicated writer is active.

## Legacy Nginx/OpenResty migration

`linux-cluster-ops` remains dedicated-writer territory. The edge migration stays release-first: safe backup/extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or an unbounded product-specific FastCGI subsystem merely because the legacy proxy co-located them.

## Release and cutover gate

Commercial release credit is forbidden until the exact protected release candidate has version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility evidence, rollback artifact/runbook, and all then-live governance checks. Consumer migration additionally requires parity → shadow/canary → observed rollback path → cutover → verified legacy removal. A Draft, queued/skipped check, static review, local p95 result, mutable supplier PR, documentation-only PR, or available administrative bypass is not equivalent to a shipped edge.

At this snapshot protected `pingora-gateway/main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases is empty, so immutable release, canary, cutover and legacy-removal counts remain zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline intentionally keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway`. Mutable PR heads are evidence candidates, not release authority. Later live evidence supersedes the exact identities in this snapshot (2026-09-05 KST).

## Product and DDD boundary

`pingora-gateway` owns transport/runtime concerns only: Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It must not duplicate product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave policy authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR heads, and hidden Shared Kernels are rejected. No material admin UI is in the current dependency root.

## Reusable runtime / buyer gap

| Area | Current state | Release/cutover gap |
| --- | --- | --- |
| Pingora executable | Implemented candidate | Every moved exact head must reacquire fmt/compile/test/Clippy/rustdoc/coverage/load/OCI/security/supply-chain evidence. |
| Ingress / routing | Implemented candidate | Production host/SNI/routing parity and shadow traffic remain required. |
| TLS | Partial | Certificate lifecycle stays outside this owner; downstream TLS/H2 and later HTTP/3/QUIC need dependency-ordered traffic evidence. |
| HTTP policy | Partial | H2→H1 Cookie normalization and zero-length chunk framing remain supplier-gated. |
| WebSocket / streaming | Partial | Long-lived-stream shutdown/backpressure/failure traffic still need release-candidate E2E evidence. |
| Runtime isolation | Partial | Body/in-flight/timeout/drain controls exist; broader header budgets and failure traffic remain open. |
| OCI | Implemented candidate | Immutable release, SBOM, provenance, reproducibility and rollback remain open. |
| Performance | Local contract only | Applicable buyer paths require realistic async/k6 p95 ≤20 ms evidence; this is not yet a production SLO. |
| Release | Not released | Protected `main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases is empty. |

## Workflow-admission dependency root (#58 / #59 / #60)

Merged #57 placed a valid workflow-coalescing delta inside #53's H2→H1 Cookie protocol lineage. Both deltas must survive. Workflow policy is promoted to the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

### RED oracle — #59

Draft #59 is based directly on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`. Current exact head is `8893ab433a06bf25b63385689118d60861fe6a78`; fresh compare is ahead 22 / behind 0 with exact merge base and exactly three Rust test contracts: `tests/workflow_concurrency_contract.rs`, `tests/workflow_tag_filter_contract.rs`, and `tests/workflow_job_admission_contract.rs`.

Predecessor `8eaccce7251b4eb8666212b57207da31c0146b9d` acquired hosted runners. Supply Chain, OCI and load-contract were GREEN; CI reached `cargo test --all-targets --locked` and failed the then-defined foundation invariants for workflow/repository/PR cancellation identity and protected-main-only duplicate-push scope. That is causal hosted semantic RED evidence but does not transfer to a moved/stronger head.

The oracle was strengthened for rerun isolation, Ready→Draft retraction, direct-job Draft admission, fail-closed event syntax, branch/tag scope, and semantic YAML parsing. Review found `  build: # comment` could evade lexical direct-job counting; `3e452c850a6585ffca94468d841a8b8b3d817040` repaired the jobs-block parser. A following self-review found `jobs: # comment` could make the lexical job/guard equality vacuous; `2d84080562d5c1d39067d89eb542147e40e217cc` added a semantic `serde_yaml` companion.

Current `8893ab433a06bf25b63385689118d60861fe6a78` makes that semantic companion complete for GitHub Actions' supported `on` value shapes (string, sequence, mapping) and adds regressions for `on: pull_request` and `on: [push, pull_request]`. This is defense-in-depth, not a newly discovered repository-wide bypass: the canonical lexical contract independently requires exact block-style top-level `on:` and already rejects inline/alternate event syntax. The real RED remains the foundation workflows' missing canonical Draft admission guards plus concurrency/push-scope invariants.

Current exact runs are CI `33946975892` and Supply Chain `33946975863`. CI remains pending and Supply Chain queued at the latest read; current exact terminal RED and fresh exact-range review remain required.

### GREEN implementation — #60

#60 is Ready on exact #59 `8893ab433a06bf25b63385689118d60861fe6a78`. Current exact head is `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a`; compare is ahead 35 / behind 0 with exact merge base and exactly two effective files: `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml`.

The implementation restricts duplicate push evidence to protected `main`; coalesces first-attempt PR work by workflow/repository/PR identity; isolates reruns with `github.run_id`; cancels only PR runs; admits `opened`, `synchronize`, `reopened`, `converted_to_draft`, `ready_for_review`; and keeps every direct PR job out of the runner queue while Draft. Exact-SHA checkout, test/load/OCI and supply-chain gates remain intact.

Current `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a` has old #60 `275aa356b59968337c60a964c6759fb3eb4378ca` as first parent and current #59 as second parent; its tree starts from the current RED parent and reapplies only the two intended workflow blobs. The branch ref advanced with `force=false`.

Current exact runs are CI `33947016327` and Supply Chain `33947016368`; current all-gates hosted GREEN and fresh exact-range review remain required.

### Documentation projection — #61

#61 is the writer-safe documentation child of current #60. It is kept documentation-only: `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. No workflow, production Rust, or test delta belongs in its effective range. Stable Ready→Draft scheduler evidence from an earlier #61 identity remains behavioral evidence for #60, not current exact-head gate credit.

Promotion order remains `#59 exact terminal RED + current review → #60 exact all-gates GREEN + current review → ordinary foundation adoption → descendant ancestry repair`.

## Compiler and supply-chain prerequisites (#56 / #54)

Draft #56 is now exact `da8a2c288c7d8664de4097e27441b0dda3c8ed06`, based on foundation #1 with 11 effective files. Exact predecessor `3b70ba734aae5f43a620b32bda5f0b59fe2b602b` acquired a hosted `ubuntu-24.04` runner: Rust 1.98.1 install/select/verify succeeded, `load-contract` and `oci-runtime` were GREEN, and `test` failed deterministically at `cargo fmt --all -- --check` before compile/test, Clippy, rustdoc, coverage and dependency-lock evidence. The formatter requested only two call-layout changes in `tests/toolchain_contract.rs`; current `da8a2c28...` applies exactly those changes without altering compiler authority, assertions, workflow behavior, or thresholds. Current exact CI `33949919187` and Supply Chain `33949919177` are queued, and fresh exact-range review has been requested; current-head hosted GREEN is not yet credited.

Draft #54 is now exact `573cad1ff9880ed6422329e204eeda6f3ef0c3f3`, an exact child of #56 `da8a2c28...` with four effective derivative-advisory evidence files. The restack is an ordinary two-parent commit: old child `02a739c0...` is first parent, current #56 is second parent, and the tree is current #56 plus only `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`; the branch advanced with `force=false`. It intentionally requires committed `Cargo.lock` to contain no `derivative`. The RUSTSEC-2024-0388 RED is valid only after #56 independently proves its compiler/bootstrap path GREEN; audit ignores or mutable supplier pins are not closure. Current exact CI `33949966019` and Supply Chain `33949966039` are queued and fresh child-range review has been requested.

## Protocol and public supplier prerequisites

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a`. Its H2→H1 Cookie real-wire fixture is one file, but the effective range is still five files because merged #57 added four workflow-policy files. Foundation repair must reach #52/#53 and restore protocol-test-only scope before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c`. Mutable supplier candidates remain evidence only. The required path is `foundation repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → downstream dependency bump → unchanged wire GREEN`. Body-framing and `derivative` removal remain separate prerequisites.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` is now `6d7fbebec8aec31d88a30a36e71ca5b3925d241d` after #1896 made startup-failure recovery tests `GITHUB_ACTIONS`-agnostic and reported 2,833 passed / 1 skipped with 100% tracked-module coverage under the CI-shaped environment. Canonical queue-health PR #1150 has also been repaired by its dedicated owner: current head `e6622a428060194b558929ad651d5b4ae3a9840f` is based on current protected main, and fresh compare is **69 ahead / 0 behind** with `6d7fbebe...` as exact merge base. Its PR body still contains older historical wording, but its actual refs/ancestry are no longer the previously observed 180-behind stale state. Pingora reports exact specimens to #712 but does not mutate `.github` source/refs/PR state while its dedicated writer is active.

## Legacy migration and release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe backup/extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Consumer migration additionally requires parity → shadow/canary → observed rollback → cutover → verified legacy removal. Protected `pingora-gateway/main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991` and GitHub Releases remains empty; merge, immutable release, canary, cutover and legacy-removal credit are therefore zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway`. Mutable PR heads are evidence candidates, not release authority. Later live evidence supersedes the exact identities in this snapshot (2026-09-05 KST).

## Product and DDD boundary

`pingora-gateway` owns transport/runtime concerns only: Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It must not duplicate product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave policy authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR heads, and hidden Shared Kernels are rejected. No material admin UI is in the current dependency root.

## Reusable runtime / buyer gap

| Area | Current state | Release/cutover gap |
| --- | --- | --- |
| Pingora executable | Implemented candidate | Every promoted exact head must reacquire fmt/compile/test/Clippy/rustdoc/coverage/load/OCI/security/supply-chain evidence. |
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

### Exact runtime RED — #59

Draft #59 is based directly on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`. Current exact head is `8893ab433a06bf25b63385689118d60861fe6a78`; the effective range is exactly three Rust workflow contracts: `tests/workflow_concurrency_contract.rs`, `tests/workflow_tag_filter_contract.rs`, and `tests/workflow_job_admission_contract.rs`.

The current head now has exact runtime RED for the Draft-admission invariant. While #59 is Draft, CI run `33946975892` materialized `oci-runtime 101254743610`, `load-contract 101254743748`, and `test 101254743868` as queued `ubuntu-24.04` jobs with `steps=[]` and no runner identity. Supply Chain run `33946975863` likewise materialized candidate work. The defect is admission itself: the foundation workflow puts runner-facing jobs into the queue for a Draft PR. A hosted checkout or terminal Cargo failure is not required to prove that invariant violation.

Historical predecessor `8eaccce7251b4eb8666212b57207da31c0146b9d` remains independent semantic evidence: Supply Chain, OCI and load-contract were GREEN, and CI reached `cargo test --all-targets --locked` before failing the then-current concurrency-identity and protected-main-only duplicate-push contracts. It is not substituted for current-head identity.

The oracle also covers rerun isolation, Ready→Draft retraction, direct-job Draft admission, branch/tag scope and semantic YAML parsing. `3e452c850a6585ffca94468d841a8b8b3d817040` repaired `build: # comment` direct-job counting; `2d84080562d5c1d39067d89eb542147e40e217cc` added the `serde_yaml` companion for `jobs: # comment`; current `8893ab433...` covers string/sequence/mapping `on` values. Alternate trigger-shape coverage is defense-in-depth because the canonical lexical contract already requires block-style top-level `on:`.

### Exact runtime Draft-admission GREEN — #60

#60 is now Draft while its parent remains the unresolved RED root. Current exact head is `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a`, based on exact #59 `8893ab433...`; its effective child delta is exactly `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml`.

The implementation restricts duplicate push evidence to protected `main`; coalesces first-attempt PR work by workflow/repository/PR identity; isolates reruns with `github.run_id`; cancels only PR runs; admits `opened`, `synchronize`, `reopened`, `converted_to_draft`, and `ready_for_review`; and guards every direct PR job with `github.event_name != 'pull_request' || github.event.pull_request.draft == false`.

Converting unchanged #60 from Ready to Draft exercised the actual `converted_to_draft` path. CI `33951166913` and Supply Chain `33951166945` both completed `skipped`. CI jobs `oci-runtime 101266101595`, `test 101266116987`, and `load-contract 101266122403` completed `skipped` with `steps=[]` and no runner assignment; Supply Chain `candidate-evidence 101266102071` did the same. This is exact current runtime GREEN for Draft admission/retraction and directly contrasts with the queued jobs on Draft #59.

Older Ready-state runs CI `33947016327` and Supply Chain `33947016368` were cancelled when #60 became Draft. They are not all-gates GREEN. Foundation adoption still requires a fresh independent exact-range review and later Ready-state all-gates hosted GREEN on the unchanged candidate after the RED parent review is complete.

### Documentation projection — #61

#61 is a Draft writer-safe documentation child of current #60. Before this baseline update its exact head was `25f2115f3618c86f77c1abc55ae2dd67ae612355`, with exactly six documentation files in the effective range: `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. No production Rust, workflow or test delta belongs in the range.

The same Draft transition independently reproduced the GREEN behavior on that documentation-only descendant: CI `33951201424` and Supply Chain `33951201439` completed `skipped`; Supply Chain `candidate-evidence 101266214201` had `steps=[]` and no runner assignment. This is behavioral confirmation, not a substitute for #60 Ready-state all-gates GREEN.

Promotion order is `#59 exact Draft-admission RED + current review → #60 exact Draft-admission GREEN + current review → #60 Ready-state all-gates GREEN → ordinary foundation adoption → descendant ancestry repair`.

## Compiler and supply-chain prerequisites (#56 / #54)

Draft #56 is exact `da8a2c288c7d8664de4097e27441b0dda3c8ed06`, based on foundation #1 with 11 effective files. Exact predecessor `3b70ba734aae5f43a620b32bda5f0b59fe2b602b` acquired a hosted `ubuntu-24.04` runner: Rust 1.98.1 install/select/verify succeeded, `load-contract` and `oci-runtime` were GREEN, and `test` failed deterministically at `cargo fmt --all -- --check` before compile/test, Clippy, rustdoc, coverage and dependency-lock evidence. Current `da8a2c28...` applies the requested formatter output without changing compiler authority or thresholds.

Current #56 exact CI `33949919187` and Supply Chain `33949919177` are now completed/cancelled, not queued. Current-head compiler/bootstrap GREEN is therefore still absent. Exact-head all-event evidence also shows both push and pull-request workflow admissions under the old foundation workflow; this remains evidence for the workflow-foundation repair rather than a reason to alter the Rust compiler contract.

Draft #54 remains the exact child lane for RUSTSEC-2024-0388 `derivative` removal evidence. Its intended RED may be credited only after #56 independently proves its compiler/bootstrap path GREEN; audit ignores or mutable supplier pins are not closure.

## Protocol and public supplier prerequisites

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a`. Its H2→H1 Cookie real-wire fixture is one file, but the effective range is still five files because merged #57 added four workflow-policy files. Foundation repair must reach #52/#53 and restore protocol-test-only scope before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the latest verified sweep. Mutable supplier candidates remain evidence only. The required path is `foundation repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → downstream dependency bump → unchanged wire GREEN`. Body-framing and `derivative` removal remain separate prerequisites.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` is `6d7fbebec8aec31d88a30a36e71ca5b3925d241d` after #1896 made startup-failure recovery tests `GITHUB_ACTIONS`-agnostic. Canonical queue-health PR #1150 has been reconciled by its dedicated owner: current head `e6622a428060194b558929ad651d5b4ae3a9840f` targets current protected main. The previous 180-behind ancestry state is no longer current authority. Pingora may report exact specimens to #712 but does not mutate `.github` source/refs/PR state while its dedicated writer is active.

## Legacy migration and release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe backup/extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Consumer migration additionally requires parity → shadow/canary → observed rollback → cutover → verified legacy removal. Protected `pingora-gateway/main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991` and GitHub Releases remains empty at the latest verified sweep; merge, immutable release, canary, cutover and legacy-removal credit are therefore zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

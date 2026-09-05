# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-05 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Production host/SNI/routing parity, H2/H3/TLS/WebSocket/streaming failure traffic, immutable packaging, SBOM/provenance/reproducibility, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. Protected `main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases is empty at the latest verified sweep.

## Workflow-admission dependency root — #58 / #59 / #60

Merged #57 placed valid workflow-coalescing policy inside #53's H2→H1 Cookie lineage. Both deltas must survive. Workflow policy is promoted at the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

Draft #59 is based on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`, exact head `8893ab433a06bf25b63385689118d60861fe6a78`, with exactly three Rust workflow contracts in its effective range. Its current Draft head materialized runner-facing CI/Supply Chain work under the foundation workflows, establishing exact admission RED before checkout. Fresh CodeRabbit review of `5a62e2fa...8893ab43` reported no new source finding; hosted execution remains runtime authority.

#60 is Ready on exact #59 and remains exact `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a`, with only `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` in its effective child range. The unchanged candidate proved Draft admission/retraction GREEN when CI `33951166913` and Supply Chain `33951166945` completed skipped with no runner work. It was then returned to Ready without changing SHA. Current Ready-state CI `33952390050` and Supply Chain `33952390006` remain the authority for all-gates GREEN; the CI jobs are still materialized without runner assignment at the latest verified reads. CodeRabbit reports no static findings for exact range `8893ab43...f1b09eb0`. Do not merge on Draft-only evidence; promotion requires current Ready-state terminal GREEN.

#61 is a Draft documentation-only child of #60. Its effective range must remain exactly `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. Draft synchronize runs that complete skipped are descendant admission parity only, not a substitute for #60 Ready-state all-gates GREEN.

Promotion order remains `#60 current Ready-state all-gates GREEN → ordinary foundation adoption → #52/#53 non-force ancestry repair`.

## Compiler and supply-chain roots — #56 / #54

Draft #56 is exact `1e2d6ab3815559c7174abec50c9a6b8801ad1d64` on foundation #1 with 11 effective files. Its compiler-authority review chain found and repaired three independent High fail-open classes without weakening the fixed Rust 1.98.1 contract:

1. YAML workflow/job/step `env` rejected `RUSTC` and `CARGO_BUILD_RUSTC` but not `RUSTUP_TOOLCHAIN`; `8f03a6ca...` added semantic YAML rejection and a step-level `RUSTUP_TOOLCHAIN: 1.98.0` regression.
2. Shell authority parsing missed Bash compound assignments because `RUSTC+=...`, `CARGO_BUILD_RUSTC+=...`, and `RUSTUP_TOOLCHAIN+=...` produced names ending in `+`; `905ce172...` reduces `=` and `+=` forms to the base variable and adds workflow/Docker regressions.
3. The Docker build boundary used raw `match_indices("cargo build")`, so a comment could impersonate the only build position while a later `cargo<TAB>build` or `car"go" build` escaped the guarded span. Current `1e2d6ab3...` extracts executable shell-form Docker `RUN` bodies, excludes comments/metadata, normalizes shell words, finds semantic Cargo build authority, and tests both obfuscated executable forms after a forbidden authority change.

Fresh independent exact-range review of `5a62e2fa...1e2d6ab3` reports no new source issue and explicitly confirms the Docker build-boundary finding is closed. The review did not execute Rust compilation/rustfmt/tests/Clippy. Current exact CI `33955624092` and Supply Chain `33955624105` remain queued, so compiler/bootstrap GREEN is absent and predecessor terminal evidence does not transfer.

Draft #54 has been non-force restacked onto current #56 with ordinary two-parent commit `d0446a50cd23826d3afd1068ffe4452506a6adb7`. Its first parent is previous child `d9700905f1eb750d685242fdf00ea47e44b75fa2`; its second parent is current #56 `1e2d6ab3...`. Fresh compare is ahead 35 / behind 0 with current #56 as exact merge base and exactly four effective child files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. No compiler-contract delta remains in the child range. Current exact CI `33955726223` and Supply Chain `33955726215` are queued. The `derivative` lock assertion is intentional RED only after #56 independently proves compiler/bootstrap GREEN; fresh exact-range review has been requested and predecessor review does not transfer.

Do not add an audit ignore, suppress OSV/RustSec, delete lock evidence, or pin mutable supplier source to manufacture supply-chain GREEN. `RUSTSEC-2024-0388` remains an INFO/Unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

## Protocol / supplier path

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a`, open/Draft/mergeable, with five changed files. Its H2→H1 Cookie real-wire fixture remains one file, but four #57 workflow deltas still contaminate the effective range. Foundation workflow repair must reach #52/#53 and restore protocol-test-only scope before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the latest supplier sweep. Public Cookie PR #901 remains open/unmerged and mergeable false at `b856ddfc6be15f1727601d2d76cb10d2d72f95f0`; it is evidence, not immutable dependency authority. Body-framing PR #936 remains open/unmerged/mergeable at `e40ed4cceb0c0ed8c05cc39eb01a8c73dea5497a` on current protected main. Supplier issue #889 for `RUSTSEC-2024-0388` remains open. Required order is `foundation repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → gateway dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` advanced during this run to exact `525d16b14b02b87d98bb3b15762f2fa5f82b591f` with merged #1906, which documents fail-closed verification of "superseded" closure claims and corrects ancestry/diff guidance. Dedicated-owner queue-health #1150 remains open at exact head `e6622a428060194b558929ad651d5b4ae3a9840f`, live base `6d7fbebec8aec31d88a30a36e71ca5b3925d241d`; fresh compare against current protected main is diverged **69 ahead / 3 behind**, with `6d7fb...` as merge base. Its description still claims an older protected base and that it is no longer behind, so live base/head/compare metadata is authority. Pingora does not mutate `.github` source/refs/PR state while its dedicated writer is active.

The Pingora specimen distinguishes three states: correct Draft-policy `skipped` jobs with zero steps; leaf-workflow defects that admit Draft jobs into the runner queue; and Ready/non-gated current jobs that materialize but remain unassigned because of owner-plane queue conditions. These must not be normalized into a generic runnerless state.

## Legacy migration / release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current merge, immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

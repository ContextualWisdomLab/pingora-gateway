# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-05 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Production host/SNI/routing parity, H2/H3/TLS/WebSocket/streaming failure traffic, immutable packaging, SBOM/provenance/reproducibility, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. Protected `main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases is empty at the latest verified sweep.

## Workflow-admission dependency root — #58 / #59 / #60

Merged #57 placed valid workflow-coalescing policy inside #53's H2→H1 Cookie lineage. Both deltas must survive. Workflow policy is promoted at the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

### Exact current RED — #59

Draft #59 is based on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`, exact head `8893ab433a06bf25b63385689118d60861fe6a78`, with exactly three Rust workflow contracts in its effective range.

The current head has direct runtime RED for Draft admission. While #59 is Draft, CI `33946975892` and Supply Chain `33946975863` materialized runner-facing work under the foundation workflows. The invariant is violated at admission, before checkout, so terminal Cargo execution is not required for this current-head RED.

Historical predecessor `8eaccce7251b4eb8666212b57207da31c0146b9d` remains independent semantic evidence: OCI/load/Supply Chain were GREEN and CI reached Cargo tests before failing the then-current concurrency/protected-main contracts. It is not transferred as current-head credit.

Fresh CodeRabbit exact-range review of `5a62e2fa56fdaa6f97c0518932711739e347c04a...8893ab433a06bf25b63385689118d60861fe6a78` reported no new findings. The review verified the three-contract-only scope, fail-closed trigger syntax, top-level event boundary, exact `[main]` combined-event push scope, per-direct-job Draft guard enforcement, semantic scalar/sequence/mapping PR trigger detection, semantic tag-filter normalization, and exclusion of push-only release/tag workflows. The review sandbox did not execute Rust tests, so hosted execution remains the runtime authority.

### Exact current GREEN candidate — #60

#60 is Ready on exact #59 head `8893ab433a06bf25b63385689118d60861fe6a78`; exact head is `f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a`, and the effective child delta is exactly `.github/workflows/ci.yml` plus `.github/workflows/supply-chain.yml`.

The repaired workflows limit duplicate push evidence to protected `main`, coalesce first-attempt PR work by workflow/repository/PR identity, isolate reruns with `github.run_id`, cancel only PR runs, listen for Ready/Draft state changes, and guard every direct PR job against Draft execution.

Converting the unchanged candidate from Ready to Draft produced CI `33951166913` and Supply Chain `33951166945`; both completed `skipped`, and all direct jobs completed skipped with `steps=[]` and no runner assignment. This is exact runtime GREEN for Draft admission/retraction and directly contrasts with Draft #59's admitted work.

After #59's current exact review completed clean, the unchanged #60 candidate was returned to Ready. The `ready_for_review` transition emitted current Ready-state CI `33952390050` and Supply Chain `33952390006`. They are queued at the latest verified read and are the authority for all-gates hosted GREEN; older Ready-state runs `33947016327` / `33947016368` were cancelled by the Draft transition and do not transfer.

CodeRabbit also reported no static findings for the exact range `8893ab433a06bf25b63385689118d60861fe6a78...f1b09eb0a669a6f9f439daf250b1c7d0d95b6c1a`: the base is an ancestor, only the two workflow files change, the parent contract is byte-identical, and `git diff --check` passed. Runtime promotion still requires the current Ready-state all-gates hosted GREEN.

### Documentation projection — #61

#61 is a Draft documentation-only child of #60. This file remains one of exactly six effective documentation files; no production Rust, workflow, or test delta belongs in the range. Draft synchronize runs on the documentation child complete skipped without runner work under the inherited admission policy; this is descendant parity evidence, not a substitute for #60 Ready-state all-gates GREEN.

Promotion order is `#60 current Ready-state all-gates GREEN → ordinary foundation adoption → descendant ancestry repair`. The RED parent and both current exact static reviews are already established and must not be re-opened by predecessor evidence transfer.

## Compiler and supply-chain roots — #56 / #54

Concurrent work advanced Draft #56 to exact `50b1f8c4b645622f61dfa25f74ac2bb82968837e` on the foundation. Commit `50b1f8c4...` is a real compiler-authority hardening delta: it normalizes shell words before authority comparison, centralizes command-basename and environment-assignment handling, rejects `RUSTC`, `CARGO_BUILD_RUSTC`, and `RUSTUP_TOOLCHAIN` authority changes, rejects shell-obfuscated `rustup default/toolchain install/override/run` after verification, and applies the same post-verification guard to the Docker compiler path. This supersedes the previous `da8a2c28...` snapshot.

Current #56 exact runs are CI `33950593604` (queued at the latest read) and Supply Chain `33950593607` (pending). Current-head compiler/bootstrap GREEN is therefore absent; predecessor terminal evidence does not transfer.

Concurrent work also restacked Draft #54 onto current #56. Its live PR metadata now reports base `50b1f8c4...`, exact head `3d65b786ec19f8c139b575117115767de3ac90be`, and four effective files. Commit `3d65b786...` is the ordinary restack carrying the current compiler-authority repair while preserving the derivative-advisory RED lane. The intended RUSTSEC-2024-0388 `derivative` RED may promote only after #56 independently proves compiler/bootstrap GREEN; audit ignores or mutable supplier pins are not closure.

## Protocol / supplier path

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a`, changed files 5. Its H2→H1 Cookie real-wire fixture is one file, but four #57 workflow files remain in the effective range. Foundation repair must reach #52/#53 and restore protocol-test-only scope before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the latest verified sweep. Mutable supplier PRs are evidence only. Required order is `foundation repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → gateway dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` is `6d7fbebec8aec31d88a30a36e71ca5b3925d241d` at the latest verified owner sweep. Queue-health #1150 has been reconciled by its dedicated owner to head `e6622a428060194b558929ad651d5b4ae3a9840f` on that base; its PR description still contains predecessor-base prose, so live base/head metadata rather than stale narrative is authority. Pingora reports exact specimens to #712 but does not mutate `.github` source/refs/PR state while its dedicated writer is active.

The current Pingora specimen distinguishes three states that must not be normalized together: correct Draft-policy `skipped` jobs with zero steps; leaf-workflow defects that admit Draft jobs into the runner queue; and Ready/non-gated current jobs that materialize but remain unassigned because of owner-plane queue conditions.

## Legacy migration / release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current merge, immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

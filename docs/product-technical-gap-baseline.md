# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. That evidence is exact-head loopback CI, not buyer-path WAN/TLS/H2/H3 performance. Production host/SNI/routing parity, realistic TLS/H2/H3/WebSocket/streaming failure traffic, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates.

Protected `main` remains exact `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`. GitHub Releases remains empty at the latest verified sweep. No protected-main merge, immutable release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission foundation promotion — #58 / #59 / #60

Merged #57 had placed valid workflow-coalescing policy inside #53's H2→H1 Cookie lineage. The workflow delta was therefore promoted at the earliest workflow-owning foundation ancestor without closing or discarding dependent protocol work.

#59 exact RED predecessor `cb458621b5cfdcec35820083bb82e19e9dc627cf` is hosted semantic evidence. CI `33965929673` acquired real `ubuntu-24.04` runners and exact checkout: load-contract and OCI succeeded, while `cargo test --all-targets --locked` failed exactly two intended workflow-concurrency invariants. The workflow-concurrency binary was 11 passed / 2 failed: the foundation CI lacked the required PR admission/cancellation event contract and did not restrict duplicate push evidence to protected `main`.

GREEN child #60 exact `e2309106eba4f41f45c770676885b27bf73aa69b` retained exactly `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` in its effective child range. CI `33966008873` and Supply Chain `33966008876` both completed success. Exact k6 artifact `9970973185` has digest `sha256:2f6e8b16813a58498a12df71bc38feae33e4a6193cc426b8351ad32185eba57d`; its 400 requests had zero failures, 400/400 HTTP-200 checks, 400/400 upstream-body checks and loopback p95 `1.56505725 ms`.

#60 was normally merged into #59, producing combined exact `cf60f0bce57a8ac530e8fff52fa9ae00be232f07`. After Draft skip evidence, #59 was marked Ready to execute the unchanged tree. Combined-head CI `33971798747` and Supply Chain `33971798802` both reached terminal success; load, OCI, formatting, all Cargo tests, strict Clippy, rustdoc, owned-production coverage, dependency-lock evidence and supply-chain candidate evidence completed successfully. All returned review threads were resolved.

#59 was then normally merged into foundation #1 as exact `0da81a93f93e869c15bb7d34c55fc87479d16522`. No force update, destructive rebase, bypass, self-approval, or predecessor-success synthesis was used. Foundation now owns the repaired workflow semantics: `main`-only push, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts.

PR #61 has non-force adopted the foundation merge and is retargeted directly to `feat/initial-pingora-runtime`. Its effective child range remains exactly `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. Documentation evidence does not replace compiler/supplier execution gates.

## Compiler and supply-chain roots — #56 / #54

Compiler repair #56 is current exact `ce3de547f5f3fe6adc1cfab9cf48f256750ffd13`, non-force reconciled onto current foundation `0da81a93f93e869c15bb7d34c55fc87479d16522`. Fresh compare uses foundation as exact merge base, ahead 73 / behind 0, with 15 compiler/bootstrap files in the effective child range. The promoted workflow admission/concurrency/tag-filter contract files are inherited from foundation rather than copied as #56 child delta.

The integrated CI/Supply Chain workflows preserve foundation admission semantics while selecting and explicitly verifying Rust 1.98.1. Rust 1.98.1 remains the required release compiler because the Rust Release Team published it on 2026-09-03 to repair the vtable-generation miscompilation introduced in 1.98.0.

The compiler-authority review chain covers job-scoped YAML environment authority, Bash compound assignments, Docker executable-build detection, explicit Cargo toolchain selectors across whitespace/continuations/quoting, shell control operators, the `command` builtin, GNU `env -S` and bundled `-iS`, child-command operand boundaries, GNU `env --`, legacy backticks, POSIX-style `$()` command substitution, parameter-expanded Cargo aliases, and Cargo executable indirection through shell-variable command words.

Exact source RED `e77955d6f8546c96dc66fbf1865e2b17a696c077` proves that `CARGO=cargo; $CARGO build --release --locked`, an absolute Cargo alias executed with a `+<toolchain>` selector, and exported `${CARGO:?}` command execution could bypass direct Cargo discovery without `$()`. Its hosted CI was cancelled pre-checkout, so this remains source RED rather than hosted RED.

Repair `2aed90c2...` introduced bounded command-position alias tracking, but exact-range CodeRabbit review found a valid Medium false positive: ordinary command-local prefix `CARGO=cargo printf ok; "$CARGO" build ...` was incorrectly persisted into the parent shell. Source RED `3296d106409631c13cd4b7333f29f7068e65b2c2` adds that allowed control. Repair `0d04619d...`, inherited by current `ce3de547...`, persists aliases only for assignment-only segments or `export`, removes recorded aliases on `unset`, and does not retain ordinary command-local assignment prefixes. `$NAME` / `${NAME...}` is rejected only when a persistent recorded Cargo alias is actually the command word.

The reconciliation head first produced Draft-policy skips `CI 33976932275` / `Supply Chain 33976932383`. #56 is now Ready solely to exercise the unchanged exact tree under the inherited `ready_for_review` contract. Current exact CI `33976999379` and Supply Chain `33976999371` are queued at the latest verified read. Predecessor execution/review results do not transfer; current `ce3de547...` is not yet hosted GREEN.

Draft #54 was non-force restacked on current #56. Current exact head is `89922d2dfd6ddbb5b4f6fc80c3cf7904888a7521`; fresh compare uses `ce3de547...` as exact merge base, ahead 60 / behind 0, and effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. Its current Draft CI `33976953832` and Supply Chain `33976953831` completed skipped, proving admission parity only.

#54 intentionally requires committed `Cargo.lock` to contain no `derivative`. That assertion becomes semantic RED only after #56 independently proves compiler/bootstrap GREEN. Do not add an audit ignore, suppress OSV/RustSec, delete lock evidence, or pin mutable supplier source to manufacture GREEN. `RUSTSEC-2024-0388` remains an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

## Protocol / supplier path

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a` with five changed files at the last verified read. Its H2→H1 Cookie real-wire fixture remains one file, but four #57 workflow deltas still contaminate the effective range. Because workflow policy is now canonical in foundation, #52/#53 must be non-force ancestry-repaired so the protocol RED becomes fixture-only before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the latest verified sweep. Cookie #901 remains open, unmerged, and non-mergeable at `b856ddfc6be15f1727601d2d76cb10d2d72f95f0`; its base `c0845a8693b0792a6ccd0626e8475990f7269af2` predates current public main. It therefore remains mutable supplier evidence requiring current-main adaptation rather than downstream pinning. Body-framing #936 and supplier issue #889 remain separate mutable prerequisites until freshly reverified and maintainer-integrated.

Required order is `foundation workflow repair (done) → #52/#53 ancestry repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → gateway dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` remains exact `7f4c5e3e0efb7bfe29f33b60d4264858effd2996` via merged #1937 at the latest verified sweep. The repaired pre-review scheduler waits while newest current-head checks are queued or running instead of merging fresh `main` into a PR after a long queue wait and cancelling/requeueing those checks.

Canonical queue-health #1150 remains a dedicated owner lane. Its current head `e6622a428060194b558929ad651d5b4ae3a9840f` is 69 commits ahead / 18 behind protected `.github/main`, with merge base `6d7fbebec8aec31d88a30a36e71ca5b3925d241d`; its PR body still describes an older protected-base reconciliation. Pingora hands off exact evidence through the existing #712 specimen but does not mutate `.github` source, refs, or PR state while its dedicated writer is active.

Runner delay remains distinct from semantic failure. Evidence classification distinguishes startup failure with zero jobs, materialized pre-checkout jobs with no runner/steps, Draft-policy skips, source-level RED, hosted semantic RED, and terminal hosted GREEN.

## Legacy migration / release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current protected-main merge, immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

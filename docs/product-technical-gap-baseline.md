# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Exact workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. That evidence is exact-head loopback CI, not buyer-path WAN/TLS/H2/H3 performance. Production host/SNI/routing parity, realistic TLS/H2/H3/WebSocket/streaming failure traffic, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates.

Protected `main` is exact `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`. GitHub Releases remains empty at the latest verified sweep. No protected-main merge, immutable release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission dependency root — #58 / #59 / #60

Merged #57 placed valid workflow-coalescing policy inside #53's H2→H1 Cookie lineage. Both deltas must survive. Workflow policy is promoted at the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

#59 exact RED predecessor `cb458621b5cfdcec35820083bb82e19e9dc627cf` is hosted semantic evidence. CI `33965929673` acquired real `ubuntu-24.04` runners and exact checkout: load-contract and OCI succeeded, while `cargo test --all-targets --locked` failed exactly two intended workflow-concurrency invariants. The workflow-concurrency binary was 11 passed / 2 failed: the foundation CI lacked the required PR admission/cancellation event contract and did not restrict duplicate push evidence to protected `main`.

Ready child #60 exact `e2309106eba4f41f45c770676885b27bf73aa69b` retained exactly `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` in its effective child range. CI `33966008873` completed success through load-contract, OCI runtime, formatting, all Cargo tests, strict Clippy, public rustdoc, owned-production coverage enforcement and dependency-lock evidence. Supply Chain `33966008876` completed success through dependency-policy audit, exact candidate image build, SPDX SBOM, image scan, exact-source binding and evidence upload. Exact k6 artifact `9970973185` has digest `sha256:2f6e8b16813a58498a12df71bc38feae33e4a6193cc426b8351ad32185eba57d`; its 400 requests had zero failures, 400/400 HTTP-200 checks, 400/400 upstream-body checks and loopback p95 `1.56505725 ms`.

#60 was normally merged into the #59 owner branch, producing combined exact head `cf60f0bce57a8ac530e8fff52fa9ae00be232f07`. Fresh foundation compare retained exactly five effective files: the two workflow repairs and the three Rust workflow contracts. Draft admission first produced skipped runs `33971786315` / `33971786316`; #59 was then marked Ready so the unchanged combined tree could exercise the `ready_for_review` contract. Current exact runs `CI 33971798747` and `Supply Chain 33971798802` remain queued. The CI `test`, `load-contract`, and `oci-runtime` jobs are materialized on exact `cf60f0bc...` but remain pre-checkout with `steps=[]` and `runner_id=0`. #60 predecessor GREEN is not transferred to the merge commit.

PR #61 is the documentation-only child of current #59. Its effective range remains `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. Current Draft skip evidence proves inherited admission parity only and does not replace current #59 combined-head all-gates GREEN.

Promotion order is `#59 combined exact CI + Supply Chain GREEN → ordinary merge into foundation #1 → #52/#53 non-force ancestry repair`.

## Compiler and supply-chain roots — #56 / #54

Draft #56 is current exact `2aed90c2b8eb5c830cfcb3c963b322d38af73ea6` on foundation #1. Rust 1.98.1 remains the required release compiler because the Rust Release Team published it on 2026-09-03 to repair the vtable-generation miscompilation introduced in 1.98.0.

The compiler-authority review chain covers job-scoped YAML environment authority, Bash compound assignments, Docker executable-build detection, explicit Cargo toolchain selectors across whitespace/continuations/quoting, shell control operators, the `command` builtin, GNU `env -S` and bundled `-iS`, child-command operand boundaries, GNU `env --`, legacy backticks, POSIX-style `$()` command substitution, parameter-expanded Cargo aliases, and now Cargo executable indirection through shell-variable command words.

Earlier REDs proved that active `$()` could hide `RUSTUP_TOOLCHAIN`/`RUSTC`/`CARGO_BUILD_RUSTC`, that partial closing-parenthesis parsing was unsafe, and that `${CARGO:?}`, `${CARGO:-cargo}`, and `${CARGO=cargo}` require quote/escape-aware parameter-name analysis. Repair `f10cd3a6...` bounded this analysis to semantic workflow `run` steps and Docker shell-form `RUN` commands rather than claiming a complete POSIX shell parser.

Fresh source inspection then found a distinct no-command-substitution fail-open. `CARGO=cargo; $CARGO build --release --locked`, `CARGO=/.../cargo; "$CARGO" +1.98.0 build ...`, and `export CARGO=cargo; ${CARGO:?} build ...` could execute Cargo while bypassing the direct Cargo-token discovery and the `$()`-gated authority helper. Exact source RED `e77955d6f8546c96dc66fbf1865e2b17a696c077` adds those three regressions before the repair.

Current repair `2aed90c2b8eb5c830cfcb3c963b322d38af73ea6` adds bounded command-position parsing to `tests/support/toolchain_command_substitution.rs`. It tracks simple and exported aliases whose value resolves to a Cargo executable, invalidates recorded aliases on reassignment, and rejects `$NAME` or `${NAME...}` only when the recorded Cargo alias appears as the command word. Variable use as an ordinary argument is not classified as Cargo execution. Existing command-substitution analysis remains separate. This is compiler-policy test-oracle work; gateway runtime behavior is unchanged.

Current #56 CI `33976211902` and Supply Chain `33976211911` are exact-head gates and remain queued at the latest verified read. Exact RED commits remain source evidence until hosted execution independently proves them; current `2aed90c2...` is source repair rather than hosted GREEN. A fresh exact-range technical review was requested for `5a62e2fa...2aed90c2`; predecessor review credit is not transferred.

Draft #54 was non-force restacked on current #56 by ordinary two-parent ancestry. Its current head is `f88efe18ffa36478d5676c9935e43ab424e6ce94`; fresh compare uses `2aed90c2...` as exact merge base, ahead 58 / behind 0, and effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. No compiler-contract/support delta belongs in the child range. Current #54 CI `33976332032` is queued and Supply Chain `33976332036` is pending; no predecessor execution credit transfers.

#54 intentionally requires committed `Cargo.lock` to contain no `derivative`. That assertion is semantic RED only after #56 independently proves compiler/bootstrap GREEN. Do not add an audit ignore, suppress OSV/RustSec, delete lock evidence, or pin mutable supplier source to manufacture GREEN. `RUSTSEC-2024-0388` remains an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

## Protocol / supplier path

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a` with five changed files. Its H2→H1 Cookie real-wire fixture remains one file, but four #57 workflow deltas still contaminate the effective range. Foundation workflow repair must reach #52/#53 and restore protocol-test-only scope before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c`. Cookie #901 remains open, unmerged, and non-mergeable at `b856ddfc6be15f1727601d2d76cb10d2d72f95f0`; its base `c0845a8693b0792a6ccd0626e8475990f7269af2` predates current public main. It therefore remains mutable supplier evidence requiring current-main adaptation rather than downstream pinning. Body-framing #936 and supplier issue #889 remain separate mutable prerequisites until freshly reverified and maintainer-integrated.

Required order is `foundation workflow repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → gateway dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` is exact `7f4c5e3e0efb7bfe29f33b60d4264858effd2996` via merged #1937, `fix(scheduler): hold pre-review branch updates while current-head checks are in flight`. The repaired pre-review scheduler waits while newest current-head checks are queued or running instead of merging fresh `main` into a PR after a long queue wait and cancelling/requeueing those checks. The owner change deliberately has no age cap so the same cancellation loop cannot restart. This is directly relevant to observed Pingora runner queue churn, but it does not convert any leaf result into success.

Dedicated queue-health work remains a separate `.github` owner lane. Pingora hands off owner evidence through the existing #712 specimen but does not mutate `.github` source, refs, or PR state while its dedicated writer is active.

Runner delay remains distinct from semantic failure. Evidence classification must distinguish startup failure with zero jobs, materialized pre-checkout jobs with no runner/steps, Draft-policy skips, source-level RED, hosted semantic RED, and terminal hosted GREEN.

## Legacy migration / release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current protected-main merge, immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-06 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Workflow-policy candidate #60 proved a bounded loopback gateway path on 400 requests with zero failed requests and `http_req_duration p(95)=1.56505725 ms`, below the repository `p(95)<20` threshold. That evidence is exact-head loopback CI, not buyer-path WAN/TLS/H2/H3 performance. Production host/SNI/routing parity, realistic TLS/H2/H3/WebSocket/streaming failure traffic, immutable packaging, SBOM/provenance/reproducibility at a protected release candidate, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates.

Protected `main` remains exact `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`. GitHub Releases remains empty at the latest verified sweep. No protected-main merge, immutable release, canary, cutover, or verified Nginx/OpenResty removal is credited.

## Workflow-admission foundation promotion — #58 / #59 / #60

Merged #57 had placed valid workflow-coalescing policy inside #53's H2→H1 Cookie protocol lineage. The workflow delta was therefore promoted at the earliest workflow-owning foundation ancestor without closing or discarding dependent protocol work.

PR `#59` exact RED predecessor `cb458621b5cfdcec35820083bb82e19e9dc627cf` is hosted semantic evidence. CI `33965929673` acquired real `ubuntu-24.04` runners and exact checkout: load-contract and OCI succeeded, while `cargo test --all-targets --locked` failed exactly two intended workflow-concurrency invariants. The workflow-concurrency binary was 11 passed / 2 failed: the foundation CI lacked the required PR admission/cancellation event contract and did not restrict duplicate push evidence to protected `main`.

GREEN child #60 exact `e2309106eba4f41f45c770676885b27bf73aa69b` retained exactly `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` in its effective child range. CI `33966008873` and Supply Chain `33966008876` both completed success. Exact k6 artifact `9970973185` has digest `sha256:2f6e8b16813a58498a12df71bc38feae33e4a6193cc426b8351ad32185eba57d`; its 400 requests had zero failures, 400/400 HTTP-200 checks, 400/400 upstream-body checks and loopback p95 `1.56505725 ms`.

PR `#60` was normally merged into #59, producing combined exact `cf60f0bce57a8ac530e8fff52fa9ae00be232f07`. After Draft skip evidence, #59 was marked Ready to execute the unchanged tree. Combined-head CI `33971798747` and Supply Chain `33971798802` both reached terminal success; load, OCI, formatting, all Cargo tests, strict Clippy, rustdoc, owned-production coverage, dependency-lock evidence and supply-chain candidate evidence completed successfully. All returned review threads were resolved.

PR `#59` was then normally merged into foundation #1 as exact `0da81a93f93e869c15bb7d34c55fc87479d16522`. No force update, destructive rebase, bypass, self-approval, or predecessor-success synthesis was used. Foundation now owns the repaired workflow semantics: `main`-only push, explicit PR lifecycle events, first-attempt PR coalescing with rerun isolation, PR-only cancellation, Draft job guards, and semantic regression contracts.

PR `#61` is a Draft documentation-only child of this foundation. Its effective child range remains six documents. Documentation evidence does not replace compiler/supplier execution gates.

## Compiler and supply-chain roots — #56 / #54

Compiler repair #56 is current exact `8e64640d4db7130fb5cd78692c5e4d7a7471e990`, non-force reconciled onto current foundation `0da81a93f93e869c15bb7d34c55fc87479d16522`. The promoted workflow admission/concurrency/tag-filter contract files are inherited from foundation rather than copied as #56 child delta.

The integrated CI/Supply Chain workflows preserve foundation admission semantics while selecting and explicitly verifying Rust 1.98.1. Rust 1.98.1 remains the required release compiler because the Rust Release Team published it on 2026-09-03 to repair a vtable-generation miscompilation in 1.98.0 that could emit a null function pointer and lead to undefined behavior.

The compiler-authority review chain covers job-scoped YAML environment authority, Bash compound assignments, Docker executable-build detection, explicit Cargo toolchain selectors across whitespace/continuations/quoting, shell control operators, the `command` builtin, GNU `env -S` and bundled `-iS`, child-command operand boundaries, GNU `env --`, legacy backticks, POSIX-style `$()` command substitution, parameter-expanded Cargo aliases, Cargo executable indirection through shell-variable command words, POSIX special-builtin persistence for `export`/`readonly`, and Bash declaration-builtin persistence for `declare`/`typeset`.

Earlier source RED `e77955d6f8546c96dc66fbf1865e2b17a696c077` proved static variable Cargo aliases could bypass direct Cargo discovery. Subsequent focused REDs covered quoted persistent command substitution (`f5d17516...`), valid unquoted `CARGO=$(...)` (`1f20ea65...`), Bash `CARGO+=$(...)` (`705ae0d2...`), and POSIX `readonly` persistence (`04576792165985373052c911cece4b8105ee8c6b`). Those repairs preserve ordinary command-local assignment prefixes as non-persistent controls.

Newest source RED `18e03ca4ab9841f33caf40a33101436999c43265` adds the Bash declaration commands used by GitHub-hosted Linux `run` steps: `declare CARGO=/opt/rust-1.98.0/bin/cargo; "$CARGO" build ...` and the `typeset` synonym persisted alternate executable authority while the bounded parser treated them as ordinary commands. CI `33981111056` and Supply Chain `33981111042` materialized for that RED but were superseded before terminal evidence, so this remains source-level RED rather than hosted RED. Current repair `8e64640d...` records assignment prefixes/operands for `export`, `readonly`, `declare`, and `typeset`, while ordinary command-local prefixes remain non-persistent. The GNU Bash Reference Manual classifies `declare`, `typeset`, `export`, `readonly`, and `local` as declaration commands whose assignment arguments have assignment-statement properties; the current repository has no `local CARGO` release-path use, and function-local shell parsing remains outside the claim unless a real release path introduces it.

Current exact #56 CI is `33981153739` and Supply Chain is `33981153766`. At the latest verified read both are nonterminal. Predecessor execution/review results do not transfer; current `8e64640d...` is not yet hosted GREEN. A fresh exact-range technical review is required.

Draft #54 was non-force restacked on current #56 by ordinary two-parent ancestry. Current exact head is `9339b6e8dd69d6a1393c38c2a49e78db15693d1d`; current #56 is the exact merge base, ahead 65 / behind 0, and effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. Compiler-oracle repairs are inherited, not copied into the child range.

PR `#54` intentionally requires committed `Cargo.lock` to contain no `derivative`. That assertion becomes semantic RED only after #56 independently proves compiler/bootstrap GREEN. Do not add an audit ignore, suppress OSV/RustSec, delete lock evidence, or pin mutable supplier source to manufacture GREEN. `RUSTSEC-2024-0388` remains an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

## Protocol / supplier path

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a` with five changed files at the last verified read. Its H2→H1 Cookie real-wire fixture remains one file, but four #57 workflow deltas still contaminate the effective range. Because workflow policy is now canonical in foundation, #52/#53 must be non-force ancestry-repaired so the protocol RED becomes fixture-only before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c` at the latest verified sweep. Supplier issue #889 remains open and continues to classify `derivative 2.2.0` as unmaintained. Mutable supplier work is not consumed as dependency authority.

Required order is `foundation workflow repair (done) → compiler #56 exact GREEN → derivative #54 semantic RED → immutable supplier repair/release → gateway dependency bump → unchanged supply-chain GREEN → #52/#53 ancestry repair → protocol-only H2→H1 Cookie RED/GREEN → immutable gateway release → shadow/canary/rollback/cutover`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` advanced independently during this run to `f2f91b806122ed233e3a0e2a325246077c2e15e4` through merged #1939; that owner-side change concerns review-model route catalog account diversity and is not copied into this repository. Canonical queue-health remains a dedicated owner lane. Pingora hands off exact evidence but does not mutate `.github` source, refs, or PR state while its dedicated writer is active.

Runner delay remains distinct from semantic failure. Evidence classification distinguishes startup failure with zero jobs, materialized pre-checkout jobs with no runner/steps, Draft-policy skips, source-level RED, hosted semantic RED, and terminal hosted GREEN.

## Legacy migration / release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current protected-main merge, immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

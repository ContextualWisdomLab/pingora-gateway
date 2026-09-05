# Product / Technical Gap Baseline

This is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-05 KST. Mutable PR heads are evidence candidates, not release authority; later live evidence supersedes exact identities below.

## Ownership boundary

`pingora-gateway` owns Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may own forwarding sanitation, connection/request limits, timeout/retry/backpressure, health/drain, payload-free low-cardinality transport telemetry, and immutable edge packaging.

It does not own product authentication/authorization, tenancy/business routing, Keyverse identity authority, Wardnet/EgressWeave authority, certificate issuance/key custody, or application-specific FastCGI/business semantics. Cross-context behavior is consumed through released contracts or explicit ACLs; source copies, cross-service application SQL, mutable sibling PR dependencies, and hidden Shared Kernels are rejected.

## Buyer-visible release gaps

The Rust/Pingora gateway is an implemented candidate, not a released edge product. Production host/SNI/routing parity, H2/H3/TLS/WebSocket/streaming failure traffic, immutable packaging, SBOM/provenance/reproducibility, observed rollback, and realistic buyer-path p95 ≤20 ms evidence remain release gates. Protected `main` is exact `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases remains empty at the latest verified sweep.

## Workflow-admission dependency root — #58 / #59 / #60

Merged #57 placed valid workflow-coalescing policy inside #53's H2→H1 Cookie lineage. Both deltas must survive. Workflow policy is promoted at the earliest workflow-owning foundation ancestor, then descendants are repaired by forward adoption and ordinary multi-parent ancestry. Force-push, destructive rebase, simple closure, test deletion, self-approval, and gate weakening are not repair mechanisms.

Draft #59 remains exact `cb458621b5cfdcec35820083bb82e19e9dc627cf` on foundation #1, with exactly three Rust workflow contracts in its effective range. Current Draft runs materialized runner-facing jobs rather than skipping them, so the Draft-admission RED remains current before checkout. Fresh exact-range CodeRabbit review reports no actionable semantic finding; technical review is not an approving human review.

Ready #60 remains exact `e2309106eba4f41f45c770676885b27bf73aa69b` based on #59, with exactly `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` in its effective child range. Predecessor `f1b09eb0...` acquired hosted runners: load-contract, OCI and Supply Chain passed; formatting and all workflow contract tests passed; CI then failed only at inherited Clippy `question_mark`. The current head contains the minimal lint repair without changing policy, but current CI `33966008873` and Supply Chain `33966008876` remain queued/pending and do not inherit predecessor terminal credit.

PR #61 is the documentation-only child of #60. Its effective range remains `CHANGELOG.md`, `SECURITY.md`, `THREAT_MODEL.md`, `TRD.md`, `docs/doctoring/TRACEABILITY.md`, and this baseline. Draft skips prove descendant admission parity only; they do not replace #60 Ready-state all-gates GREEN.

Promotion order remains `#59 current RED authority + review → #60 current all-gates GREEN + governance → ordinary foundation adoption → #52/#53 non-force ancestry repair`.

## Compiler and supply-chain roots — #56 / #54

Draft #56 is current exact `a32b706122076838f75c92ff6f427b81430a1b0e` on foundation #1. Rust 1.98.1 remains the required release compiler because the Rust Release Team published it on 2026-09-03 to repair the vtable-generation miscompilation introduced in 1.98.0.

The compiler-authority review chain covers job-scoped YAML environment authority, Bash compound assignments, Docker executable-build detection, explicit Cargo toolchain selectors across whitespace/continuations/quoting, shell control operators, the `command` builtin, GNU `env -S` and bundled `-iS`, child-command operand boundaries, GNU `env --`, legacy backticks, and POSIX-style `$()` command substitution. POSIX.1-2024 Shell Command Language §2.6.3 is the primary shell authority; it defines both command-substitution forms as executable in a subshell environment.

The first `$()` RED `c6eb125d2929a78af5308e16d53bc56fee44f768` proved `echo $(RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked)` could bypass the earlier word scanner. An initial matcher then exposed a deeper design flaw: matching the closing `)` with partial quote/parenthesis logic is not shell grammar. Exact RED `9b5cc1db7755b5e1c4444860c03fcd691a9d4331` uses valid `case x in x) ... esac` syntax to prove the matcher can terminate early and miss compiler authority later in the same substitution.

Current source repair `a32b7061...` removes that partial matcher. The contract parses GitHub Actions YAML into direct step `run` scripts, joins Docker shell-form `RUN` continuations, and applies the command-substitution guard only to those already-bounded executable units. The guard detects an active `$(` outside single quotes/escapes and rejects the bounded unit when compiler environment authority feeds Cargo, explicit `cargo +<toolchain>` appears, or rustup changes toolchain authority. Benign repository uses including checkout identity, `$(seq ...)`, Docker inspection, `cargo metadata`, quoted parentheses, nested ordinary substitutions, single-quoted `$()` text, and escaped `$()` text remain accepted. The guard deliberately does not claim to parse POSIX shell grammar.

Current #56 CI `33970882226` and Supply Chain `33970882197` are exact-head gates. CI jobs `101319057268`, `101319057384`, and `101319057390` are materialized but pre-checkout without runner identity at the latest read; source RED→repair evidence is not hosted GREEN. Fresh exact-range review is required after this current movement.

Draft #54 has been non-force restacked to current #56. Its current head is `f96cf3da5c3f44f23091708c0f90ad26cfd4746f`; effective child delta remains exactly four files: `CHANGELOG.md`, `TEST_STRATEGY.md`, `docs/doctoring/TRACEABILITY.md`, and `tests/supply_chain_policy.rs`. No compiler-contract/support delta belongs in the child range.

#54 intentionally requires committed `Cargo.lock` to contain no `derivative`. That assertion is semantic RED only after #56 independently proves compiler/bootstrap GREEN. Do not add an audit ignore, suppress OSV/RustSec, delete lock evidence, or pin mutable supplier source to manufacture GREEN. `RUSTSEC-2024-0388` remains an unmaintained advisory with no patched version; the release block is CWL supply-chain policy rather than a memory-safety-CVE claim.

## Protocol / supplier path

Draft #53 remains exact `bf5436a7b482fffd2c22fb847672076d1063a26a`, open/Draft/mergeable, with five changed files. Its H2→H1 Cookie real-wire fixture remains one file, but four #57 workflow deltas still contaminate the effective range. Foundation workflow repair must reach #52/#53 and restore protocol-test-only scope before supplier RED/GREEN, merge, or release credit.

Protected public `cloudflare/pingora/main` remains `09696b51bc59315353d96686355861604d0bb48c`. Cookie #901 remains open/unmerged/non-mergeable at `b856ddfc6be15f1727601d2d76cb10d2d72f95f0`; body-framing #936 remains open/unmerged/mergeable at `e40ed4cceb0c0ed8c05cc39eb01a8c73dea5497a`; supplier issue #889 remains open. These are mutable supplier evidence, not immutable dependency authority.

Required order is `foundation workflow repair → protocol-only H2→H1 Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity/release → gateway dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions authority remains in `ContextualWisdomLab/.github`. Protected `.github/main` is exact `3f88e13af9dcde4b9da6958c02a78ce3b5c85800`; merged #1926 serializes the dispatched CodeQL scan matrix with `toJSON` because an array cannot be assigned directly to an Actions `env:` scalar. This owner-side repair is relevant evidence for distinguishing YAML validity from GitHub Actions template/runtime validation, but it does not constitute Pingora leaf GREEN.

Dedicated queue-health #1150 remains open at `e6622a428060194b558929ad651d5b4ae3a9840f` and is 69 commits ahead / 17 behind protected `.github/main`, with merge base `6d7fbebec8aec31d88a30a36e71ca5b3925d241d`; its body still describes older protected/main and head identities. Pingora records and hands off this drift but does not mutate `.github` source, refs, or PR state while its dedicated writer is active.

Runner delay remains distinct from semantic failure. Evidence classification must distinguish startup failure with zero jobs, materialized pre-checkout jobs with no runner/steps, Draft-policy skips, source-level RED, hosted semantic RED, and terminal hosted GREEN.

## Legacy migration / release gate

`linux-cluster-ops` remains dedicated-writer territory. Migration stays release-first: safe extraction and structural inventory in the owner repository → explicit certificate/edge/application responsibility split → immutable `pingora-gateway` artifact → parity/shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. The gateway must not absorb certificate issuance/key custody or unbounded product-specific FastCGI semantics merely because the legacy proxy co-located them.

Commercial release credit requires exact protected candidate version/CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility, rollback artifact/runbook, and all live governance checks. Current merge, immutable release, canary, cutover, and legacy-removal credit remains zero.

Primary standards and research citations belong in `docs/doctoring/TRACEABILITY.md`; this baseline keeps code-current decisions, exact execution dependencies, buyer-visible gaps, and next actions.

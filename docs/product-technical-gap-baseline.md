# Product / Technical Gap Baseline

This file is the code-current commercial-development baseline for `ContextualWisdomLab/pingora-gateway`. It records current ownership, executable evidence, dependency roots, and buyer-visible gaps. Mutable PR heads are evidence candidates, not release authority. Before any promotion, re-read the exact head/base, reviews, checks, security and supply-chain runs, protected branch, supplier identity, release artifacts, and consumer deployment state.

Snapshot: 2026-09-05 KST. Later live evidence supersedes every exact identity below.

## Product and DDD boundary

`pingora-gateway` is a Generic/Supporting edge runtime. Its bounded contexts are Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation. It may implement transport-level admission, forwarding sanitation, connection limits, timeout/retry/backpressure, health/drain, low-cardinality transport telemetry, and immutable runtime packaging.

It must not become a second authority for product authentication or authorization, tenancy, business routing, identity, egress governance, malware/policy decisions, or certificate issuance simply because a legacy Nginx/OpenResty deployment co-located those responsibilities. Keyverse remains the identity backend. Wardnet/EgressWeave remain their security-policy authorities. Certificate issuance/ACME/key custody stays with the actual certificate/secret owner. Product-specific semantics cross this boundary only through released contracts or an explicit Anti-Corruption Layer; source copies, cross-service application SQL, mutable dependencies, and hidden Shared Kernels are not accepted.

The code/API/test vocabulary must continue to match these bounded contexts. Route/config identities and transport limits are Value Objects; admitted configuration must be validated before listener activation; runtime policy belongs behind narrow domain services/adapters. There is no material admin UI in the current migration lane, so the Figma/Storybook/i18n UI gate is not applicable to the dependency root below.

## Current reusable runtime state

| Area | Current state | Buyer-visible gap before release/cutover |
| --- | --- | --- |
| Executable Pingora path | Implemented candidate on the foundation stack | Every moved exact head must reacquire fmt/compile/test/Clippy/rustdoc/coverage/load/OCI/security/supply-chain evidence. |
| Ingress / routing | Implemented candidate | Representative production host/SNI/routing parity and consumer shadow traffic remain required. |
| TLS | Partial | Upstream trust/SNI behavior is characterized. Certificate lifecycle stays outside this owner. Downstream TLS/H2 and later HTTP/3/QUIC need dependency-ordered traffic evidence. |
| HTTP policy | Partial | Hop-by-hop sanitation and forwarding-identity distrust/reconstruction exist. H2→H1 Cookie normalization and zero-length chunk framing remain supplier-gated. |
| WebSocket / streaming | Partial | Upgrade/streaming behavior exists in the supplier/runtime path, but long-lived-stream shutdown, backpressure and failure traffic still need release-candidate E2E evidence. |
| Runtime isolation | Partial | Body/in-flight/connection/timeout/drain controls exist. Broader header budgets, parked-read shutdown and representative failure traffic remain open. |
| Observability | Partial | Payload-free, low-cardinality edge telemetry exists. Product/identity/business telemetry remains outside this owner; production trace and incident evidence remains open. |
| OCI | Implemented candidate | Rootless/read-only-root/capability-free/no-new-privileges checks exist. Immutable release, SBOM, provenance, reproducibility and rollback still gate commercial readiness. |
| Performance | Local executable contract only | Applicable gateway paths use realistic async/load contracts with p95 `<20 ms`; this is not a production SLO until TLS/network/origin/deployment traffic is measured without sample shrinking or artificial warm-up. |
| Release / rollback | Not released | Protected `main` is still `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; GitHub Releases are empty. No immutable gateway artifact exists for consumer canary/cutover. |

## Dependency root: workflow admission repair (#58 / #59 / #60)

Issue #58 owns a wrong-base repair. Merged #57 placed a valid repository workflow-coalescing delta inside #53's H2→H1 Cookie protocol lineage. Both deltas must survive: workflow policy moves to the earliest workflow-owning foundation ancestor, while the Cookie wire fixture remains in its protocol lane. Repair is by forward adoption and ordinary multi-parent ancestry, not force-push, destructive rebase, closure, or test deletion.

### RED oracle — #59

Draft #59 is based directly on foundation #1 `5a62e2fa56fdaa6f97c0518932711739e347c04a`. Current exact head is `e7778c865e4b3b3a0198ade4a8d8a030b960926d`; fresh compare is ahead 18 / behind 0 with the foundation as exact merge base. Effective range is exactly two Rust test contracts: `tests/workflow_concurrency_contract.rs` and `tests/workflow_tag_filter_contract.rs`.

Predecessor `8eaccce7251b4eb8666212b57207da31c0146b9d` acquired hosted runners and reached `cargo test --all-targets --locked`. At that point 10/12 workflow-concurrency tests passed and the two then-defined foundation invariants failed: missing workflow/repository/PR cancellation identity and missing protected-main-only duplicate-push scope. OCI, load-contract and supply-chain evidence on that predecessor were GREEN. That is causal hosted semantic RED evidence, but it does not transfer to a moved head.

A subsequent valid GREEN-lane delta added two necessary admission invariants: reruns must use run-id isolation instead of displacing current first-attempt PR evidence, and every direct PR job must reject draft admission while PR events are limited to `opened`, `synchronize`, `reopened`, and `ready_for_review`. Those assertions are oracle authority, so `e7778c...` moved the strengthened contract into #59 without copying workflow source. Current exact runs are CI `33941789141` (pending at the latest sweep) and Supply Chain `33941789144` (queued). Because the current contract is stronger than the executed predecessor, terminal hosted RED must be reacquired on this exact head.

### GREEN implementation — #60

#60 is Ready on exact #59 `e7778c...`. Current exact head is `318c9d3adbfb9aea7eccbd24987d64890fe27f42`; fresh compare is ahead 26 / behind 0 with exact merge base and exactly two effective files: `.github/workflows/ci.yml` and `.github/workflows/supply-chain.yml` (`+17/-9`). The Rust oracle is no longer part of the GREEN range.

The implementation restricts duplicate push evidence to protected `main`; coalesces first-attempt PR work by workflow/repository/PR; sends reruns to `github.run_id`; cancels only PR runs; admits only useful PR lifecycle events; and rejects draft admission on every direct job. Existing exact-SHA checkout, test/load/OCI and supply-chain gates remain. The branch also retains narrow actionlint repairs for EXIT-trap cleanup without changing runtime acceptance thresholds.

Predecessor execution already proved the former contract passed the complete Cargo test suite and failed only later on an inherited Clippy lifetime warning; that lint was repaired in the RED parent. Current exact runs are CI `33941798179` and Supply Chain `33941798193`, both queued at the latest sweep. Current all-gates hosted GREEN and a fresh exact-range independent review remain mandatory before foundation adoption.

### Documentation projection — #61

#61 is a documentation-only child of current #60 and must keep exactly six documentation files in its effective range. Its own exact head is intentionally not frozen into this baseline because changing this file creates a new documentation head; the live PR is the authority for its current head, compare, checks, and review state.

The admission policy nevertheless has stable predecessor evidence. While predecessor `14ce12b95005f464c2b883ae9289569f7e127b77` was Draft, CI `33941828678` and Supply Chain `33941828695` both completed `skipped`; this is expected under the direct-job draft guard, not a missing-run condition. Marking the same head Ready materialized `ready_for_review` runs, proving re-admission without changing source identity. Subsequent documentation-only commits make those run IDs historical execution evidence rather than current-head gate credit. Re-read #61 directly for its current exact runs and review before any merge.

Promotion order is fixed: `#59 exact terminal RED + current review → #60 exact all-gates GREEN + current review → ordinary foundation adoption → descendant ancestry repair`. Scheduler admission, skipped Draft evidence, bot/static review, Cargo semantic execution and final all-gates execution are distinct evidence classes.

## Compiler and supply-chain prerequisites (#56 / #54)

Draft #56 current exact head is `3b70ba734aae5f43a620b32bda5f0b59fe2b602b`, based on foundation #1 with 11 effective files. It moves release-producing paths to Rust 1.98.1 and contains the canonical compiler-authority oracle, including whitespace/continuation/quoting selector regressions plus fail-closed checks for `RUSTC` and `CARGO_BUILD_RUSTC` supplied through shell assignments or GitHub Actions workflow/job/step `env` mappings. Current runs remain CI `33939316667` queued and Supply Chain `33939316673` pending. Exact-head GREEN and current-range review are still required.

Draft #54 current exact head is `02a739c0a3fa2772a649ef54bbd3563ac533ec5b`, a non-force child of #56 with exactly four derivative-advisory evidence files. It intentionally requires committed `Cargo.lock` to contain no `derivative`. Current runs are CI `33939346427` pending and Supply Chain `33939346408` pending. The intended RUSTSEC-2024-0388 RED is valid only after #56 proves its compiler/bootstrap path GREEN; an audit ignore or mutable supplier pin is not closure.

## Protocol and public supplier prerequisites

Draft #53 is still exact `bf5436a7b482fffd2c22fb847672076d1063a26a`, base `996ee1f3cd59d8843c66fd2e39cab9bd0a76255c`, with five effective files. Its real-wire H2→H1 Cookie fixture is one 583-line test file, but four wrong-base workflow files from #57 remain in the lineage. It cannot receive protocol RED/GREEN, review-complete, merge or release credit until #58's foundation repair is adopted down #52/#53 and restores protocol-test-only scope.

Protected public `cloudflare/pingora/main` is `09696b51bc59315353d96686355861604d0bb48c` at this snapshot. Latest published release is Pingora `0.8.1` (2026-06-04); the GitHub release object is not marked immutable. Mutable supplier work therefore remains evidence only:

- #901 is open at `b856ddfc6be15f1727601d2d76cb10d2d72f95f0`. It implements RFC 9113 §8.2.3 H2 Cookie concatenation against an older `PeerOptions.h2_to_h1_concat_cookies` shape and still needs current-main adaptation to `HttpUpstreamRequestPolicy`, with default-on normalization and a narrow opt-out that does not disable unrelated hop-by-hop protections.
- #936 is open at `e40ed4cceb0c0ed8c05cc39eb01a8c73dea5497a`, based on current protected main. It keeps zero-length chunked writes from emitting a premature H1 terminator in both async and cancel-safe paths. It is a separate body-framing prerequisite.
- #889 remains open for RUSTSEC-2024-0388 (`derivative 2.2.0` unmaintained). No downstream exception or mutable fork is accepted as supplier closure.

The Cookie path remains `#58 foundation repair → #53 protocol-only exact range → real TLS/H2 two-Cookie client evidence → raw H1 single-Cookie RED → current-main supplier adaptation/review/integration → immutable supplier identity → downstream dependency bump → unchanged wire GREEN`.

## Organization Actions owner-plane

Organization-wide Actions queue/admission authority remains in `ContextualWisdomLab/.github`, not in Pingora leaf source. Protected `.github/main` is `8272e4f95c253ab067592460cc9288581bf3a422` at this snapshot, with #1880's repository-side Strix retry removal included. The Pingora writer only reads this authority and reports reproducible specimens to its owner; it does not mutate `.github` source, refs or PR state. Delayed runner assignment is not treated as a source race and does not justify no-op commits or runner-selector churn.

## Legacy Nginx/OpenResty operational migration

`linux-cluster-ops` has a dedicated writer, so this lane is read-only there. The live owner path remains blocked. PR #251 is open at `3e5598c4d89f233dd2862d6d6bc20c797037232b`; its body still has `관련 이슈: #N/A` and describes `tar -U` as restore hardening. The previously identified safe-extraction requirement therefore remains: extract into an empty root-owned staging directory; validate archive member paths/types/links/ownership; promote only after validation; preserve its valid ReDoS/path-security delta.

Issue #267 remains open with `status: blocked`. It correctly separates certificate lifecycle, edge runtime, and OJS/PHP/FastCGI application ownership. The shared gateway must not absorb ACME/key custody or grow an unbounded product-specific FastCGI subsystem. A released gateway artifact is a prerequisite for the owner repository's structural inventory, shadow/canary and Nginx removal.

## Decision / evidence ledger

| Problem | Constraint and rejected shortcut | Current decision | Exact evidence / next action |
| --- | --- | --- | --- |
| Workflow policy landed in Cookie lineage | Preserve both valid deltas; no close/rebase/force-push | Promote policy to foundation and restack descendants | #59 `e7778c...` RED oracle; #60 `318c9d3...` workflow-only GREEN; await terminal runs and fresh reviews. |
| GREEN lane had test-oracle delta | RED must own the acceptance contract | Moved strengthened test blob to #59, ordinary two-parent restack | Fresh #59→#60 compare is exactly two workflow files. |
| Draft PRs consumed runner admission | Do not weaken required checks for Ready PRs | Skip every direct job while draft; reacquire on `ready_for_review` | #61 predecessor `14ce12b...` Draft runs skipped; Ready event materialized new runs on the same source head. |
| Rust 1.98.0 compiler risk | Release-producing path must use repaired compiler authority | #56 moves release path to 1.98.1 and forbids alternate compiler authority | #56 `3b70ba...`; current exact runs still outstanding. |
| `derivative` unmaintained | No audit-ignore manufactured GREEN | Keep intentional lock RED after compiler parent GREEN | #54 `02a739c...`; Cloudflare #889 remains open. |
| H2 Cookie downgrade | No CWL shim or mutable supplier pin | Real-wire oracle, then current-main supplier repair | #53 blocked by ancestry; Cloudflare #901 remains mutable. |
| Zero-length chunk framing | Separate from Cookie behavior | Maintain independent supplier prerequisite | Cloudflare #936 remains open on current main. |
| Nginx operational removal | Dedicated writer; certificate/app boundaries must survive | Release gateway first, then owner-side structural cutover | `linux-cluster-ops` #251/#267 remain open/blocked. |

## Release and cutover gate

Commercial release credit is forbidden until the exact protected release candidate has version and CHANGELOG alignment, immutable tag/package/image, SBOM, provenance, reproducibility evidence, rollback artifact/runbook, and all then-live governance checks. The active organization ruleset on the default branch additionally requires an approving review, resolution of review threads, organization-required workflows, and blocks deletion/non-fast-forward updates; administrative bypass is not an acceptance path. Consumer migration then requires parity → shadow/canary → observed rollback path → cutover → verified legacy Nginx/OpenResty removal. A source merge, queued check, mutable supplier PR, local p95 result, or documentation-only PR is not equivalent to a shipped edge.

At this snapshot protected `pingora-gateway/main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991` and GitHub Releases are empty. Therefore immutable release, canary, cutover and legacy-removal counts remain zero.

Historical source/standards reasoning and primary references belong in `docs/doctoring/TRACEABILITY.md`; this baseline intentionally keeps only the current decision state and exact execution dependencies so it can remain code-current as heads move.

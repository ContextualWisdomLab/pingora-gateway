# Product / Technical Gap Baseline

This document is the code-current commercial baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-18 KST. It records durable product/technical gaps, responsibility boundaries, acceptance contracts, and dependency order. Mutable PR heads, workflow run IDs, artifact digests, and branch tips are operational evidence snapshots; issue #58 and the owning PR/Issue remain the live exact-evidence ledger and supersede stale identities here.

## Product and DDD responsibility boundary

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It owns reusable Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation behavior only where those concerns are genuinely shared edge responsibility.

The Edge Contract bounded context owns admission of network authority. `GatewayConfig` is the aggregate root; listener authority, upstream authority, TLS identity, request-body limits, and I/O budgets are invariants admitted before network activation. Pingora-specific types remain delivery details and must not cross into the Edge Contract. `GatewayCommand` is the startup application service and Pingora Delivery is the anti-corruption adapter.

The gateway does **not** own product authentication/authorization, tenant or business routing, Keyverse identity authority, Wardnet/EgressWeave policy authority, certificate issuance/private-key custody, application workflow state, or product-specific semantics. Those authorities remain with their canonical owners and are consumed only through released/versioned contracts or explicit ACLs. Source copies, cross-service application SQL, mutable sibling-PR dependencies, and hidden Shared Kernels are rejected.

Legacy Nginx/OpenResty presence is not itself a migration trigger. Static-file serving, FastCGI, certificate issuance/key custody, product authentication, and domain-specific routing stay with the correct owner unless a separately evidenced shared-edge responsibility exists.

## Evidence and promotion invariants

A change is promoted by exact behavior, not by a release name, mutable contributor branch, historical GREEN, or documentation claim. The normal sequence is `realistic RED → minimal causal repair at the correct owner → exact-head GREEN → maintainer/protected integration → release-qualified dependency identity → consumer pin/lock transition → unchanged consumer GREEN → protected integration → release/cutover evidence`.

Predecessor execution and review do not transfer across a new exact head. Draft/Proposed work is not accepted merely because a bot or candidate branch is GREEN. Do not force-push, destructively rebase, self-approve, weaken required checks, suppress an advisory, reduce samples/concurrency, artificially warm measured paths, hand-edit generated lockfiles, or pin mutable supplier PRs to manufacture promotion evidence.

Owned production rustdoc, test, and edge-case coverage remain fail-closed at the repository contract. Applicable buyer-path performance uses the compiled Rust path and realistic traffic, with p95 `<= 20 ms` without sample reduction or measurement exclusion. Controlled loopback evidence is a regression bound, not an Internet/TLS/WAN or representative NUMA production SLO.

## Current authority snapshot

Protected CWL `main` remains outside the dependency-ordered candidate stack and has no GitHub Release object. The current product therefore remains an implemented candidate, not a released edge product.

Public supplier authority is split deliberately. Protected `cloudflare/pingora/main` is prospective source authority; the latest published dependency release remains Pingora `0.9.0`, published on 2026-09-09. Its GitHub Release metadata is mutable (`immutable=false`) and has no attached assets, so downstream package authority comes from exact registry source/checksum plus Cargo-generated lock evidence rather than the GitHub Release object alone.

Released 0.9.0 closes the previously characterized graceful-shutdown lost-wakeup supplier correctness gap, but does **not** close the current `derivative 2.2.0 / RUSTSEC-2024-0388` intake root, configurable monotonic whole-H1-header lifetime, configurable H1 parser admission, H2→H1 Cookie normalization, or zero-length H1 chunk-terminator defect. Those remain separate supplier-owner paths.

## Foundation and supply-chain security root — #1 / #54 / #62 / upstream #889

Foundation #1 has terminal GREEN ordinary CI, Supply Chain, SAST, and required CodeQL on its current exact. Its Security Scan remains intentionally RED because the committed resolved graph introduces exactly `derivative 2.2.0 / RUSTSEC-2024-0388`; the protected base scan is clean. This is a real supplier-admission failure, not a scanner/runtime failure. StartupLock issue #108 is closed on the unchanged foundation exact.

Upstream #889 remains open and protected Pingora `main` still has no release-qualified derivative-removal identity. The accepted repair is maintainer-integrated removal from the relevant production graph with equivalent `PeerOptions` and `Backend` semantics, supplier fmt/tests/Clippy/rustdoc/audit, and a later release-qualified identity containing that repair. Advisory ignores, SARIF/scanner suppression, deleted lock evidence, downstream forks, or mutable contributor pins are not accepted.

#54 remains the deliberate package-absence RED owner and #62 remains the supplier-semantics/load/runtime control. Both are currently Draft on historical compiler ancestry. Their valid deltas remain preserved, but they intentionally wait for the release-qualified supplier repair so they can reconcile once against current compiler/foundation ancestry and the exact repaired supplier identity rather than churn through two immediately stale restacks.

## Release compiler root — #56

#56 current exact has independently reacquired terminal GREEN CI and Supply Chain under Rust `1.98.1`, including formatting, all-target compile/test, strict Clippy, warning-denied rustdoc, complete owned-production coverage, load, least-privilege OCI runtime, resolved-lock verification, SBOM/image-scan/source-binding evidence, and current-range technical review.

The compiler implementation/execution gate is therefore closed. Independent governance/approval is still required before normal integration; technical COMMENT evidence is not substituted for an approving review, and administrator/self-approval bypass is not used. The supplier Security RED above remains a separate foundation-promotion blocker rather than a compiler failure.

## Shared HTTP intermediary / forwarding root — #15

#15 owns forwarding-header trust reconstruction, `Via`, TRACE/OPTIONS `Max-Forwards`, bounded intermediary-local outcomes, and pg-erd forwarding compatibility without taking product auth/business authority.

Its predecessor exact proved that the prior public-boundary coverage repairs were valid but still left one owned-production line/region uncovered in `src/forwarding_policy.rs`. Current exact adds a test-only public-boundary authority-parser coverage repair for bracketed/default-port, percent-encoded reg-name, and fail-closed malformed authority shapes. No production forwarding semantics, coverage threshold, sample count, or exclusion changed.

Current-exact CI and Supply Chain remain incomplete evidence until terminal. #15 must independently reacquire the complete 100% owned-production gate before normal integration. Descendant #16 and parallel #102/#104/#106 remain Draft and may reconcile only after normal #15 integration; they must inherit, not copy, the parent tests/fixtures/contracts.

## Pg-erd response lifetime — #39

#37 has normally integrated its upstream-TLS parent work. #39 is the current versioned response-body lifetime child: pg-erd v2 admits positive `max_upstream_response_body_ms`, starts one lifetime budget at the first non-informational upstream response header, checks non-empty body progress, and preserves existing per-read timeout semantics.

The reconciled branch repaired two important causal interactions: current-parent forwarding/admission/failure/observability behavior survives the historical overlay, and a lifetime breach after a final response is already committed does not fabricate a second downstream status (`Session::response_written()` is the final-response oracle). The realistic slow-drip fixture uses application-level `/readyz`/`/metrics` readiness and absolute operation deadlines so partial progress cannot renew the evidence window.

The semantic predecessor failed hosted CI only at Rust formatting; current exact applies the emitted formatter-only rewrite. Current CI/Supply are still incomplete evidence, so #39 is not integrated and #42 remains Draft behind it. No predecessor success is transferred to the formatter head.

## Released shutdown consumer — #70

#70 is no longer waiting for a Pingora 0.9.0 consumer bump: its current candidate already uses exact crates.io `pingora = "=0.9.0"` and `pingora-prometheus = "=0.9.0"` with Cargo-generated registry lock authority. Historical consumer evidence established that the released supplier contains the shutdown correctness repair.

The current exact nevertheless has a hosted CI RED. Supply Chain, bounded-origin capacity, load, OCI, ordinary tests, Clippy, and rustdoc reached GREEN, but the coverage-instrumented workload hit `ConnectionReset` in the upstream-failure fixture immediately after the old release-line test helper accepted bare TCP connect as readiness. Current foundation #1 already owns the causal fix: real bounded `/readyz` HTTP 200 readiness plus listener-reservation handoff.

This is an ancestry defect, not evidence that Pingora 0.9.0 regressed graceful shutdown. Do **not** copy the foundation readiness helper into #70. The release lineage must ordinary/non-force inherit current foundation through #44/#47, then #70 must preserve only its released-supplier consumer delta and reacquire formatting/test/Clippy/rustdoc/100% coverage/load/OCI/Supply/capacity/review on the reconciled exact. #73 and later release descendants remain Draft until that parent path is current.

## Downstream H1 whole-request-header lifetime — #71 / upstream #447

#71 is the real-socket RED for a monotonic whole-request-header deadline, distinct from parser byte/count admission and relative per-read inactivity. It exercises continuous slow progress on fresh and reused H1 connections and requires one absolute acquisition bound.

Released 0.9.0 does not expose the needed supported monotonic whole-header lifetime. Upstream #447 remains open. Closure requires a maintainer-integrated capability in a later release-qualified supplier identity and, only if CWL exposes an operator control, an explicit positive versioned Admin Config transition. The unchanged #71 traffic then has to turn GREEN without origin-admission, privacy, or recovery regressions.

## Downstream H1 parser admission — #72 / upstream #993 and #1000

#72 remains the executable parser-admission RED: oversized single-field and excessive-field-count traffic must fail before application/origin admission rather than after large untrusted buffering.

Contributor #1000 remains open and unmerged. Its current candidate materially improves parser-level byte/count admission and has candidate-scope GREEN evidence, but a contributor head is not maintainer integration or release authority. CWL must not pin that mutable branch. Closure remains `maintainer integration → release-qualified supplier identity → optional explicit positive CWL Admin Config → unchanged #72 parser/application/origin acceptance GREEN`.

## Mixed protocol: H2 downstream to H1 upstream — #53 / upstream #901 and #936/#976

#53 owns the real-wire H2→H1 Cookie normalization RED. Upstream #901 remains open and unmerged; accepted behavior must coalesce only the H2→H1 Cookie case using exact downstream protocol knowledge while preserving ordinary hop-by-hop protections and existing policy authority.

The zero-length application-write/chunked-terminator defect is separate. Upstream #936 remains open and unmerged, with #976 a separate alternative lineage. Accepted behavior keeps zero-length application writes as no-ops and `finish()` as the sole chunk terminator owner, including async and cancel-safe task regressions. Full mixed-protocol release credit requires release-qualified supplier authority for each relevant root or a deployment contract that makes the affected downgrade path unreachable, followed by unchanged gateway traffic GREEN.

## Pg-erd dynamic configuration / Traefik succession — #109 / #110

Protected `pg-erd-cloud` evidence uses Traefik's file provider with `filename`, `watch=true`, and a single read-only bind-mounted `dynamic.yaml`. Repository history and documented production operation do not currently prove that operators depend on live in-place/rename replacement semantics; a configured watcher alone is not a sufficient product requirement.

#110 therefore remains a Draft writer-safe characterization lane behind the #5→#6→#7→#11→#12 migration ancestry. Its harness has been hardened against outgoing-container log loss, stale-recovery observations, fixed-delay false GREEN, host-port TOCTOU, missing/partial artifact uploads, detached source identity, and receipt/source-marker mix-and-match. Draft-skipped hosted runs are lifecycle evidence only, not GREEN.

Do not implement a generic shared Pingora hot-reload subsystem merely because Traefik `watch=true` exists. After the parent stack is current and #110 integrates, run the source-bound manual characterization from protected `main`. If the product owner confirms live mutation as an operational contract, design versioned Admin Config reload semantics with validated full snapshots, atomic generation publication, invalid-generation rejection, last-known-good/fail-closed behavior, bounded synchronization, recovery/drain/rollback, and realistic traffic evidence. Without positive consumer evidence, the default migration contract is versioned startup configuration plus controlled restart/redeployment with readiness, drain, and rollback.

## Buyer-visible release and cutover gap

`pingora-gateway` still has no GitHub Release inventory. A release-ready exact protected head must establish, without predecessor transfer:

- version, CHANGELOG, tag/package, and an immutable gateway release/artifact;
- SBOM, authenticated provenance, reproducibility, source/binary/image identity, and rollback evidence;
- non-root/read-only runtime plus supply-chain/security gates;
- TLS, HTTP/1.1, HTTP/2 and applicable HTTP/3 behavior;
- WebSocket/streaming where an actual migrated product requires it;
- timeout/retry/backpressure, header/cookie/client-IP/body-limit, health/drain, and failure traffic;
- exact-head owned production rustdoc, test, and edge-case coverage contracts;
- realistic concurrency/load with applicable buyer-path p95 `<= 20 ms`, no artificial warm-up or sample removal, and representative NUMA evidence where the claim depends on it;
- parity → shadow/canary → observed rollback → cutover → verified legacy Nginx/OpenResty removal for each responsibility actually migrated.

Repository Pages publication, contributor CI, loopback p95, mutable supplier metadata, or disappearance of Nginx strings is never sufficient by itself.

## Current causal order

The dependency-rooted order is:

`#56 independent governance + maintainer-integrated/release-qualified #889 repair → #54/#62 ordinary reconciliation and absence/semantics/security revalidation on current compiler/foundation ancestry → foundation-equivalent Security/Supply/CodeQL/review GREEN + remaining #31/#61/#15/#104 prerequisites → normal protected foundation integration → #5/#6/#7/#11/#12 migration ancestry reconciliation and exact evidence → #110 protected-source characterization/owner disposition → ordinary release-line foundation inheritance through #44/#47 → #70 exact revalidation → protocol supplier roots (#447, #1000 successor, #901, #936/#976 successor) consumed only when release-qualified and required by the promoted path → immutable gateway release/provenance/reproducibility/rollback → representative parity/shadow/canary → cutover → verified legacy removal`.

Independent service/traffic children such as #39 move only after their direct parents are current and must reacquire their own exact evidence after any head change. Queued/in-progress checks remain incomplete evidence. A failed check is RCA/fix/rerun work, not a reason to weaken the gate. If a supplier or concurrent writer moves, reread and adopt/adapt the intervening delta instead of treating movement as a race or force-restacking it.

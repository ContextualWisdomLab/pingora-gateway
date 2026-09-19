# Product / Technical Gap Baseline

This document is the code-current commercial baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-20 KST. It records durable product/technical gaps, responsibility boundaries, acceptance contracts, and dependency order. Mutable PR heads, workflow run IDs, artifact digests, and branch tips are operational evidence snapshots; issue #58 and the owning PR/Issue remain the live exact-evidence ledger and supersede stale identities here.

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

Protected CWL `main` remains `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`, outside the dependency-ordered candidate stack, and the repository still has no GitHub Release object. The current product is therefore an implemented candidate, not a released edge product.

Public supplier authority is split deliberately. Protected `cloudflare/pingora/main` remains prospective source authority; the latest published dependency release remains Pingora `0.9.0`, published on 2026-09-09. Its GitHub Release metadata is mutable (`immutable=false`) and has no attached assets, so downstream package authority comes from exact registry source/checksum plus Cargo-generated lock evidence rather than the GitHub Release object alone.

Released 0.9.0 closes the previously characterized graceful-shutdown lost-wakeup supplier correctness gap, but does **not** close the current `derivative 2.2.0 / RUSTSEC-2024-0388` intake root, configurable monotonic whole-H1-header lifetime, configurable H1 parser admission, H2→H1 Cookie normalization, zero-length H1 chunk-terminator defect, Brotli response finalization, or the HTTP/1 Upgrade/WebSocket fast-`101` race. Those remain separate supplier-owner paths.

## Foundation and supply-chain security root — #1 / #54 / #62 / upstream #889

Foundation #1 current exact is `38db1949354f5721dc0ecfeea395bcf958a64ace`, the normal merge identity of Rust 1.98.1 compiler child #56. The foundation remains Draft.

Current-exact SAST `35426685280` is terminal **SUCCESS**. Current-exact Security Scan `35426685265` is terminal **FAILURE** for a deterministic supplier-admission finding: protected base `main@f8b4c99...` produced zero OSV findings, while `38db194...` produced exactly one, `Cargo.lock: derivative@2.2.0 / RUSTSEC-2024-0388`. The hard gate failed at the PR-introduced OSV reporter; Scorecard, dependency-review and Trivy filesystem jobs succeeded. OSV SARIF upload succeeded and debug artifact `10586184717` was produced. This is a valid Security RED, not a scanner/bootstrap failure and not a compiler regression.

CodeQL `35426685246` remains nonterminal, but the state is classified. Language detection succeeded; the Python, JavaScript/TypeScript, and Actions compatibility shards failed only at the central pending-verdict handoff after reading the dispatch verdict. Coordinator job `105921625299` remains queued without an assigned runner or executed steps, so no current-head CodeQL SARIF verdict has executed yet. Those compatibility-shard failures are handoff receipts, not gateway vulnerability findings. Draft-triggered CI `35426685241` and Supply `35426685243` are skipped and are not GREEN.

Upstream #889 remains the supplier owner for `derivative`. The accepted transition is maintainer-integrated removal from the relevant production graph while preserving the characterized `PeerOptions` and `Backend` semantics, followed by a release-qualified supplier identity. Advisory ignores, SARIF/scanner suppression, deleted lock evidence, downstream forks, or mutable contributor pins are not accepted.

#54 remains the package-absence RED owner and #62 the supplier-semantics/load/runtime control. Their historical valid deltas stay preserved; they should reconcile ordinary/non-force against the current compiler/foundation ancestry and exact repaired supplier identity only after a release-qualified upstream transition exists.

## Release compiler root — #56

#56 is no longer an open prerequisite. Exact `4f2f12de110aa5ee066584c3550606267fdac09e` independently completed CI `35305409034` and Supply Chain `35305409030` under Rust `1.98.1`, including formatting, all-target compile/test, strict Clippy, warning-denied rustdoc, complete owned-production coverage, load, least-privilege OCI runtime, resolved-lock verification, SBOM/image-scan/source-binding evidence, and technical review. It normally merged into foundation as `38db1949354f5721dc0ecfeea395bcf958a64ace`.

The compiler implementation/execution gate is therefore inherited by the foundation. The remaining promotion failure is the supplier Security root and incomplete current-foundation evidence, not compiler governance.

## Shared intermediary / Runtime Isolation stack — #15 / #16 / #119 / #18 / #19 / #102 / #104 / #106

#15 normally integrated as `840b03873d8e25ebd960438183884ea609b173ff`, preserving generic forwarding-header sanitization, RFC 9110 `Via`, TRACE/OPTIONS `Max-Forwards`, bounded intermediary-local outcomes, and pg-erd forwarding compatibility without moving product auth/business authority into the gateway.

#16 then normally integrated its runtime-isolation acceptance as `bf38bc571dbb7c75e651bcaccdc66904c8c92dab`. Verified successor #119 repaired the refused-origin fixture itself — listener reservation through bind handoff, real HTTP `/readyz`/`/metrics` readiness, owned refusal precondition, bounded header receipt — and normally integrated as `40e10607601bf9722d642d4a4bb18f02806c20cc`. Historical #17 closed only after successor inheritance.

#18 exact `8c0d32332388fc0e398ee3f7b47c6810e72f24d8` then completed current-head CI `35448764275` and Supply Chain `35448764235` SUCCESS and normally integrated the connected upstream read-stall acceptance as `c78a296a87972845f8f9724e2cf2072d95a7df34`. Its controlled loopback load receipt recorded 400/400 requests, zero failed-request rate, and p95 `1.2343667 ms` against the `<20 ms` threshold. That integration is now ancestry, not a pending prerequisite.

The live children of integrated #18 are now:

- #19 `3194f49a4218986ff5d43d0c4c2d275958859efd`: pg-erd OCI least-privilege/runtime evidence. It is Ready/mergeable, behind 0 with merge-base exactly #18, and its effective range remains four paths. CI `35467756899` and Supply Chain `35467756872` are queued; no historical GREEN transfers.
- #102 `b9447d58ce90e58c76a2af4acd2ef99c4648b9b9`: privileged process-health bypass remains source-bound to body-free completed GET/HEAD probes using `Session::is_body_done()`. The earlier invalid private `HeaderValue` import was repaired through the public header API before ordinary/non-force adoption of #18. CI `35468136887` is queued and Supply Chain `35468136918` is pending. The requested real HTTP/2 `HEADERS(no END_STREAM) → DATA` listener acceptance remains #75 authority and is not duplicated here.
- #104 `9e5d897e0b7b643dc9e93986c2349edc6106f255`: private-production rustdoc enforcement only; executable Rust semantics remain unchanged after ordinary/non-force adoption of #18. CI `35468276852` and Supply Chain `35468276846` are queued.
- #106 `5f87459ea78c6e9f5dfe98ec38ffa70550f14626`: shared Max-Forwards authority and pg-erd traffic acceptance, ahead 27 / behind 0 on integrated #18. Fresh reconciliation review found the first seven-path tree had dropped three valid owner-contract deltas; ordinary append-only repairs restored `API_CONFIG_CONTRACT.md`, `CHANGELOG.md`, and `TEST_STRATEGY.md`, returning the intended ten-path scope without changing Max-Forwards behavior. CI `35468678908` and Supply Chain `35468678900` are queued.

Queued/pending checks are incomplete evidence and are not a reason for freshness-only commits or blind reruns. Parent or sibling GREEN does not transfer across these current exact heads.

## Pg-erd response lifetime and bounded-origin capacity — #39 / #42

#39 normally integrated as `511174a9842c187338b6ff885d76f9d44e183129`. It owns pg-erd Admin Config v2 response-body lifetime: one monotonic budget begins at the first non-informational upstream response header, complements per-read inactivity, and does not fabricate a second downstream status after `Session::response_written()` indicates the final response has already been committed.

#42 then completed current exact CI `35413351894`, Supply `35413351919`, and dedicated bounded-origin capacity `35413351906`, and normally integrated as `fab59139e74b0eec6988174f4a25fc31d7076633`. Its 16-VU/1600-iteration release-mode loopback measurements are a controlled regression bound only, not WAN/TLS/NUMA production SLO evidence.

## Released shutdown consumer — #70

#70 exact `df5d2f05fc5fbdd94bbfb487283bf2a6d73a55bf` already consumes crates.io `pingora = "=0.9.0"` and `pingora-prometheus = "=0.9.0"` with generated lock authority, but its lineage remains stale relative to current foundation `38db194...`.

Historical current-branch execution showed Supply, bounded-origin capacity, OCI and load GREEN while coverage-instrumented traffic failed after an old bare-TCP readiness helper produced a `ConnectionReset`. The current foundation already owns the causal repair: bounded real `/readyz` HTTP 200 readiness plus listener-reservation handoff. Do not duplicate that source in #70.

Required order remains parent-first: settle foundation/supplier roots, reconcile #44/#47 so current foundation readiness and Rust 1.98.1 are inherited, ordinary/non-force adopt only #70's released-supplier shutdown-consumer delta, then reacquire formatting/test/Clippy/rustdoc/100% coverage/load/OCI/Supply/capacity/current-range review. #73 and later release descendants remain Draft until that parent path is current.

## Downstream H1 whole-request-header lifetime — #71 / upstream #447

#71 remains the real-socket RED for a monotonic whole-request-header deadline, distinct from parser byte/count admission and relative per-read inactivity. It exercises continuous slow progress on fresh and reused H1 connections and requires one absolute acquisition bound.

Released 0.9.0 does not expose the needed supported monotonic whole-header lifetime. Upstream #447 remains the supplier path. Closure requires maintainer integration in a later release-qualified identity and, only if CWL exposes an operator control, an explicit positive versioned Admin Config transition. The unchanged #71 traffic then has to turn GREEN without origin-admission, privacy, or recovery regressions.

## Downstream H1 parser admission — #72 / upstream #993 / #1000

#72 remains the executable parser-admission RED: oversized single-field and excessive-field-count traffic must fail before application/origin admission rather than after large untrusted buffering.

Contributor #1000 is not dependency authority while open/unmerged. Closure remains `maintainer integration → release-qualified supplier identity → optional explicit positive CWL Admin Config → unchanged #72 parser/application/origin acceptance GREEN`.

## Mixed protocol: H2 downstream to H1 upstream — #53 / upstream #901 / #936/#976

#53 owns the real-wire H2→H1 Cookie normalization RED. Accepted behavior must coalesce only the H2→H1 Cookie case using exact downstream protocol knowledge while preserving ordinary hop-by-hop protections and existing policy authority.

The zero-length application-write/chunked-terminator defect is separate. Accepted behavior keeps zero-length application writes as no-ops and `finish()` as the sole chunk terminator owner, including async and cancel-safe task regressions. Full mixed-protocol release credit requires release-qualified supplier authority for each relevant root or a deployment contract that makes the affected downgrade path unreachable, followed by unchanged gateway traffic GREEN.

## Pg-erd dynamic configuration / Traefik succession — #109 / #110

Protected `pg-erd-cloud` evidence uses Traefik's file provider with `filename`, `watch=true`, and a single read-only bind-mounted `dynamic.yaml`. That does not by itself prove live in-place/rename replacement semantics are a product requirement.

#110 remains a writer-safe characterization lane behind migration ancestry. Do not implement a generic shared Pingora hot-reload subsystem merely because Traefik `watch=true` exists. If protected-source characterization and product-owner evidence establish live mutation as an operational contract, then design versioned Admin Config reload semantics with validated full snapshots, atomic generation publication, invalid-generation rejection, last-known-good/fail-closed behavior, bounded synchronization, recovery/drain/rollback, and realistic traffic evidence. Otherwise use versioned startup configuration plus controlled restart/redeployment.

## WebSocket / protocol transition — #112 / #113 / upstream #946/#947

The reusable production gateway remains intentionally fail-closed for uncharacterized HTTP/1 Upgrade. Removing a guard is not WebSocket enablement.

#113 current exact `fbad4e031927b0a92adc7244224d2bfdaba7d513` has settled the missing supplier oracle. CI `35431884205`, Supply `35431884180`, bounded-origin capacity `35431884182`, and dedicated WebSocket characterization `35431884211` all succeeded in their intended evidence semantics. The dedicated job proved the ordinary all-target graph excludes the feature-gated supplier RED, then reproduced Pingora 0.9.0's fast-`101` tunnel teardown with the bounded fingerprint `client server-frame prefix closed before the frame completed`. This is an expected-RED receipt, not WebSocket capability GREEN.

Accordingly the gap is no longer “no real-wire fast-101 oracle.” The gap is the absence of a maintainer-integrated, **release-qualified** supplier identity that makes the unchanged oracle GREEN. Open contributor code must not be pinned or copied into the gateway. #113 remains Draft because #70 is stale and must later reconcile ordinary/non-force before any supplier transition is credited.

#112 owns the future reusable transport acceptance. HTTP/1.1 WebSocket, HTTP/2 Extended CONNECT, and HTTP/3 Extended CONNECT remain separate protocol contracts. A future GREEN path must prove fast-`101` ordering under constrained CPU/concurrency, bidirectional post-handshake traffic, close/reset/half-close propagation, bounded backpressure without whole-message buffering, long-lived timeout semantics, SIGTERM/drain and health behavior, rootless/read-only OCI execution, payload-free low-cardinality observability, 100% owned-production coverage, and current exact Supply/Security/review. H2 and H3 remain separate RED→GREEN lanes.

## Brotli supplier characterization — #115

#115 exact `0afde3c2a5927a918005c999806fd477e03cf35b` has settled ordinary CI, Supply, capacity, and its dedicated strict-decoder characterization as an expected released-supplier RED. The fixture requires a standards-complete Brotli stream and byte-for-byte strict decoder success; released Pingora 0.9.0 still fails finalization. Production compression remains absent/fail-closed. A handcrafted terminal byte, lenient decoder, supplier patch copy, or opportunistic production enablement is not accepted.

## Buyer-visible release and cutover gap

`pingora-gateway` still has no GitHub Release inventory. A release-ready exact protected head must establish, without predecessor transfer:

- version, CHANGELOG, tag/package, and an immutable gateway release/artifact;
- SBOM, authenticated provenance, reproducibility, source/binary/image identity, and rollback evidence;
- non-root/read-only runtime plus supply-chain/security gates;
- TLS, HTTP/1.1, HTTP/2 and applicable HTTP/3 behavior;
- WebSocket/streaming only where an actual migrated product requires it and only after release-qualified supplier/transport acceptance;
- timeout/retry/backpressure, header/cookie/client-IP/body-limit, health/drain, and failure traffic;
- exact-head owned production rustdoc, test, and edge-case coverage contracts;
- realistic concurrency/load with applicable buyer-path p95 `<= 20 ms`, no artificial warm-up or sample removal, and representative NUMA evidence where the claim depends on it;
- parity → shadow/canary → observed rollback → cutover → verified legacy Nginx/OpenResty removal for each responsibility actually migrated.

Repository Pages publication, contributor CI, loopback p95, mutable supplier metadata, or disappearance of Nginx/OpenResty strings is never sufficient by itself.

## Current causal order

The dependency-rooted order is:

`foundation #1 current Security RED established + CodeQL settlement → maintainer-integrated/release-qualified #889 repair → #54/#62 ordinary reconciliation and absence/semantics/security revalidation on current compiler/foundation ancestry → foundation-equivalent full CI/Supply/SAST/Security/CodeQL/review GREEN → remaining owner prerequisites including #31/#61/#104 as applicable → normal protected foundation integration → migration ancestry and exact consumer evidence → ordinary release-line foundation inheritance through #44/#47 → #70 exact revalidation → protocol supplier roots consumed only when maintainer-integrated, release-qualified and required by the promoted path → #112 versioned WebSocket transport acceptance only for a real consumer requirement → immutable gateway release/provenance/reproducibility/rollback → representative parity/shadow/canary → cutover → verified legacy removal`.

Independent traffic children move only after their direct parents are current and must reacquire their own exact evidence after any head change. Queued/in-progress checks remain incomplete evidence. A failed check is RCA/fix/rerun work, not a reason to weaken the gate. If a supplier or concurrent writer moves, reread and adopt/adapt the intervening delta instead of treating movement as a race or force-restacking it.
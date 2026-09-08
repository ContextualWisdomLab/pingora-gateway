# Product / Technical Gap Baseline

This file is the durable, code-current migration baseline for `ContextualWisdomLab/pingora-gateway`. Live protected refs, pull requests, issues, review state, workflow/security runs, releases, supplier refs, and consumer deployment evidence remain the exact authority. Historical RED/GREEN detail belongs in commits, PRs, workflow receipts, ADRs, `CHANGELOG.md`, and focused traceability documents; mutable current-head SHAs and run IDs are deliberately not copied here because an evidence-refresh commit would immediately stale its own statement.

## Authority and bounded contexts

`pingora-gateway` owns reusable edge/runtime behavior only: Ingress, Edge Routing, TLS consumption, HTTP Policy, Load Balancing mechanics, Observability, Admin Config, and Runtime Isolation. Product authentication, authorization, tenancy, and business routing remain product-owned. Keyverse remains identity authority; Wardnet and EgressWeave retain their respective security/policy authority. The gateway consumes released/versioned contracts or explicit operator transport inputs and does not copy sibling source, issue cross-service application SQL, or depend on mutable sibling PR heads.

Generic v1 remains a one-upstream Rust/Pingora process. The bounded `cwl-pingora-pg-erd-migration` composition root exists only for the characterized `pg-erd-cloud` edge contract; it is not a general product-routing DSL. `PgErdMigrationConfig` admits only listener/runtime budgets and characterized `backend`/`frontend` transport authority. Routes and response policy remain compiled migration contracts. Pg-erd config version 2 adds only the explicit upstream response-body progress lifetime; version 1 and generic v1 do not silently inherit that semantic.

## Promotion root

Foundation #1 and compiler prerequisite #56 remain the protected-promotion root. #56 has terminal exact-head CI/Supply Chain evidence for Rust 1.98.1 but still requires the organization-required independent `APPROVED` review; owner, bot, or model technical comments are not promoted into that governance credit.

Supplier-intake #54 remains an intentional hosted RED because the committed production graph contains unmaintained `derivative 2.2.0` / `RUSTSEC-2024-0388`. Audit ignores, deleted lock evidence, mutable supplier pins, scanner suppression, muted regressions, and known-incompatible downgrades are not repairs. Supplier-semantics #62 remains the independent GREEN characterization for the required `PeerOptions` non-hook Debug surface, `Backend` equality/hash/order semantics, and bounded Rust-origin load contract.

The accepted sequence remains: maintainer-integrated, release-qualified immutable Pingora repair removing `derivative` from the resolved production graph while preserving #62 semantics → gateway supplier bump and committed lock regeneration → unchanged #54 absence regression GREEN plus #62 revalidation → #56 independent approval and then-live protected-path governance → protected foundation integration.

## Retained pg-erd migration stack

Every retained child was repaired by ordinary/non-force ancestry adoption when its parent moved. A parent receipt never transfers across a changed child head. The durable contracts are:

| Slice | Durable contract |
| --- | --- |
| #12 | Strict bounded pg-erd Admin Config and dedicated composition root; public activation revalidates deserialized config before listener creation. |
| #14 | Shared listener/socket-authority invariants, including port-zero rejection, wildcard/dual-stack alias handling, and hardened dual-profile OCI packaging. |
| #15 | Complete generic forwarding-identity sanitization without claiming trusted-client identity. |
| #16 | Real pg-erd body-limit and in-flight backpressure traffic with readiness, telemetry, and recovery. |
| #17 | Deterministic refused-backend failure/recovery. |
| #18 | Connected-but-silent upstream read-inactivity failure/recovery; `read_ms` remains a per-read inactivity budget, not a whole-response lifetime. |
| #19 | One admitted process identity per OCI image/profile with pg-erd metrics identity exercised under non-root, read-only, capability-free runtime constraints. The historical package-both-binaries approach is obsolete. |
| #20 | Routed in-flight SIGTERM drain acceptance within the shared external termination budget. |
| #21 | Post-header partial-response behavior: preserve committed 200/framing, terminate incomplete body, avoid fabricated second status/failover, and recover. |
| #22 | Routed pg-erd four-VU/400-request Rust-origin latency regression with exact route/body identity, zero failures, per-route sample floors, and p95 `<20 ms`. |
| #23 | Compiled payload-free shared-observability acceptance with request-sensitive sentinels reaching the origin but not shared stderr. |
| #24 | Linux pre-header upstream TCP RST characterization with exact status/metric oracles and bounded fixture I/O. |
| #25 | Linux post-commit TCP RST characterization preserving the already-committed response boundary. |
| #27 | Shared prevention of recursive upstream authority into gateway traffic or metrics listeners. |
| #29 | HTTP/1 protocol-transition admission fails closed with 501 before application admission/route/upstream selection; the finite no-origin observation is not overstated. |
| #31 | Process-wide Pingora-family diagnostic payload minimization while preserving operator-selected log levels/targets. |
| #33 | Immutable Pingora peers use `HttpUpstreamRequestPolicy::deny_upgrades()` as defense in depth under the request-level protocol-transition guard. |
| #37 | Dedicated pg-erd upstream local-CA/SNI success and hostname-mismatch fail-closed traffic; downstream TLS remains separate. |
| #39 | Pg-erd v2 monotonic upstream response-body progress lifetime starting at the first non-informational response header; v1/generic semantics remain unchanged. |
| #42 | Distinct bounded-origin capacity lane: one-worker serialization self-check, then four workers per backend/frontend origin, 16 VUs / 1600 iterations, route sample floors, zero failures, and aggregate/backend/frontend p95 `<20 ms` under a Rust 1.98.1-verified release build. |
| #44 | Documentation-only projection of three distinct supplier-bound header controls: #43/#993 downstream H1 parser byte/count admission, #45/#447 downstream H1 whole-request-header lifetime, and #40/#992 upstream incomplete response-header lifetime. Final #44 independently closed exact-head CI, Supply Chain, bounded-origin capacity, and current-range technical evidence before successor movement. |

Controlled loopback results are regression evidence, not WAN/TLS/multi-hop/container-orchestrator or production SLO proof. Product auth/business semantics, Keyverse identity, and Wardnet/EgressWeave policy never move into these slices.

## #47 parked-read graceful-shutdown supplier projection

#47 is a documentation-only successor to final #44. It keeps issue #46 separate from the three header controls above and from #20's already-admitted in-flight drain proof.

At the pinned public Pingora revision, each `HttpProxy` instance owns one shared `tokio::sync::Notify` for the HTTP/1 reads handled by that instance while they are parked waiting for a request. Upstream #844 reports 65.66% combined off-CPU futex wait on a 128-core NUMA host and is `Accepted` only into Cloudflare's internal line; that status is not a public immutable release. Public PR #969 remains open/unmerged. Independent supplier-path review reproduces a one-shot lost-wakeup window on public main when shutdown occurs after `read_request()` is pending but before the waiter is registered. The contributor branch is evidence only and is not consumed, pinned, or vendored by CWL.

Issue #46 therefore requires a maintainer-integrated immutable/released supplier capability or a separately governed provenance-bound backport before Runtime Isolation can claim closure. RED/GREEN acceptance must repeatedly jitter SIGTERM across waiter registration, exercise realistic configured service/runtime worker topology and many parked/reused HTTP/1 keep-alive connections, measure shutdown wall time and CPU/off-CPU contention where available, require zero parked-read survivors, and preserve #20's admitted in-flight completion semantics. Shorter grace, fewer workers/connections, disabling keep-alive, callback workarounds, or copied supplier proxy internals cannot manufacture GREEN. ADR 0011 remains `Proposed` until that release-bound evidence exists.

Any #47 documentation movement must independently reacquire exact-head CI, Supply Chain, applicable capacity/evidence lanes, and current-range technical review. Final #44 receipts do not transfer to a changed #47 head.

## Capability state and buyer-visible gaps

| Area | Current durable state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Bounded pg-erd config plus shared listener/upstream authority invariants are characterized and tested on retained final heads. | Revalidate every changed descendant/release candidate; do not turn migration config into product routing. |
| Request-header admission / lifetime | Supplier has finite fixed H1 parser ceilings and per-read inactivity, but CWL has neither operator-controlled parser byte/count admission nor a monotonic whole-request-header deadline. | #43/#993 parser admission and #45/#447 downstream whole-header lifetime; keep both distinct from #40/#992 upstream response-header lifetime and from H2 decoded-header accounting. |
| Runtime isolation / recovery | Body/in-flight rejection, refused/read-stall origins, partial/reset phases, routed drain, TLS identity failure, and pg-erd v2 response-body progress lifetime are retained contracts. | #46 parked-read shutdown correctness/scaling, upstream/downstream incomplete-header lifetime, broader admitted long-lived streaming, and deployment rollback traffic. |
| Upstream TLS | Generic and pg-erd trust/SNI behavior is characterized. | Representative TLS/network performance and downstream TLS termination remain unproven; certificate issuance/rotation stays outside gateway authority. |
| Protocol scope | Ordinary HTTP/1 is admitted; HTTP/1 Upgrade is denied both at admission and immutable peer policy. | Downstream TLS/H2, H2→H1 Cookie/body-framing prerequisites, a separately versioned WebSocket/Extended CONNECT contract if a consumer proves it is required, and explicit H3/QUIC disposition. |
| OCI / supply chain | Dual profiles, non-root/read-only/capability-free runtime, dependency audit, SPDX SBOM, image scans, and exact-source binding exist on retained exact heads. | Immutable registry/package identity, signing/attestation, release-bound SBOM/provenance/reproducibility, supplier repair, and protected release. |
| Performance | Generic, routed pg-erd, and bounded-origin local regressions preserve p95 `<20 ms` without sample reduction or application-route warm-up. | Representative production origin capacity, TLS/network/multi-hop, scheduler/container/k8s measurements, and buyer-visible deployment SLO evidence. |
| Observability | Shared request telemetry and Pingora dependency diagnostics are payload-minimized; failure/backpressure counters are characterized. | Distributed tracing and product logging remain separate owner concerns; revalidate data-minimization on every changed release candidate. |
| Graceful drain | Already-admitted generic and routed pg-erd requests have explicit drain evidence. | #46 must prove prompt/race-safe parked-read shutdown and realistic many-core scaling on an admissible immutable supplier identity. |
| Rollback / cutover | Documented only. | Immutable protected artifact → parity → shadow/canary → rollback rehearsal → protected cutover → verified Nginx/OpenResty/legacy removal. |

## Execution order

The current dependency order is `#54 derivative RED + #62 exact semantics/load GREEN → maintainer-integrated release-qualified immutable supplier repair → gateway supplier bump and committed lock regeneration → #54 GREEN + #62 revalidation → #56 independent APPROVED/governance → protected foundation integration → retained pg-erd stack revalidated on the promoted ancestry → #44 header-control projection → #47 parked-read shutdown projection → remaining downstream TLS/H2/H2→H1/WebSocket/H3-QUIC and representative deployment acceptance → immutable gateway version/tag/package/image + release-bound SBOM/provenance/reproducibility → shadow/canary → rollback rehearsal → cutover → verified legacy edge removal`.

Draft state, predecessor receipts, bot/model review, local image IDs, mutable supplier PRs, queued jobs, controlled loopback measurements, and routine administrator bypass are never treated as protected merge, immutable release, canary, cutover, rollback, or legacy-removal evidence.

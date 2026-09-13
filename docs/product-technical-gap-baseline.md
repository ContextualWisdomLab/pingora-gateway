# Product / Technical Gap Baseline

This document is the code-current commercial baseline for `ContextualWisdomLab/pingora-gateway` as of 2026-09-10 KST. It records durable product/technical gaps, responsibility boundaries, acceptance contracts, and promotion order. Mutable PR heads, workflow run IDs, artifact digests, and protected-branch tips are operational evidence snapshots; issue #58 and the owning PR/Issue remain the live exact-evidence ledger and supersede stale identities here.

## Product and DDD responsibility boundary

`pingora-gateway` is a Supporting/Generic edge-runtime subdomain. It owns reusable Ingress, Edge Routing, TLS transport policy, HTTP Policy, Load Balancing, Observability, Admin Config, and Runtime Isolation behavior only where those concerns are genuinely shared edge responsibility.

The Edge Contract bounded context owns admission of network authority. `GatewayConfig` is the aggregate root; listener authority, upstream authority, TLS identity, request-body limits, and I/O budgets are invariants admitted before network activation. Pingora-specific types remain delivery details and must not cross into the Edge Contract. `GatewayCommand` is the startup application service and Pingora Delivery is the anti-corruption adapter.

The gateway does **not** own product authentication/authorization, tenant or business routing, Keyverse identity authority, Wardnet/EgressWeave policy authority, certificate issuance/private-key custody, static-site product behavior, or application-specific semantics. Those authorities remain with their canonical owners and are consumed only through released/versioned contracts or explicit ACLs. Source copies, cross-service application SQL, mutable sibling-PR dependencies, and hidden Shared Kernels are rejected.

Legacy Nginx/OpenResty presence is not itself a migration trigger. Static-file serving, FastCGI, certificate issuance/key custody, product authentication, or domain-specific routing stay with the correct owner unless a separately evidenced shared-edge responsibility exists.

## Evidence and promotion invariants

A change is promoted by exact behavior, not by a release name, mutable contributor branch, or documentation claim. The normal sequence is `realistic RED → minimal causal repair at the correct owner → exact-head GREEN → maintainer/protected integration → release-qualified dependency identity → consumer pin/lock transition → unchanged consumer GREEN → protected integration → release/cutover evidence`.

Predecessor execution and review do not transfer across a new exact head. Draft/Proposed work is not accepted merely because a bot or candidate branch is GREEN. Do not force-push, destructively rebase, self-approve, weaken required checks, suppress an advisory, reduce samples/concurrency, warm caches artificially, hand-edit a generated lockfile, or pin a mutable supplier PR to manufacture promotion evidence.

Gateway performance/security acceptance must use the compiled Rust path and realistic traffic. Controlled loopback evidence may satisfy a component regression gate but is not represented as Internet/TLS/WAN production SLO evidence. Where the buyer path is applicable, p95 must remain `<= 20 ms` without sample reduction or measurement exclusion.

## Current supplier baseline

Protected public `cloudflare/pingora/main` is on the 0.9.0 release line. Pingora 0.9.0 is published and the direct crates.io packages needed by the gateway (`pingora 0.9.0`, `pingora-prometheus 0.9.0`) exist, so package availability is no longer a blocker by itself.

That release does **not** close every CWL edge gap. Released 0.9.0 contains the graceful-shutdown lost-wakeup repair, but it still lacks the configurable downstream H1 parser-admission capability required by #43/#72, still lacks a supported monotonic whole-request-header lifetime required by #45/#71, and still contains `derivative 2.2.0` in the relevant supplier graph. The mixed-protocol Cookie and zero-length chunk-framing roots also remain outside released authority.

The GitHub Release object is mutable metadata and is not used as the dependency identity when crates.io publication is available. Consumer promotion requires Cargo-resolved registry source/checksum evidence in a generated lockfile plus unchanged gateway acceptance.

## Foundation compiler/governance root — #56

Ready #56 remains the compiler/release-path prerequisite. Its Rust 1.98.1 exact-head CI and Supply Chain evidence are GREEN, including formatting, compile/test, strict lint, public rustdoc, owned-production coverage, resolved-lock verification, non-root/read-only OCI execution, load contract, SBOM, image scan, and exact-source binding.

The remaining #56 blocker is governance: the required independent `APPROVED` review is still absent. Technical bot/static evidence is not substituted for that approval, and administrator bypass/self-approval is not used.

## Supply-chain intake — #54 and #62 / upstream #889

Ready #54 is the deliberate committed-lock RED for `RUSTSEC-2024-0388`. The finding is that `derivative 2.2.0` remains in the resolved graph; the advisory is an unmaintained-package advisory, not a claimed memory-safety CVE. Audit ignores, scanner suppression, deletion of lock evidence, or muted tests are invalid fixes.

Ready #62 is the independent supplier-semantics control. It preserves the required non-hook `PeerOptions` Debug value surface, the address+weight `Backend` equality/hash/order semantics with opaque extensions excluded, and the bounded Rust-origin load/runtime/Supply-Chain contract. Its current old-pin evidence is GREEN but must be re-run after any supplier transition; it does not substitute for #54.

Pingora 0.9.0 still contains `derivative 2.2.0`, and upstream #889 remains the owner path. Commercial closure requires maintainer-integrated removal from the relevant workspace/core/load-balancing production graph, regenerated supplier lock, preservation of #62 semantics, supplier fmt/tests/Clippy/rustdoc/audit, a later release-qualified identity containing the removal, then an ordinary gateway pin/lock regeneration followed by unchanged #54 GREEN and #62 revalidation.

## Runtime Isolation / graceful shutdown — #70

Draft #70 is the executable downstream shutdown RED against the characterized older Pingora revision. Its generic and pg-erd post-notification keep-alive cases remain alive beyond the one-second evidence bound while the ordinary admitted-work drain tests pass. This isolates the supplier lost-wakeup behavior from normal graceful drain. Same-head load/OCI/Supply Chain/bounded-origin lanes are independently GREEN; their loopback latency evidence is component evidence only.

Released Pingora 0.9.0 contains the maintainer-integrated shutdown successor, so historical upstream #844/#969 is provenance rather than a current integration prerequisite. The next consumer step is **not** another supplier patch: change the two direct Pingora dependencies to exact crates.io `=0.9.0`, let Cargo generate the corresponding `Cargo.lock` with registry source/checksum authority, then rerun the unchanged #70 traffic contract. A manifest-only transition or hand-authored lock is invalid.

Correctness closure requires the unchanged post-notification generic+pg-erd assertions to turn GREEN while admitted-work drain remains GREEN. Configured-worker/NUMA contention is a separate performance closure and is not inferred from functional shutdown success.

## Downstream H1 whole-request-header lifetime — #71 / upstream #447

Draft #71 is a real-socket RED for a monotonic whole-request-header deadline, separate from parser byte/count admission and from ordinary per-read inactivity. It sends incomplete header progress every 100 ms, so the connection continually makes sub-timeout progress, yet requires the whole header acquisition to terminate within the declared test budget on both a fresh H1 connection and a sequentially reused keep-alive connection.

Released Pingora 0.9.0 still applies relative per-read timeout semantics and exposes no supported monotonic whole-header lifetime. Upstream #447 remains the canonical supplier owner. Closure requires a maintainer-integrated capability in a later release-qualified supplier identity and, only if CWL exposes an operator control, an explicit positive versioned Admin Config transition. The unchanged #71 fresh+reused traffic must then turn GREEN without origin admission or privacy/recovery regressions.

## Downstream H1 parser admission — #72 / upstream #993 and #1000

Draft #72 is the executable real-socket parser-admission RED. A single-large-field request exceeding the byte budget and a many-small-fields request exceeding the field-count budget both reach the production `ProxyHttp` lifecycle instead of failing before application/origin admission. Same-head load, OCI, Supply Chain, and bounded-origin capacity are independently GREEN.

Released Pingora 0.9.0 predates this capability and therefore does not close #72. Current contributor #1000 has materially advanced on the 0.9.0 main line. Exact candidate `6a90c79b61fbbc70b518709de6802165668cba2c` repairs all seven CWL findings accumulated across the earlier candidate lineage at **mutable-candidate scope**:

1. configured byte limits reject zero and values above the legacy `MAX_HEADER_SIZE` ceiling;
2. configured header-count limits reject zero and values above `MAX_HEADERS` through supported configuration/mutation paths;
3. a prebuffered/pipelined current header is measured independently of a large suffix, the suffix is preserved, and a genuinely oversized current header is still rejected;
4. public `ServerSession` and `HttpSession` setters are fallible and share the same zero/upper-bound validation;
5. the actual `HttpServerApp::process_new()` activation path validates options before serving and propagates setter errors for new and reused H1 sessions;
6. the underlying read is bounded by the remaining configured header-byte budget with `take(remaining as u64)` rather than merely rejecting after an over-read;
7. an exact-budget `Partial` header is rejected before another socket read while exact-budget `Complete(s)` remains accepted.

The candidate includes regressions for exact-limit complete success, exact-limit incomplete immediate rejection/no-extra-read, remaining-budget multi-read rejection, exact-limit multi-read success, direct setter bounds, and pipelined suffix preservation. Exact Semgrep and upstream build are now GREEN; the full Rust 1.97.1 and nightly lanes pass fmt/check/test/doc-test/clippy/audit/machete and the reduced Rust 1.85.0 lane passes its configured checks. Exact-current technical COMMENT review reports no new actionable semantic/resource-bound defect in the four-file candidate range, but neither candidate execution nor COMMENT review is maintainer approval, protected integration, or release authority.

The next edge is `maintainer review/integration of the exact-head GREEN candidate → later release-qualified supplier identity containing the capability → optional explicit positive versioned CWL Admin Config → unchanged #72 parser/application/origin GREEN`. CWL must not pin the mutable contributor head or substitute a callback-only 431 response for parser admission.

## Mixed protocol: H2 downstream to H1 upstream — #53 / upstream #901 and #936

Draft #53 is the real-wire Cookie normalization RED. It negotiates actual downstream TLS/H2, sends distinct H2 Cookie field values, retains H1 upstream transport, and requires the raw H1 origin to observe one `Cookie` field joined with exact `b"; "`. The released 0.9.0 sanitizer still lacks this H2-to-H1 Cookie coalescing behavior.

Contributor #901 remains open but is not release authority. The accepted repair must remain narrow: preserve normal hop-by-hop/Connection protections, use exact downstream protocol knowledge, coalesce only the H2→H1 Cookie case, and keep any compatibility opt-out in the existing request-policy authority rather than creating a competing product-policy aggregate.

The zero-length application-write/chunked-terminator defect is a separate mixed-protocol root. Contributor #936 remains open and is not release authority. The accepted behavior keeps zero-length application writes as no-ops and `finish()` as the sole chunk terminator owner, with async and cancel-safe task regressions. Full mixed-protocol release credit requires both roots to be resolved in release-qualified supplier authority and the unchanged real-wire gateway contracts to turn GREEN.

## pg-erd and consumer succession boundary

The surviving pg-erd stack remains downstream of the shared foundation/supplier roots. Its Admin Config/network-authority work must preserve the Edge Contract boundary, including rejection of overlapping socket authority such as wildcard/concrete same-port conflicts and IPv4-mapped IPv6 aliases where they represent the same authority. Product authentication/business routing stays outside the gateway.

Historical PRs are closed only after every valid production/test/fixture/contract/evidence delta has been verified in a successor. A generic Rust-origin or common edge delta does not by itself prove succession of routed pg-erd traffic. Current PR bodies and #58 are authority for exact stack heads and hosted execution state.

## Buyer-visible release and cutover gap

`pingora-gateway` remains an implemented candidate, not a released edge product. Protected gateway `main` is outside the dependency-ordered Draft/Ready stack and the gateway GitHub Releases collection remains empty at this baseline update.

Release-ready exact protected head must prove, without predecessor transfer:

- version, CHANGELOG, tag/package and an immutable gateway artifact;
- SBOM, provenance, reproducibility and rollback evidence;
- non-root/read-only runtime and supply-chain/security gates;
- TLS, HTTP/1.1, HTTP/2 and applicable HTTP/3 behavior;
- WebSocket/streaming where product scope requires it;
- timeout/retry/backpressure, header/cookie/client-IP/body-limit, health/drain and failure traffic;
- exact-head owned production rustdoc, test and edge-case coverage requirements;
- realistic concurrency/load with applicable buyer-path p95 `<= 20 ms` and no artificial warm-up/sample reduction;
- parity → shadow/canary → observed rollback → cutover → verified legacy Nginx/OpenResty removal for each actual migrated responsibility.

No Pingora release, mutable contributor CI result, loopback p95, or disappearance of Nginx strings is sufficient by itself.

## Current causal order

The dependency-rooted order is:

`#56 independent approval/governance + supplier roots closed on release-qualified source (#889 derivative, #447 whole-header lifetime, #993 parser admission, H2 Cookie normalization, zero-length chunk framing; shutdown already exists in released 0.9.0 and is at the consumer bump boundary) → ordinary gateway registry pin/lock regeneration → unchanged #70/#71/#72/#53 and #54/#62 contracts re-run on exact identities → current pg-erd/protocol ancestry repaired non-force → protected integration → immutable gateway release/SBOM/provenance/reproducibility/rollback → parity/shadow/canary → cutover → verified legacy removal`.

Queued/in-progress checks remain incomplete evidence. A failed check is RCA/fix/rerun work, not a reason to weaken the gate. If a supplier candidate moves, reread and adapt to the intervening delta rather than treating the movement as a race or force-restacking it.
